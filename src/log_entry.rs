use chrono::NaiveDateTime;
use regex::Regex;
use std::sync::LazyLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Profile,
    Unknown,
}

impl LogLevel {
    pub fn short_name(&self) -> &'static str {
        match self {
            LogLevel::Trace => "TRC",
            LogLevel::Debug => "DBG",
            LogLevel::Info => "INF",
            LogLevel::Warn => "WRN",
            LogLevel::Error => "ERR",
            LogLevel::Profile => "PRF",
            LogLevel::Unknown => "???",
        }
    }
}

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub index: usize,
    pub level: LogLevel,
    pub timestamp: Option<NaiveDateTime>,
    pub raw_line: String,
    pub continuation_lines: Vec<String>,
}

impl LogEntry {
    pub fn full_text(&self) -> String {
        if self.continuation_lines.is_empty() {
            self.raw_line.clone()
        } else {
            let mut text = self.raw_line.clone();
            for line in &self.continuation_lines {
                text.push('\n');
                text.push_str(line);
            }
            text
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    WdLog,
    WpcLog,
    Unknown,
}

// Regex patterns for parsing
// wd.log format:
//   Prefix: 2 spaces OR marker char (*, !, #) + space
//   Level: ~~~~~, =====, TRACE, INFO, WARN, ERROR (with optional trailing spaces)
//   Timestamp: MM-dd HH:mm:ss.fff
//   Rest: [thread] component|subcomponent "message"
//
// Examples:
//   ~~~~~ 02-03 18:10:37.564 [T32289|#6] HTTP|DspWebConnection "msg"
//   ===== 02-03 18:11:02.570 [Alarm] SCHED|Scheduler "msg"
//   TRACE 02-03 18:10:39.720 [#10] HTTP|DspWebServer "msg"
//   INFO  02-03 18:11:02.577 [Alarm] SPL|WatchdocContext "msg"
// * ERROR 02-05 11:23:38.795 [#34] API|PrintApiController10 "msg"
// ! WARN  02-05 11:23:38.801 [#10] HTTP|DspWebServer "msg"
static WD_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^([*!]?\s+|[*!#]\s)(~~~~~|=====|TRACE|INFO|WARN|ERROR)\s+(\d{2}-\d{2}\s+\d{2}:\d{2}:\d{2}\.\d{3})").unwrap()
});

static WPC_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(DBG|INF|WRN|ERR)\s+(\d{2}-\d{2}\s+\d{2}:\d{2}:\d{2}\.\d{3})").unwrap()
});

static TIMESTAMP_FORMAT: &str = "%m-%d %H:%M:%S%.3f";

/// Detect log format by sniffing first non-comment line
pub fn detect_format(content: &str) -> LogFormat {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        // Check wd.log pattern on the original line (not trimmed, prefix matters)
        if WD_PATTERN.is_match(line) {
            return LogFormat::WdLog;
        }
        if WPC_PATTERN.is_match(trimmed) {
            return LogFormat::WpcLog;
        }
        // First non-comment line doesn't match either pattern
        return LogFormat::Unknown;
    }
    LogFormat::Unknown
}

/// Parse log level from wd.log token
fn parse_wd_level(token: &str) -> LogLevel {
    match token.trim() {
        "TRACE" => LogLevel::Trace,
        "INFO" => LogLevel::Info,
        "WARN" => LogLevel::Warn,
        "ERROR" => LogLevel::Error,
        "=====" => LogLevel::Debug,
        "~~~~~" => LogLevel::Profile,
        _ => LogLevel::Unknown,
    }
}

/// Parse log level from wpc.log token
fn parse_wpc_level(token: &str) -> LogLevel {
    match token {
        "DBG" => LogLevel::Debug,
        "INF" => LogLevel::Info,
        "WRN" => LogLevel::Warn,
        "ERR" => LogLevel::Error,
        _ => LogLevel::Unknown,
    }
}

