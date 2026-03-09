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
    /// Bit index for bitmask filtering (matches level_filters array order)
    pub fn filter_bit_index(self) -> u8 {
        match self {
            LogLevel::Error => 0,
            LogLevel::Warn => 1,
            LogLevel::Info => 2,
            LogLevel::Debug => 3,
            LogLevel::Trace => 4,
            LogLevel::Profile => 5,
            LogLevel::Unknown => 6,
        }
    }

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
    /// Pretty-printed continuation lines (lazily populated on first expand)
    pub pretty_continuation: Option<Vec<String>>,
    /// Index into App::sources identifying which file this entry came from
    pub source_idx: u8,
    /// Entry's ordinal position within its source file (stable across merges)
    pub source_local_idx: usize,
    /// Byte offset where the message portion starts in raw_line (after timestamp/level/metadata)
    pub message_offset: Option<usize>,
}

impl LogEntry {
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

    /// Add a continuation line, invalidating caches
    pub fn add_continuation(&mut self, line: String) {
        self.cached_full_text = None;
        self.pretty_continuation = None;
        self.continuation_lines.push(line);
    }

    /// Detect and pretty-print JSON in continuation lines. Idempotent.
    pub fn ensure_pretty_continuation(&mut self) {
        if self.pretty_continuation.is_some() || self.continuation_lines.is_empty() {
            return;
        }

        // Check if the main line ends with '{' or '[' (JSON starts on the entry line)
        let raw_trimmed = self.raw_line.trim_end();
        let json_prefix = if raw_trimmed.ends_with('{') {
            Some("{")
        } else if raw_trimmed.ends_with('[') {
            Some("[")
        } else {
            None
        };

        // Try joining all continuation lines (with prefix from main line if applicable)
        let joined = self.continuation_lines.join("\n");

        // Strategy 1: with prefix from main line
        if let Some(prefix) = json_prefix {
            let combined = format!("{}\n{}", prefix, joined);
            let trimmed = combined.trim();
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed)
                && let Ok(pretty) = serde_json::to_string_pretty(&val)
            {
                // Skip the first line if it's the prefix we injected (e.g. "{")
                // — it already appears at the end of raw_line
                let mut lines = pretty.lines().map(str::to_owned);
                if lines.next().as_deref() == Some(prefix) {
                    self.pretty_continuation = Some(lines.collect());
                } else {
                    self.pretty_continuation = Some(pretty.lines().map(str::to_owned).collect());
                }
                return;
            }
        }

