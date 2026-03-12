use crate::app::{App, FocusState};
use crate::theme::Theme;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::Style,
    widgets::{Block, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};
use std::borrow::Cow;

mod clusters_panel;
mod log_view;
mod matches_panel;
mod overlays;
mod status_bar;
pub(crate) mod syntax;

/// Line prefix width: bookmark(1) + timestamp(14) + level badge(5) + space(1)
pub const LINE_PREFIX_WIDTH: usize = 21;

/// Main UI drawing function
pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    // Main layout: log view + status bar
    let chunks = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(area);

    // When search bar is active, shrink log view by 1 row so it doesn't overlap
    let content_area = match &app.focus {
        FocusState::Search { .. } => Rect {
            y: chunks[0].y + 1,
            height: chunks[0].height.saturating_sub(1),
            ..chunks[0]
        },
        _ => chunks[0],
    };

    // Split content area when search panel is open
    if app.search_panel_open {
        let split = Layout::vertical([Constraint::Percentage(67), Constraint::Percentage(33)])
            .split(content_area);

        log_view::draw_log_view(frame, app, split[0]);
        matches_panel::draw_matches_panel(frame, app, split[1]);
    } else {
        log_view::draw_log_view(frame, app, content_area);
    }

    status_bar::draw_status_bar(frame, app, chunks[1]);

    // Draw overlays on top
    match &app.focus {
        FocusState::Normal => {}
        FocusState::CommandPalette { .. } => overlays::draw_command_palette(frame, app),
        FocusState::Search { .. } => overlays::draw_search_bar(frame, app),
        FocusState::DateFilter { .. } => overlays::draw_date_filter(frame, app),
        FocusState::FileOpen { .. } => overlays::draw_file_open(frame, app),
        FocusState::Detail { .. } => overlays::draw_detail_popup(frame, app),
        FocusState::Help { .. } => overlays::draw_help(frame, app),
        FocusState::FilterManager { .. } => overlays::draw_filter_manager(frame, app),
        FocusState::ExportDialog { .. } => overlays::draw_export_dialog(frame, app),
        FocusState::Clusters { .. } => clusters_panel::draw_clusters(frame, app),
        FocusState::ThemePicker { .. } => overlays::draw_theme_picker(frame, app),
        FocusState::Stats { .. } => overlays::draw_stats(frame, app),
    }

    // Toast notification (bottom-right, above status bar)
    if matches!(app.focus, FocusState::Normal)
        && let Some((ref msg, _)) = app.toast
    {
        draw_toast(frame, msg, chunks[0], &app.theme);
    }
}

/// Extract message portion from raw log line, stripping outer quotes from wd.log.
/// If `offset` is provided (from parser), use it directly; otherwise fall back to heuristics.
pub fn extract_message(raw_line: &str, offset: Option<usize>) -> Cow<'_, str> {
    // If parser provided an offset, use it directly
    if let Some(off) = offset
        && off <= raw_line.len()
    {
        let msg = raw_line[off..].trim_start();
        // Strip outer quotes (wd.log wraps messages in "...")
        if msg.ends_with('"')
            && let Some(open) = msg.find('"')
            && open < msg.len() - 1
        {
            let prefix = &msg[..open];
            let inner = &msg[open + 1..msg.len() - 1];
            return Cow::Owned(format!("{}{}", prefix, inner));
        }
        return Cow::Borrowed(msg);
    }

    // Handle qconsole bracket format: [timestamp] message
    if raw_line.starts_with('[')
        && let Some(end) = raw_line.find("] ")
    {
        return Cow::Borrowed(&raw_line[end + 2..]);
    }

    // Find the message after timestamp
    // Pattern: either after "HH:mm:ss.fff " or just return the whole line
    if let Some(pos) = raw_line.find(|c: char| c.is_ascii_digit()) {
        // Skip past timestamp pattern "MM-dd HH:mm:ss.fff" (e.g. "03-21 14:23:01.234")
        // Validate the span actually looks like a timestamp before skipping
        let ts_candidate = raw_line.get(pos..pos + 18).unwrap_or("");
        // "MM-dd HH:mm:ss.fff": dash at 2, space at 5, colons at 8+11, dot at 14
        let looks_like_ts = ts_candidate.len() == 18
            && ts_candidate.chars().enumerate().all(|(i, c)| match i {
                2 => c == '-',
                5 => c == ' ',
                8 | 11 => c == ':',
                14 => c == '.',
                _ => c.is_ascii_digit(),
            });
        if looks_like_ts && raw_line.len() > pos + 18 {
            let msg = raw_line[pos + 18..].trim_start();
            // Strip outer quotes (wd.log wraps messages in "...")
            // The message part after component prefix looks like: `SPL|Context "actual message"`
            if msg.ends_with('"')
                && let Some(open) = msg.find('"')
                && open < msg.len() - 1
            {
                let prefix = &msg[..open];
                let inner = &msg[open + 1..msg.len() - 1];
                return Cow::Owned(format!("{}{}", prefix, inner));
            }
            return Cow::Borrowed(msg);
        }
    }
    Cow::Borrowed(raw_line)
}

/// Render a vertical scrollbar if content exceeds area height
pub fn render_scrollbar(frame: &mut Frame, area: Rect, position: usize, content_length: usize) {
    let height = area.height as usize;
    if content_length <= height {
        return;
    }
    let mut state = ScrollbarState::new(content_length.saturating_sub(height)).position(position);
    frame.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::VerticalRight),
        area,
        &mut state,
    );
}

/// Create a centered rect
pub fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
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

/// Draw a toast notification in the bottom-right corner
fn draw_toast(frame: &mut Frame, msg: &str, content_area: Rect, theme: &Theme) {
    let width = (msg.len() as u16 + 4).min(content_area.width.saturating_sub(2));
    let height = 3;
    let x = content_area.x + content_area.width.saturating_sub(width + 1);
    let y = content_area.y + content_area.height.saturating_sub(height + 1);
    let area = Rect::new(x, y, width, height);
    frame.render_widget(Clear, area);
    let block = Block::bordered().border_style(Style::default().fg(theme.accent).bg(theme.bg));
    let paragraph = Paragraph::new(msg)
        .style(Style::default().fg(theme.fg).bg(theme.bg))
        .block(block);
    frame.render_widget(paragraph, area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_message_heuristic_bug() {
        let line = "1 Error: something went wrong and it is long enough";
        let extracted = extract_message(line, None);
        // Heuristic should not eat the message if it's not a real timestamp
        assert!(extracted.contains("Error"), "Should contain the word 'Error'");
    }
}
