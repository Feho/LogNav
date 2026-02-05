use crate::app::{App, DateFilterField, FocusState};
use crate::log_entry::LogLevel;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
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
    app.ensure_selected_visible_with_height(viewport_height);

    let items: Vec<ListItem> = app
        .filtered_indices
        .iter()
        .skip(app.scroll_offset)
        .take(viewport_height)
        .enumerate()
        .map(|(view_idx, &entry_idx)| {
            let entry = &app.entries[entry_idx];
            let is_selected = app.scroll_offset + view_idx == app.selected_index;

            // Build the display line
            let timestamp = entry
                .timestamp
                .map(|ts| ts.format("%m-%d %H:%M:%S").to_string())
                .unwrap_or_else(|| "             ".to_string());

            let level_span = Span::styled(
                format!(" {} ", entry.level.short_name()),
                level_style(entry.level),
            );

            // Get message content (after timestamp and level in raw line)
            let message = extract_message(&entry.raw_line);

            // Handle horizontal scroll and wrap
            let display_msg = if app.wrap_enabled {
                message.to_string()
            } else {
                let skip = app.horizontal_scroll.min(message.len());
                message.chars().skip(skip).collect()
            };

            let mut spans = vec![
                Span::styled(timestamp, Style::default().fg(Color::DarkGray)),
                level_span,
                Span::raw(" "),
                Span::raw(display_msg),
            ];

            // Show continuation indicator
            if !entry.continuation_lines.is_empty() {
                spans.push(Span::styled(
                    format!(" [+{}]", entry.continuation_lines.len()),
                    Style::default().fg(Color::DarkGray),
                ));
            }

            let line = Line::from(spans);

            let style = if is_selected {
                Style::default()
                    .bg(level_color(entry.level))
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            ListItem::new(line).style(style)
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, area);
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
            " | n/N: next/prev | Enter: close ",
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

    let (path, selected) = match &app.focus {
        FocusState::FileOpen {
            path,
            selected_recent,
        } => (path.as_str(), *selected_recent),
        _ => return,
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Open Log File ");

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Path input
    let input_line = Line::from(vec![
        Span::styled("Path: ", Style::default().fg(Color::Cyan)),
        Span::raw(path),
    ]);
    let input_area = Rect { height: 1, ..inner };
    frame.render_widget(Paragraph::new(input_line), input_area);

    // Recent files
    if !app.recent_files.is_empty() {
        let recent_area = Rect {
            y: inner.y + 2,
            height: inner.height.saturating_sub(2),
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

        let items: Vec<ListItem> = app
            .recent_files
            .iter()
            .enumerate()
            .map(|(i, file)| {
                let style = if i == selected {
                    Style::default().bg(Color::Cyan).fg(Color::Black)
                } else {
                    Style::default()
                };
                ListItem::new(format!("  {}", file)).style(style)
            })
            .collect();

        frame.render_widget(List::new(items), list_area);
    }
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
