use super::{LogParser, parse_timestamp};
use crate::log_entry::LogLevel;
use chrono::NaiveDateTime;
use regex::Regex;
use std::sync::LazyLock;

/// Parser for wd.log format
///
/// Format:
///   Prefix: 2 spaces OR marker char (*, !, #) + space
///   Level: ~~~~~, =====, TRACE, INFO, WARN, ERROR (with optional trailing spaces)
///   Timestamp: MM-dd HH:mm:ss.fff
///   Rest: [thread] component|subcomponent "message"
///
/// Examples:
///   ~~~~~ 02-03 18:10:37.564 [T32289|#6] HTTP|DspWebConnection "msg"
///   ===== 02-03 18:11:02.570 [Alarm] SCHED|Scheduler "msg"
///   TRACE 02-03 18:10:39.720 [#10] HTTP|DspWebServer "msg"
///   INFO  02-03 18:11:02.577 [Alarm] SPL|WatchdocContext "msg"
/// * ERROR 02-05 11:23:38.795 [#34] API|PrintApiController10 "msg"
/// ! WARN  02-05 11:23:38.801 [#10] HTTP|DspWebServer "msg"
#[derive(Debug, Clone, Copy)]
pub struct WdParser;

static WD_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^([*!]?\s+|[*!#]\s)(~~~~~|=====|TRACE|INFO|WARN|ERROR)\s+(\d{2}-\d{2}\s+\d{2}:\d{2}:\d{2}\.\d{3})").unwrap()
});

impl LogParser for WdParser {
    fn name(&self) -> &str {
        "wd.log"
    }

    fn detect(&self, first_line: &str) -> f64 {
        if WD_PATTERN.is_match(first_line) {
            1.0
        } else {
            0.0
        }
    }

    fn parse_line(&self, line: &str) -> Option<(LogLevel, Option<NaiveDateTime>)> {
        WD_PATTERN.captures(line).map(|caps| {
            let level = parse_wd_level(&caps[2]);
            let timestamp = parse_timestamp(&caps[3]);
            (level, timestamp)
        })
    }
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
