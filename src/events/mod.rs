use crate::app::{App, FocusState};
use crossterm::event::{Event, KeyEvent};

mod command;
mod date_filter;
mod detail;
mod file_open;
mod mouse;
mod normal;
mod search;

/// Handle crossterm events
pub fn handle_event(app: &mut App, event: Event) {
    match event {
        Event::Key(key) => handle_key(app, key),
        Event::Mouse(mouse) => mouse::handle_mouse(app, mouse),
        Event::Resize(_, _) => {} // Ratatui handles this
        _ => {}
    }
}

/// Handle keyboard events
fn handle_key(app: &mut App, key: KeyEvent) {
    match &app.focus {
        FocusState::Normal => normal::handle_normal_key(app, key),
        FocusState::CommandPalette { .. } => command::handle_command_palette_key(app, key),
        FocusState::Search { .. } => search::handle_search_key(app, key),
        FocusState::DateFilter { .. } => date_filter::handle_date_filter_key(app, key),
        FocusState::FileOpen { .. } => file_open::handle_file_open_key(app, key),
        FocusState::Detail { .. } => detail::handle_detail_key(app, key),
    }
}
