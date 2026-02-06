use crate::app::{App, DateFilterField, FocusState};
use crate::log_entry::LogLevel;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
};

/// Main UI drawing function
pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    // Main layout: log view + status bar
    let chunks = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(area);

    draw_log_view(frame, app, chunks[0]);
    draw_status_bar(frame, app, chunks[1]);

    // Draw overlays on top
    match &app.focus {
        FocusState::Normal => {}
        FocusState::CommandPalette { .. } => draw_command_palette(frame, app),
        FocusState::Search { .. } => draw_search_bar(frame, app),
        FocusState::DateFilter { .. } => draw_date_filter(frame, app),
        FocusState::FileOpen { .. } => draw_file_open(frame, app),
    }
}

/// Draw the main log view
fn draw_log_view(frame: &mut Frame, app: &mut App, area: Rect) {
    let viewport_height = area.height as usize;
    let viewport_width = area.width as usize;

    if app.wrap_enabled {
        draw_log_view_wrapped(frame, app, area, viewport_height, viewport_width);
    } else {
        draw_log_view_nowrap(frame, app, area, viewport_height);
    }
}

/// Draw log view without wrapping (manual rendering for expand support)
fn draw_log_view_nowrap(frame: &mut Frame, app: &mut App, area: Rect, viewport_height: usize) {
    app.ensure_selected_visible_with_height(viewport_height);

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
            Span::raw(display_msg),
        ];

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
                let line = Line::from(vec![
                    Span::raw("              "),             // timestamp placeholder
                    Span::styled("     ", Style::default()), // level placeholder
                    Span::raw(" "),
                    Span::styled(display, Style::default().fg(Color::DarkGray)),
                ]);
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
) {
    // For wrapped mode, we need to calculate how many visual lines each entry takes
    // and handle scrolling based on visual lines, not entries

    // Prefix width: timestamp (14) + level badge (5) + space (1) = 20 chars
    let prefix_width = 20;
    let msg_width = viewport_width.saturating_sub(prefix_width);
    if msg_width == 0 {
        return;
    }

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
                    Span::raw(part.clone()),
                ];
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
                Line::from(vec![
                    Span::raw(" ".repeat(prefix_width)),
                    Span::raw(part.clone()),
                ])
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
                    let line = Line::from(vec![
                        Span::raw(" ".repeat(prefix_width)),
                        Span::styled(part, Style::default().fg(Color::DarkGray)),
                    ]);
                    visual_lines.push((line, is_selected, entry.level));
                }
            }
        }

        current_entry_idx += 1;
    }

    // Update app state for proper selection visibility
    app.ensure_selected_visible_with_height(viewport_height);

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

/// Draw status bar
fn draw_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let total = app.entries.len();
    let shown = app.filtered_indices.len();
    let levels = app.active_levels_display();
    let tail = if app.tail_enabled {
        "Tail ON"
    } else {
        "Tail OFF"
    };

    let file_display = if app.file_path.is_empty() {
        "No file".to_string()
    } else {
        // Show just filename
        std::path::Path::new(&app.file_path)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| app.file_path.clone())
    };

    let left = format!(
        " {} | {}/{} | {} | {} ",
        file_display, shown, total, levels, tail
    );
    let right = "Ctrl+P ";

    let left_len = left.len();
    let right_len = right.len();
    let padding = (area.width as usize).saturating_sub(left_len + right_len);

    let line = Line::from(vec![
        Span::styled(left, Style::default().fg(Color::White)),
        Span::raw(" ".repeat(padding)),
        Span::styled(right, Style::default().fg(Color::DarkGray)),
    ]);

    let paragraph =
        Paragraph::new(line).style(Style::default().bg(Color::DarkGray).fg(Color::White));

    frame.render_widget(paragraph, area);
}

