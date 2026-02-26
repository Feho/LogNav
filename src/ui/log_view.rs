use crate::app::SOURCE_COLORS;
use crate::app::{App, FocusState};
use crate::clusters::display_template;
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
        ref input,
        regex_mode,
        ..
    } = app.focus
        && !input.text().is_empty()
    {
        let query = input.text();
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

/// Compute the fixed gutter width needed for cluster annotations.
/// Delegates to App method; kept as a thin wrapper for rendering code.
fn cluster_gutter_width(app: &App) -> usize {
    app.cluster_gutter_width()
}

/// Build cluster gutter span, padded to `width`.
/// offset==0 → "▼N×", last → "└  ", middle → "│  "
fn cluster_gutter_span(
    app: &App,
    cluster_id: usize,
    offset: usize,
    occ_len: usize,
    width: usize,
) -> Span<'static> {
    let style = Style::default().fg(Color::DarkGray);
    if offset == 0 {
        let count = app.clusters[cluster_id].count;
        let label = format!("▼{}×", count);
        Span::styled(format!("{:<width$}", label, width = width), style)
    } else if offset == occ_len - 1 {
        Span::styled(format!("└{:<width$}", "", width = width - 1), style)
    } else {
        Span::styled(format!("│{:<width$}", "", width = width - 1), style)
    }
}

/// Build a blank cluster gutter span for non-clustered lines
fn cluster_gutter_blank(width: usize) -> Span<'static> {
    Span::raw(" ".repeat(width))
}

/// Build a continuation gutter span: "│" if inside a cluster (not the last
/// entry), blank otherwise. Used for wrapped-continuation and expanded lines.
fn cluster_continuation_span(
    cluster_info: Option<(usize, usize, usize)>,
    cg_width: usize,
) -> Span<'static> {
    let is_last = cluster_info.is_some_and(|(_, off, gl)| off == gl - 1);
    if cluster_info.is_some() && !is_last {
        Span::styled(
            format!("│{:<width$}", "", width = cg_width - 1),
            Style::default().fg(Color::DarkGray),
        )
    } else {
        cluster_gutter_blank(cg_width)
    }
}

/// Build a fold summary line (rendered as the entry's own row, no extra row)
fn cluster_fold_line(app: &App, cluster_id: usize, occurrence_len: usize) -> Line<'static> {
    let cluster = &app.clusters[cluster_id];
    let tmpl = display_template(&cluster.template);
    let hidden = occurrence_len.saturating_sub(1);
    let text = if cluster.sequence_len > 1 {
        format!(
            "▶ {}× [{} lines] {} ({} lines hidden)",
            cluster.count, cluster.sequence_len, tmpl, hidden
        )
    } else {
        format!("▶ {}× {} ({} lines hidden)", cluster.count, tmpl, hidden)
    };
    Line::from(Span::styled(text, Style::default().fg(Color::DarkGray)))
}

