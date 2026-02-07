use chrono::NaiveDateTime;

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
    /// Cached full text for search (raw_line + continuation_lines joined)
    pub(crate) cached_full_text: Option<String>,
}

impl LogEntry {
    /// Get full text, using cache if available
    pub fn full_text(&mut self) -> &str {
        if self.cached_full_text.is_none() {
            self.cached_full_text = Some(if self.continuation_lines.is_empty() {
                self.raw_line.clone()
            } else {
                let mut text = self.raw_line.clone();
                for line in &self.continuation_lines {
                    text.push('\n');
                    text.push_str(line);
                }
                text
            });
        }
        self.cached_full_text.as_ref().unwrap()
    }

    /// Get searchable text - includes continuation lines
    pub fn searchable_text(&self) -> &str {
        // Cache should be populated during parsing via ensure_search_cache()
        self.cached_full_text.as_deref().unwrap_or(&self.raw_line)
    }

    /// Ensure the search cache is populated (call after parsing)
    pub fn ensure_search_cache(&mut self) {
        if self.cached_full_text.is_none() && !self.continuation_lines.is_empty() {
            let mut text = self.raw_line.clone();
            for line in &self.continuation_lines {
                text.push('\n');
                text.push_str(line);
            }
            self.cached_full_text = Some(text);
        }
    }

    /// Add a continuation line, invalidating the cache
    pub fn add_continuation(&mut self, line: String) {
        self.cached_full_text = None;
        self.continuation_lines.push(line);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsers::{detect_parser, fallback_parser, parse_with_parser};

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum LogFormat {
        WdLog,
        WpcLog,
        Unknown,
    }

    fn detect_format(content: &str) -> LogFormat {
        match detect_parser(content).as_deref().map(|p| p.name()) {
            Some("wd.log") => LogFormat::WdLog,
            Some("wpc.log") => LogFormat::WpcLog,
            _ => LogFormat::Unknown,
        }
    }

    fn parse_log(content: &str) -> Vec<LogEntry> {
        let parser = detect_parser(content).unwrap_or_else(fallback_parser);
        parse_with_parser(content, &*parser)
    }

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
    fn test_searchable_text_includes_continuation() {
        let content =
            "  INFO  02-03 18:11:11.526 [#32] Test \"first line\nsecond line\nthird line\"";
        let entries = parse_log(content);
        assert_eq!(entries.len(), 1);
        let search_text = entries[0].searchable_text();
        assert!(search_text.contains("first line"));
        assert!(search_text.contains("second line"));
        assert!(search_text.contains("third line"));
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

    #[test]
    fn test_parse_wpc_log_verbose() {
        let content = "VRB 03-21 14:23:01.234 Verbose message";
        let entries = parse_log(content);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].level, LogLevel::Trace);
    }

    #[test]
    fn test_parse_wpc_log_vrb_not_continuation() {
        let content = "INF 03-21 14:23:01.234 Info message\nVRB 03-21 14:23:01.235 Verbose message";
        let entries = parse_log(content);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].level, LogLevel::Info);
        assert_eq!(entries[1].level, LogLevel::Trace);
    }
}
