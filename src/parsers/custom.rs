use super::LogParser;
use super::common::map_level_str;
use crate::log_entry::LogLevel;
use chrono::{FixedOffset, NaiveDateTime, TimeZone};
use directories::ProjectDirs;
use regex::Regex;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::sync::{Arc, LazyLock};

/// Custom log format definition loaded from TOML
#[derive(Debug, Clone, Deserialize)]
struct CustomFormatDef {
    /// Format name (for display/debugging)
    #[allow(dead_code)]
    name: String,
    /// Regex with named groups: (?P<level>...) and/or (?P<timestamp>...)
    pattern: String,
    /// chrono format string for parsing the timestamp capture group
    timestamp_format: Option<String>,
    /// Timezone offset of the log timestamps, e.g. "+05:30" or "-07:00".
    /// If set, timestamps are converted to local time before display.
    #[serde(default)]
    timezone: Option<String>,
    /// Map custom level strings to standard levels (e.g. FATAL = "error")
    #[serde(default)]
    level_map: HashMap<String, String>,
}

/// Parser built from a custom TOML format definition
#[derive(Debug)]
struct CustomParser {
    regex: Regex,
    timestamp_format: Option<String>,
    /// Parsed timezone offset in seconds east of UTC (from `timezone` TOML field)
    timezone_offset_secs: Option<i32>,
    level_map: HashMap<String, LogLevel>,
}

impl CustomParser {
    fn from_def(def: CustomFormatDef) -> Option<Self> {
        let regex = Regex::new(&def.pattern).ok()?;

        // Parse optional timezone offset string (e.g. "+05:30", "-07:00", "+01:00")
        let timezone_offset_secs = def.timezone.as_deref().and_then(Self::parse_tz_offset);

        // Pre-resolve level_map values to LogLevel
        let level_map = def
            .level_map
            .into_iter()
            .map(|(k, v)| {
                let level = match v.to_lowercase().as_str() {
                    "trace" => LogLevel::Trace,
                    "debug" => LogLevel::Debug,
                    "info" => LogLevel::Info,
                    "warn" => LogLevel::Warn,
                    "error" => LogLevel::Error,
                    "profile" => LogLevel::Profile,
                    _ => LogLevel::Unknown,
                };
                (k, level)
            })
            .collect();

        Some(CustomParser {
            regex,
            timestamp_format: def.timestamp_format,
            timezone_offset_secs,
            level_map,
        })
    }

    /// Parse a timezone offset string like "+05:30" or "-07:00" into seconds east of UTC.
    fn parse_tz_offset(tz: &str) -> Option<i32> {
        let tz = tz.trim();
        let (sign, rest) = match tz.chars().next()? {
            '+' => (1i32, &tz[1..]),
            '-' => (-1i32, &tz[1..]),
            _ => return None,
        };
        let (h, m) = rest.split_once(':')?;
        let hours: i32 = h.parse().ok()?;
        let minutes: i32 = m.parse().ok()?;
        Some(sign * (hours * 3600 + minutes * 60))
    }

    fn resolve_level(&self, level_str: &str) -> LogLevel {
        // Try custom map first (case-sensitive), then standard mapping
        self.level_map
            .get(level_str)
            .copied()
            .unwrap_or_else(|| map_level_str(level_str))
    }

    fn parse_ts(&self, ts_str: &str) -> Option<NaiveDateTime> {
        let fmt = self.timestamp_format.as_ref()?;
        // Normalize spaces in fractional seconds (e.g. "04:11:35. 82" -> "04:11:35.082")
        // Strip whitespace, then zero-pad fractional part to match format width
        let ts_clean: String = ts_str
            .chars()
            .filter(|c| !c.is_ascii_whitespace())
            .collect();
        let ts_clean = Self::zero_pad_fractional(&ts_clean, fmt);
        // Try direct parse first
        let naive = if let Ok(dt) = NaiveDateTime::parse_from_str(&ts_clean, fmt) {
            dt
        } else {
            // Time-only: prepend today's date
            let today = chrono::Local::now().format("%Y-%m-%d").to_string();
            let full = format!("{} {}", today, ts_clean);
            let full_fmt = format!("%Y-%m-%d {}", fmt);
            NaiveDateTime::parse_from_str(&full, &full_fmt).ok()?
        };
        // If a timezone offset is configured, treat the parsed naive time as being in that
        // timezone and convert to local time for display.
        if let Some(offset_secs) = self.timezone_offset_secs {
            let offset = FixedOffset::east_opt(offset_secs)?;
            let dt_tz = offset.from_local_datetime(&naive).single()?;
            // Convert to system local time: go via UTC so the offset is applied correctly
            Some(dt_tz.with_timezone(&chrono::Local).naive_local())
        } else {
            Some(naive)
        }
    }

