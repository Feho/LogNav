use crate::theme::Theme;
use ratatui::{
    style::{Modifier, Style},
    text::Span,
};
use regex::Regex;
use std::sync::LazyLock;

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

fn token_style(capture: &regex::Captures, base_style: Style, theme: &Theme) -> Style {
    if capture.name("url").is_some() {
        base_style
            .fg(theme.syntax_url)
            .add_modifier(Modifier::UNDERLINED)
    } else if capture.name("uuid").is_some() {
        base_style.fg(theme.syntax_uuid)
    } else if capture.name("quoted").is_some() {
        base_style.fg(theme.syntax_string)
    } else if capture.name("kv").is_some() {
        base_style.fg(theme.syntax_key_value)
    } else if capture.name("ip").is_some() {
        base_style.fg(theme.syntax_ip)
    } else if capture.name("hex").is_some() {
        base_style.fg(theme.syntax_hex)
    } else if capture.name("path").is_some() {
        base_style.fg(theme.syntax_path)
    } else if capture.name("bool").is_some() {
        base_style.fg(theme.syntax_boolean)
    } else if capture.name("error").is_some() {
        base_style.fg(theme.syntax_error_keyword)
    } else if capture.name("number").is_some() {
        base_style.fg(theme.syntax_number)
    } else {
        base_style
    }
}

/// Tokenize text into syntax-colored spans
fn syntax_highlight_spans(text: &str, base_style: Style, theme: &Theme) -> Vec<Span<'static>> {
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
            token_style(&cap, base_style, theme),
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
fn apply_search_overlay(
    spans: Vec<Span<'static>>,
    regex: &Regex,
    hl_style: Style,
) -> Vec<Span<'static>> {
    // Reconstruct full text and find matches across the whole string,
    // then walk through spans splitting at match boundaries.
    let full_text: String = spans.iter().map(|s| s.content.as_ref()).collect();
    let matches: Vec<(usize, usize)> = regex
        .find_iter(&full_text)
        .map(|m| (m.start(), m.end()))
        .collect();

    if matches.is_empty() {
        return spans;
    }

    let mut result = Vec::new();
    // byte position within full_text for the start of the current span
    let mut span_start = 0;

    for span in spans {
        let text = span.content.to_string();
        let style = span.style;
        let span_end = span_start + text.len();

        let mut pos = span_start; // current byte within full_text

        for &(m_start, m_end) in &matches {
            if m_end <= pos || m_start >= span_end {
                continue;
            }
            // Part before match (still in this span)
            let before_start = pos;
            let before_end = m_start.max(pos);
            if before_end > before_start {
                result.push(Span::styled(
                    text[before_start - span_start..before_end - span_start].to_string(),
                    style,
                ));
            }
            // Highlighted part (clipped to this span)
            let hl_start = m_start.max(pos);
            let hl_end = m_end.min(span_end);
            if hl_end > hl_start {
                result.push(Span::styled(
                    text[hl_start - span_start..hl_end - span_start].to_string(),
                    hl_style,
                ));
            }
            pos = hl_end;
        }

        // Remaining tail of span after all matches
        if pos < span_end {
            result.push(Span::styled(text[pos - span_start..].to_string(), style));
        }

        span_start = span_end;
    }

    result
}

/// Overlay underline on spans within a char range by splitting at boundaries
pub fn apply_underline(spans: Vec<Span<'static>>, start: usize, end: usize) -> Vec<Span<'static>> {
    apply_underline_overlay(spans, start, end)
}

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

/// Build styled spans with optional syntax highlighting, alert overlays, search overlay, and underline.
/// Order: base/syntax → alert patterns (each with own color) → search (always on top).
pub fn styled_spans(
    text: &str,
    hl_regex: Option<&Regex>,
    alert_patterns: &[(Regex, Style)],
    base_style: Style,
    syntax_enabled: bool,
    underline_range: Option<(usize, usize)>,
    theme: &Theme,
) -> Vec<Span<'static>> {
    let hl_style = theme.search_highlight_style();

    // Step 1: base or syntax-highlighted spans (no search yet)
    let mut spans = if syntax_enabled {
        syntax_highlight_spans(text, base_style, theme)
    } else {
        vec![Span::styled(text.to_string(), base_style)]
    };

    // Step 2: alert keyword highlights (each pattern gets its own color)
    for (regex, style) in alert_patterns {
        spans = apply_search_overlay(spans, regex, *style);
    }

    // Step 3: search highlight on top (wins over alert colors)
    if let Some(regex) = hl_regex {
        spans = apply_search_overlay(spans, regex, hl_style);
    }

    if spans.is_empty() {
        spans.push(Span::styled(text.to_string(), base_style));
    }

    if let Some((start, end)) = underline_range {
        spans = apply_underline_overlay(spans, start, end);
    }

    spans
}