/// Draw command palette overlay
fn draw_command_palette(frame: &mut Frame, app: &App) {
    let area = centered_rect(50, 60, frame.area());

    // Clear the area behind
    frame.render_widget(Clear, area);

    let (input, selected) = match &app.focus {
        FocusState::CommandPalette { input, selected } => (input.as_str(), *selected),
        _ => return,
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Commands ");

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Input line
    let input_area = Rect { height: 1, ..inner };
    let input_line = Line::from(vec![
        Span::styled("> ", Style::default().fg(Color::Cyan)),
        Span::raw(input),
    ]);
    frame.render_widget(Paragraph::new(input_line), input_area);

    // Command list
    let list_area = Rect {
        y: inner.y + 1,
        height: inner.height.saturating_sub(1),
        ..inner
    };

    let commands = app.get_filtered_commands(input);
    let items: Vec<ListItem> = commands
        .iter()
        .enumerate()
        .map(|(i, (_, cmd, _))| {
            let style = if i == selected {
                Style::default().bg(Color::Cyan).fg(Color::Black)
            } else {
                Style::default()
            };

            let line = Line::from(vec![
                Span::raw("  "),
                Span::styled(cmd.name, style),
                Span::raw(" ".repeat(30usize.saturating_sub(cmd.name.len()))),
                Span::styled(cmd.shortcut, Style::default().fg(Color::DarkGray)),
            ]);

            ListItem::new(line).style(style)
        })
        .collect();

    frame.render_widget(List::new(items), list_area);
}

/// Draw search bar at top
fn draw_search_bar(frame: &mut Frame, app: &App) {
    let area = Rect {
        x: 0,
        y: 0,
        width: frame.area().width,
        height: 1,
    };

    let (query, match_count, current) = match &app.focus {
        FocusState::Search {
            query,
            match_indices,
            current_match,
        } => (query.as_str(), match_indices.len(), *current_match),
        _ => return,
    };

    let match_info = if match_count > 0 {
        format!("{}/{}", current + 1, match_count)
    } else if !query.is_empty() {
        "No matches".to_string()
    } else {
        String::new()
    };

    let line = Line::from(vec![
        Span::styled(" / ", Style::default().fg(Color::Yellow)),
        Span::raw(query),
        Span::raw(" "),
        Span::styled(match_info, Style::default().fg(Color::DarkGray)),
        Span::styled(
            " | C-n/C-p: navigate | Enter: filter | Esc: cancel ",
            Style::default().fg(Color::DarkGray),
        ),
    ]);

    let paragraph = Paragraph::new(line).style(Style::default().bg(Color::Black));

    frame.render_widget(Clear, area);
    frame.render_widget(paragraph, area);
}

/// Draw date filter dialog
fn draw_date_filter(frame: &mut Frame, app: &App) {
    let area = centered_rect(40, 30, frame.area());
    frame.render_widget(Clear, area);

    let (from, to, focused) = match &app.focus {
        FocusState::DateFilter {
            from,
            to,
            focused_field,
        } => (from.as_str(), to.as_str(), *focused_field),
        _ => return,
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Filter by Date ");

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let from_style = if focused == DateFilterField::From {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };
    let to_style = if focused == DateFilterField::To {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let lines = vec![
        Line::from(vec![Span::styled("From: ", from_style), Span::raw(from)]),
        Line::from(""),
        Line::from(vec![Span::styled("  To: ", to_style), Span::raw(to)]),
        Line::from(""),
        Line::from(Span::styled(
            "Format: MM-dd HH:mm | Tab: switch | Enter: apply",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(paragraph, inner);
}

/// Draw file open dialog
fn draw_file_open(frame: &mut Frame, app: &App) {
    let area = centered_rect(60, 50, frame.area());
    frame.render_widget(Clear, area);

    let (path, selected, cursor_pos, error) = match &app.focus {
        FocusState::FileOpen {
            path,
            selected_recent,
            cursor_pos,
            error,
        } => (path.as_str(), *selected_recent, *cursor_pos, error.as_deref()),
        _ => return,
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Open Log File ");

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Path input with cursor
    let cursor_style = Style::default().bg(Color::White).fg(Color::Black);
    let chars: Vec<char> = path.chars().collect();
    let input_line = if cursor_pos >= chars.len() {
        // Cursor at end: show text + cursor block (space with inverted style)
        Line::from(vec![
            Span::styled("Path: ", Style::default().fg(Color::Cyan)),
            Span::raw(path),
            Span::styled(" ", cursor_style),
        ])
    } else {
        // Cursor in middle: split into before | cursor_char | after
        let before: String = chars[..cursor_pos].iter().collect();
        let cursor_char: String = chars[cursor_pos].to_string();
        let after: String = chars[cursor_pos + 1..].iter().collect();
        Line::from(vec![
            Span::styled("Path: ", Style::default().fg(Color::Cyan)),
            Span::raw(before),
            Span::styled(cursor_char, cursor_style),
            Span::raw(after),
        ])
    };
    let input_area = Rect { height: 1, ..inner };
    frame.render_widget(Paragraph::new(input_line), input_area);

    // Error message
    if let Some(err) = error {
        let error_area = Rect {
            y: inner.y + 1,
            height: 1,
            ..inner
        };
        let error_line = Line::from(Span::styled(err, Style::default().fg(Color::Red)));
        frame.render_widget(Paragraph::new(error_line), error_area);
    }

    // Recent files
    if !app.recent_files.is_empty() {
        let recent_area = Rect {
            y: inner.y + 2,
            height: inner.height.saturating_sub(3), // -3: input + gap + help line
            ..inner
        };

        let header = Line::from(Span::styled(
            "Recent:",
            Style::default().fg(Color::DarkGray),
        ));
        frame.render_widget(
            Paragraph::new(header),
            Rect {
                height: 1,
                ..recent_area
            },
        );

        let list_area = Rect {
            y: recent_area.y + 1,
            height: recent_area.height.saturating_sub(1),
            ..recent_area
        };

        let typing = !path.is_empty();
        let items: Vec<ListItem> = app
            .recent_files
            .iter()
            .enumerate()
            .map(|(i, file)| {
                let style = if typing {
                    Style::default().fg(Color::DarkGray)
                } else if i == selected {
                    Style::default().bg(Color::Cyan).fg(Color::Black)
                } else {
                    Style::default()
                };
                ListItem::new(format!("  {}", file)).style(style)
            })
            .collect();

        frame.render_widget(List::new(items), list_area);
    }

    // Help text at bottom
    let help_area = Rect {
        x: inner.x,
        y: inner.y + inner.height.saturating_sub(1),
        width: inner.width,
        height: 1,
    };
    let help_line = Line::from(Span::styled(
        "Esc: cancel | Enter: open | Tab: fill | ^U: clear",
        Style::default().fg(Color::DarkGray),
    ));
    frame.render_widget(Paragraph::new(help_line), help_area);
}

/// Get color for log level
fn level_color(level: LogLevel) -> Color {
    match level {
        LogLevel::Error => Color::Red,
        LogLevel::Warn => Color::Yellow,
        LogLevel::Info => Color::White,
        LogLevel::Debug => Color::Cyan,
        LogLevel::Trace => Color::DarkGray,
        LogLevel::Profile => Color::Magenta,
        LogLevel::Unknown => Color::DarkGray,
    }
}

/// Get style for level badge
fn level_style(level: LogLevel) -> Style {
    Style::default()
        .fg(Color::Black)
        .bg(level_color(level))
        .add_modifier(Modifier::BOLD)
}

/// Extract message portion from raw log line
fn extract_message(raw_line: &str) -> &str {
    // Find the message after timestamp
    // Pattern: either after "HH:mm:ss.fff " or just return the whole line
    if let Some(pos) = raw_line.find(|c: char| c.is_ascii_digit()) {
        // Skip past timestamp pattern "MM-dd HH:mm:ss.fff"
        if raw_line.len() > pos + 18 {
            let after_ts = &raw_line[pos + 18..];
            return after_ts.trim_start();
        }
    }
    raw_line
}

/// Create a centered rect
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(area);

    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(popup_layout[1])[1]
}
