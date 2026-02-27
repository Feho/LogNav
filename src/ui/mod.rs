use crate::app::{App, FocusState};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState},
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
        // Skip past timestamp pattern "MM-dd HH:mm:ss.fff"
        if raw_line.len() > pos + 18 {
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