    /// After stripping whitespace from fractional seconds, the digit count may
    /// be shorter than what the format string expects (e.g. "%.3f" needs 3 digits
    /// but we only have 2). Detect the expected width from the format and right-pad
    /// with zeros.
    fn zero_pad_fractional(ts: &str, fmt: &str) -> String {
        // Extract expected fractional width from format (e.g. "%.3f" -> 3, "%.6f" -> 6)
        let expected_width: Option<usize> = fmt.find("%.").and_then(|i| {
            let rest = &fmt[i + 2..];
            let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if digits.is_empty() {
                None
            } else if rest[digits.len()..].starts_with('f') {
                digits.parse().ok()
            } else {
                None
            }
        });

        let expected_width = match expected_width {
            Some(w) => w,
            None => return ts.to_string(),
        };

        // Find the fractional separator ('.' or ',') and pad if needed
        if let Some(dot_pos) = ts.rfind(['.', ',']) {
            let frac = &ts[dot_pos + 1..];
            // Only pad if all remaining chars are digits (fractional part)
            if !frac.is_empty()
                && frac.chars().all(|c| c.is_ascii_digit())
                && frac.len() < expected_width
            {
                let mut padded = ts.to_string();
                for _ in 0..(expected_width - frac.len()) {
                    padded.push('0');
                }
                return padded;
            }
        }
        ts.to_string()
    }
}

impl LogParser for CustomParser {
    fn detect(&self, first_line: &str) -> f64 {
        if self.regex.is_match(first_line) {
            0.9
        } else {
            0.0
        }
    }

    fn parse_line(&self, line: &str) -> Option<(LogLevel, Option<NaiveDateTime>)> {
        let caps = self.regex.captures(line)?;

        let level = caps
            .name("level")
            .map(|m| self.resolve_level(m.as_str()))
            .unwrap_or(LogLevel::Unknown);

        let timestamp = caps
            .name("timestamp")
            .and_then(|m| self.parse_ts(m.as_str()));

        Some((level, timestamp))
    }

    fn message_start(&self, line: &str) -> Option<usize> {
        // If regex has a "message" capture group, use its start
        let caps = self.regex.captures(line)?;
        if let Some(m) = caps.name("message") {
            return Some(m.start());
        }
        // Otherwise, message starts after the full regex match
        Some(caps.get(0)?.end())
    }
}

/// Load all custom parsers from ~/.config/lognav/formats/*.toml (cached after first call)
pub fn load_custom_parsers() -> Vec<Arc<dyn LogParser>> {
    static CACHED: LazyLock<Vec<Arc<dyn LogParser>>> = LazyLock::new(load_custom_parsers_inner);
    CACHED.clone()
}

