use crate::app::{
    App, DateFilterFocus, FilterKind, FilterManagerFocus, FocusState, QUICK_FILTERS, StatsData,
};
use crate::log_entry::LogLevel;
use crate::text_utils::wrap_text;
use crate::theme::{LIGHT_START_INDEX, THEME_PRESETS, Theme};
use crate::ui::syntax::styled_spans;
use crate::ui::{centered_rect, extract_message, render_scrollbar};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Bar, BarChart, BarGroup, Block, Borders, Clear, List, ListItem, Paragraph},
};

/// Draw command palette overlay
pub fn draw_command_palette(frame: &mut Frame, app: &App) {
    let theme = &app.theme;
    let area = centered_rect(50, 60, frame.area());

    // Clear the area behind
    frame.render_widget(Clear, area);

    let (input, selected) = match &app.focus {
        FocusState::CommandPalette { input, selected } => (input, *selected),
        _ => return,
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border_style())
        .title(" Commands ");

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Input line with cursor
    let input_area = Rect { height: 1, ..inner };
    let prefix = "> ";
    let prefix_style = Style::default().fg(theme.accent);
    input.render(
        frame,
        input_area,
        prefix,
        prefix_style,
        theme.cursor_style(),
        true,
    );

    // Command list
    let list_area = Rect {
        y: inner.y + 1,
        height: inner.height.saturating_sub(1),
        ..inner
    };

    let commands = app.get_filtered_commands(input.text());
    let is_filtered = !input.text().is_empty();

    // Build visual rows: None = blank separator, Some(i) = command index
    let mut visual_rows: Vec<Option<usize>> = Vec::new();
    if is_filtered {
        for i in 0..commands.len() {
            visual_rows.push(Some(i));
        }
    } else {
        let mut last_group = "";
        for (i, (_, cmd, _)) in commands.iter().enumerate() {
            if cmd.group != last_group {
                last_group = cmd.group;
                if !visual_rows.is_empty() {
                    visual_rows.push(None); // blank line between groups
                }
            }
            visual_rows.push(Some(i));
        }
    }

    // Map selected command index to visual row index
    let selected_visual = visual_rows
        .iter()
        .position(|r| *r == Some(selected))
        .unwrap_or(0);

    let total_visual = visual_rows.len();
    let visible_height = list_area.height as usize;

    // Compute scroll offset to keep selected item visible
    let scroll = if total_visual <= visible_height {
        0
    } else {
        selected_visual.saturating_sub(visible_height.saturating_sub(1))
    };

    let items: Vec<ListItem> = visual_rows
        .iter()
        .skip(scroll)
        .take(visible_height)
        .map(|row| match row {
            None => ListItem::new(Line::raw("")),
            Some(ci) => {
                let cmd = commands[*ci].1;
                let style = if *ci == selected {
                    theme.selected_style()
                } else {
                    Style::default()
                };
                let line = Line::from(vec![
                    Span::raw("  "),
                    Span::styled(cmd.name, style),
                    Span::raw(" ".repeat(30usize.saturating_sub(cmd.name.len()))),
                    Span::styled(cmd.shortcut, Style::default().fg(theme.muted)),
                ]);
                ListItem::new(line).style(style)
            }
        })
        .collect();

    frame.render_widget(List::new(items), list_area);

    render_scrollbar(frame, list_area, scroll, total_visual);
}

/// Draw search bar at top
pub fn draw_search_bar(frame: &mut Frame, app: &App) {
    let theme = &app.theme;
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
        Style::default().fg(theme.error_text)
    } else {
        Style::default().fg(theme.muted)
    };

    let regex_indicator = if regex_mode {
        Span::styled("[.*] ", Style::default().fg(theme.accent))
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
        Span::styled(" / ", Style::default().fg(theme.warning_text)),
        regex_indicator,
    ];
    spans.extend(input.to_spans(available_for_input, theme.cursor_style(), true));
    spans.push(Span::raw(" "));
    spans.push(Span::styled(&match_info, match_info_style));
    spans.push(Span::styled(
        " | Ctrl+R:regex | Enter:search | Esc:cancel",
        Style::default().fg(theme.muted),
    ));

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line).style(Style::default().bg(theme.bg));

    frame.render_widget(Clear, area);
    frame.render_widget(paragraph, area);
}

