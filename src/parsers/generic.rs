use super::LogParser;
use super::common::map_level_str;
use crate::log_entry::LogLevel;
use chrono::NaiveDateTime;
use regex::Regex;
use std::sync::LazyLock;

/// Level pattern matching common log level strings (case-insensitive)
static LEVEL_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(TRACE|DEBUG|INFO|WARN(?:ING)?|ERROR|FATAL|CRITICAL|SEVERE|VERBOSE|TRC|DBG|INF|WRN|ERR|VRB|VERB|CRIT)\b").unwrap()
});

struct TsPattern {
    regex: LazyLock<Regex>,
    chrono_fmt: &'static str,
    needs_year: bool,
}

macro_rules! ts_pattern {
    ($re:expr, $fmt:expr, $year:expr) => {
        TsPattern {
            regex: LazyLock::new(|| Regex::new($re).unwrap()),
            chrono_fmt: $fmt,
            needs_year: $year,
        }
    };
}

/// Timestamp patterns ordered by specificity (most specific first)
static TS_PATTERNS: LazyLock<Vec<TsPattern>> = LazyLock::new(|| {
    vec![
        // ISO 8601 with fractional seconds
        ts_pattern!(
            r"\[?(\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}[.,]\d{1,6})\]?",
            "%Y-%m-%d %H:%M:%S%.f",
            false
        ),
        // ISO 8601 no fraction
        ts_pattern!(
            r"\[?(\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2})\]?",
            "%Y-%m-%d %H:%M:%S",
            false
        ),
        // YYYY/MM/DD HH:MM:SS
        ts_pattern!(
            r"\[?(\d{4}/\d{2}/\d{2} \d{2}:\d{2}:\d{2})\]?",
            "%Y/%m/%d %H:%M:%S",
            false
        ),
        // MM/DD/YYYY HH:MM:SS
        ts_pattern!(
            r"\[?(\d{2}/\d{2}/\d{4} \d{2}:\d{2}:\d{2})\]?",
            "%m/%d/%Y %H:%M:%S",
            false
        ),
        // MM-dd HH:mm:ss with optional fraction (year-less)
        ts_pattern!(
            r"\[?(\d{2}-\d{2} \d{2}:\d{2}:\d{2}(?:[.,]\d{1,3})?)\]?",
            "%m-%d %H:%M:%S%.f",
            true
        ),
        // Time only HH:MM:SS with optional fraction
        ts_pattern!(
            r"\[?(\d{2}:\d{2}:\d{2}(?:[.,]\d{1,6})?)\]?",
            "%H:%M:%S%.f",
            false
        ),
    ]
});

/// Cached today's date string (YYYY-MM-DD) to avoid repeated formatting
fn current_today_str() -> String {
    use chrono::Datelike;
    thread_local! {
        static CACHED: std::cell::Cell<(u32, [u8; 10])> = const { std::cell::Cell::new((0, [0; 10])) };
    }
    let now = chrono::Local::now();
    let ordinal = now.year() as u32 * 1000 + now.ordinal();
    CACHED.with(|c| {
        let (cached_ord, cached_bytes) = c.get();
        if cached_ord == ordinal {
            unsafe { String::from_utf8_unchecked(cached_bytes[..10].to_vec()) }
        } else {
            let s = now.format("%Y-%m-%d").to_string();
            let mut bytes = [0u8; 10];
            bytes.copy_from_slice(s.as_bytes());
            c.set((ordinal, bytes));
            s
        }
    })
}

/// Generic parser that auto-detects timestamp/level patterns from sample lines
#[derive(Debug)]
pub struct GenericParser {
    /// Index into TS_PATTERNS for the matched timestamp pattern
    ts_pattern_idx: Option<usize>,
    has_level: bool,
}

impl GenericParser {
    /// Learn format from sample lines. Returns None if no recognizable patterns found.
    pub fn learn(lines: &[&str]) -> Option<Self> {
        if lines.is_empty() {
            return None;
        }

        // Find best timestamp pattern by match count
        let mut best_ts_idx: Option<usize> = None;
        let mut best_ts_count = 0usize;

        for (i, pat) in TS_PATTERNS.iter().enumerate() {
            let count = lines.iter().filter(|l| pat.regex.is_match(l)).count();
            if count > best_ts_count {
                best_ts_count = count;
                best_ts_idx = Some(i);
            }
        }

        let level_count = lines.iter().filter(|l| LEVEL_PATTERN.is_match(l)).count();

        // Need >=30% match rate for at least one pattern
        let threshold = lines.len() * 3 / 10;
        let has_ts = best_ts_count >= threshold.max(1);
        let has_level = level_count >= threshold.max(1);

        if !has_ts && !has_level {
            return None;
        }

        Some(GenericParser {
            ts_pattern_idx: if has_ts { best_ts_idx } else { None },
            has_level,
        })
    }

