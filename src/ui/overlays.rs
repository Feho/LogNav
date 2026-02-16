use crate::app::{App, DateFilterFocus, FocusState, QUICK_FILTERS};
use crate::text_utils::wrap_text;
use crate::ui::{centered_rect, extract_message, level_color, render_scrollbar};
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

    let item_count = items.len();
    frame.render_widget(List::new(items), list_area);

    render_scrollbar(frame, list_area, selected, item_count);
}

/// Draw search bar at top
pub fn draw_search_bar(frame: &mut Frame, app: &App) {
    let area = Rect {
        x: 0,
        y: 0,
        width: frame.area().width,
        height: 1,
    };

    let (query, match_count, current, regex_mode, regex_error) = match &app.focus {
        FocusState::Search {
            query,
            match_indices,
            current_match,
            regex_mode,
            regex_error,
        } => (
            query.as_str(),
            match_indices.len(),
            *current_match,
            *regex_mode,
            regex_error.as_deref(),
        ),
        _ => return,
    };

    let match_info = if let Some(err) = regex_error {
        // Truncate long regex errors
        let short = if err.len() > 30 { &err[..30] } else { err };
        format!("[err: {}]", short)
    } else if match_count > 0 {
        format!("{}/{}", current + 1, match_count)
    } else if !query.is_empty() {
        "No matches".to_string()
    } else {
        String::new()
    };

    let match_info_style = if regex_error.is_some() {
        Style::default().fg(Color::Red)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let regex_indicator = if regex_mode {
        Span::styled("[.*] ", Style::default().fg(Color::Magenta))
    } else {
        Span::raw("")
    };

    let line = Line::from(vec![
        Span::styled(" / ", Style::default().fg(Color::Yellow)),
        regex_indicator,
        Span::raw(query),
        Span::raw(" "),
        Span::styled(match_info, match_info_style),
        Span::styled(
            " | Ctrl+R:regex | Enter:search | Esc:cancel",
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

    // Path input with cursor (horizontally scrolled to keep cursor visible)
    let cursor_style = Style::default().bg(Color::White).fg(Color::Black);
    let prefix = "Path: ";
    let prefix_len = prefix.len() as u16;
    let chars: Vec<char> = path.chars().collect();
    let available = inner.width.saturating_sub(prefix_len + 1) as usize; // +1 for cursor block

    // Compute scroll offset so cursor stays visible
    let scroll_offset = if available == 0 {
        0
    } else if cursor_pos >= available {
        cursor_pos - available + 1
    } else {
        0
    };

    let visible_end = (scroll_offset + available).min(chars.len());
    let visible: String = chars[scroll_offset..visible_end].iter().collect();
    let input_line = if cursor_pos >= chars.len() {
        // Cursor at end
        Line::from(vec![
            Span::styled(prefix, Style::default().fg(Color::Cyan)),
            Span::raw(visible),
            Span::styled(" ", cursor_style),
        ])
    } else {
        // Cursor in middle of visible text
        let before: String = chars[scroll_offset..cursor_pos].iter().collect();
        let cursor_char: String = chars[cursor_pos].to_string();
        let after: String = chars[cursor_pos + 1..visible_end].iter().collect();
        Line::from(vec![
            Span::styled(prefix, Style::default().fg(Color::Cyan)),
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

        let item_count = items.len();
        frame.render_widget(List::new(items), list_area);

        render_scrollbar(frame, list_area, selected, item_count);
    }

    // Help text at bottom
    let help_area = Rect {
        x: inner.x,
        y: inner.y + inner.height.saturating_sub(1),
        width: inner.width,
        height: 1,
    };
    let help_line = Line::from(Span::styled(
        "Esc:cancel | Enter:open | Tab:fill recent | Ctrl+W:delete word | Ctrl+U:clear",
        Style::default().fg(Color::DarkGray),
    ));
    frame.render_widget(Paragraph::new(help_line), help_area);
}

/// Draw help dialog with virtual scroll
pub fn draw_help(frame: &mut Frame, app: &mut App) {
    let area = centered_rect(70, 80, frame.area());
    frame.render_widget(Clear, area);

    let scroll_offset = match &app.focus {
        FocusState::Help { scroll_offset } => *scroll_offset,
        _ => return,
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" LogNav Help ");

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let help_text = vec![
        Line::from(vec![Span::styled(
            "NAVIGATION",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from("  j/\u{2193}       Next entry"),
        Line::from("  k/\u{2191}       Previous entry"),
        Line::from("  g/Home    Go to top"),
        Line::from("  G/End     Go to bottom"),
        Line::from("  e/E       Next/previous error"),
        Line::from("  w/W       Next/previous warning"),
        Line::from("  h/l/\u{2190}/\u{2192}   Scroll horizontally"),
        Line::from("  Enter     Expand/collapse entry"),
        Line::from("  a         Expand/collapse all"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "SEARCH & FILTER",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from("  / or Ctrl+F  Open search (live highlight)"),
        Line::from("  Ctrl+R       Toggle regex mode in search"),
        Line::from("  Enter        Apply search, open search results panel"),
        Line::from("  n/N          Next/previous match (vim-style)"),
        Line::from("  Tab          Switch focus between main / search results panel"),
        Line::from("  Esc          Close search results panel, clear search"),
        Line::from("  Ctrl+D       Date range filter"),
        Line::from("  0-6          Toggle levels: 0:Reset 1:ERR 2:WRN 3:INF 4:DBG 5:TRC 6:PRF"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "VIEW",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from("  Ctrl+W    Toggle word wrap"),
        Line::from("  s         Toggle syntax highlighting"),
        Line::from("  t         Toggle tail mode (auto-scroll)"),
        Line::from("  d         Show entry detail popup"),
        Line::from("  c         Copy current entry to clipboard"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "FILE",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from("  o         Open file dialog"),
        Line::from("  Tab       Fill path from recent files"),
        Line::from("  Ctrl+W    Delete word in path input"),
        Line::from("  ~         Tilde expansion for home directory"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "OTHER",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from("  Ctrl+P    Command palette (all commands)"),
        Line::from("  q         Quit (normal mode)"),
        Line::from("  ? or F1   Show this help"),
        Line::from("  Esc       Close dialog (doesn't quit)"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "LEVEL FILTERS",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from("  Press 1-6 to toggle, 0 to reset to defaults"),
        Line::from("  Default: ERR, WRN, INF, DBG are ON"),
        Line::from("  TRC (trace) and PRF (profile) are OFF"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "SEARCH MODES",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from("  Live search: / to search, matches highlighted"),
        Line::from("  Enter: opens split-screen search results panel"),
        Line::from("  n/N: jump to next/prev match (works after panel closed)"),
        Line::from("  Tab: switch focus between main view and search results"),
        Line::from("  Esc: close search results panel and clear search"),
    ];

    let visible_height = inner.height as usize;
    let total_lines = help_text.len();
    let max_scroll = total_lines.saturating_sub(visible_height);
    let scroll = scroll_offset.min(max_scroll);

    // Clamp stored offset so keyboard nav stays in range
    if let FocusState::Help { scroll_offset } = &mut app.focus {
        *scroll_offset = scroll;
    }

    // Slice to visible lines
    let visible_lines: Vec<Line> = help_text
        .into_iter()
        .skip(scroll)
        .take(visible_height)
        .collect();

    frame.render_widget(Paragraph::new(visible_lines), inner);

    render_scrollbar(frame, inner, scroll, total_lines);
}

/// Draw detail popup showing full entry text with wrapping
pub fn draw_detail_popup(frame: &mut Frame, app: &mut App) {
    let area = centered_rect(80, 70, frame.area());
    frame.render_widget(Clear, area);

    let scroll_offset = match &app.focus {
        FocusState::Detail { scroll_offset } => *scroll_offset,
        _ => return,
    };

    // Get the selected entry
    let entry = match app.selected_entry() {
        Some(e) => e,
        None => {
            // No entry selected, show empty popup
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" Entry Detail ");
            frame.render_widget(block, area);
            return;
        }
    };

    // Build title with timestamp and level
    let title = format!(
        " {} [{}] ",
        entry
            .timestamp
            .map(|ts| ts.format("%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "Unknown".to_string()),
        entry.level.short_name()
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(level_color(entry.level)))
        .title(title);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Build lines from entry
    let mut lines: Vec<Line> = Vec::new();

    // Main message
    let message = extract_message(&entry.raw_line);
    for wrapped_line in wrap_text(&message, inner.width as usize) {
        lines.push(Line::from(Span::raw(wrapped_line)));
    }

    // Continuation lines (if any)
    if !entry.continuation_lines.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Continuation lines:",
            Style::default().fg(Color::DarkGray),
        )));
        for cont_line in &entry.continuation_lines {
            for wrapped_line in wrap_text(cont_line, inner.width as usize) {
                lines.push(Line::from(Span::styled(
                    wrapped_line,
                    Style::default().fg(Color::DarkGray),
                )));
            }
        }
    }

    // Calculate visible range based on scroll
    let visible_height = inner.height as usize;
    let total_lines = lines.len();
    let max_scroll = total_lines.saturating_sub(visible_height);
    let scroll = scroll_offset.min(max_scroll);

    // Clamp stored offset so keyboard nav stays in range
    if let FocusState::Detail { scroll_offset } = &mut app.focus {
        *scroll_offset = scroll;
    }

    // Slice lines for display
    let visible_lines: Vec<Line> = lines
        .into_iter()
        .skip(scroll)
        .take(visible_height)
        .collect();

    // Render the text
    frame.render_widget(
        Paragraph::new(visible_lines).wrap(ratatui::widgets::Wrap { trim: false }),
        inner,
    );

    render_scrollbar(frame, inner, scroll, total_lines);
}