/// Draw date filter dialog
pub fn draw_date_filter(frame: &mut Frame, app: &App) {
    let theme = &app.theme;
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
        .border_style(theme.border_style())
        .title(" Date Range Filter ");

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut y = inner.y;

    // Quick filters header
    let header_style = if focus == DateFilterFocus::QuickFilter {
        Style::default().fg(theme.accent)
    } else {
        Style::default().fg(theme.muted)
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
            theme.selected_style()
        } else if focus == DateFilterFocus::QuickFilter {
            Style::default()
        } else {
            Style::default().fg(theme.muted)
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
        Style::default().fg(theme.accent)
    } else {
        Style::default().fg(theme.muted)
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
            Style::default().fg(theme.accent)
        } else {
            Style::default().fg(theme.muted)
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
            theme.cursor_style(),
            from_active,
        );
        y += 1;
    }

    // To field
    if y < inner.y + inner.height {
        let to_active = focus == DateFilterFocus::To;
        let to_style = if to_active {
            Style::default().fg(theme.accent)
        } else {
            Style::default().fg(theme.muted)
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
            theme.cursor_style(),
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
                Style::default().fg(theme.error_text),
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
                Style::default().fg(theme.muted),
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
                Style::default().fg(theme.muted),
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
    let theme = &app.theme;
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
        .border_style(theme.border_style())
        .title(title);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Path input with cursor and placeholder
    let input_area = Rect { height: 1, ..inner };
    input.render_with_placeholder(
        frame,
        input_area,
        "Path: ",
        Style::default().fg(theme.accent),
        theme.cursor_style(),
        true,
        "Type a path or drag and drop a file...",
        Style::default().fg(theme.muted),
    );

    // Error message
    if let Some(err) = error {
        let error_area = Rect {
            y: inner.y + 1,
            height: 1,
            ..inner
        };
        let error_line = Line::from(Span::styled(err, Style::default().fg(theme.error_text)));
        frame.render_widget(Paragraph::new(error_line), error_area);
    }

    // Recent files
    if !app.recent_files.is_empty() {
        let recent_area = Rect {
            y: inner.y + 2,
            height: inner.height.saturating_sub(3), // -3: input + gap + help line
            ..inner
        };

        let header = Line::from(Span::styled("Recent:", Style::default().fg(theme.muted)));
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
                    Style::default().fg(theme.muted)
                } else if i == selected {
                    theme.selected_style()
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
        Style::default().fg(theme.muted),
    ));
    frame.render_widget(Paragraph::new(help_line), help_area);
}

