use ratatui::{
    style::{Color, Style},
    text::Span,
};
use regex::Regex;
use std::sync::LazyLock;

/// Color scheme for syntax highlighting (non-conflicting with level colors)
const STRING_COLOR: Color = Color::Green;
const COMPONENT_COLOR: Color = Color::LightBlue;
const THREAD_COLOR: Color = Color::Cyan;
const NUMBER_COLOR: Color = Color::LightMagenta;
const KEY_COLOR: Color = Color::LightYellow;
const URL_COLOR: Color = Color::LightCyan;
const ERROR_COLOR: Color = Color::LightRed;
const PATH_COLOR: Color = Color::LightGreen;

/// Styles for syntax elements
const STRING_STYLE: Style = Style::new().fg(STRING_COLOR);
const COMPONENT_STYLE: Style = Style::new().fg(COMPONENT_COLOR);
const THREAD_STYLE: Style = Style::new().fg(THREAD_COLOR);
const NUMBER_STYLE: Style = Style::new().fg(NUMBER_COLOR);
const KEY_STYLE: Style = Style::new().fg(KEY_COLOR);
const URL_STYLE: Style = Style::new().fg(URL_COLOR);
const ERROR_STYLE: Style = Style::new().fg(ERROR_COLOR);
const PATH_STYLE: Style = Style::new().fg(PATH_COLOR);

/// Compiled regex patterns for syntax elements
static QUOTED_STRING_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#""([^"\\]|\\.)*""#).unwrap());

static COMPONENT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b[A-Z][a-zA-Z0-9]*(?:\|[A-Z][a-zA-Z0-9]*)+\b").unwrap());

static THREAD_ID_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[[A-Z]?#?\d+(?:\|[A-Z]?#?\d+)*\]").unwrap());

static NUMBER_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b\d+(?:\.\d+)?(?:[KMGT]B?|ms|s|h)?\b").unwrap());

static KEY_VALUE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b([a-zA-Z_][a-zA-Z0-9_.]*)\s*=").unwrap());

static URL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(?:https?://|ftp://|file://|www\.)\S+\b|\b(?:\d{1,3}\.){3}\d{1,3}(?::\d+)?\b")
        .unwrap()
});

static UUID_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}\b")
        .unwrap()
});

static FILE_PATH_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:[a-zA-Z]:)?[\\/][\w\\/\-\.]+\.[a-zA-Z0-9]+|\b[\w\-/]+\.[a-zA-Z0-9]+\b").unwrap()
});

static ERROR_KEYWORDS_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(?:error|failed|failure|exception|panic|abort|fatal|crash|timeout|refused|unauthorized|forbidden)\b", ).unwrap()
});

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TokenType {
    QuotedString,
    Component,
    ThreadId,
    Number,
    Key,
    Url,
    Uuid,
    FilePath,
    ErrorKeyword,
}

#[derive(Debug)]
struct Token {
    start: usize,
    end: usize,
    token_type: TokenType,
}

/// Syntax highlight a message string, returning styled spans
pub fn highlight_syntax(text: &str, base_style: Style) -> Vec<Span<'static>> {
    if text.is_empty() {
        return vec![Span::styled(text.to_string(), base_style)];
    }

    let tokens = tokenize(text);
    let mut spans = Vec::new();
    let mut last_end = 0;

    for token in tokens {
        // Add plain text between tokens
        if token.start > last_end {
            spans.push(Span::styled(
                text[last_end..token.start].to_string(),
                base_style,
            ));
        }

        // Add styled token
        let style = match token.token_type {
            TokenType::QuotedString => STRING_STYLE,
            TokenType::Component => COMPONENT_STYLE,
            TokenType::ThreadId => THREAD_STYLE,
            TokenType::Number => NUMBER_STYLE,
            TokenType::Key => KEY_STYLE,
            TokenType::Url => URL_STYLE,
            TokenType::Uuid => PATH_STYLE,
            TokenType::FilePath => PATH_STYLE,
            TokenType::ErrorKeyword => ERROR_STYLE,
        };

        spans.push(Span::styled(
            text[token.start..token.end].to_string(),
            style,
        ));
        last_end = token.end;
    }

    // Add remaining plain text
    if last_end < text.len() {
        spans.push(Span::styled(text[last_end..].to_string(), base_style));
    }

    if spans.is_empty() {
        spans.push(Span::styled(text.to_string(), base_style));
    }

    spans
}

