use super::{LogParser, parse_timestamp};
use crate::log_entry::LogLevel;
use chrono::NaiveDateTime;
use regex::Regex;
use std::sync::LazyLock;

/// Parser for wpc.log format
///
/// Format:
///   Level: VRB, DBG, INF, WRN, ERR
///   Timestamp: MM-dd HH:mm:ss.fff
///   Rest: message
///
/// Example:
///   INF 03-21 14:23:01.234 Test message
#[derive(Debug, Clone, Copy)]
pub struct WpcParser;

static WPC_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(VRB|DBG|INF|WRN|ERR)\s+(\d{2}-\d{2}\s+\d{2}:\d{2}:\d{2}\.\d{3})").unwrap()
});

impl LogParser for WpcParser {
    fn detect(&self, first_line: &str) -> f64 {
        // Note: must trim internally as the pattern expects start of line
        let trimmed = first_line.trim();
        if WPC_PATTERN.is_match(trimmed) {
            1.0
        } else {
            0.0
        }
    }

    fn parse_line(&self, line: &str) -> Option<(LogLevel, Option<NaiveDateTime>)> {
        WPC_PATTERN.captures(line).map(|caps| {
            let level = parse_wpc_level(&caps[1]);
            let timestamp = parse_timestamp(&caps[2]);
            (level, timestamp)
        })
    }
}

/// Parse log level from wpc.log token
fn parse_wpc_level(token: &str) -> LogLevel {
    match token {
        "VRB" => LogLevel::Trace,
        "DBG" => LogLevel::Debug,
        "INF" => LogLevel::Info,
        "WRN" => LogLevel::Warn,
        "ERR" => LogLevel::Error,
        _ => LogLevel::Unknown,
    }
}
