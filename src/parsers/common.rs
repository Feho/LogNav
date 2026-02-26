use crate::log_entry::LogLevel;

/// Map common level strings to LogLevel
pub fn map_level_str(s: &str) -> LogLevel {
    match s.to_uppercase().as_str() {
        "TRACE" | "TRC" | "FINEST" | "VERBOSE" | "VRB" | "VERB" => LogLevel::Trace,
        "DEBUG" | "DBG" | "FINE" => LogLevel::Debug,
        "INFO" | "INF" | "INFORMATION" => LogLevel::Info,
        "WARN" | "WRN" | "WARNING" => LogLevel::Warn,
        "ERROR" | "ERR" | "SEVERE" | "FATAL" | "CRITICAL" | "CRIT" => LogLevel::Error,
        _ => LogLevel::Unknown,
    }
}
