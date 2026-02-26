use super::{parse_timestamp, LogParser};
use crate::log_entry::LogLevel;
use chrono::NaiveDateTime;
use regex::Regex;
use std::sync::LazyLock;

/// Parser for wd.log format
///
/// Format:
///   Prefix: 1-2 chars from [*!?# ] (e.g. "  ", "* ", "? ", "**")
///   Level: ~~~~~, =====, TRACE, AUDIT, INFO, WARN, ERROR, FATAL
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
/// ? AUDIT 02-12 11:37:17.453 [#111] AUTH|MetaAuthority "msg"
/// **FATAL 02-12 11:37:20.688 [#47] SQL|SqlProxy "msg"
#[derive(Debug, Clone, Copy)]
pub struct WdParser;

static WD_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[*!?# ]{1,2}(~~~~~|=====|TRACE|AUDIT|INFO|WARN|ERROR|FATAL)\s+(\d{2}-\d{2}\s+\d{2}:\d{2}:\d{2}\.\d{3})").unwrap()
});

impl LogParser for WdParser {
    fn detect(&self, first_line: &str) -> f64 {
        if WD_PATTERN.is_match(first_line) {
            1.0
        } else {
            0.0
        }
    }

    fn parse_line(&self, line: &str) -> Option<(LogLevel, Option<NaiveDateTime>)> {
        WD_PATTERN.captures(line).map(|caps| {
            let level = parse_wd_level(&caps[1]);
            let timestamp = parse_timestamp(&caps[2]);
            (level, timestamp)
        })
    }
}

/// Parse log level from wd.log token
fn parse_wd_level(token: &str) -> LogLevel {
    match token.trim() {
        "TRACE" | "AUDIT" => LogLevel::Trace,
        "INFO" => LogLevel::Info,
        "WARN" => LogLevel::Warn,
        "ERROR" | "FATAL" => LogLevel::Error,
        "=====" => LogLevel::Debug,
        "~~~~~" => LogLevel::Profile,
        _ => LogLevel::Unknown,
    }
}
