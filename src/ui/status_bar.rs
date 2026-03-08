use crate::app::{App, FocusState};
use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
};

/// Draw status bar
pub fn draw_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let total = app.entries.len();
    let shown = app.filtered_indices.len();
    let levels = app.active_levels_display();

    let file_display = if app.sources.is_empty() {
        "No file".to_string()
    } else if app.sources.len() == 1 {
        app.sources[0].label.clone()
    } else {
        app.sources
            .iter()
            .map(|s| s.label.as_str())
            .collect::<Vec<_>>()
            .join(" + ")
    };

    // Build status components
    let mut parts = vec![file_display, format!("{}/{}", shown, total), levels];

    if app.is_loading() {
        let count = app.loading_entry_count;
        let display = if count >= 1_000_000 {
            format!("Loading\u{2026} {:.1}M entries", count as f64 / 1_000_000.0)
        } else if count >= 1_000 {
            format!("Loading\u{2026} {:.1}K entries", count as f64 / 1_000.0)
        } else {
            format!("Loading\u{2026} {} entries", count)
        };
        parts.insert(0, display);
    }

    // Add wrap status
    if app.wrap_enabled {
        parts.push("Wrap:ON".to_string());
    }

    // Add syntax highlight status (show only when off since it's on by default)
    if !app.syntax_highlight {
        parts.push("Syn:OFF".to_string());
    }

    // Add visual select mode indicator
    if let Some((lo, hi)) = app.visual_range() {
        parts.push(format!("VISUAL:{}", hi - lo + 1));
    }

    // Add tail status
    if app.tail_enabled {
        parts.push("Live".to_string());
    }

    // Add search/highlight info
    if app.search_panel_open {
        parts.push(format!("Search:\"{}\"", app.search.query));
    }

    // Add horizontal scroll indicator if scrolled
    if app.horizontal_scroll > 0 {
        parts.push(format!("Col:{}", app.horizontal_scroll));
    }

    // Add date filter if active
    if let Some(date_filter) = app.date_filter_display() {
        parts.push(date_filter);
    }

    // Add exclude filter count if active
    if !app.exclude_patterns.is_empty() {
        parts.push(format!("Exclude:{}", app.exclude_patterns.len()));
    }

    // Add include filter count if active
    if !app.include_patterns.is_empty() {
        parts.push(format!("Include:{}", app.include_patterns.len()));
    }

    // Add alert keyword count if active
    if !app.alert_patterns.is_empty() {
        parts.push(format!("Alert:{}", app.alert_patterns.len()));
    }

    // If there's a status message, show it instead of normal status
    let left = if let Some(ref msg) = app.status_message {
        format!(" {} ", msg)
    } else {
        format!(" {} ", parts.join(" | "))
    };

    // Check if the currently selected entry is expandable (has continuation lines)
    let current_expandable = matches!(app.focus, FocusState::Normal)
        && app
            .filtered_indices
            .get(app.selected_index)
            .is_some_and(|&idx| !app.entries[idx].continuation_lines.is_empty());

    // Right side: context-aware hints
    let normal_hints: String;
    let right = match app.focus {
        FocusState::Normal if app.search_panel_open && app.search_panel_focused => {
            "n/N:next/previous match | Tab:main | Esc:close | /:search"
        }
        FocusState::Normal if app.search_panel_open => {
            "n/N:next/previous match | Tab:panel | Esc:close | /:search"
        }
        FocusState::Normal if app.search.regex.is_some() => {
            "n/N:next/previous match | /:search | ?:help | q:quit"
        }
        FocusState::Normal if app.visual_anchor.is_some() => {
            "j/k:select | c:copy line(s) | Esc:cancel"
        }
        FocusState::Normal => {
            let expand_hint = if current_expandable {
                " | Enter:expand"
            } else {
                ""
            };
            normal_hints = format!(
                "Ctrl+P:commands | /:search{} | o:open file | ?:help | q:quit",
                expand_hint
            );
            &normal_hints
        }
        FocusState::Search { .. } => "Ctrl+r:regex | Enter:search | Esc:cancel ",
        FocusState::CommandPalette { .. } => "Esc:close | Enter:run ",
        FocusState::DateFilter { .. } => "Tab:switch | Enter:apply | Esc:close ",
        FocusState::FileOpen { .. } => "Tab:fill | Enter:open | Esc:cancel ",
        FocusState::Detail { .. } => "Esc:close ",
        FocusState::Help { .. } => "j/k:scroll | Esc/q:close ",
        FocusState::FilterManager { .. } => "Tab:switch | Enter:add | d:remove | Esc:close ",
        FocusState::ExportDialog { .. } => "Enter:export | Esc:cancel ",
        FocusState::Clusters { .. } => "Enter:jump | Esc:close ",
        FocusState::ThemePicker { .. } => "Enter:confirm | Esc:cancel ",
        FocusState::Stats { .. } => {
            "e:export | +/-:zoom | \u{2190}\u{2192}:pan | 0:reset | Esc/q:close "
        }
    };

    let left_len = left.len();
    let right_len = right.len();
    let padding = (area.width as usize).saturating_sub(left_len + right_len);

    let left_style = if app.status_message.is_some() {
        Style::default().fg(theme.warning_text)
    } else {
        Style::default().fg(theme.fg)
    };

    let line = Line::from(vec![
        Span::styled(left, left_style),
        Span::raw(" ".repeat(padding)),
        Span::styled(right, Style::default().fg(theme.hint)),
    ]);

    let paragraph = Paragraph::new(line).style(theme.status_bar_style());

    frame.render_widget(paragraph, area);
}
