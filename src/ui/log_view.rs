use crate::app::SOURCE_COLORS;
use crate::app::{App, FocusState};
use crate::log_entry::LogLevel;
use crate::text_utils::wrap_text;
use crate::ui::extract_message;
use crate::ui::syntax::styled_spans;
use crate::ui::{LINE_PREFIX_WIDTH, level_color, level_style, render_scrollbar};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};
use regex::Regex;

/// Create a colored gutter span for source file indication
fn source_gutter_span(source_idx: u8) -> Span<'static> {
    let color = SOURCE_COLORS[source_idx as usize % SOURCE_COLORS.len()];
    Span::styled("▌", Style::default().fg(color))
}

/// Compile regex from the live search overlay query
fn compile_overlay_regex(app: &App) -> Option<Regex> {
    if let FocusState::Search {
        ref query,
        regex_mode,
        ..
    } = app.focus
        && !query.is_empty()
    {
        let pattern = if regex_mode {
            format!("(?i){}", query)
        } else {
            format!("(?i){}", regex::escape(query))
        };
        return Regex::new(&pattern).ok();
    }
    None
}

/// Compute underline range for a given terminal row if hover_word matches.
/// Returns char range in display-text coordinates (after horizontal_scroll skip).
fn underline_range_for_row(app: &App, terminal_row: usize) -> Option<(usize, usize)> {
    let hover = app.hover_word.as_ref()?;
    if hover.row != terminal_row {
        return None;
    }
    let start = hover.char_start.saturating_sub(app.horizontal_scroll);
    let end = hover.char_end.saturating_sub(app.horizontal_scroll);
    if start >= end {
        return None;
    }
    Some((start, end))
}

/// Draw the main log view
pub fn draw_log_view(frame: &mut Frame, app: &mut App, area: Rect) {
    let viewport_height = area.height as usize;
    let viewport_width = area.width as usize;

    // Store viewport dimensions for mouse scrolling
    app.viewport_height = viewport_height;
    app.viewport_width = viewport_width;

    // Compute highlight regex once: use committed search regex or live overlay query
    let overlay_regex = compile_overlay_regex(app);
    let hl_regex = app
        .search
        .regex
        .as_ref()
        .or(overlay_regex.as_ref())
        .cloned();
    let hl_regex_ref = hl_regex.as_ref();

    // Show start screen when no file loaded
    if app.sources.is_empty() && app.entries.is_empty() {
        draw_start_screen(frame, area);
        return;
    }

    if app.wrap_enabled {
        draw_log_view_wrapped(
            frame,
            app,
            area,
            viewport_height,
            viewport_width,
            hl_regex_ref,
        );
    } else {
        draw_log_view_nowrap(frame, app, area, viewport_height, hl_regex_ref);
    }
}

