use crate::app::App;
use crate::ui::extract_message;
use crate::ui::render_scrollbar;
use crate::ui::syntax::styled_spans;
use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

/// Draw the search matches panel (bottom split)
pub fn draw_matches_panel(frame: &mut Frame, app: &mut App, area: Rect) {
    let theme = &app.theme;
    app.search_panel_height = area.height as usize;

    let total = app.search_panel_matches.len();
    let current = app.search_panel_selected;

    // Title shows match position
    let title = if total > 0 {
        format!(
            " Search results {}/{} (press <tab> to focus)",
            current + 1,
            total
        )
    } else {
        " Search results (0) ".to_string()
    };

    let border_color = if app.search_panel_focused {
        theme.warning_text
    } else {
        theme.muted
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(title);

    let inner = block.inner(area);
    frame.render_widget(Clear, area);
    frame.render_widget(block, area);

    if total == 0 || inner.height == 0 {
        return;
    }

    let hl_regex = app.search.regex.as_ref();
    let syntax_on = app.syntax_highlight;
    // Render visible matches
    let mut y = 0u16;
    let start = app.search_panel_scroll;

    for match_idx in start..total {
        if y >= inner.height {
            break;
        }

        let filtered_pos = app.search_panel_matches[match_idx];
        let entry_idx = match app.filtered_indices.get(filtered_pos) {
            Some(&idx) => idx,
            None => continue,
        };
        let entry = match app.entries.get(entry_idx) {
            Some(e) => e,
            None => continue,
        };

        let is_selected = match_idx == current;

        // Build the line (same format as log_view)
        let timestamp = entry
            .timestamp
            .map(|ts| ts.format("%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "             ".to_string());

        let level_span = Span::styled(
            format!(" {} ", entry.level.short_name()),
            theme.level_style(entry.level),
        );

        let message = extract_message(&entry.raw_line, entry.message_offset);
        let display_msg: &str = &message;

        let mut spans = vec![
            Span::styled(timestamp, Style::default().fg(theme.muted)),
            level_span,
            Span::raw(" "),
        ];
        spans.extend(styled_spans(
            display_msg,
            hl_regex,
            &[],
            Style::default(),
            syntax_on && !is_selected,
            None,
            theme,
        ));

        let line_area = Rect {
            x: inner.x,
            y: inner.y + y,
            width: inner.width,
            height: 1,
        };

        let style = if is_selected {
            theme.cursor_line_style(entry.level)
        } else {
            Style::default()
        };

        let paragraph = Paragraph::new(Line::from(spans)).style(style);
        frame.render_widget(paragraph, line_area);

        y += 1;
    }

    render_scrollbar(frame, inner, app.search_panel_scroll, total);
}
