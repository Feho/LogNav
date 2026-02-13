use ratatui::{
    style::{Color, Modifier, Style},
    text::Span,
};
use regex::Regex;
use std::sync::LazyLock;

const HIGHLIGHT_STYLE: Style = Style::new().fg(Color::Black).bg(Color::Yellow);

static SYNTAX_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
        r"(?P<url>https?://\S+)",
        r"|(?P<uuid>[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})",
        r#"|(?P<quoted>"[^"]*"|'[^']*')"#,
        r"|(?P<kv>\b[\w.]+=[^\s,;)\]]+)",
        r"|(?P<ip>\b\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}(?::\d+)?\b)",
        r"|(?P<hex>\b0x[0-9a-fA-F]+\b)",
        r"|(?P<path>(?:/[\w.\-]+){2,})",
        r"|(?P<bool>\b(?:true|false|True|False|TRUE|FALSE)\b)",
        r"|(?P<error>(?i)\b(?:error|failed|failure|exception|panic|abort|fatal|crash|timeout|refused|unauthorized|forbidden)\b)",
        r"|(?P<number>\b\d+\.?\d*(?:KB|MB|GB|TB|ms|us|ns|s|m|h|d|%)?\b)",
    ))
    .unwrap()
});

fn token_style(capture: &regex::Captures, base_style: Style) -> Style {
    if capture.name("url").is_some() {
        base_style
            .fg(Color::LightBlue)
            .add_modifier(Modifier::UNDERLINED)
    } else if capture.name("uuid").is_some() {
        base_style.fg(Color::Gray)
    } else if capture.name("quoted").is_some() {
        base_style.fg(Color::Green)
    } else if capture.name("kv").is_some() {
        base_style.fg(Color::LightYellow)
    } else if capture.name("ip").is_some() {
        base_style.fg(Color::Gray)
    } else if capture.name("hex").is_some() {
        base_style.fg(Color::LightCyan)
    } else if capture.name("path").is_some() {
        base_style.fg(Color::Blue)
    } else if capture.name("bool").is_some() {
        base_style.fg(Color::LightMagenta)
    } else if capture.name("error").is_some() {
        base_style.fg(Color::LightRed)
    } else if capture.name("number").is_some() {
        base_style.fg(Color::LightCyan)
    } else {
        base_style
    }
}

/// Tokenize text into syntax-colored spans
fn syntax_highlight_spans(text: &str, base_style: Style) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut last_end = 0;

    for cap in SYNTAX_RE.captures_iter(text) {
        let m = cap.get(0).unwrap();
        if m.start() > last_end {
            spans.push(Span::styled(
                text[last_end..m.start()].to_string(),
                base_style,
            ));
        }
        spans.push(Span::styled(
            text[m.start()..m.end()].to_string(),
            token_style(&cap, base_style),
        ));
        last_end = m.end();
    }

    if last_end < text.len() {
        spans.push(Span::styled(text[last_end..].to_string(), base_style));
    }

    if spans.is_empty() {
        spans.push(Span::styled(text.to_string(), base_style));
    }

    spans
}

/// Overlay search highlights on pre-styled spans by splitting at match boundaries
fn apply_search_overlay(spans: Vec<Span<'static>>, regex: &Regex) -> Vec<Span<'static>> {
    let mut result = Vec::new();

    for span in spans {
        let text = span.content.to_string();
        let style = span.style;
        let mut last_end = 0;

        for m in regex.find_iter(&text) {
            if m.start() > last_end {
                result.push(Span::styled(text[last_end..m.start()].to_string(), style));
            }
            result.push(Span::styled(
                text[m.start()..m.end()].to_string(),
                HIGHLIGHT_STYLE,
            ));
            last_end = m.end();
        }

        if last_end < text.len() {
            result.push(Span::styled(text[last_end..].to_string(), style));
        } else if last_end == 0 {
            result.push(Span::styled(text, style));
        }
    }

    result
}

/// Overlay underline on spans within a char range by splitting at boundaries
fn apply_underline_overlay(
    spans: Vec<Span<'static>>,
    char_start: usize,
    char_end: usize,
) -> Vec<Span<'static>> {
    let mut result = Vec::new();
    let mut pos = 0;

    for span in spans {
        let text = span.content.to_string();
        let span_len = text.chars().count();
        let span_end = pos + span_len;

        if span_end <= char_start || pos >= char_end {
            // No overlap
            result.push(Span::styled(text, span.style));
        } else {
            let chars: Vec<char> = text.chars().collect();
            let overlap_start = char_start.saturating_sub(pos);
            let overlap_end = (char_end - pos).min(span_len);

            if overlap_start > 0 {
                let before: String = chars[..overlap_start].iter().collect();
                result.push(Span::styled(before, span.style));
            }
            let mid: String = chars[overlap_start..overlap_end].iter().collect();
            result.push(Span::styled(
                mid,
                span.style.add_modifier(Modifier::UNDERLINED),
            ));
            if overlap_end < span_len {
                let after: String = chars[overlap_end..].iter().collect();
                result.push(Span::styled(after, span.style));
            }
        }

        pos = span_end;
    }

    result
}

/// Build styled spans with optional syntax highlighting, search overlay, and underline
pub fn styled_spans(
    text: &str,
    hl_regex: Option<&Regex>,
    base_style: Style,
    syntax_enabled: bool,
    underline_range: Option<(usize, usize)>,
) -> Vec<Span<'static>> {
    let mut spans = match (syntax_enabled, hl_regex) {
        (false, None) => vec![Span::styled(text.to_string(), base_style)],
        (false, Some(regex)) => {
            let mut spans = Vec::new();
            let mut last_end = 0;
            for m in regex.find_iter(text) {
                if m.start() > last_end {
                    spans.push(Span::styled(
                        text[last_end..m.start()].to_string(),
                        base_style,
                    ));
                }
                spans.push(Span::styled(
                    text[m.start()..m.end()].to_string(),
                    HIGHLIGHT_STYLE,
                ));
                last_end = m.end();
            }
            if last_end < text.len() {
                spans.push(Span::styled(text[last_end..].to_string(), base_style));
            }
            if spans.is_empty() {
                spans.push(Span::styled(text.to_string(), base_style));
            }
            spans
        }
        (true, None) => syntax_highlight_spans(text, base_style),
        (true, Some(regex)) => {
            let spans = syntax_highlight_spans(text, base_style);
            apply_search_overlay(spans, regex)
        }
    };

    if let Some((start, end)) = underline_range {
        spans = apply_underline_overlay(spans, start, end);
    }

    spans
}