/// Check if a filtered entry is a folded interior (should be skipped)
fn is_folded_interior(app: &App, filtered_idx: usize) -> bool {
    if let Some(&(cluster_id, offset, _)) = app.cluster_map.get(&filtered_idx) {
        offset > 0 && app.folded_clusters.contains(&cluster_id)
    } else {
        false
    }
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
        draw_start_screen(frame, app, area);
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
fn draw_start_screen(frame: &mut Frame, app: &mut App, area: Rect) {
    let tip = app.tips_manager.get_current_tip().to_string();

    let hints: Vec<Line<'_>> = vec![
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

    let tips: Vec<Line<'_>> = vec![
        Line::from(vec![
            Span::styled(
                "Tip: ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::DIM),
            ),
            Span::styled(tip, Style::default().fg(Color::Gray)),
        ]),
        Line::from(vec![Span::styled(
            "Press Space for next tip",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::DIM),
        )]),
    ];

    let hints_height = hints.len() as u16;
    let tips_height = tips.len() as u16;
    let gap: u16 = 1;
    let total_height = hints_height + gap + tips_height;

    let hints_width = hints.iter().map(|l| l.width() as u16).max().unwrap_or(25);
    let tips_width = tips.iter().map(|l| l.width() as u16).max().unwrap_or(25);

    let top_y = area.y + area.height.saturating_sub(total_height) / 2;

    let hints_rect = Rect {
        x: area.x + area.width.saturating_sub(hints_width) / 2,
        y: top_y,
        width: hints_width.min(area.width),
        height: hints_height.min(area.height),
    };
    let tips_rect = Rect {
        x: area.x + area.width.saturating_sub(tips_width) / 2,
        y: top_y + hints_height + gap,
        width: tips_width.min(area.width),
        height: tips_height.min(area.height),
    };

    frame.render_widget(Paragraph::new(hints), hints_rect);
    frame.render_widget(Paragraph::new(tips), tips_rect);
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
    let cg_width = cluster_gutter_width(app);

    // Build visual lines: (line, is_selected, level)
    let mut visual_lines: Vec<(Line<'_>, bool, LogLevel)> = Vec::with_capacity(viewport_height);
    let mut current_entry_idx = app.scroll_offset;
    let mut terminal_row = 0usize;

    while visual_lines.len() < viewport_height && current_entry_idx < app.filtered_indices.len() {
        // Skip folded interior entries
        if is_folded_interior(app, current_entry_idx) {
            current_entry_idx += 1;
            continue;
        }

        let cluster_info = app.cluster_map.get(&current_entry_idx).copied();
        let is_folded = cluster_info
            .is_some_and(|(cid, off, _)| off == 0 && app.folded_clusters.contains(&cid));

        // Folded cluster: render summary as the entry's own row
        if let Some((cluster_id, 0, group_len)) = cluster_info
            && is_folded
        {
            let is_selected = current_entry_idx == app.selected_index;
            visual_lines.push((
                cluster_fold_line(app, cluster_id, group_len),
                is_selected,
                LogLevel::Info,
            ));
            terminal_row += 1;
            current_entry_idx += 1;
            continue;
        }

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
        // Source gutter first, then cluster gutter
        if is_merged {
            spans.push(source_gutter_span(entry.source_idx));
        }
        if cg_width > 0 {
            if let Some((cid, off, gl)) = cluster_info {
                spans.push(cluster_gutter_span(app, cid, off, gl, cg_width));
            } else {
                spans.push(cluster_gutter_blank(cg_width));
            }
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
            let style = if !is_expanded
                && hl_regex.is_some_and(|r| entry.continuation_lines.iter().any(|l| r.is_match(l)))
            {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            };
            spans.push(Span::styled(indicator, style));
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
                if cg_width > 0 {
                    cont_spans.push(cluster_continuation_span(cluster_info, cg_width));
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
    let cg_width = cluster_gutter_width(app);
    let gutter_extra = if is_merged { 1 } else { 0 } + cg_width;
    let prefix_width = LINE_PREFIX_WIDTH + gutter_extra;
    let msg_width = viewport_width.saturating_sub(prefix_width);
    if msg_width == 0 {
        return;
    }

    // Ensure selected entry is visible BEFORE building visual lines
    // This accounts for word wrapping when calculating visibility
    app.ensure_selected_visible_with_height(viewport_height, viewport_width);
    let syntax_on = app.syntax_highlight;

    // Build visual lines: (line, is_selected, level)
    let mut visual_lines: Vec<(Line<'_>, bool, LogLevel)> = Vec::with_capacity(viewport_height);
    let mut current_entry_idx = app.scroll_offset;
    let mut terminal_row = 0usize;

    while visual_lines.len() < viewport_height && current_entry_idx < app.filtered_indices.len() {
        // Skip folded interior entries
        if is_folded_interior(app, current_entry_idx) {
            current_entry_idx += 1;
            continue;
        }

        let cluster_info = app.cluster_map.get(&current_entry_idx).copied();
        let is_folded = cluster_info
            .is_some_and(|(cid, off, _)| off == 0 && app.folded_clusters.contains(&cid));

        // Folded cluster: render summary as the entry's own row
        if let Some((cluster_id, 0, group_len)) = cluster_info
            && is_folded
        {
            let is_selected = current_entry_idx == app.selected_index;
            visual_lines.push((
                cluster_fold_line(app, cluster_id, group_len),
                is_selected,
                LogLevel::Info,
            ));
            terminal_row += 1;
            current_entry_idx += 1;
            continue;
        }

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
                // Source gutter first, then cluster gutter
                if is_merged {
                    spans.push(source_gutter_span(entry.source_idx));
                }
                if cg_width > 0 {
                    if let Some((cid, off, gl)) = cluster_info {
                        spans.push(cluster_gutter_span(app, cid, off, gl, cg_width));
                    } else {
                        spans.push(cluster_gutter_blank(cg_width));
                    }
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
                    let style = if !is_expanded
                        && hl_regex
                            .is_some_and(|r| entry.continuation_lines.iter().any(|l| r.is_match(l)))
                    {
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD)
                    };
                    spans.push(Span::styled(ind.clone(), style));
                }
                Line::from(spans)
            } else {
                // Wrapped continuation: indent to align with message
                let mut spans = Vec::new();
                if is_merged {
                    spans.push(Span::raw(" ")); // source gutter placeholder
                }
                if cg_width > 0 {
                    spans.push(cluster_continuation_span(cluster_info, cg_width));
                }
                spans.push(Span::raw(" ".repeat(LINE_PREFIX_WIDTH)));
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
                    let mut cont_spans = Vec::new();
                    if is_merged {
                        cont_spans.push(Span::raw(" ")); // source gutter placeholder
                    }
                    if cg_width > 0 {
                        cont_spans.push(cluster_continuation_span(cluster_info, cg_width));
                    }
                    cont_spans.push(Span::raw(" ".repeat(LINE_PREFIX_WIDTH)));
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
