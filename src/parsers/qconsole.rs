use super::LogParser;
use crate::log_entry::LogLevel;
use chrono::NaiveDateTime;
use regex::Regex;
use std::sync::LazyLock;

/// Parser for qconsole.log format (OpenMoHAA game server console)
///
/// Format:
///   [YYYY-MM-DD HH:MM:SS UTC±H.000] message
///
/// Examples:
///   [2026-01-09 18:48:38 UTC+1.000] logfile opened on Fri Jan  9 18:48:38 2026
///   [2026-01-09 18:48:38 UTC+1.000] Cvar_Set2: sv_hostname -=[PN]=- | Realism
///   [2026-01-09 19:05:01 UTC+1.000] ^~^~^ Script Error : Can't find 'file.scr'
///   [2026-01-09 18:48:52 UTC+1.000] ^3Player Dimitri_47 is under fire!
///
/// Log levels are inferred from message content (no explicit level field).
/// Quake-style color codes (^0-^9) are stripped via clean_line().
#[derive(Debug, Clone, Copy)]
pub struct QConsoleParser;

static QCONSOLE_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\[(\d{4}-\d{2}-\d{2}\s+\d{2}:\d{2}:\d{2})\s+UTC[+-][\d.]+\]\s*(.*)").unwrap()
});

static COLOR_CODE_PATTERN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\^[0-9]").unwrap());

impl LogParser for QConsoleParser {
    fn detect(&self, first_line: &str) -> f64 {
        if QCONSOLE_PATTERN.is_match(first_line) {
            1.0
        } else if first_line.contains("logfile opened on") {
            0.8
        } else {
            0.0
        }
    }

    fn parse_line(&self, line: &str) -> Option<(LogLevel, Option<NaiveDateTime>)> {
        QCONSOLE_PATTERN.captures(line).map(|caps| {
            let timestamp = parse_qconsole_timestamp(&caps[1]);
            let message = &caps[2];
            let level = infer_level(message);
            (level, timestamp)
        })
    }

    fn clean_line(&self, line: &str) -> String {
        COLOR_CODE_PATTERN.replace_all(line, "").into_owned()
    }
}

/// Parse qconsole timestamp "YYYY-MM-DD HH:MM:SS" into NaiveDateTime
fn parse_qconsole_timestamp(ts: &str) -> Option<NaiveDateTime> {
    NaiveDateTime::parse_from_str(ts, "%Y-%m-%d %H:%M:%S").ok()
}

/// Infer log level from message content
fn infer_level(message: &str) -> LogLevel {
    // Strip color codes for level detection
    let clean = COLOR_CODE_PATTERN.replace_all(message, "");
    let clean = clean.trim();

    // Error patterns
    if clean.contains("^~^~^ Script Error") || clean.starts_with("Error") {
        return LogLevel::Error;
    }

    // Warn patterns
    if clean.starts_with("WARNING:")
        || clean.starts_with("^~^~^ Warning:")
        || clean.starts_with("Hitch warning:")
        || clean.starts_with("^~^~^")
    {
        return LogLevel::Warn;
    }

    LogLevel::Info
}