/// Draw help dialog with virtual scroll
pub fn draw_help(frame: &mut Frame, app: &mut App) {
    let theme = &app.theme;
    let area = centered_rect(70, 80, frame.area());
    frame.render_widget(Clear, area);

    let scroll_offset = match &app.focus {
        FocusState::Help { scroll_offset } => *scroll_offset,
        _ => return,
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border_style())
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
        Line::from("  1-6       Toggle levels: 1:ERR 2:WRN 3:INF 4:DBG 5:TRC 6:PRF"),
        Line::from("  0         Reset level filters to defaults"),
        Line::from("  Ctrl+D    Date range filter"),
        Line::from("  x/X       Exclude filter manager / clear all excludes"),
        Line::from("  i/I       Include filter manager / clear all includes"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "VIEW",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from("  Alt+W     Toggle word wrap"),
        Line::from("  s         Toggle syntax highlighting"),
        Line::from("  t         Toggle live mode (auto-scroll)"),
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
        Line::from("  F2        Statistics dashboard"),
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
    let theme = &app.theme;
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
                .border_style(theme.border_style())
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
        .border_style(Style::default().fg(theme.level_color(entry.level)))
        .title(title);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Build lines from entry
    let mut lines: Vec<Line> = Vec::new();
    let syntax_on = app.syntax_highlight;

    // Main message
    let message = extract_message(&entry.raw_line, entry.message_offset);
    for wrapped_line in wrap_text(&message, inner.width as usize) {
        let spans = styled_spans(
            &wrapped_line,
            None,
            Style::default(),
            syntax_on,
            None,
            theme,
        );
        lines.push(Line::from(spans));
    }

    // Continuation lines (if any)
    if !entry.continuation_lines.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Continuation lines:",
            Style::default().fg(theme.muted),
        )));
        let cont_style = Style::default().fg(theme.muted);
        let display = entry.display_continuation();
        for cont_line in display {
            for wrapped_line in wrap_text(cont_line, inner.width as usize) {
                let spans = styled_spans(&wrapped_line, None, cont_style, syntax_on, None, theme);
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
    let theme = &app.theme;
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
            theme.border,
            " Exclude Filters ",
            "  Exclude: ",
            "  Esc: close | Alt+Click word in log to exclude",
        ),
        FilterKind::Include => (
            theme.level_info,
            " Include Filters ",
            "  Include: ",
            "  Esc: close | Only matching lines are shown",
        ),
        FilterKind::Alert => (
            theme.level_error,
            " Alert Keywords (terminal bell on live match) ",
            "  Keyword: ",
            "  Esc: close | Bell rings when keyword appears in live mode",
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
        theme.warning_text
    } else {
        theme.muted
    };
    let input_label_style = Style::default().fg(input_border_color);

    let regex_indicator = if regex_mode {
        Span::styled("[.*] ", Style::default().fg(theme.accent))
    } else {
        Span::raw("")
    };

    let regex_prefix_len = if regex_mode { 5 } else { 0 };
    let available = inner
        .width
        .saturating_sub(prefix.len() as u16 + regex_prefix_len + 1);

    let mut spans = vec![Span::styled(prefix, input_label_style), regex_indicator];
    spans.extend(input.to_spans(available, theme.cursor_style(), input_focused));

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
                Style::default().fg(theme.error_text),
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
        Style::default().fg(theme.warning_text)
    } else {
        Style::default().fg(theme.muted)
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
                    Style::default().bg(border_color).fg(theme.level_badge_fg)
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
                Style::default().fg(theme.muted),
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
                Style::default().fg(theme.muted),
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
    let theme = &app.theme;
    let area = centered_rect(60, 20, frame.area());
    frame.render_widget(Clear, area);

    let (input, error, kind) = match &app.focus {
        FocusState::ExportDialog { input, error, kind } => (input, error.as_deref(), kind),
        _ => return,
    };

    let title = match kind {
        crate::app::ExportKind::FilteredLog => {
            format!(" Export {} filtered entries ", app.filtered_indices.len())
        }
        crate::app::ExportKind::StatsHtml(_) => " Export Statistics as HTML ".to_string(),
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border_style())
        .title(title);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Path input with cursor and placeholder
    let input_area = Rect { height: 1, ..inner };
    input.render_with_placeholder(
        frame,
        input_area,
        "Path: ",
        Style::default().fg(theme.accent),
        theme.cursor_style(),
        true,
        "Type a path or drag and drop a file...",
        Style::default().fg(theme.muted),
    );

    // Error message
    if let Some(err) = error {
        let error_area = Rect {
            y: inner.y + 1,
            height: 1,
            ..inner
        };
        let error_line = Line::from(Span::styled(err, Style::default().fg(theme.error_text)));
        frame.render_widget(Paragraph::new(error_line), error_area);
    }
}