/// Draw start screen when no file is loaded
fn draw_start_screen(frame: &mut Frame, area: Rect) {
    let lines: Vec<Line<'_>> = vec![
        Line::from(vec![
            Span::styled(
                "LogNav",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" v{}", env!("CARGO_PKG_VERSION")),
                Style::default().fg(Color::DarkGray),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("o       ", Style::default().fg(Color::Cyan)),
            Span::styled("Open file", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled("M       ", Style::default().fg(Color::Cyan)),
            Span::styled("Merge file", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled("Ctrl+p  ", Style::default().fg(Color::Cyan)),
            Span::styled("Command palette", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled("?       ", Style::default().fg(Color::Cyan)),
            Span::styled("Help", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled("q       ", Style::default().fg(Color::Cyan)),
            Span::styled("Quit", Style::default().fg(Color::DarkGray)),
        ]),
    ];

    let content_height = lines.len() as u16;
    let content_width = 25u16; // approx max line width
    let y = area.y + area.height.saturating_sub(content_height) / 2;
    let x = area.x + area.width.saturating_sub(content_width) / 2;

    let centered = Rect {
        x,
        y,
        width: content_width.min(area.width),
        height: content_height.min(area.height),
    };

    frame.render_widget(Paragraph::new(lines), centered);
}

/// Draw log view without wrapping (manual rendering for expand support)
fn draw_log_view_nowrap(
    frame: &mut Frame,
    app: &mut App,
    area: Rect,
    viewport_height: usize,
    hl_regex: Option<&Regex>,
) {
    app.ensure_selected_visible_with_height(viewport_height, 0); // 0 = no wrapping
    let syntax_on = app.syntax_highlight;
    let is_merged = app.is_merged();

    // Build visual lines, accounting for expanded entries
    let mut visual_lines: Vec<(Line<'_>, bool, LogLevel)> = Vec::with_capacity(viewport_height);
    let mut current_entry_idx = app.scroll_offset;
    let mut terminal_row = 0usize;

    while visual_lines.len() < viewport_height && current_entry_idx < app.filtered_indices.len() {
        let entry_idx = app.filtered_indices[current_entry_idx];
        let entry = &app.entries[entry_idx];
        let is_selected = current_entry_idx == app.selected_index;
        let is_expanded = app.is_expanded(entry_idx);
        let is_bookmarked = app.bookmarks.contains(&entry_idx);

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

        let ul_range = underline_range_for_row(app, terminal_row);

        let bookmark_span = if is_bookmarked {
            Span::styled(
                "●",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::raw(" ")
        };

        let mut spans = Vec::new();
        if is_merged {
            spans.push(source_gutter_span(entry.source_idx));
        }
        spans.extend([
            bookmark_span,
            Span::styled(timestamp, Style::default().fg(Color::DarkGray)),
            level_span,
            Span::raw(" "),
        ]);
        spans.extend(styled_spans(
            &display_msg,
            hl_regex,
            Style::default(),
            syntax_on && !is_selected,
            ul_range,
        ));

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
        terminal_row += 1;

        // Add continuation lines if expanded
        if is_expanded {
            for cont_line in entry.display_continuation() {
                if visual_lines.len() >= viewport_height {
                    break;
                }
                let skip = app.horizontal_scroll.min(cont_line.len());
                let display: String = cont_line.chars().skip(skip).collect();
                let cont_style = Style::default().fg(Color::DarkGray);

                let ul_range = underline_range_for_row(app, terminal_row);

                let mut cont_spans = Vec::new();
                if is_merged {
                    cont_spans.push(source_gutter_span(entry.source_idx));
                }
                cont_spans.extend([
                    Span::raw(" "),                          // bookmark placeholder
                    Span::raw("              "),             // timestamp placeholder
                    Span::styled("     ", Style::default()), // level placeholder
                    Span::raw(" "),
                ]);
                cont_spans.extend(styled_spans(
                    &display, hl_regex, cont_style, syntax_on, ul_range,
                ));
                let line = Line::from(cont_spans);
                visual_lines.push((line, false, entry.level));
                terminal_row += 1;
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

    render_scrollbar(frame, area, app.scroll_offset, app.filtered_indices.len());
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

    let is_merged = app.is_merged();
    let gutter_extra = if is_merged { 1 } else { 0 };
    let prefix_width = LINE_PREFIX_WIDTH + gutter_extra;
    let msg_width = viewport_width.saturating_sub(prefix_width);
    if msg_width == 0 {
        return;
    }

    // Ensure selected entry is visible BEFORE building visual lines
    // This accounts for word wrapping when calculating visibility
    app.ensure_selected_visible_with_height(viewport_height, viewport_width);
    let syntax_on = app.syntax_highlight;

    // Build visual lines for display, starting from scroll_offset
    let mut visual_lines: Vec<(Line<'_>, bool, LogLevel)> = Vec::with_capacity(viewport_height);
    let mut current_entry_idx = app.scroll_offset;
    let mut terminal_row = 0usize;

    while visual_lines.len() < viewport_height && current_entry_idx < app.filtered_indices.len() {
        let entry_idx = app.filtered_indices[current_entry_idx];
        let entry = &app.entries[entry_idx];
        let is_selected = current_entry_idx == app.selected_index;
        let is_expanded = app.is_expanded(entry_idx);
        let is_bookmarked = app.bookmarks.contains(&entry_idx);

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

        let bookmark_span = if is_bookmarked {
            Span::styled(
                "●",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::raw(" ")
        };

        // Wrap the main message
        let wrapped_parts = wrap_text(&message, msg_width);

        for (i, part) in wrapped_parts.iter().enumerate() {
            if visual_lines.len() >= viewport_height {
                break;
            }

            let ul_range = underline_range_for_row(app, terminal_row);

            let line = if i == 0 {
                // First line: show timestamp and level
                let mut spans = Vec::new();
                if is_merged {
                    spans.push(source_gutter_span(entry.source_idx));
                }
                spans.extend([
                    bookmark_span.clone(),
                    Span::styled(timestamp.clone(), Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        format!(" {} ", entry.level.short_name()),
                        level_style(entry.level),
                    ),
                    Span::raw(" "),
                ]);
                spans.extend(styled_spans(
                    part,
                    hl_regex,
                    Style::default(),
                    syntax_on && !is_selected,
                    ul_range,
                ));
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
                spans.extend(styled_spans(
                    part,
                    hl_regex,
                    Style::default(),
                    syntax_on && !is_selected,
                    ul_range,
                ));
                Line::from(spans)
            };

            visual_lines.push((line, is_selected, entry.level));
            terminal_row += 1;
        }

        // Add expanded continuation lines
        if is_expanded {
            for cont_line in entry.display_continuation() {
                if visual_lines.len() >= viewport_height {
                    break;
                }
                let wrapped_cont = wrap_text(cont_line, msg_width);
                for part in wrapped_cont {
                    if visual_lines.len() >= viewport_height {
                        break;
                    }
                    let cont_style = Style::default().fg(Color::DarkGray);
                    let ul_range = underline_range_for_row(app, terminal_row);
                    let mut cont_spans = vec![Span::raw(" ".repeat(prefix_width))];
                    cont_spans.extend(styled_spans(
                        &part, hl_regex, cont_style, syntax_on, ul_range,
                    ));
                    let line = Line::from(cont_spans);
                    visual_lines.push((line, false, entry.level));
                    terminal_row += 1;
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

    render_scrollbar(frame, area, app.scroll_offset, app.filtered_indices.len());
}
