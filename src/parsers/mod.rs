use crate::log_entry::{LogEntry, LogLevel};
use chrono::NaiveDateTime;
use std::sync::Arc;

mod qconsole;
mod wd;
mod wpc;

pub use qconsole::QConsoleParser;
pub use wd::WdParser;
pub use wpc::WpcParser;

/// Trait for log parsers
pub trait LogParser: Send + Sync {
    /// Detect if this parser can handle the given line
    /// Returns confidence score 0.0-1.0 (highest wins)
    fn detect(&self, first_line: &str) -> f64;

    /// Parse a single line into (level, timestamp)
    /// Returns None if line is a continuation (doesn't start a new entry)
    fn parse_line(&self, line: &str) -> Option<(LogLevel, Option<NaiveDateTime>)>;

    /// Clean a line by stripping format-specific artifacts (e.g. color codes)
    fn clean_line(&self, line: &str) -> String {
        line.to_string()
    }
}

/// Parse timestamp string into NaiveDateTime (assumes current year)
pub fn parse_timestamp(ts: &str) -> Option<NaiveDateTime> {
    const TIMESTAMP_FORMAT: &str = "%m-%d %H:%M:%S%.3f";
    let current_year = chrono::Local::now().format("%Y").to_string();
    let full_ts = format!("{}-{}", current_year, ts);
    NaiveDateTime::parse_from_str(&full_ts, &format!("%Y-{}", TIMESTAMP_FORMAT)).ok()
}

/// Get all registered parsers
pub fn all_parsers() -> Vec<Arc<dyn LogParser>> {
    vec![
        Arc::new(WdParser),
        Arc::new(WpcParser),
        Arc::new(QConsoleParser),
    ]
}

/// Detect the best parser for the given content
pub fn detect_parser(content: &str) -> Option<Arc<dyn LogParser>> {
    let parsers = all_parsers();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Find best matching parser
        let mut best_parser: Option<Arc<dyn LogParser>> = None;
        let mut best_confidence = 0.0;

        for parser in &parsers {
            let confidence = parser.detect(line);
            if confidence > best_confidence {
                best_confidence = confidence;
                best_parser = Some(Arc::clone(parser));
            }
        }

        if best_confidence > 0.0 {
            return best_parser;
        }

        // First non-comment line doesn't match any pattern
        return None;
    }

    None
}

/// Fallback parser that tries all parsers in order
pub struct FallbackParser;

impl LogParser for FallbackParser {
    fn detect(&self, _first_line: &str) -> f64 {
        0.0
    }

    fn parse_line(&self, line: &str) -> Option<(LogLevel, Option<NaiveDateTime>)> {
        // Try each parser in order
        WdParser
            .parse_line(line)
            .or_else(|| WpcParser.parse_line(line))
            .or_else(|| QConsoleParser.parse_line(line))
    }
}

/// Get the fallback parser
pub fn fallback_parser() -> Arc<dyn LogParser> {
    Arc::new(FallbackParser)
}

/// Parse content with a specific parser
pub fn parse_with_parser(content: &str, parser: &dyn LogParser) -> Vec<LogEntry> {
    let mut entries: Vec<LogEntry> = Vec::new();
    let mut index = 0;
    let mut in_header = true;

    for line in content.lines() {
        // Handle comment lines at start of file (header section)
        if in_header && line.starts_with('#') {
            continue;
        }
        in_header = false;

        // Try to parse as a new entry
        if let Some((level, timestamp)) = parser.parse_line(line) {
            entries.push(LogEntry {
                index,
                level,
                timestamp,
                raw_line: parser.clean_line(line),
                continuation_lines: Vec::new(),
                cached_full_text: None,
            });
            index += 1;
        } else if !entries.is_empty() {
            // This is a continuation line
            if let Some(last) = entries.last_mut() {
                last.add_continuation(parser.clean_line(line));
            }
        }
    }

    // Build search cache for entries with continuations
    for entry in &mut entries {
        entry.ensure_search_cache();
    }

    entries
}

/// Parse incremental content (for tailing), continuing from last entry
pub fn parse_incremental_with_parser(
    content: &str,
    parser: &dyn LogParser,
    start_index: usize,
    mut pending_continuation: Option<&mut LogEntry>,
) -> Vec<LogEntry> {
    let mut entries: Vec<LogEntry> = Vec::new();
    let mut index = start_index;

    for line in content.lines() {
        // Try to parse as a new entry
        if let Some((level, timestamp)) = parser.parse_line(line) {
            entries.push(LogEntry {
                index,
                level,
                timestamp,
                raw_line: parser.clean_line(line),
                continuation_lines: Vec::new(),
                cached_full_text: None,
            });
            index += 1;
        } else {
            // This is a continuation line
            if let Some(last) = entries.last_mut() {
                last.add_continuation(parser.clean_line(line));
            } else if let Some(pending) = pending_continuation.as_deref_mut() {
                pending.add_continuation(parser.clean_line(line));
            }
        }
    }

    // Build search cache for entries with continuations
    for entry in &mut entries {
        entry.ensure_search_cache();
    }
    // Also update pending entry's cache if it received continuations
    if let Some(pending) = pending_continuation {
        pending.ensure_search_cache();
    }

    entries
}