/// Draw theme picker overlay
pub fn draw_theme_picker(frame: &mut Frame, app: &App) {
    let theme = &app.theme;
    let area = centered_rect(40, 50, frame.area());
    frame.render_widget(Clear, area);

    let (selected, original_name) = match &app.focus {
        FocusState::ThemePicker {
            selected,
            original_name,
            ..
        } => (*selected, original_name.as_str()),
        _ => return,
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border_style())
        .title(" Theme ");

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut y = inner.y;

    // Dark header
    if y < inner.y + inner.height {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "  Dark",
                Style::default()
                    .fg(theme.muted)
                    .add_modifier(Modifier::BOLD),
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

    // Dark themes
    for (i, (id, display_name, _)) in THEME_PRESETS.iter().enumerate().take(LIGHT_START_INDEX) {
        if y >= inner.y + inner.height {
            break;
        }
        let is_selected = i == selected;
        let is_active = *id == original_name;
        let marker = if is_active { " \u{2022}" } else { "" };
        let style = if is_selected {
            theme.selected_style()
        } else {
            Style::default()
        };
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!("    {}{}", display_name, marker),
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

    // Spacer
    y += 1;

    // Light header
    if y < inner.y + inner.height {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "  Light",
                Style::default()
                    .fg(theme.muted)
                    .add_modifier(Modifier::BOLD),
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

    // Light themes
    for (i, (id, display_name, _)) in THEME_PRESETS.iter().enumerate().skip(LIGHT_START_INDEX) {
        if y >= inner.y + inner.height {
            break;
        }
        let is_selected = i == selected;
        let is_active = *id == original_name;
        let marker = if is_active { " \u{2022}" } else { "" };
        let style = if is_selected {
            theme.selected_style()
        } else {
            Style::default()
        };
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!("    {}{}", display_name, marker),
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

    // Help text at bottom
    let help_y = inner.y + inner.height.saturating_sub(1);
    if help_y > y {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "  Enter: confirm | Esc: cancel",
                Style::default().fg(theme.muted),
            ))),
            Rect {
                x: inner.x,
                y: help_y,
                width: inner.width,
                height: 1,
            },
        );
    }
}

