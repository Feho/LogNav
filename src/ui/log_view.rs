use crate::app::{App, FocusState};
use crate::log_entry::LogLevel;
use crate::ui::{extract_message, level_color, level_style};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};
use regex::Regex;

const HIGHLIGHT_STYLE: Style = Style::new().fg(Color::Black).bg(Color::Yellow);

/// Compile regex from the live search overlay query
fn compile_overlay_regex(app: &App) -> Option<Regex> {
    if let FocusState::Search { ref query, .. } = app.focus {
        if !query.is_empty() {
            return Regex::new(&format!("(?i){}", regex::escape(query))).ok();
        }
    }
    None
}

/// Split text into owned spans, highlighting search matches
fn highlight_spans(text: &str, regex: Option<&Regex>, base_style: Style) -> Vec<Span<'static>> {
    let regex = match regex {
        Some(r) => r,
        None => return vec![Span::styled(text.to_string(), base_style)],
    };

    let mut spans = Vec::new();
    let mut last_end = 0;

    for m in regex.find_iter(text) {
        if m.start() > last_end {
            spans.push(Span::styled(text[last_end..m.start()].to_string(), base_style));
        }
        spans.push(Span::styled(text[m.start()..m.end()].to_string(), HIGHLIGHT_STYLE));
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

/// Draw the main log view
pub fn draw_log_view(frame: &mut Frame, app: &mut App, area: Rect) {
    let viewport_height = area.height as usize;
    let viewport_width = area.width as usize;

    // Store viewport height for mouse scrolling
    app.viewport_height = viewport_height;

    // Compute highlight regex once: use committed regex, or compile from overlay query
    let overlay_regex = compile_overlay_regex(app);
    let hl_regex = app.search_regex.as_ref().or(overlay_regex.as_ref())
        .cloned();
    let hl_regex_ref = hl_regex.as_ref();

    if app.wrap_enabled {
        draw_log_view_wrapped(frame, app, area, viewport_height, viewport_width, hl_regex_ref);
    } else {
        draw_log_view_nowrap(frame, app, area, viewport_height, hl_regex_ref);
    }
}

/// Draw log view without wrapping (manual rendering for expand support)
fn draw_log_view_nowrap(frame: &mut Frame, app: &mut App, area: Rect, viewport_height: usize, hl_regex: Option<&Regex>) {
    app.ensure_selected_visible_with_height(viewport_height, 0); // 0 = no wrapping

    // Build visual lines, accounting for expanded entries
    let mut visual_lines: Vec<(Line<'_>, bool, LogLevel)> = Vec::with_capacity(viewport_height);
    let mut current_entry_idx = app.scroll_offset;

    while visual_lines.len() < viewport_height && current_entry_idx < app.filtered_indices.len() {
        let entry_idx = app.filtered_indices[current_entry_idx];
        let entry = &app.entries[entry_idx];
        let is_selected = current_entry_idx == app.selected_index;
        let is_expanded = app.is_expanded(entry_idx);

        // Build the main line
        let timestamp = entry
            .timestamp
            .map(|ts| ts.format("%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "             ".to_string());

        let level_span = Span::styled(
            format!(" {} ", entry.level.short_name()),
            level_style(entry.level),
        );

        let message = extract_message(&entry.raw_line);
        let skip = app.horizontal_scroll.min(message.len());
        let display_msg: String = message.chars().skip(skip).collect();

        let mut spans = vec![
            Span::styled(timestamp, Style::default().fg(Color::DarkGray)),
            level_span,
            Span::raw(" "),
        ];
        spans.extend(highlight_spans(&display_msg, hl_regex, Style::default()));

        // Show expand indicator
        if !entry.continuation_lines.is_empty() {
            let indicator = if is_expanded {
                format!(" [-{}]", entry.continuation_lines.len())
            } else {
                format!(" [+{}]", entry.continuation_lines.len())
            };
            spans.push(Span::styled(
                indicator,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ));
        }

        visual_lines.push((Line::from(spans), is_selected, entry.level));

        // Add continuation lines if expanded
        if is_expanded {
            for cont_line in &entry.continuation_lines {
                if visual_lines.len() >= viewport_height {
                    break;
                }
                let skip = app.horizontal_scroll.min(cont_line.len());
                let display: String = cont_line.chars().skip(skip).collect();
                let cont_style = Style::default().fg(Color::DarkGray);
                let mut cont_spans = vec![
                    Span::raw("              "),             // timestamp placeholder
                    Span::styled("     ", Style::default()), // level placeholder
                    Span::raw(" "),
                ];
                cont_spans.extend(highlight_spans(&display, hl_regex, cont_style));
                let line = Line::from(cont_spans);
                visual_lines.push((line, is_selected, entry.level));
            }
        }

        current_entry_idx += 1;
    }

    // Render each visual line
    for (i, (line, is_selected, level)) in visual_lines.into_iter().enumerate() {
        let y = area.y + i as u16;
        if y >= area.y + area.height {
            break;
        }

        let line_area = Rect {
            x: area.x,
            y,
            width: area.width,
            height: 1,
        };

        let style = if is_selected {
            Style::default()
                .bg(level_color(level))
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        let paragraph = Paragraph::new(line).style(style);
        frame.render_widget(paragraph, line_area);
    }
}

/// Draw log view with word wrapping (manual line rendering)
fn draw_log_view_wrapped(
    frame: &mut Frame,
    app: &mut App,
    area: Rect,
    viewport_height: usize,
    viewport_width: usize,
    hl_regex: Option<&Regex>,
) {
    // For wrapped mode, we need to calculate how many visual lines each entry takes
    // and handle scrolling based on visual lines, not entries

    // Prefix width: timestamp (14) + level badge (5) + space (1) = 20 chars
    let prefix_width = 20;
    let msg_width = viewport_width.saturating_sub(prefix_width);
    if msg_width == 0 {
        return;
    }

    // Ensure selected entry is visible BEFORE building visual lines
    // This accounts for word wrapping when calculating visibility
    app.ensure_selected_visible_with_height(viewport_height, viewport_width);

    // Build visual lines for display, starting from scroll_offset
    let mut visual_lines: Vec<(Line<'_>, bool, LogLevel)> = Vec::with_capacity(viewport_height);
    let mut current_entry_idx = app.scroll_offset;

    while visual_lines.len() < viewport_height && current_entry_idx < app.filtered_indices.len() {
        let entry_idx = app.filtered_indices[current_entry_idx];
        let entry = &app.entries[entry_idx];
        let is_selected = current_entry_idx == app.selected_index;
        let is_expanded = app.is_expanded(entry_idx);

        let timestamp = entry
            .timestamp
            .map(|ts| ts.format("%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "             ".to_string());

        let message = extract_message(&entry.raw_line);

        // Build expand indicator separately so it can be styled
        let indicator = if !entry.continuation_lines.is_empty() {
            Some(if is_expanded {
                format!(" [-{}]", entry.continuation_lines.len())
            } else {
                format!(" [+{}]", entry.continuation_lines.len())
            })
        } else {
            None
        };

        // Wrap the main message
        let wrapped_parts = wrap_text(message, msg_width);

        for (i, part) in wrapped_parts.iter().enumerate() {
            if visual_lines.len() >= viewport_height {
                break;
            }

            let line = if i == 0 {
                // First line: show timestamp and level
                let mut spans = vec![
                    Span::styled(timestamp.clone(), Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        format!(" {} ", entry.level.short_name()),
                        level_style(entry.level),
                    ),
                    Span::raw(" "),
                ];
                spans.extend(highlight_spans(part, hl_regex, Style::default()));
                if let Some(ref ind) = indicator {
                    spans.push(Span::styled(
                        ind.clone(),
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ));
                }
                Line::from(spans)
            } else {
                // Wrapped continuation: indent to align with message
                let mut spans = vec![Span::raw(" ".repeat(prefix_width))];
                spans.extend(highlight_spans(part, hl_regex, Style::default()));
                Line::from(spans)
            };

            visual_lines.push((line, is_selected, entry.level));
        }

        // Add expanded continuation lines
        if is_expanded {
            for cont_line in &entry.continuation_lines {
                if visual_lines.len() >= viewport_height {
                    break;
                }
                let wrapped_cont = wrap_text(cont_line, msg_width);
                for part in wrapped_cont {
                    if visual_lines.len() >= viewport_height {
                        break;
                    }
                    let cont_style = Style::default().fg(Color::DarkGray);
                    let mut cont_spans = vec![Span::raw(" ".repeat(prefix_width))];
                    cont_spans.extend(highlight_spans(&part, hl_regex, cont_style));
                    let line = Line::from(cont_spans);
                    visual_lines.push((line, is_selected, entry.level));
                }
            }
        }

        current_entry_idx += 1;
    }

    // Render each visual line
    for (i, (line, is_selected, level)) in visual_lines.into_iter().enumerate() {
        let y = area.y + i as u16;
        if y >= area.y + area.height {
            break;
        }

        let line_area = Rect {
            x: area.x,
            y,
            width: area.width,
            height: 1,
        };

        let style = if is_selected {
            Style::default()
                .bg(level_color(level))
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        let paragraph = Paragraph::new(line).style(style);
        frame.render_widget(paragraph, line_area);
    }
}

/// Wrap text to fit within a given width
fn wrap_text(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![text.to_string()];
    }

    let mut result = Vec::new();
    let mut current_line = String::new();
    let mut current_width = 0;

    for word in text.split_inclusive(|c: char| c.is_whitespace()) {
        let word_width = word.chars().count();

        if current_width + word_width <= width {
            current_line.push_str(word);
            current_width += word_width;
        } else if word_width > width {
            // Word is longer than width, need to split it
            if !current_line.is_empty() {
                result.push(current_line);
                current_line = String::new();
                current_width = 0;
            }
            // Split long word
            let mut chars = word.chars().peekable();
            while chars.peek().is_some() {
                let chunk: String = chars.by_ref().take(width).collect();
                if chars.peek().is_some() {
                    result.push(chunk);
                } else {
                    current_line = chunk;
                    current_width = current_line.chars().count();
                }
            }
        } else {
            // Start new line
            if !current_line.is_empty() {
                result.push(current_line);
            }
            current_line = word.to_string();
            current_width = word_width;
        }
    }

    if !current_line.is_empty() {
        result.push(current_line);
    }

    if result.is_empty() {
        result.push(String::new());
    }

    result
}
