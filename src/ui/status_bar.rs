use crate::app::{App, FocusState};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

/// Draw status bar
pub fn draw_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let total = app.entries.len();
    let shown = app.filtered_indices.len();
    let levels = app.active_levels_display();

    let file_display = if app.file_path.is_empty() {
        "No file".to_string()
    } else {
        // Show just filename
        std::path::Path::new(&app.file_path)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| app.file_path.clone())
    };

    // Mode indicator
    let mode = match app.focus {
        FocusState::Normal => "[NORMAL]",
        FocusState::CommandPalette { .. } => "[COMMAND]",
        FocusState::Search { .. } => "[SEARCH]",
        FocusState::DateFilter { .. } => "[DATE FILTER]",
        FocusState::FileOpen { .. } => "[OPEN FILE]",
        FocusState::Detail { .. } => "[DETAIL]",
        FocusState::Help { .. } => "[HELP]",
    };

    // Build status components
    let mut parts = vec![
        mode.to_string(),
        file_display,
        format!("{}/{}", shown, total),
        levels,
    ];

    // Add wrap status
    if app.wrap_enabled {
        parts.push("Wrap:ON".to_string());
    }

    // Add syntax highlight status (show only when off since it's on by default)
    if !app.syntax_highlight {
        parts.push("Syn:OFF".to_string());
    }

    // Add tail status
    if app.tail_enabled {
        parts.push("Tail".to_string());
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

    let left = format!(" {} ", parts.join(" | "));

    // Right side: context-aware hints
    let right = match app.focus {
        FocusState::Normal if app.search_panel_open && app.search_panel_focused => {
            "j/k:move | n/N:match | Tab:main | Esc:close | /:search"
        }
        FocusState::Normal if app.search_panel_open => {
            "n/N:match | Tab:panel | Esc:close | /:search"
        }
        FocusState::Normal if app.search.regex.is_some() => {
            "n/N:match | j/k:move | /:search | ?:help | q:quit"
        }
        FocusState::Normal => "j/k:move | /:search | Enter:expand | o:open | ?:help | q:quit",
        FocusState::Search { .. } => "C-r:regex | Enter:search | Esc:cancel ",
        FocusState::CommandPalette { .. } => "Esc:close | Enter:run ",
        FocusState::DateFilter { .. } => "Tab:switch | Enter:apply | Esc:close ",
        FocusState::FileOpen { .. } => "Tab:fill | Enter:open | Esc:cancel ",
        FocusState::Detail { .. } => "j/k:scroll | Esc:close ",
        FocusState::Help { .. } => "j/k:scroll | Esc/q:close ",
    };

    let left_len = left.len();
    let right_len = right.len();
    let padding = (area.width as usize).saturating_sub(left_len + right_len);

    let line = Line::from(vec![
        Span::styled(left, Style::default().fg(Color::White)),
        Span::raw(" ".repeat(padding)),
        Span::styled(right, Style::default().fg(Color::Cyan)),
    ]);

    let paragraph = Paragraph::new(line).style(Style::default().bg(Color::Black).fg(Color::White));

    frame.render_widget(paragraph, area);
}
