use crate::app::App;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

/// Draw status bar
pub fn draw_status_bar(frame: &mut Frame, app: &App, area: Rect) {
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

    let date_filter = app
        .date_filter_display()
        .map(|d| format!(" | {}", d))
        .unwrap_or_default();

    let left = format!(
        " {} | {}/{} | {} | {}{} ",
        file_display, shown, total, levels, tail, date_filter
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