/// Wrap pre-styled spans at `width` chars, preserving styles across wrap boundaries.
/// Returns one Vec<Span> per visual line.
pub fn wrap_spans(spans: Vec<Span<'static>>, width: usize) -> Vec<Vec<Span<'static>>> {
    if width == 0 {
        return vec![spans];
    }

    // Flatten spans into (char, style) pairs, then re-wrap
    let mut chars: Vec<(char, Style)> = Vec::new();
    for span in &spans {
        let style = span.style;
        for c in span.content.chars() {
            chars.push((c, style));
        }
    }

    if chars.is_empty() {
        return vec![vec![Span::raw(String::new())]];
    }

    // Word-wrap: collect word boundaries (same logic as wrap_text)
    // We work in char indices
    let mut lines: Vec<Vec<Span<'static>>> = Vec::new();
    let mut line_start = 0;
    let mut line_width = 0usize;
    let mut word_start = 0;
    let mut i = 0;

    while i <= chars.len() {
        let is_space = i < chars.len() && chars[i].0.is_whitespace();
        let is_end = i == chars.len();

        if is_space || is_end {
            let word_len = i - word_start;
            if word_len == 0 {
                if is_space {
                    // include the space
                    line_width += 1;
                    i += 1;
                    word_start = i;
                    continue;
                }
                break;
            }

            if line_width + word_len <= width {
                // Word fits on current line (include trailing space if any)
                let end = if is_space { i + 1 } else { i };
                line_width += end - word_start;
                i = end;
                word_start = i;
            } else if word_len > width {
                // Long word: flush current line, then split word across lines
                if line_width > 0 {
                    lines.push(spans_from_chars(&chars, line_start, word_start));
                }
                let mut w = word_start;
                while w < i {
                    let chunk_end = (w + width).min(i);
                    lines.push(spans_from_chars(&chars, w, chunk_end));
                    w = chunk_end;
                }
                line_start = w;
                line_width = 0;
                word_start = if is_space { i + 1 } else { i };
                i = word_start;
            } else {
                // Start new line with this word
                lines.push(spans_from_chars(&chars, line_start, word_start));
                line_start = word_start;
                let end = if is_space { i + 1 } else { i };
                line_width = end - word_start;
                i = end;
                word_start = i;
            }
        } else {
            i += 1;
        }
    }

    // Remaining chars on the last line
    if line_start < chars.len() {
        lines.push(spans_from_chars(&chars, line_start, chars.len()));
    }

    if lines.is_empty() {
        lines.push(vec![Span::raw(String::new())]);
    }

    lines
}

/// Build Vec<Span<'static>> from a slice of (char, style), merging adjacent same-style chars
fn spans_from_chars(chars: &[(char, Style)], start: usize, end: usize) -> Vec<Span<'static>> {
    let mut result: Vec<Span<'static>> = Vec::new();
    let slice = &chars[start..end];
    if slice.is_empty() {
        return result;
    }
    let mut text = String::new();
    let mut cur_style = slice[0].1;
    for &(c, style) in slice {
        if style == cur_style {
            text.push(c);
        } else {
            result.push(Span::styled(text, cur_style));
            text = String::new();
            text.push(c);
            cur_style = style;
        }
    }
    if !text.is_empty() {
        result.push(Span::styled(text, cur_style));
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_overlay_multi_span_bug() {
        let theme = Theme::dark();
        let text = "URL: http://example.com/foo.bar is here";
        // Syntax highlighting will likely pick up the URL as a separate span.
        // If we search for something that starts before the URL and ends inside it,
        // it might fail if implemented per-span.
        
        let regex = Regex::new("URL: http").unwrap();
        let spans = styled_spans(text, Some(&regex), &[], Style::default(), true, None, &theme);
        
        let hl_style = theme.search_highlight_style();
        let has_highlight = spans.iter().any(|s| s.style.fg == hl_style.fg);
        
        assert!(has_highlight, "Should highlight 'URL: http' even with syntax highlighting on");
    }
}