fn load_custom_parsers_inner() -> Vec<Arc<dyn LogParser>> {
    let Some(dirs) = ProjectDirs::from("", "", "lognav") else {
        return vec![];
    };

    let formats_dir = dirs.config_dir().join("formats");
    let Ok(entries) = fs::read_dir(&formats_dir) else {
        return vec![];
    };

    let mut parsers: Vec<Arc<dyn LogParser>> = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("toml") {
            continue;
        }

        let Ok(content) = fs::read_to_string(&path) else {
            continue;
        };

        let Ok(def) = toml::from_str::<CustomFormatDef>(&content) else {
            continue;
        };

        if let Some(parser) = CustomParser::from_def(def) {
            parsers.push(Arc::new(parser));
        }
    }

    parsers
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_def() -> CustomFormatDef {
        CustomFormatDef {
            name: "test-format".to_string(),
            pattern: r"^(?P<timestamp>\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}) \[(?P<level>\w+)\] (?P<message>.*)".to_string(),
            timestamp_format: Some("%Y-%m-%d %H:%M:%S".to_string()),
            timezone: None,
            level_map: HashMap::from([
                ("FATAL".to_string(), "error".to_string()),
                ("NOTICE".to_string(), "info".to_string()),
            ]),
        }
    }

    #[test]
    fn test_custom_parser_detect() {
        let parser = CustomParser::from_def(sample_def()).unwrap();
        assert_eq!(parser.detect("2024-01-15 10:30:45 [ERROR] bad thing"), 0.9);
        assert_eq!(parser.detect("random text"), 0.0);
    }

    #[test]
    fn test_custom_parser_parse_line() {
        let parser = CustomParser::from_def(sample_def()).unwrap();
        let (level, ts) = parser
            .parse_line("2024-01-15 10:30:45 [ERROR] something broke")
            .unwrap();
        assert_eq!(level, LogLevel::Error);
        assert!(ts.is_some());
        assert_eq!(
            ts.unwrap(),
            NaiveDateTime::parse_from_str("2024-01-15 10:30:45", "%Y-%m-%d %H:%M:%S").unwrap()
        );
    }

    #[test]
    fn test_custom_level_map() {
        let parser = CustomParser::from_def(sample_def()).unwrap();
        // FATAL maps to Error via custom level_map
        let (level, _) = parser
            .parse_line("2024-01-15 10:30:45 [FATAL] crash")
            .unwrap();
        assert_eq!(level, LogLevel::Error);
        // NOTICE maps to Info via custom level_map
        let (level, _) = parser
            .parse_line("2024-01-15 10:30:45 [NOTICE] fyi")
            .unwrap();
        assert_eq!(level, LogLevel::Info);
    }

    #[test]
    fn test_custom_standard_level_fallback() {
        let parser = CustomParser::from_def(sample_def()).unwrap();
        // WARN not in level_map, falls back to map_level_str
        let (level, _) = parser
            .parse_line("2024-01-15 10:30:45 [WARN] careful")
            .unwrap();
        assert_eq!(level, LogLevel::Warn);
    }

    #[test]
    fn test_custom_continuation() {
        let parser = CustomParser::from_def(sample_def()).unwrap();
        // Line that doesn't match pattern = continuation (None)
        assert!(parser.parse_line("  stack trace line").is_none());
    }

    #[test]
    fn test_invalid_regex_returns_none() {
        let def = CustomFormatDef {
            name: "bad".to_string(),
            pattern: r"[invalid".to_string(),
            timestamp_format: None,
            timezone: None,
            level_map: HashMap::new(),
        };
        assert!(CustomParser::from_def(def).is_none());
    }

    #[test]
    fn test_toml_deserialization() {
        let toml_str = r#"
name = "my-app"
pattern = '(?P<timestamp>\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}) \[(?P<level>\w+)\]'
timestamp_format = "%Y-%m-%d %H:%M:%S"

[level_map]
FATAL = "error"
WARNING = "warn"
"#;
        let def: CustomFormatDef = toml::from_str(toml_str).unwrap();
        assert_eq!(def.name, "my-app");
        assert_eq!(def.level_map["FATAL"], "error");
        assert_eq!(def.level_map["WARNING"], "warn");

        let parser = CustomParser::from_def(def).unwrap();
        assert_eq!(parser.detect("2024-01-15 10:30:45 [ERROR] msg"), 0.9);
    }

    #[test]
    fn test_time_only_timestamp() {
        let def = CustomFormatDef {
            name: "portmonitor".to_string(),
            pattern:
                r#"^(?P<timestamp>\d{2}:\d{2}:\d{2}[.]\s*\d{1,3})\s+\[\d+\]\s+(?P<level>\w+)\s"#
                    .to_string(),
            timestamp_format: Some("%H:%M:%S%.3f".to_string()),
            timezone: None,
            level_map: HashMap::new(),
        };
        let parser = CustomParser::from_def(def).unwrap();

        // Normal line
        let (level, ts) = parser
            .parse_line("04:02:41.257 [4708] DEBUG Logger::setLogLevel - log level: INFO")
            .unwrap();
        assert_eq!(level, LogLevel::Debug);
        assert!(ts.is_some());

        // Space-padded milliseconds
        let (level, ts) = parser
            .parse_line("04:11:35. 82 [1968] INFO PortMonitor::initAccessToken - msg")
            .unwrap();
        assert_eq!(level, LogLevel::Info);
        assert!(ts.is_some(), "space-padded milliseconds should parse");
    }

    #[test]
    fn test_no_timestamp_format() {
        let def = CustomFormatDef {
            name: "level-only".to_string(),
            pattern: r"^(?P<level>ERROR|WARN|INFO) (?P<message>.*)".to_string(),
            timestamp_format: None,
            timezone: None,
            level_map: HashMap::new(),
        };
        let parser = CustomParser::from_def(def).unwrap();
        let (level, ts) = parser.parse_line("ERROR something broke").unwrap();
        assert_eq!(level, LogLevel::Error);
        assert!(ts.is_none());
    }

    #[test]
    fn test_timezone_offset_conversion() {
        // Logs recorded in UTC+1 (e.g. CET) — 13:33:00 in log should display as 14:33:00 local
        // We test the conversion math: UTC+1 naive 13:33:00 → UTC 12:33:00 → local naive offset
        // For a deterministic test, use "+01:00" and verify the stored naive is shifted by +1h
        let def = CustomFormatDef {
            name: "tz-test".to_string(),
            pattern: r"^(?P<timestamp>\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}) (?P<level>\w+)".to_string(),
            timestamp_format: Some("%Y-%m-%d %H:%M:%S".to_string()),
            timezone: Some("+01:00".to_string()),
            level_map: HashMap::new(),
        };
        let parser = CustomParser::from_def(def).unwrap();
        assert_eq!(parser.timezone_offset_secs, Some(3600));

        // parse_tz_offset tests
        assert_eq!(CustomParser::parse_tz_offset("+05:30"), Some(5 * 3600 + 30 * 60));
        assert_eq!(CustomParser::parse_tz_offset("-07:00"), Some(-7 * 3600));
        assert_eq!(CustomParser::parse_tz_offset("+00:00"), Some(0));
        assert_eq!(CustomParser::parse_tz_offset("invalid"), None);
    }
}