/// Parse timestamp string into NaiveDateTime (assumes current year)
fn parse_timestamp(ts: &str) -> Option<NaiveDateTime> {
    let current_year = chrono::Local::now().format("%Y").to_string();
    let full_ts = format!("{}-{}", current_year, ts);
    NaiveDateTime::parse_from_str(&full_ts, &format!("%Y-{}", TIMESTAMP_FORMAT)).ok()
}

/// Check if a line is a continuation line (doesn't start a new log entry)
fn is_continuation_line(line: &str, format: LogFormat) -> bool {
    if line.is_empty() {
        return true;
    }

    // Comment lines in the middle of the file are continuation
    if line.starts_with('#') {
        return true;
    }

    // If it matches a log pattern, it's not a continuation
    match format {
        LogFormat::WdLog => !WD_PATTERN.is_match(line),
        LogFormat::WpcLog => !WPC_PATTERN.is_match(line),
        LogFormat::Unknown => {
            // For unknown format, check both patterns
            !WD_PATTERN.is_match(line) && !WPC_PATTERN.is_match(line)
        }
    }
}

/// Parse a single line in wd.log format
fn parse_wd_line(line: &str) -> Option<(LogLevel, Option<NaiveDateTime>)> {
    WD_PATTERN.captures(line).map(|caps| {
        let level = parse_wd_level(&caps[2]);
        let timestamp = parse_timestamp(&caps[3]);
        (level, timestamp)
    })
}

/// Parse a single line in wpc.log format
fn parse_wpc_line(line: &str) -> Option<(LogLevel, Option<NaiveDateTime>)> {
    WPC_PATTERN.captures(line).map(|caps| {
        let level = parse_wpc_level(&caps[1]);
        let timestamp = parse_timestamp(&caps[2]);
        (level, timestamp)
    })
}

/// Parse log content into entries
pub fn parse_log(content: &str) -> Vec<LogEntry> {
    let format = detect_format(content);
    parse_log_with_format(content, format)
}

/// Parse log content with specified format
pub fn parse_log_with_format(content: &str, format: LogFormat) -> Vec<LogEntry> {
    let mut entries: Vec<LogEntry> = Vec::new();
    let mut index = 0;
    let mut in_header = true;

    for line in content.lines() {
        // Handle comment lines at start of file (header section)
        if in_header && line.starts_with('#') {
            continue;
        }
        in_header = false;

        // Check if this is a continuation line
        if !entries.is_empty() && is_continuation_line(line, format) {
            if let Some(last) = entries.last_mut() {
                last.continuation_lines.push(line.to_string());
            }
            continue;
        }

        // Try to parse as a new entry
        let parsed = match format {
            LogFormat::WdLog => parse_wd_line(line),
            LogFormat::WpcLog => parse_wpc_line(line),
            LogFormat::Unknown => parse_wd_line(line).or_else(|| parse_wpc_line(line)),
        };

        if let Some((level, timestamp)) = parsed {
            entries.push(LogEntry {
                index,
                level,
                timestamp,
                raw_line: line.to_string(),
                continuation_lines: Vec::new(),
            });
            index += 1;
        } else if !entries.is_empty() {
            // Doesn't match pattern, treat as continuation
            if let Some(last) = entries.last_mut() {
                last.continuation_lines.push(line.to_string());
            }
        }
    }

    entries
}