    fn parse_timestamp_str(ts: &str, chrono_fmt: &str, needs_year: bool) -> Option<NaiveDateTime> {
        // Normalize comma decimal separator to dot
        let ts = ts.replace(',', ".");

        if needs_year {
            let year = super::current_year_str();
            let full = format!("{}-{}", year, ts);
            let full_fmt = format!("%Y-{}", chrono_fmt);
            NaiveDateTime::parse_from_str(&full, &full_fmt).ok()
        } else if chrono_fmt == "%H:%M:%S%.f" {
            // Time-only: use today's date
            let today = current_today_str();
            let full = format!("{} {}", today, ts);
            NaiveDateTime::parse_from_str(&full, "%Y-%m-%d %H:%M:%S%.f").ok()
        } else {
            let ts = ts.replace('T', " ");
            NaiveDateTime::parse_from_str(&ts, chrono_fmt).ok()
        }
    }
}

impl LogParser for GenericParser {
    fn detect(&self, first_line: &str) -> f64 {
        let has_ts = self
            .ts_pattern_idx
            .is_some_and(|i| TS_PATTERNS[i].regex.is_match(first_line));
        let has_level = LEVEL_PATTERN.is_match(first_line);

        match (has_ts, has_level) {
            (true, true) => 0.5,
            (true, false) | (false, true) => 0.3,
            (false, false) => 0.0,
        }
    }

    fn parse_line(&self, line: &str) -> Option<(LogLevel, Option<NaiveDateTime>)> {
        let timestamp = self.ts_pattern_idx.and_then(|i| {
            let pat = &TS_PATTERNS[i];
            pat.regex.captures(line).and_then(|caps| {
                Self::parse_timestamp_str(&caps[1], pat.chrono_fmt, pat.needs_year)
            })
        });

        let level = if self.has_level {
            LEVEL_PATTERN
                .captures(line)
                .map(|caps| map_level_str(&caps[1]))
                .unwrap_or(LogLevel::Unknown)
        } else {
            LogLevel::Unknown
        };

        if timestamp.is_none() && level == LogLevel::Unknown {
            return None;
        }

        Some((level, timestamp))
    }

