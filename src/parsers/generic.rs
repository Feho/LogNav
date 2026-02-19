use super::LogParser;
use crate::log_entry::LogLevel;
use chrono::NaiveDateTime;
use regex::Regex;
use std::sync::LazyLock;

/// Generic parser for common log formats (syslog, ISO 8601, log4j, etc.)
///
/// Detects lines starting with a timestamp and optionally containing a level keyword.
/// Handles most real-world log formats including:
///   - ISO 8601: `2024-01-19T14:08:32.123Z`, `2024-01-19 14:08:32`
///   - Syslog:   `Jan 19 14:08:32`
///   - Log4j:    `2024-01-19 14:08:32,123 ERROR ...`
///   - Bracketed: `[2024-01-19 14:08:32]`
///   - Apache:   `19/Jan/2024:14:08:32 +0000`
///
/// Lines without a leading timestamp are treated as continuations.
#[derive(Debug, Clone, Copy)]
pub struct GenericParser;

// ISO 8601 variants: 2024-01-19T14:08:32, 2024-01-19 14:08:32.123, with optional Z/offset
static ISO_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\[?(\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2})[.,]?\d*Z?\]?").unwrap()
});

// Syslog: Jan 19 14:08:32 (month day time)
static SYSLOG_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^((?:Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)\s+\d{1,2}\s+\d{2}:\d{2}:\d{2})").unwrap()
});

// Apache/nginx: 19/Jan/2024:14:08:32
static APACHE_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:^|\[)(\d{2}/(?:Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)/\d{4}:\d{2}:\d{2}:\d{2})").unwrap()
});

// Time-only: 14:08:32.123 or 14:08:32 (date inferred from filename or omitted)
static TIME_ONLY_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(\d{2}:\d{2}:\d{2})(?:[.,]\d+)?").unwrap()
});

// Matches any of the above timestamp patterns at line start (for detection/line splitting)
static ANY_TIMESTAMP: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
        r"^(?:",
        r"\[?\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}",  // ISO
        r"|(?:Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)\s+\d{1,2}\s+\d{2}:\d{2}:\d{2}",  // syslog
        r"|[\[:]?\d{2}/(?:Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)/\d{4}:\d{2}:\d{2}:\d{2}",  // apache
        r"|\d{2}:\d{2}:\d{2}[.,]\d{3}",  // time-only with millis
        r")",
    ))
    .unwrap()
});

// Level keywords (case-insensitive matching done at call site)
static LEVEL_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(FATAL|CRITICAL|ERROR|ERR|WARN(?:ING)?|NOTICE|INFO|DEBUG|DBG|TRACE|TRC|VERBOSE)\b").unwrap()
});

impl LogParser for GenericParser {
    fn detect(&self, first_line: &str) -> f64 {
        if ANY_TIMESTAMP.is_match(first_line) {
            0.5
        } else {
            0.0
        }
    }

    fn parse_line(&self, line: &str) -> Option<(LogLevel, Option<NaiveDateTime>)> {
        if !ANY_TIMESTAMP.is_match(line) {
            return None; // continuation
        }

        let timestamp = parse_generic_timestamp(line);
        let level = detect_level(line);
        Some((level, timestamp))
    }
}

/// Try each timestamp format in order, return first match
fn parse_generic_timestamp(line: &str) -> Option<NaiveDateTime> {
    // ISO 8601
    if let Some(caps) = ISO_PATTERN.captures(line) {
        let ts = &caps[1];
        // Try with T separator then space
        if let Ok(dt) = NaiveDateTime::parse_from_str(ts, "%Y-%m-%dT%H:%M:%S") {
            return Some(dt);
        }
        if let Ok(dt) = NaiveDateTime::parse_from_str(ts, "%Y-%m-%d %H:%M:%S") {
            return Some(dt);
        }
    }

    // Syslog — no year, assume current
    if let Some(caps) = SYSLOG_PATTERN.captures(line) {
        let ts = &caps[1];
        let year = chrono::Local::now().format("%Y").to_string();
        let full = format!("{} {}", year, ts);
        if let Ok(dt) = NaiveDateTime::parse_from_str(&full, "%Y %b %d %H:%M:%S") {
            return Some(dt);
        }
    }

    // Apache: 19/Jan/2024:14:08:32
    if let Some(caps) = APACHE_PATTERN.captures(line) {
        let ts = &caps[1];
        if let Ok(dt) = NaiveDateTime::parse_from_str(ts, "%d/%b/%Y:%H:%M:%S") {
            return Some(dt);
        }
    }

    // Time-only: 14:08:32 — no date, use today
    if let Some(caps) = TIME_ONLY_PATTERN.captures(line) {
        let ts = &caps[1];
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let full = format!("{} {}", today, ts);
        if let Ok(dt) = NaiveDateTime::parse_from_str(&full, "%Y-%m-%d %H:%M:%S") {
            return Some(dt);
        }
    }

    None
}

