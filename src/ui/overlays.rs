use crate::app::{
    App, DateFilterFocus, FilterKind, FilterManagerFocus, FocusState, QUICK_FILTERS,
};
use crate::text_utils::wrap_text;
use crate::ui::syntax::styled_spans;
use crate::ui::{centered_rect, extract_message, level_color, render_scrollbar};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};

/// Consistent cursor style used across all input bars
const CURSOR_STYLE: Style = Style::new().bg(Color::White).fg(Color::Black);

/// Draw command palette overlay
pub fn draw_command_palette(frame: &mut Frame, app: &App) {
    let area = centered_rect(50, 60, frame.area());

    // Clear the area behind
    frame.render_widget(Clear, area);

    let (input, selected) = match &app.focus {
        FocusState::CommandPalette { input, selected } => (input, *selected),
        _ => return,
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Commands ");

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Input line with cursor
    let input_area = Rect { height: 1, ..inner };
    let prefix = "> ";
    let prefix_style = Style::default().fg(Color::Cyan);
    input.render(frame, input_area, prefix, prefix_style, CURSOR_STYLE, true);

    // Command list
    let list_area = Rect {
        y: inner.y + 1,
        height: inner.height.saturating_sub(1),
        ..inner
    };

    let commands = app.get_filtered_commands(input.text());
    let item_count = commands.len();
    let visible_height = list_area.height as usize;

    // Compute scroll offset to keep selected item visible
    let scroll = if item_count <= visible_height {
        0
    } else {
        selected.saturating_sub(visible_height.saturating_sub(1))
    };

    let items: Vec<ListItem> = commands
        .iter()
        .enumerate()
        .skip(scroll)
        .take(visible_height)
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

    render_scrollbar(frame, list_area, scroll, item_count);
}

/// Draw search bar at top
pub fn draw_search_bar(frame: &mut Frame, app: &App) {
    let area = Rect {
        x: 0,
        y: 0,
        width: frame.area().width,
        height: 1,
    };

    let (input, match_count, current, regex_mode, regex_error) = match &app.focus {
        FocusState::Search {
            input,
            match_indices,
            current_match,
            regex_mode,
            regex_error,
        } => (
            input,
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
    } else if !input.is_empty() {
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

    // Build prefix spans manually, then append input spans
    let prefix_width = 3 + if regex_mode { 5 } else { 0 }; // " / " + optional "[.*] "
    let suffix = format!(" {} | Ctrl+R:regex | Enter:search | Esc:cancel", match_info);
    let available_for_input = area
        .width
        .saturating_sub(prefix_width as u16 + suffix.len() as u16);

    let mut spans = vec![
        Span::styled(" / ", Style::default().fg(Color::Yellow)),
        regex_indicator,
    ];
    spans.extend(input.to_spans(available_for_input, CURSOR_STYLE, true));
    spans.push(Span::raw(" "));
    spans.push(Span::styled(&match_info, match_info_style));
    spans.push(Span::styled(
        " | Ctrl+R:regex | Enter:search | Esc:cancel",
        Style::default().fg(Color::DarkGray),
    ));

    let line = Line::from(spans);
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
        } => (from, to, *focus, *selected_quick, error.as_deref()),
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
        let from_active = focus == DateFilterFocus::From;
        let from_style = if from_active {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let field_area = Rect {
            x: inner.x,
            y,
            width: inner.width,
            height: 1,
        };
        from.render(
            frame,
            field_area,
            "  From: ",
            from_style,
            CURSOR_STYLE,
            from_active,
        );
        y += 1;
    }

    // To field
    if y < inner.y + inner.height {
        let to_active = focus == DateFilterFocus::To;
        let to_style = if to_active {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let field_area = Rect {
            x: inner.x,
            y,
            width: inner.width,
            height: 1,
        };
        to.render(
            frame,
            field_area,
            "    To: ",
            to_style,
            CURSOR_STYLE,
            to_active,
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

    let (input, selected, error, is_merge) = match &app.focus {
        FocusState::FileOpen {
            input,
            selected_recent,
            error,
            is_merge,
            ..
        } => (input, *selected_recent, error.as_deref(), *is_merge),
        _ => return,
    };

    let title = if is_merge {
        " Merge Log File "
    } else {
        " Open Log File "
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(title);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Path input with cursor
    let input_area = Rect { height: 1, ..inner };
    input.render(
        frame,
        input_area,
        "Path: ",
        Style::default().fg(Color::Cyan),
        CURSOR_STYLE,
        true,
    );

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

        let typing = !input.is_empty();
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
        "Esc:cancel | Enter:open | Tab:complete path | Ctrl+W:delete segment | Ctrl+U:clear",
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
        Line::from("  m         Toggle bookmark on current line"),
        Line::from("  b/B       Next/previous bookmark"),
        Line::from("  h/l/\u{2190}/\u{2192}   Scroll horizontally"),
        Line::from("  Enter     Expand/collapse entry"),
        Line::from("  a         Expand/collapse all"),
        Line::from("  Space     Fold/unfold cluster at cursor"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "SEARCH & FILTER",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from("  /         Search (live highlight, Ctrl+R for regex)"),
        Line::from("  n/N       Next/previous match"),
        Line::from("  Enter     Open search results panel (Tab to switch focus)"),
        Line::from("  Ctrl+Click  Search word under cursor"),
        Line::from("  Alt+Click   Exclude word under cursor"),
        Line::from("  1-5       Toggle levels: 1:ERR 2:WRN 3:INF 4:DBG 5:TRC"),
        Line::from("  0         Reset level filters to defaults"),
        Line::from("  Ctrl+D    Date range filter"),
        Line::from("  x/X       Exclude filter manager / clear all excludes"),
        Line::from("  i/I       Include filter manager / clear all includes"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "VIEW",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from("  Ctrl+W    Toggle word wrap"),
        Line::from("  s         Toggle syntax highlighting"),
        Line::from("  t         Toggle tail mode (auto-scroll)"),
        Line::from("  d         Show entry detail popup"),
        Line::from("  v         Visual select mode (then move to extend range)"),
        Line::from("  c         Copy current entry (or visual selection) to clipboard"),
        Line::from("  Ctrl+S    Export filtered results to file"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "OTHER",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from("  o         Open file"),
        Line::from("  M         Merge file (add to merged view)"),
        Line::from("  Ctrl+P    Command palette"),
        Line::from("  ? / F1    Show this help"),
        Line::from("  Esc       Close dialog"),
        Line::from("  q         Quit"),
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
    let syntax_on = app.syntax_highlight;

    // Main message
    let message = extract_message(&entry.raw_line, entry.message_offset);
    for wrapped_line in wrap_text(&message, inner.width as usize) {
        let spans = styled_spans(&wrapped_line, None, Style::default(), syntax_on, None);
        lines.push(Line::from(spans));
    }

    // Continuation lines (if any)
    if !entry.continuation_lines.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Continuation lines:",
            Style::default().fg(Color::DarkGray),
        )));
        let cont_style = Style::default().fg(Color::DarkGray);
        let display = entry.display_continuation();
        for cont_line in display {
            for wrapped_line in wrap_text(cont_line, inner.width as usize) {
                let spans = styled_spans(&wrapped_line, None, cont_style, syntax_on, None);
                lines.push(Line::from(spans));
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

/// Draw filter manager overlay (shared by exclude and include)
pub fn draw_filter_manager(frame: &mut Frame, app: &App) {
    let area = centered_rect(50, 55, frame.area());
    frame.render_widget(Clear, area);

    let (kind, input, selected, regex_mode, regex_error, focus) = match &app.focus {
        FocusState::FilterManager {
            kind,
            input,
            selected,
            regex_mode,
            regex_error,
            focus,
        } => (
            *kind,
            input,
            *selected,
            *regex_mode,
            regex_error.as_deref(),
            *focus,
        ),
        _ => return,
    };

    let input_focused = focus == FilterManagerFocus::Input;
    let list_focused = focus == FilterManagerFocus::List;

    // Kind-specific styling
    let (border_color, title, prefix, help_line2) = match kind {
        FilterKind::Exclude => (
            Color::Cyan,
            " Exclude Filters ",
            "  Exclude: ",
            "  Esc: close | Alt+Click word in log to exclude",
        ),
        FilterKind::Include => (
            Color::Green,
            " Include Filters ",
            "  Include: ",
            "  Esc: close | Only matching lines are shown",
        ),
    };

    let patterns = app.filter_patterns(kind);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(title);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut y = inner.y;

    // Input bar with cursor
    let input_border_color = if input_focused {
        Color::Yellow
    } else {
        Color::DarkGray
    };
    let input_label_style = Style::default().fg(input_border_color);

    let regex_indicator = if regex_mode {
        Span::styled("[.*] ", Style::default().fg(Color::Magenta))
    } else {
        Span::raw("")
    };

    let regex_prefix_len = if regex_mode { 5 } else { 0 };
    let available = inner
        .width
        .saturating_sub(prefix.len() as u16 + regex_prefix_len + 1);

    let mut spans = vec![Span::styled(prefix, input_label_style), regex_indicator];
    spans.extend(input.to_spans(available, CURSOR_STYLE, input_focused));

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

    // Error message
    if let Some(err) = regex_error {
        let short = if err.len() > 40 { &err[..40] } else { err };
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!("  Error: {}", short),
                Style::default().fg(Color::Red),
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

    y += 1; // spacer

    // List header
    let list_header_style = if list_focused {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let count = patterns.len();
    let header_text = if count == 0 {
        "  Active filters: (none)".to_string()
    } else {
        format!("  Active filters ({})", count)
    };
    if y < inner.y + inner.height {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(header_text, list_header_style))),
            Rect {
                x: inner.x,
                y,
                width: inner.width,
                height: 1,
            },
        );
        y += 1;
    }

    // List of patterns
    let list_area = Rect {
        x: inner.x,
        y,
        width: inner.width,
        height: inner.height.saturating_sub(y - inner.y).saturating_sub(2),
    };

    if count > 0 {
        let visible_height = list_area.height as usize;
        let scroll = if count <= visible_height {
            0
        } else {
            selected.saturating_sub(visible_height.saturating_sub(1))
        };

        let items: Vec<ListItem> = patterns
            .iter()
            .enumerate()
            .skip(scroll)
            .take(visible_height)
            .map(|(i, fp)| {
                let style = if list_focused && i == selected {
                    Style::default().bg(border_color).fg(Color::Black)
                } else {
                    Style::default()
                };
                ListItem::new(format!("    {}", fp.query)).style(style)
            })
            .collect();

        frame.render_widget(List::new(items), list_area);
        render_scrollbar(frame, list_area, scroll, count);
    }

    // Help text at bottom
    let help_y = inner.y + inner.height.saturating_sub(2);
    if help_y > y {
        let help1 = if list_focused {
            "  d/Del: remove selected | Tab: switch to input"
        } else {
            "  Enter: add pattern | Ctrl+R: regex | Tab: switch to list"
        };
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                help1,
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
                help_line2,
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

/// Draw export dialog overlay
pub fn draw_export_dialog(frame: &mut Frame, app: &App) {
    let area = centered_rect(60, 20, frame.area());
    frame.render_widget(Clear, area);

    let (input, error) = match &app.focus {
        FocusState::ExportDialog { input, error } => (input, error.as_deref()),
        _ => return,
    };

    let title = format!(" Export {} filtered entries ", app.filtered_indices.len());
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(title);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Path input with cursor
    let input_area = Rect { height: 1, ..inner };
    input.render(
        frame,
        input_area,
        "Path: ",
        Style::default().fg(Color::Cyan),
        CURSOR_STYLE,
        true,
    );

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
}
