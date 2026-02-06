use crate::app::{App, DateFilterFocus, FocusState, QUICK_FILTERS};
use crate::ui::centered_rect;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};

/// Draw command palette overlay
pub fn draw_command_palette(frame: &mut Frame, app: &App) {
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
pub fn draw_search_bar(frame: &mut Frame, app: &App) {
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
pub fn draw_date_filter(frame: &mut Frame, app: &App) {
    let area = centered_rect(50, 55, frame.area());
    frame.render_widget(Clear, area);

    let (from, to, focus, selected_quick, error) = match &app.focus {
        FocusState::DateFilter {
            from,
            to,
            focus,
            selected_quick,
            error,
        } => (
            from.as_str(),
            to.as_str(),
            *focus,
            *selected_quick,
            error.as_deref(),
        ),
        _ => return,
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Date Range Filter ");

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut y = inner.y;

    // Quick filters header
    let header_style = if focus == DateFilterFocus::QuickFilter {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled("Quick Filters:", header_style))),
        Rect {
            x: inner.x,
            y,
            width: inner.width,
            height: 1,
        },
    );
    y += 1;

    // Quick filter options
    for (i, name) in QUICK_FILTERS.iter().enumerate() {
        if y >= inner.y + inner.height {
            break;
        }

        let is_selected = focus == DateFilterFocus::QuickFilter && i == selected_quick;
        let style = if is_selected {
            Style::default().bg(Color::Cyan).fg(Color::Black)
        } else if focus == DateFilterFocus::QuickFilter {
            Style::default()
        } else {
            Style::default().fg(Color::DarkGray)
        };

        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!("  {}. {}", i + 1, name),
                style,
            ))),
            Rect {
                x: inner.x,
                y,
                width: inner.width,
                height: 1,
            },
        );
        y += 1;
    }

    y += 1; // Spacer

    // Custom range header
    let custom_style = if matches!(focus, DateFilterFocus::From | DateFilterFocus::To) {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    if y < inner.y + inner.height {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled("Custom range:", custom_style))),
            Rect {
                x: inner.x,
                y,
                width: inner.width,
                height: 1,
            },
        );
        y += 1;
    }

    // From field
    if y < inner.y + inner.height {
        let from_style = if focus == DateFilterFocus::From {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let mut spans = vec![Span::styled("  From: ", from_style), Span::raw(from)];
        if focus == DateFilterFocus::From {
            spans.push(Span::styled(
                "_",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::SLOW_BLINK),
            ));
        }
        frame.render_widget(
            Paragraph::new(Line::from(spans)),
            Rect {
                x: inner.x,
                y,
                width: inner.width,
                height: 1,
            },
        );
        y += 1;
    }

    // To field
    if y < inner.y + inner.height {
        let to_style = if focus == DateFilterFocus::To {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let mut spans = vec![Span::styled("    To: ", to_style), Span::raw(to)];
        if focus == DateFilterFocus::To {
            spans.push(Span::styled(
                "_",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::SLOW_BLINK),
            ));
        }
        frame.render_widget(
            Paragraph::new(Line::from(spans)),
            Rect {
                x: inner.x,
                y,
                width: inner.width,
                height: 1,
            },
        );
        y += 1;
    }

    // Error message
    if let Some(err) = error
        && y < inner.y + inner.height
    {
        y += 1;
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!("  {}", err),
                Style::default().fg(Color::Red),
            ))),
            Rect {
                x: inner.x,
                y,
                width: inner.width,
                height: 1,
            },
        );
    }

    // Help text at bottom
    let help_y = inner.y + inner.height.saturating_sub(2);
    if help_y > y {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "  Formats: MM-dd HH:mm, -2h, today, now",
                Style::default().fg(Color::DarkGray),
            ))),
            Rect {
                x: inner.x,
                y: help_y,
                width: inner.width,
                height: 1,
            },
        );
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "  Tab: switch | Enter: apply | Esc: close",
                Style::default().fg(Color::DarkGray),
            ))),
            Rect {
                x: inner.x,
                y: help_y + 1,
                width: inner.width,
                height: 1,
            },
        );
    }
}

/// Draw file open dialog
pub fn draw_file_open(frame: &mut Frame, app: &App) {
    let area = centered_rect(60, 50, frame.area());
    frame.render_widget(Clear, area);

    let (path, selected, cursor_pos, error) = match &app.focus {
        FocusState::FileOpen {
            path,
            selected_recent,
            cursor_pos,
            error,
        } => (
            path.as_str(),
            *selected_recent,
            *cursor_pos,
            error.as_deref(),
        ),
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
