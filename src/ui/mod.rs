use crate::app::{App, FocusState};
use crate::log_entry::LogLevel;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
};
use std::borrow::Cow;

mod log_view;
mod overlays;
mod status_bar;
mod syntax;

/// Main UI drawing function
pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    // Main layout: log view + status bar
    let chunks = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(area);

    // When search bar is active, shrink log view by 1 row so it doesn't overlap
    let log_area = match &app.focus {
        FocusState::Search { .. } => Rect {
            y: chunks[0].y + 1,
            height: chunks[0].height.saturating_sub(1),
            ..chunks[0]
        },
        _ => chunks[0],
    };

    log_view::draw_log_view(frame, app, log_area);
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
    }
}

/// Get color for log level
pub fn level_color(level: LogLevel) -> Color {
    match level {
        LogLevel::Error => Color::Red,
        LogLevel::Warn => Color::Yellow,
        LogLevel::Info => Color::White,
        LogLevel::Debug => Color::Cyan,
        LogLevel::Trace => Color::DarkGray,
        LogLevel::Profile => Color::Magenta,
        LogLevel::Unknown => Color::DarkGray,
    }
}

/// Get style for level badge
pub fn level_style(level: LogLevel) -> Style {
    Style::default()
        .fg(Color::Black)
        .bg(level_color(level))
        .add_modifier(Modifier::BOLD)
}

/// Extract message portion from raw log line, stripping outer quotes from wd.log
pub fn extract_message(raw_line: &str) -> Cow<'_, str> {
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
            if msg.ends_with('"') {
                if let Some(open) = msg.find('"') {
                    if open < msg.len() - 1 {
                        let prefix = &msg[..open];
                        let inner = &msg[open + 1..msg.len() - 1];
                        return Cow::Owned(format!("{}{}", prefix, inner));
                    }
                }
            }
            return Cow::Borrowed(msg);
        }
    }
    Cow::Borrowed(raw_line)
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