/// Tokenize text into syntax-highlighted segments
fn tokenize(text: &str) -> Vec<Token> {
    let mut tokens: Vec<Token> = Vec::new();

    // Find all quoted strings first (highest priority)
    for m in QUOTED_STRING_RE.find_iter(text) {
        tokens.push(Token {
            start: m.start(),
            end: m.end(),
            token_type: TokenType::QuotedString,
        });
    }

    // Find other patterns, avoiding overlaps with existing tokens
    find_non_overlapping(&mut tokens, text, &COMPONENT_RE, TokenType::Component);
    find_non_overlapping(&mut tokens, text, &THREAD_ID_RE, TokenType::ThreadId);
    find_non_overlapping(&mut tokens, text, &URL_RE, TokenType::Url);
    find_non_overlapping(&mut tokens, text, &UUID_RE, TokenType::Uuid);
    find_non_overlapping(&mut tokens, text, &FILE_PATH_RE, TokenType::FilePath);

    // Find key names in key=value patterns
    for cap in KEY_VALUE_RE.captures_iter(text) {
        if let Some(m) = cap.get(1) && !is_overlapping(&tokens, m.start(), m.end()) {
            tokens.push(Token {
                start: m.start(),
                end: m.end(),
                token_type: TokenType::Key,
            });
        }
    }

    // Find numbers (avoid matches inside quoted strings or other tokens)
    for m in NUMBER_RE.find_iter(text) {
        if !is_overlapping(&tokens, m.start(), m.end()) {
            tokens.push(Token {
                start: m.start(),
                end: m.end(),
                token_type: TokenType::Number,
            });
        }
    }

    // Find error keywords
    for m in ERROR_KEYWORDS_RE.find_iter(text) {
        if !is_overlapping(&tokens, m.start(), m.end()) {
            tokens.push(Token {
                start: m.start(),
                end: m.end(),
                token_type: TokenType::ErrorKeyword,
            });
        }
    }

    // Sort by start position
    tokens.sort_by_key(|t| t.start);
    tokens
}

/// Find regex matches that don't overlap with existing tokens
fn find_non_overlapping(
    existing: &mut Vec<Token>,
    text: &str,
    regex: &Regex,
    token_type: TokenType,
) {
    for m in regex.find_iter(text) {
        if !is_overlapping(existing, m.start(), m.end()) {
            existing.push(Token {
                start: m.start(),
                end: m.end(),
                token_type,
            });
        }
    }
}

/// Check if a range overlaps with any existing token
fn is_overlapping(tokens: &[Token], start: usize, end: usize) -> bool {
    tokens.iter().any(|t| start < t.end && end > t.start)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highlight_quoted_string() {
        let text = r#"Message: "hello world" done"#;
        let spans = highlight_syntax(text, Style::default());
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[1].content, r#""hello world""#);
    }

    #[test]
    fn test_highlight_component() {
        let text = "HTTP|DspWebConnection message";
        let spans = highlight_syntax(text, Style::default());
        assert!(spans.iter().any(|s| s.content == "HTTP|DspWebConnection"));
    }

    #[test]
    fn test_highlight_thread_id() {
        let text = "Processing [T32289|#6] data";
        let spans = highlight_syntax(text, Style::default());
        assert!(spans.iter().any(|s| s.content == "[T32289|#6]"));
    }

    #[test]
    fn test_highlight_url() {
        let text = "Connecting to http://example.com/api";
        let spans = highlight_syntax(text, Style::default());
        assert!(spans.iter().any(|s| s.content == "http://example.com/api"));
    }

    #[test]
    fn test_highlight_ip() {
        let text = "Server at 192.168.1.1:8080 ready";
        let spans = highlight_syntax(text, Style::default());
        assert!(spans.iter().any(|s| s.content == "192.168.1.1:8080"));
    }

    #[test]
    fn test_highlight_uuid() {
        let text = "ID: 550e8400-e29b-41d4-a716-446655440000 found";
        let spans = highlight_syntax(text, Style::default());
        assert!(spans
            .iter()
            .any(|s| s.content == "550e8400-e29b-41d4-a716-446655440000"));
    }

    #[test]
    fn test_highlight_key_value() {
        let text = "config.timeout = 5000";
        let spans = highlight_syntax(text, Style::default());
        assert!(spans.iter().any(|s| s.content == "config.timeout"));
    }

    #[test]
    fn test_highlight_number() {
        let text = "Count: 42 items";
        let spans = highlight_syntax(text, Style::default());
        assert!(spans.iter().any(|s| s.content == "42"));
    }

    #[test]
    fn test_highlight_error_keyword() {
        let text = "Operation failed with error";
        let spans = highlight_syntax(text, Style::default());
        assert!(spans.iter().any(|s| s.content == "failed"));
        assert!(spans.iter().any(|s| s.content == "error"));
    }

    #[test]
    fn test_highlight_file_path() {
        let text = "Loading config/data/settings.json";
        let spans = highlight_syntax(text, Style::default());
        assert!(spans
            .iter()
            .any(|s| s.content == "config/data/settings.json"));
    }

    #[test]
    fn test_precedence_quoted_over_number() {
        // Numbers inside quoted strings should not be highlighted separately
        let text = r#""value = 123""#;
        let spans = highlight_syntax(text, Style::default());
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].content, r#""value = 123""#);
    }

    #[test]
    fn test_empty_string() {
        let spans = highlight_syntax("", Style::default());
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].content, "");
    }
}