/// Detect log level from keywords in the line
fn detect_level(line: &str) -> LogLevel {
    if let Some(caps) = LEVEL_PATTERN.captures(line) {
        match caps[1].to_ascii_uppercase().as_str() {
            "FATAL" | "CRITICAL" => LogLevel::Error,
            "ERROR" | "ERR" => LogLevel::Error,
            "WARN" | "WARNING" => LogLevel::Warn,
            "NOTICE" | "INFO" => LogLevel::Info,
            "DEBUG" | "DBG" => LogLevel::Debug,
            "TRACE" | "TRC" | "VERBOSE" => LogLevel::Trace,
            _ => LogLevel::Unknown,
        }
    } else {
        LogLevel::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_iso() {
        let p = GenericParser;
        assert_eq!(p.detect("2024-01-19 14:08:32 ERROR something"), 0.5);
        assert_eq!(p.detect("2024-01-19T14:08:32.123Z ERROR something"), 0.5);
        assert_eq!(p.detect("[2024-01-19 14:08:32] ERROR something"), 0.5);
    }

    #[test]
    fn test_detect_syslog() {
        let p = GenericParser;
        assert_eq!(p.detect("Jan 19 14:08:32 hostname sshd[1234]: msg"), 0.5);
        assert_eq!(p.detect("Dec  1 03:00:00 host kernel: msg"), 0.5);
    }

    #[test]
    fn test_detect_apache() {
        let p = GenericParser;
        assert_eq!(p.detect("[19/Jan/2024:14:08:32 +0000] \"GET /\""), 0.5);
    }

    #[test]
    fn test_detect_time_only() {
        let p = GenericParser;
        assert_eq!(
            p.detect("13:29:36.736 [15056] DEBUG Logger::setLogLevel - log level: DEBUG"),
            0.5
        );
    }

    #[test]
    fn test_detect_no_match() {
        let p = GenericParser;
        assert_eq!(p.detect("just some random text"), 0.0);
        assert_eq!(p.detect("ERROR without timestamp"), 0.0);
    }

    #[test]
    fn test_parse_iso_line() {
        let p = GenericParser;
        let result = p.parse_line("2024-01-19 14:08:32 ERROR something broke");
        assert!(result.is_some());
        let (level, ts) = result.unwrap();
        assert_eq!(level, LogLevel::Error);
        assert!(ts.is_some());
    }

    #[test]
    fn test_parse_iso_t_separator() {
        let p = GenericParser;
        let result = p.parse_line("2024-01-19T14:08:32.123Z INFO startup");
        assert!(result.is_some());
        let (level, ts) = result.unwrap();
        assert_eq!(level, LogLevel::Info);
        assert!(ts.is_some());
    }

    #[test]
    fn test_parse_syslog_line() {
        let p = GenericParser;
        let result = p.parse_line("Jan 19 14:08:32 myhost sshd[1234]: Failed password");
        assert!(result.is_some());
        let (level, ts) = result.unwrap();
        assert_eq!(level, LogLevel::Unknown); // no level keyword
        assert!(ts.is_some());
    }

    #[test]
    fn test_parse_continuation() {
        let p = GenericParser;
        assert!(p.parse_line("    at com.example.Foo.bar(Foo.java:42)").is_none());
        assert!(p.parse_line("Caused by: java.lang.NullPointerException").is_none());
    }

    #[test]
    fn test_level_detection() {
        assert_eq!(detect_level("something ERROR happened"), LogLevel::Error);
        assert_eq!(detect_level("WARN: disk full"), LogLevel::Warn);
        assert_eq!(detect_level("WARNING: disk full"), LogLevel::Warn);
        assert_eq!(detect_level("DEBUG checking value"), LogLevel::Debug);
        assert_eq!(detect_level("FATAL crash"), LogLevel::Error);
        assert_eq!(detect_level("no level here"), LogLevel::Unknown);
    }

    #[test]
    fn test_bracketed_timestamp() {
        let p = GenericParser;
        let result = p.parse_line("[2024-01-19 14:08:32] [ERROR] something broke");
        assert!(result.is_some());
        let (level, ts) = result.unwrap();
        assert_eq!(level, LogLevel::Error);
        assert!(ts.is_some());
    }

    #[test]
    fn test_log4j_comma_millis() {
        let p = GenericParser;
        let result = p.parse_line("2024-01-19 14:08:32,123 ERROR com.example.Main - crash");
        assert!(result.is_some());
        let (level, ts) = result.unwrap();
        assert_eq!(level, LogLevel::Error);
        assert!(ts.is_some());
    }

    #[test]
    fn test_time_only_portmonitor() {
        let p = GenericParser;
        let result =
            p.parse_line("13:29:36.736 [15056] DEBUG Logger::setLogLevel - log level: DEBUG");
        assert!(result.is_some());
        let (level, ts) = result.unwrap();
        assert_eq!(level, LogLevel::Debug);
        assert!(ts.is_some());
    }

    #[test]
    fn test_time_only_info() {
        let p = GenericParser;
        let result =
            p.parse_line("13:29:36.741 [15056] INFO PortMonitor::PortMonitor - Port monitor loaded");
        assert!(result.is_some());
        let (level, _) = result.unwrap();
        assert_eq!(level, LogLevel::Info);
    }
}