    fn message_start(&self, line: &str) -> Option<usize> {
        let mut end = 0usize;

        // Find end of timestamp match
        if let Some(i) = self.ts_pattern_idx
            && let Some(m) = TS_PATTERNS[i].regex.find(line)
        {
            end = end.max(m.end());
        }

        // Find end of level match
        if self.has_level
            && let Some(m) = LEVEL_PATTERN.find(line)
        {
            end = end.max(m.end());
        }

        if end > 0 { Some(end) } else { None }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_learn_iso8601() {
        let lines = vec![
            "2024-01-15 10:30:45 ERROR something happened",
            "2024-01-15 10:30:46 INFO all good",
            "2024-01-15 10:30:47 WARN be careful",
        ];
        let parser = GenericParser::learn(&lines).unwrap();
        assert!(parser.ts_pattern_idx.is_some());
        assert!(parser.has_level);
    }

    #[test]
    fn test_learn_iso8601_fractional() {
        let lines = vec![
            "2024-01-15 10:30:45.123 ERROR something happened",
            "2024-01-15 10:30:46.456 INFO all good",
        ];
        let parser = GenericParser::learn(&lines).unwrap();
        let (level, ts) = parser
            .parse_line("2024-01-15 10:30:45.123 ERROR boom")
            .unwrap();
        assert_eq!(level, LogLevel::Error);
        assert!(ts.is_some());
    }

    #[test]
    fn test_learn_bracketed() {
        let lines = vec![
            "[2024-01-15 10:30:45] [INFO] message one",
            "[2024-01-15 10:30:46] [ERROR] message two",
        ];
        let parser = GenericParser::learn(&lines).unwrap();
        let (level, ts) = parser
            .parse_line("[2024-01-15 10:30:45] [ERROR] bad thing")
            .unwrap();
        assert_eq!(level, LogLevel::Error);
        assert!(ts.is_some());
    }

    #[test]
    fn test_learn_level_only() {
        let lines = vec![
            "ERROR something went wrong",
            "INFO all is well",
            "DEBUG details here",
        ];
        let parser = GenericParser::learn(&lines).unwrap();
        assert!(parser.ts_pattern_idx.is_none());
        assert!(parser.has_level);

        let (level, ts) = parser.parse_line("ERROR kaboom").unwrap();
        assert_eq!(level, LogLevel::Error);
        assert!(ts.is_none());
    }

    #[test]
    fn test_learn_timestamp_only() {
        let lines = vec![
            "2024-01-15 10:30:45 some message",
            "2024-01-15 10:30:46 another message",
            "2024-01-15 10:30:47 yet another",
        ];
        let parser = GenericParser::learn(&lines).unwrap();
        assert!(parser.ts_pattern_idx.is_some());
        assert!(!parser.has_level);

        let (level, ts) = parser.parse_line("2024-01-15 10:30:45 hello").unwrap();
        assert_eq!(level, LogLevel::Unknown);
        assert!(ts.is_some());
    }

    #[test]
    fn test_learn_no_patterns() {
        let lines = vec!["just some random text", "nothing special here", "no logs"];
        assert!(GenericParser::learn(&lines).is_none());
    }

    #[test]
    fn test_continuation_detection() {
        let lines = vec![
            "2024-01-15 10:30:45 ERROR something happened",
            "2024-01-15 10:30:46 INFO all good",
        ];
        let parser = GenericParser::learn(&lines).unwrap();
        assert!(parser.parse_line("  additional context here").is_none());
    }

    #[test]
    fn test_level_before_timestamp() {
        let lines = vec![
            "ERROR 2024-01-15 10:30:45 something happened",
            "INFO 2024-01-15 10:30:46 all good",
        ];
        let parser = GenericParser::learn(&lines).unwrap();
        let (level, ts) = parser.parse_line("ERROR 2024-01-15 10:30:45 boom").unwrap();
        assert_eq!(level, LogLevel::Error);
        assert!(ts.is_some());
    }

    #[test]
    fn test_case_insensitive_levels() {
        let lines = vec![
            "2024-01-15 10:30:45 error msg",
            "2024-01-15 10:30:46 Info msg",
        ];
        let parser = GenericParser::learn(&lines).unwrap();
        let (level, _) = parser.parse_line("2024-01-15 10:30:45 error boom").unwrap();
        assert_eq!(level, LogLevel::Error);
    }

    #[test]
    fn test_time_only() {
        let lines = vec![
            "10:30:45 ERROR msg one",
            "10:30:46 INFO msg two",
            "10:30:47 WARN msg three",
        ];
        let parser = GenericParser::learn(&lines).unwrap();
        let (level, ts) = parser.parse_line("10:30:45 ERROR boom").unwrap();
        assert_eq!(level, LogLevel::Error);
        assert!(ts.is_some());
    }

    #[test]
    fn test_detect_confidence() {
        let lines = vec![
            "2024-01-15 10:30:45 ERROR msg",
            "2024-01-15 10:30:46 INFO msg",
        ];
        let parser = GenericParser::learn(&lines).unwrap();
        assert_eq!(parser.detect("2024-01-15 10:30:45 ERROR msg"), 0.5);
        assert_eq!(parser.detect("2024-01-15 10:30:45 msg"), 0.3);
        assert_eq!(parser.detect("random text"), 0.0);
    }

    #[test]
    fn test_yearless_timestamp() {
        let lines = vec![
            "02-15 10:30:45 ERROR msg one",
            "02-15 10:30:46 INFO msg two",
        ];
        let parser = GenericParser::learn(&lines).unwrap();
        let (level, ts) = parser.parse_line("02-15 10:30:45 ERROR boom").unwrap();
        assert_eq!(level, LogLevel::Error);
        assert!(ts.is_some());
        let year = chrono::Local::now().format("%Y").to_string();
        assert_eq!(ts.unwrap().format("%Y").to_string(), year);
    }

    #[test]
    fn test_comma_decimal_separator() {
        let lines = vec![
            "2024-01-15 10:30:45,123 ERROR msg",
            "2024-01-15 10:30:46,456 INFO msg",
        ];
        let parser = GenericParser::learn(&lines).unwrap();
        let (_, ts) = parser
            .parse_line("2024-01-15 10:30:45,123 ERROR msg")
            .unwrap();
        assert!(ts.is_some());
    }

    #[test]
    fn test_t_separator() {
        let lines = vec![
            "2024-01-15T10:30:45 ERROR msg",
            "2024-01-15T10:30:46 INFO msg",
        ];
        let parser = GenericParser::learn(&lines).unwrap();
        let (_, ts) = parser.parse_line("2024-01-15T10:30:45 ERROR msg").unwrap();
        assert!(ts.is_some());
    }
}
