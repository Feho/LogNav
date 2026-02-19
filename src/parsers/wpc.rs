use super::{LogParser, parse_timestamp};
use crate::log_entry::LogLevel;
use chrono::NaiveDateTime;
use regex::Regex;
use std::sync::LazyLock;

/// Parser for wpc.log format
///
/// Handles two timestamp variants:
///   Short: `LVL MM-dd HH:mm:ss.fff ...`
///   Full:  `LVL YYYY-MM-DD HH:mm:ss.fff ...`
///
/// Levels: VRB, DBG, INF, WRN, ERR, ---
///
/// Examples:
///   INF 03-21 14:23:01.234 Test message
///   DBG 2025-12-10 14:16:19.408 IPC  Connecting to pipe...
///   --- 2025-12-10 14:16:20.149 AppSettings  Initializing...
#[derive(Debug, Clone, Copy)]
pub struct WpcParser;

// Short format: LVL MM-dd HH:mm:ss.fff
static WPC_SHORT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(VRB|DBG|INF|WRN|ERR|---)\s+(\d{2}-\d{2}\s+\d{2}:\d{2}:\d{2}\.\d{3})")
        .unwrap()
});

// Full format: LVL YYYY-MM-DD HH:mm:ss.fff
static WPC_FULL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(VRB|DBG|INF|WRN|ERR|---)\s+(\d{4}-\d{2}-\d{2}\s+\d{2}:\d{2}:\d{2}\.\d{3})")
        .unwrap()
});

impl LogParser for WpcParser {
    fn detect(&self, first_line: &str) -> f64 {
        let trimmed = first_line.trim();
        if WPC_SHORT.is_match(trimmed) || WPC_FULL.is_match(trimmed) {
            1.0
        } else {
            0.0
        }
    }

    fn parse_line(&self, line: &str) -> Option<(LogLevel, Option<NaiveDateTime>)> {
        // Try full date first (more specific)
        if let Some(caps) = WPC_FULL.captures(line) {
            let level = parse_wpc_level(&caps[1]);
            let timestamp =
                NaiveDateTime::parse_from_str(&caps[2], "%Y-%m-%d %H:%M:%S%.3f").ok();
            return Some((level, timestamp));
        }
        WPC_SHORT.captures(line).map(|caps| {
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
        "---" => LogLevel::Trace,
        _ => LogLevel::Unknown,
    }
}