/// Parse incremental content (for tailing), continuing from last entry
pub fn parse_incremental(
    content: &str,
    format: LogFormat,
    start_index: usize,
    mut pending_continuation: Option<&mut LogEntry>,
) -> Vec<LogEntry> {
    let mut entries: Vec<LogEntry> = Vec::new();
    let mut index = start_index;

    for line in content.lines() {
        // Check if this is a continuation line
        let is_cont = is_continuation_line(line, format);

        if is_cont {
            // Add to last entry or pending continuation
            if let Some(last) = entries.last_mut() {
                last.continuation_lines.push(line.to_string());
            } else if let Some(pending) = pending_continuation.as_deref_mut() {
                pending.continuation_lines.push(line.to_string());
            }
            continue;
        }

        // Try to parse as a new entry
        let parsed = match format {
            LogFormat::WdLog => parse_wd_line(line),
            LogFormat::WpcLog => parse_wpc_line(line),
            LogFormat::Unknown => parse_wd_line(line).or_else(|| parse_wpc_line(line)),
        };

        if let Some((level, timestamp)) = parsed {
            entries.push(LogEntry {
                index,
                level,
                timestamp,
                raw_line: line.to_string(),
                continuation_lines: Vec::new(),
            });
            index += 1;
        } else {
            // Doesn't match pattern, treat as continuation
            if let Some(last) = entries.last_mut() {
                last.continuation_lines.push(line.to_string());
            } else if let Some(pending) = pending_continuation.as_deref_mut() {
                pending.continuation_lines.push(line.to_string());
            }
        }
    }

    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_wd_format() {
        let content = "  ~~~~~ 02-03 18:10:37.564 [T32289|#6] HTTP|DspWebConnection \"msg\"";
        assert_eq!(detect_format(content), LogFormat::WdLog);
    }

    #[test]
    fn test_detect_wd_format_with_marker() {
        let content = "* ERROR 02-05 11:23:38.795 [#34] API|PrintApiController10 \"msg\"";
        assert_eq!(detect_format(content), LogFormat::WdLog);
    }

    #[test]
    fn test_detect_wd_format_with_header() {
        let content = "# Starting new log\n# Previous log was 20,974,832 bytes\n  ~~~~~ 02-03 18:10:37.564 [T32289|#6] HTTP|DspWebConnection \"msg\"";
        assert_eq!(detect_format(content), LogFormat::WdLog);
    }

    #[test]
    fn test_detect_wpc_format() {
        let content = "INF 03-21 14:23:01.234 Test message";
        assert_eq!(detect_format(content), LogFormat::WpcLog);
    }

    #[test]
    fn test_parse_wd_log_profile() {
        let content = "  ~~~~~ 02-03 18:10:37.564 [T32289|#6] HTTP|DspWebConnection \"msg\"";
        let entries = parse_log(content);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].level, LogLevel::Profile);
    }

    #[test]
    fn test_parse_wd_log_debug() {
        let content = "  ===== 02-03 18:11:02.570 [Alarm] SCHED|Scheduler \"msg\"";
        let entries = parse_log(content);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].level, LogLevel::Debug);
    }

    #[test]
    fn test_parse_wd_log_info() {
        let content = "  INFO  02-03 18:11:02.577 [Alarm] SPL|WatchdocContext \"msg\"";
        let entries = parse_log(content);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].level, LogLevel::Info);
    }

    #[test]
    fn test_parse_wd_log_error_with_marker() {
        let content = "* ERROR 02-05 11:23:38.795 [#34] API|PrintApiController10 \"msg\"";
        let entries = parse_log(content);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].level, LogLevel::Error);
    }

    #[test]
    fn test_parse_wd_log_warn_with_marker() {
        let content = "! WARN  02-05 11:23:38.801 [#10] HTTP|DspWebServer \"msg\"";
        let entries = parse_log(content);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].level, LogLevel::Warn);
    }

    #[test]
    fn test_parse_wd_log_with_continuation() {
        let content = "  ~~~~~ 02-03 18:11:11.526 [#32] MSGQ|HttpMessageSender \"Sending JSON batch => [\n\t{\n\t\t\"Id\": \"52dc5014\"\n\t}\n]\"";
        let entries = parse_log(content);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].level, LogLevel::Profile);
        assert_eq!(entries[0].continuation_lines.len(), 4);
    }

    #[test]
    fn test_parse_wd_log_skips_header() {
        let content = "# Starting new log\n# Previous log\n  ~~~~~ 02-03 18:10:37.564 [T32289|#6] HTTP \"msg\"";
        let entries = parse_log(content);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].level, LogLevel::Profile);
    }

    #[test]
    fn test_parse_wpc_log() {
        let content = "ERR 03-21 14:23:01.234 Error message";
        let entries = parse_log(content);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].level, LogLevel::Error);
    }
}