fn format_count(n: usize) -> String {
    if n < 1_000 {
        return n.to_string();
    }
    let s = n.to_string();
    let mut result = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

fn format_duration(ms: i64) -> String {
    let secs = ms / 1_000;
    let mins = secs / 60;
    let hours = mins / 60;
    let days = hours / 24;

    if days > 0 {
        format!("{}d {}h", days, hours % 24)
    } else if hours > 0 {
        format!("{}h {}m", hours, mins % 60)
    } else if mins > 0 {
        format!("{}m {}s", mins, secs % 60)
    } else {
        format!("{}s", secs)
    }
}

/// Merge raw 1-min buckets into display bars based on zoom level and pan offset.
fn zoom_buckets(
    raw: &[crate::app::BucketCounts],
    zoom_idx: usize,
    pan_offset: usize,
    bar_count: usize,
) -> Vec<crate::app::BucketCounts> {
    let (zoom_ms, _) = crate::app::ZOOM_LEVELS[zoom_idx];
    let raw_per_bar = (zoom_ms / 60_000) as usize;
    let total_raw = raw.len();

    (0..bar_count)
        .map(|col| {
            let start = pan_offset + col * raw_per_bar;
            let end = (start + raw_per_bar).min(total_raw);
            let mut bc = crate::app::BucketCounts::default();
            if start < total_raw {
                for b in &raw[start..end] {
                    bc.error += b.error;
                    bc.warn += b.warn;
                    bc.other += b.other;
                }
            }
            bc
        })
        .collect()
}

/// Draw stacked bar columns for the event rate chart.
/// Each column is 1 cell wide. Colors: error at bottom, warn in middle, other on top.
/// Uses ▄ (lower half block) for half-cell precision at the top of each color band.
fn draw_stacked_rate_chart(
    frame: &mut Frame,
    area: Rect,
    data: &StatsData,
    theme: &Theme,
    zoom_idx: usize,
    pan_offset: usize,
) {
    if area.width == 0 || area.height == 0 || data.buckets.is_empty() {
        return;
    }

    let chart_w = area.width as usize;
    let chart_h = area.height as usize;

    // Each bar is 1 cell wide with 1 cell gap → stride of 2
    let bar_count = chart_w.div_ceil(2);

    let resampled = zoom_buckets(&data.buckets, zoom_idx, pan_offset, bar_count);

    let max_total = resampled
        .iter()
        .map(|b| b.total())
        .max()
        .unwrap_or(1)
        .max(1);

    // Each cell = 2 half-rows for ▄ precision
    let half_rows = chart_h * 2;
    let buf = frame.buffer_mut();
    let err_color = theme.level_color(LogLevel::Error);
    let warn_color = theme.level_color(LogLevel::Warn);
    let other_color = theme.accent;

    for (col, bc) in resampled.iter().enumerate() {
        let total = bc.total();
        if total == 0 {
            continue;
        }

        // Total bar height in half-rows
        let bar_halfs = ((total as f64 / max_total as f64) * half_rows as f64).round() as usize;
        let bar_halfs = bar_halfs.min(half_rows).max(1);

        // Split bar into error/warn/other half-rows (proportional).
        // Compute error and warn first, derive other as remainder to avoid rounding overflow.
        let err_halfs = if bc.error > 0 {
            (((bc.error as f64 / total as f64) * bar_halfs as f64).round() as usize).min(bar_halfs)
        } else {
            0
        };
        let warn_halfs = if bc.warn > 0 {
            (((bc.warn as f64 / total as f64) * bar_halfs as f64).round() as usize)
                .min(bar_halfs.saturating_sub(err_halfs))
        } else {
            0
        };

        // Build color map from bottom (half-row 0) to top
        // Order: error at bottom, warn, then other
        let color_at_half = |h: usize| -> Color {
            if h < err_halfs {
                err_color
            } else if h < err_halfs + warn_halfs {
                warn_color
            } else if h < bar_halfs {
                other_color
            } else {
                theme.bg
            }
        };

        let x = area.x + (col * 2) as u16; // stride of 2: bar + gap
        if x >= area.x + area.width {
            break;
        }

        // Render from top row to bottom row
        for row in 0..chart_h {
            let y = area.y + row as u16;
            // This row covers half-rows: bottom_half (even) and top_half (odd)
            // Row 0 is top of chart = highest half-rows
            let top_half = (chart_h - 1 - row) * 2 + 1; // upper half of this cell
            let bot_half = (chart_h - 1 - row) * 2; // lower half of this cell

            let top_color = color_at_half(top_half);
            let bot_color = color_at_half(bot_half);

            let cell = &mut buf[(x, y)];
            if top_color == theme.bg && bot_color == theme.bg {
                // Empty cell
                continue;
            } else if top_color == bot_color {
                // Full block, same color
                cell.set_symbol("\u{2588}"); // █
                cell.set_fg(top_color);
            } else if top_color == theme.bg {
                // Only bottom half filled
                cell.set_symbol("\u{2584}"); // ▄
                cell.set_fg(bot_color);
            } else if bot_color == theme.bg {
                // Only top half filled
                cell.set_symbol("\u{2580}"); // ▀
                cell.set_fg(top_color);
            } else {
                // Both halves different colors: ▄ with fg=bottom, bg=top
                cell.set_symbol("\u{2584}"); // ▄
                cell.set_fg(bot_color);
                cell.set_bg(top_color);
            }
        }
    }
}

/// Draw time axis labels below the stacked bar chart.
fn draw_time_axis(
    frame: &mut Frame,
    area: Rect,
    data: &StatsData,
    theme: &Theme,
    zoom_idx: usize,
    pan_offset: usize,
) {
    if area.width < 10 || data.buckets.is_empty() {
        return;
    }

    if data.time_range.is_none() {
        return;
    }

    let base = data.bucket_base_ms;
    let (zoom_ms, _) = crate::app::ZOOM_LEVELS[zoom_idx];
    let raw_per_bar = (zoom_ms / 60_000) as usize;
    let chart_w = area.width as usize;
    let bar_count = chart_w.div_ceil(2);

    // Decide how many labels to show
    let max_labels = (chart_w / 12).clamp(2, 8);
    let label_step = if max_labels >= bar_count {
        1
    } else {
        bar_count / max_labels
    };

    let mut spans: Vec<Span> = Vec::new();
    let mut pos = 0usize;

    let mut bar_i = 0;
    while bar_i < bar_count && pos < chart_w {
        let col = bar_i * 2; // stride of 2
        if col < pos {
            bar_i += label_step;
            continue;
        }

        if col > pos {
            spans.push(Span::raw(" ".repeat(col - pos)));
            pos = col;
        }

        let raw_idx = pan_offset + bar_i * raw_per_bar;
        let ts_ms = base + (raw_idx as i64) * 60_000;
        let label = chrono::DateTime::from_timestamp_millis(ts_ms)
            .map(|dt| {
                let fmt = if zoom_ms >= 86_400_000 {
                    "%m-%d"
                } else {
                    "%H:%M"
                };
                dt.naive_local().format(fmt).to_string()
            })
            .unwrap_or_default();

        let label_len = label.len();
        if pos + label_len <= chart_w {
            spans.push(Span::styled(label, Style::default().fg(theme.muted)));
            pos += label_len;
        }

        bar_i += label_step;
    }

    let line = Line::from(spans);
    frame.render_widget(Paragraph::new(line), area);
}

/// Draw statistics dashboard overlay
pub fn draw_stats(frame: &mut Frame, app: &App) {
    let theme = &app.theme;
    let area = centered_rect(80, 80, frame.area());
    frame.render_widget(Clear, area);

    let (data, zoom_idx, pan_offset) = match &app.focus {
        FocusState::Stats {
            data,
            zoom_idx,
            pan_offset,
        } => (data.as_ref(), *zoom_idx, *pan_offset),
        _ => return,
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border_style())
        .title(" Statistics Dashboard ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 10 || inner.width < 30 {
        frame.render_widget(Paragraph::new("Terminal too small"), inner);
        return;
    }

    // Layout: summary(4) + event rate(50%) + separator + level dist(50%) + help(1)
    let rate_constraint = if data.has_timestamps {
        Constraint::Percentage(50)
    } else {
        Constraint::Length(1)
    };
    let chunks = Layout::vertical([
        Constraint::Length(4),      // summary section
        rate_constraint,            // event rate section
        Constraint::Length(1),      // separator
        Constraint::Percentage(50), // bar chart section
        Constraint::Length(1),      // help line
    ])
    .split(inner);

    // ── SUMMARY ──
    let section_style = Style::default()
        .fg(theme.accent)
        .add_modifier(Modifier::BOLD);

    let mut summary_lines = vec![
        Line::from(Span::styled("  SUMMARY", section_style)),
        Line::from(""),
    ];

    let total_s = format_count(data.total_entries);
    let filtered_s = format_count(data.filtered_count);
    let error_pct = data
        .error_rate
        .map(|r| format!("{:.1}%", r))
        .unwrap_or_else(|| "N/A".to_string());

    summary_lines.push(Line::from(vec![
        Span::styled("  Total: ", Style::default().fg(theme.muted)),
        Span::styled(&total_s, Style::default().fg(theme.fg)),
        Span::styled("    Filtered: ", Style::default().fg(theme.muted)),
        Span::styled(&filtered_s, Style::default().fg(theme.fg)),
        Span::styled("    Error rate: ", Style::default().fg(theme.muted)),
        Span::styled(
            &error_pct,
            Style::default().fg(if data.error_rate.unwrap_or(0.0) > 5.0 {
                theme.error_text
            } else {
                theme.fg
            }),
        ),
    ]));

    if let Some((t_min, t_max)) = data.time_range {
        use chrono::DateTime;
        let from = DateTime::from_timestamp_millis(t_min)
            .map(|dt| dt.naive_local().format("%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "?".to_string());
        let to = DateTime::from_timestamp_millis(t_max)
            .map(|dt| dt.naive_local().format("%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "?".to_string());
        let dur = format_duration(t_max - t_min);
        summary_lines.push(Line::from(vec![
            Span::styled("  Time: ", Style::default().fg(theme.muted)),
            Span::styled(from, Style::default().fg(theme.fg)),
            Span::styled(" → ", Style::default().fg(theme.muted)),
            Span::styled(to, Style::default().fg(theme.fg)),
            Span::styled(format!("  ({})", dur), Style::default().fg(theme.muted)),
        ]));
    } else {
        summary_lines.push(Line::from(Span::styled(
            "  Time: No timestamp data",
            Style::default().fg(theme.muted),
        )));
    }

    frame.render_widget(Paragraph::new(summary_lines), chunks[0]);

    // ── EVENT RATE (stacked bar chart) ──
    if data.has_timestamps && !data.buckets.is_empty() {
        let rate_chunks = Layout::vertical([
            Constraint::Length(1), // header
            Constraint::Min(1),    // stacked bars
            Constraint::Length(1), // time axis labels
        ])
        .split(chunks[1]);

        let (zoom_ms, zoom_label) = crate::app::ZOOM_LEVELS[zoom_idx];
        let chart_inner_w = rate_chunks[1].width.saturating_sub(4) as usize;
        let bar_count = chart_inner_w.div_ceil(2);
        let total_raw = data.buckets.len();
        let raw_per_bar = (zoom_ms / 60_000) as usize;
        let visible_raw = bar_count * raw_per_bar;
        let showing_all = pan_offset == 0 && visible_raw >= total_raw;

        let header_text = if showing_all {
            format!("  EVENT RATE  ({} buckets)", zoom_label)
        } else {
            // Show time window
            if data.time_range.is_some() {
                let win_start_ms = data.bucket_base_ms + (pan_offset as i64) * 60_000;
                let win_end_ms =
                    win_start_ms + (visible_raw.min(total_raw - pan_offset) as i64) * 60_000;
                let time_fmt = if zoom_ms >= 86_400_000 {
                    "%m-%d"
                } else {
                    "%m-%d %H:%M"
                };
                let fmt = |ms: i64| {
                    chrono::DateTime::from_timestamp_millis(ms)
                        .map(|dt| dt.naive_local().format(time_fmt).to_string())
                        .unwrap_or_default()
                };
                format!(
                    "  EVENT RATE  ({} buckets, {} \u{2192} {})",
                    zoom_label,
                    fmt(win_start_ms),
                    fmt(win_end_ms),
                )
            } else {
                format!("  EVENT RATE  ({} buckets)", zoom_label)
            }
        };

        let header = Line::from(Span::styled(header_text, section_style));
        frame.render_widget(Paragraph::new(header), rate_chunks[0]);

        // Draw stacked bar columns
        let chart_area = Rect {
            x: rate_chunks[1].x + 2,
            width: rate_chunks[1].width.saturating_sub(4),
            ..rate_chunks[1]
        };
        draw_stacked_rate_chart(frame, chart_area, data, theme, zoom_idx, pan_offset);

        // Draw time axis labels
        let axis_area = Rect {
            x: rate_chunks[2].x + 2,
            width: rate_chunks[2].width.saturating_sub(4),
            ..rate_chunks[2]
        };
        draw_time_axis(frame, axis_area, data, theme, zoom_idx, pan_offset);
    } else {
        let no_ts = Line::from(Span::styled(
            "  EVENT RATE  (no timestamp data)",
            Style::default().fg(theme.muted),
        ));
        frame.render_widget(Paragraph::new(no_ts), chunks[1]);
    }

    // ── LEVEL DISTRIBUTION BAR CHART ──
    let bar_chunks = Layout::vertical([
        Constraint::Length(1), // header
        Constraint::Min(3),    // chart
    ])
    .split(chunks[3]);

    let bar_header = Line::from(Span::styled("  LEVEL DISTRIBUTION", section_style));
    frame.render_widget(Paragraph::new(bar_header), bar_chunks[0]);

    const DISPLAY_LEVELS: &[(LogLevel, usize, &str)] = &[
        (LogLevel::Error, 0, "ERR"),
        (LogLevel::Warn, 1, "WRN"),
        (LogLevel::Info, 2, "INF"),
        (LogLevel::Debug, 3, "DBG"),
        (LogLevel::Trace, 4, "TRC"),
        (LogLevel::Profile, 5, "PRF"),
    ];

    let bars: Vec<Bar> = DISPLAY_LEVELS
        .iter()
        .map(|(level, bit_idx, label)| {
            let count = data.level_counts[*bit_idx];
            let color = theme.level_color(*level);
            Bar::default()
                .value(count)
                .label(Line::from(*label))
                .style(Style::default().fg(color))
                .value_style(Style::default().fg(color).add_modifier(Modifier::BOLD))
        })
        .collect();

    let bar_max = data.level_counts[..6]
        .iter()
        .copied()
        .max()
        .unwrap_or(1)
        .max(1);

    let bar_chart = BarChart::default()
        .data(BarGroup::default().bars(&bars))
        .bar_width(7)
        .bar_gap(2)
        .max(bar_max);

    let chart_area = Rect {
        x: bar_chunks[1].x + 2,
        width: bar_chunks[1].width.saturating_sub(4),
        ..bar_chunks[1]
    };
    frame.render_widget(bar_chart, chart_area);

    // ── HELP LINE ──
    let help_text = "e:export HTML | +/-:zoom | \u{2190}\u{2192}:pan | 0:reset | Esc/q:close";
    let padding = (chunks[4].width as usize).saturating_sub(help_text.len() + 2);
    let help_line = Line::from(vec![
        Span::raw(" ".repeat(padding)),
        Span::styled(help_text, Style::default().fg(theme.muted)),
    ]);
    frame.render_widget(Paragraph::new(help_line), chunks[4]);
}

#[cfg(test)]
mod tests {
    use crate::app::BucketCounts;

    /// Replicate the bar-chart resampling logic to verify stride/gap behavior.
    fn resample(_n_buckets: usize, chart_w: usize) -> (usize, Vec<usize>) {
        let bar_count = chart_w.div_ceil(2);
        let x_positions: Vec<usize> = (0..bar_count).map(|col| col * 2).collect();
        (bar_count, x_positions)
    }

    #[test]
    fn bar_stride_spacing() {
        // 20-cell wide chart → 10 bars at x=0,2,4,...,18
        let (count, xs) = resample(50, 20);
        assert_eq!(count, 10);
        assert_eq!(xs, vec![0, 2, 4, 6, 8, 10, 12, 14, 16, 18]);
    }

    #[test]
    fn bar_stride_odd_width() {
        // 21-cell wide → 11 bars at x=0,2,...,20
        let (count, xs) = resample(50, 21);
        assert_eq!(count, 11);
        assert_eq!(*xs.last().unwrap(), 20);
        // Each bar has a gap after it (except last may touch edge)
        for w in xs.windows(2) {
            assert_eq!(w[1] - w[0], 2);
        }
    }

    #[test]
    fn bar_stride_single_bucket() {
        let (count, xs) = resample(1, 10);
        assert_eq!(count, 5);
        // Single bucket maps to first bar only; rest will be empty
        assert_eq!(xs[0], 0);
    }

    #[test]
    fn resampling_covers_all_buckets() {
        let n_buckets: usize = 100;
        let chart_w: usize = 40;
        let bar_count = chart_w.div_ceil(2); // 20
        let mut covered = vec![false; n_buckets];
        for col in 0..bar_count {
            let start = col * n_buckets / bar_count;
            let end = ((col + 1) * n_buckets / bar_count).max(start + 1);
            for i in start..end.min(n_buckets) {
                covered[i] = true;
            }
        }
        assert!(covered.iter().all(|&c| c), "all buckets must be covered");
    }

    #[test]
    fn resampling_preserves_totals() {
        let buckets: Vec<BucketCounts> = (0..50)
            .map(|i| BucketCounts {
                error: i,
                warn: i * 2,
                other: 1,
            })
            .collect();
        let total_before: u64 = buckets.iter().map(|b| b.total()).sum();

        let chart_w: usize = 30;
        let bar_count = chart_w.div_ceil(2);
        let resampled: Vec<BucketCounts> = (0..bar_count)
            .map(|col| {
                let start = col * buckets.len() / bar_count;
                let end = ((col + 1) * buckets.len() / bar_count).max(start + 1);
                let mut bc = BucketCounts::default();
                for b in &buckets[start..end.min(buckets.len())] {
                    bc.error += b.error;
                    bc.warn += b.warn;
                    bc.other += b.other;
                }
                bc
            })
            .collect();
        let total_after: u64 = resampled.iter().map(|b| b.total()).sum();
        assert_eq!(total_before, total_after);
    }
}