        // Strategy 2: continuation lines alone as a single JSON blob
        let trimmed = joined.trim();
        if (trimmed.starts_with('{') || trimmed.starts_with('['))
            && let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed)
            && let Ok(pretty) = serde_json::to_string_pretty(&val)
        {
            self.pretty_continuation = Some(pretty.lines().map(str::to_owned).collect());
            return;
        }

        // Strategy 3: per-line detection, pass non-JSON verbatim
        let mut result: Vec<String> = Vec::new();
        let mut any_json = false;
        for line in &self.continuation_lines {
            let t = line.trim();
            if (t.starts_with('{') || t.starts_with('['))
                && let Ok(val) = serde_json::from_str::<serde_json::Value>(t)
                && let Ok(pretty) = serde_json::to_string_pretty(&val)
            {
                result.extend(pretty.lines().map(str::to_owned));
                any_json = true;
            } else {
                result.push(line.clone());
            }
        }

        self.pretty_continuation = if any_json { Some(result) } else { None };
    }

    /// Get display lines for expanded view (pretty if available, raw otherwise)
    pub fn display_continuation(&self) -> &[String] {
        self.pretty_continuation
            .as_deref()
            .unwrap_or(&self.continuation_lines)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsers::{
        LogParser, QConsoleParser, WdParser, WpcParser, detect_parser, fallback_parser,
        parse_with_parser,
    };

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum LogFormat {
        WdLog,
        WpcLog,
        QConsole,
        Unknown,
    }

    fn detect_format(content: &str) -> LogFormat {
        if detect_parser(content).is_none() {
            return LogFormat::Unknown;
        }
        let first_line = content
            .lines()
            .find(|l| !l.trim().is_empty() && !l.starts_with('#'))
            .unwrap_or("");
        if WdParser.detect(first_line) > 0.0 {
            LogFormat::WdLog
        } else if WpcParser.detect(first_line) > 0.0 {
            LogFormat::WpcLog
        } else if QConsoleParser.detect(first_line) > 0.0 {
            LogFormat::QConsole
        } else {
            LogFormat::Unknown
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
    fn test_parse_wd_log_audit() {
        let content = "? AUDIT 02-12 11:37:17.453 [#111] AUTH|MetaAuthority \"msg\"";
        let entries = parse_log(content);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].level, LogLevel::Trace);
    }

    #[test]
    fn test_parse_wd_log_fatal() {
        let content = "**FATAL 02-12 11:37:20.688 [#47] SQL|SqlProxy \"msg\"";
        let entries = parse_log(content);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].level, LogLevel::Error);
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

    // --- QConsole format tests ---

    #[test]
    fn test_detect_qconsole_format() {
        let content = "[2026-01-09 18:48:38 UTC+1.000] logfile opened on Fri Jan  9 18:48:38 2026";
        assert_eq!(detect_format(content), LogFormat::QConsole);
    }

    #[test]
    fn test_parse_qconsole_timestamp() {
        let content = "[2026-01-09 18:48:38 UTC+1.000] some message";
        let entries = parse_log(content);
        assert_eq!(entries.len(), 1);
        let ts = entries[0].timestamp.unwrap();
        assert_eq!(ts.to_string(), "2026-01-09 18:48:38");
    }

    #[test]
    fn test_parse_qconsole_info() {
        let content = "[2026-01-09 18:48:38 UTC+1.000] Cvar_Set2: sv_hostname Test";
        let entries = parse_log(content);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].level, LogLevel::Info);
    }

    #[test]
    fn test_parse_qconsole_error() {
        let content = "[2026-01-09 19:05:01 UTC+1.000] ^~^~^ Script Error : Can't find 'file.scr'";
        let entries = parse_log(content);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].level, LogLevel::Error);
    }

    #[test]
    fn test_parse_qconsole_warn_tiki() {
        let content =
            "[2026-01-09 18:48:39 UTC+1.000] ^~^~^ TIKI_InitTiki: Couldn't load model.tik";
        let entries = parse_log(content);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].level, LogLevel::Warn);
    }

    #[test]
    fn test_parse_qconsole_warn_warning() {
        let content = "[2026-01-09 18:48:39 UTC+1.000] WARNING: Couldn't find voting options file";
        let entries = parse_log(content);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].level, LogLevel::Warn);
    }

    #[test]
    fn test_parse_qconsole_color_codes_stripped() {
        let content = "[2026-01-09 18:48:52 UTC+1.000] ^3Player Dimitri_47 is under fire!";
        let entries = parse_log(content);
        assert_eq!(entries.len(), 1);
        assert!(!entries[0].raw_line.contains("^3"));
        assert!(
            entries[0]
                .raw_line
                .contains("Player Dimitri_47 is under fire!")
        );
    }

    #[test]
    fn test_parse_qconsole_continuation_lines() {
        let content = "[2026-01-09 19:05:01 UTC+1.000] ^~^~^ Script Error : Can't find 'file.scr'\n\t\texec global/ac/console_feedback.scr\n\t\t^\n[2026-01-09 19:05:01 UTC+1.000] next entry";
        let entries = parse_log(content);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].continuation_lines.len(), 2);
    }

    #[test]
    fn test_parse_qconsole_negative_utc_offset() {
        let content = "[2026-01-09 18:48:38 UTC-5.000] some message";
        let entries = parse_log(content);
        assert_eq!(entries.len(), 1);
        let ts = entries[0].timestamp.unwrap();
        assert_eq!(ts.to_string(), "2026-01-09 18:48:38");
    }

    #[test]
    fn test_pretty_json_duplication() {
        let mut entry = LogEntry {
            index: 0,
            level: LogLevel::Info,
            timestamp: None,
            raw_line: "INFO {".to_string(),
            continuation_lines: vec!["  \"foo\": \"bar\"".to_string(), "}".to_string()],
            cached_full_text: None,
            pretty_continuation: None,
            source_idx: 0,
            source_local_idx: 0,
            message_offset: None,
        };

        entry.ensure_pretty_continuation();
        let display = entry.display_continuation();
        // If duplicated, display[0] will be "{" which matches the end of raw_line
        assert_ne!(display[0], "{", "Should not duplicate the opening brace in continuation");
    }
}
