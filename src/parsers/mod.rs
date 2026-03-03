use crate::log_entry::{LogEntry, LogLevel};
use chrono::NaiveDateTime;
use std::borrow::Cow;
use std::sync::Arc;

mod common;
mod custom;
mod generic;
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

    /// Return the byte offset where the message portion starts in the line
    /// (after timestamp, level, and any metadata like thread IDs).
    /// Returns None to fall back to heuristic extraction.
    fn message_start(&self, _line: &str) -> Option<usize> {
        None
    }

    /// Clean a line by stripping format-specific artifacts (e.g. color codes)
    fn clean_line<'a>(&self, line: &'a str) -> Cow<'a, str> {
        Cow::Borrowed(line)
    }
}

/// Parse timestamp string into NaiveDateTime (assumes current year)
pub fn parse_timestamp(ts: &str) -> Option<NaiveDateTime> {
    const TIMESTAMP_FORMAT: &str = "%Y-%m-%d %H:%M:%S%.3f";
    let full_ts = format!("{}-{}", current_year_str(), ts);
    NaiveDateTime::parse_from_str(&full_ts, TIMESTAMP_FORMAT).ok()
}

/// Cached current year string to avoid repeated formatting
fn current_year_str() -> String {
    use chrono::Datelike;
    thread_local! {
        static CACHED: std::cell::Cell<(i32, [u8; 4])> = const { std::cell::Cell::new((0, [0; 4])) };
    }
    let year = chrono::Local::now().year();
    CACHED.with(|c| {
        let (cached_year, cached_bytes) = c.get();
        if cached_year == year {
            unsafe { String::from_utf8_unchecked(cached_bytes[..4].to_vec()) }
        } else {
            let s = year.to_string();
            let mut bytes = [0u8; 4];
            bytes.copy_from_slice(s.as_bytes());
            c.set((year, bytes));
            s
        }
    })
}

/// Get all registered parsers (built-in + custom)
pub fn all_parsers() -> Vec<Arc<dyn LogParser>> {
    let mut parsers: Vec<Arc<dyn LogParser>> = vec![
        Arc::new(WdParser),
        Arc::new(WpcParser),
        Arc::new(QConsoleParser),
    ];
    parsers.extend(custom::load_custom_parsers());
    parsers
}

/// Detect the best parser for the given content
pub fn detect_parser(content: &str) -> Option<Arc<dyn LogParser>> {
    let parsers = all_parsers();

    // Collect sample lines for generic parser fallback
    let sample_lines: Vec<&str> = content
        .lines()
        .filter(|l| {
            let t = l.trim();
            !t.is_empty() && !t.starts_with('#')
        })
        .take(20)
        .collect();

    // Try built-in + custom parsers on first non-comment line
    if let Some(first_line) = sample_lines.first() {
        let mut best_parser: Option<Arc<dyn LogParser>> = None;
        let mut best_confidence = 0.0;

        for parser in &parsers {
            let confidence = parser.detect(first_line);
            if confidence > best_confidence {
                best_confidence = confidence;
                best_parser = Some(Arc::clone(parser));
            }
        }

        if best_confidence > 0.0 {
            return best_parser;
        }
    }

    // Before trying generic, check if any built-in parser can handle the sample lines
    // (handles cases where first line doesn't match but subsequent lines do)
    for parser in &parsers {
        let match_count = sample_lines
            .iter()
            .filter(|l| parser.parse_line(l).is_some())
            .count();
        if match_count > sample_lines.len() / 3 {
            return Some(Arc::clone(parser));
        }
    }

    // Fall back to generic parser (learns from sample lines)
    generic::GenericParser::learn(&sample_lines).map(|p| Arc::new(p) as Arc<dyn LogParser>)
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

/// Parse content with a specific parser (used by tests)
#[cfg(test)]
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
            let clean = parser.clean_line(line);
            let msg_off = parser.message_start(&clean);
            entries.push(LogEntry {
                index,
                level,
                timestamp,
                raw_line: clean.into_owned(),
                continuation_lines: Vec::new(),
                cached_full_text: None,
                pretty_continuation: None,
                source_idx: 0,
                source_local_idx: index,
                message_offset: msg_off,
            });
            index += 1;
        } else if !entries.is_empty() {
            // This is a continuation line
            if let Some(last) = entries.last_mut() {
                last.add_continuation(parser.clean_line(line).into_owned());
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
            let clean = parser.clean_line(line);
            let msg_off = parser.message_start(&clean);
            entries.push(LogEntry {
                index,
                level,
                timestamp,
                raw_line: clean.into_owned(),
                continuation_lines: Vec::new(),
                cached_full_text: None,
                pretty_continuation: None,
                source_idx: 0,
                source_local_idx: index,
                message_offset: msg_off,
            });
            index += 1;
        } else {
            // This is a continuation line
            if let Some(last) = entries.last_mut() {
                last.add_continuation(parser.clean_line(line).into_owned());
            } else if let Some(pending) = pending_continuation.as_deref_mut() {
                pending.add_continuation(parser.clean_line(line).into_owned());
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
