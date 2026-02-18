use crate::app::{App, FocusState};
use crossterm::event::{Event, KeyEvent, KeyEventKind};
use std::time::Instant;

mod command;
mod date_filter;
mod detail;
mod exclude_manager;
mod file_open;
mod help;
mod mouse;
mod normal;
mod search;
mod search_panel;

pub use search::flush_search;

/// Handle crossterm events
pub fn handle_event(app: &mut App, event: Event) {
    match event {
        Event::Key(key) if key.kind == KeyEventKind::Press => handle_key(app, key),
        Event::Mouse(mouse) => mouse::handle_mouse(app, mouse),
        Event::Paste(text) => handle_paste(app, text),
        Event::Resize(_, _) => {} // Ratatui handles this
        _ => {}
    }
}

/// Handle keyboard events
fn handle_key(app: &mut App, key: KeyEvent) {
    // When search panel is focused, dispatch to panel handler first
    if app.search_panel_open && app.search_panel_focused && matches!(app.focus, FocusState::Normal)
    {
        search_panel::handle_search_panel_key(app, key);
        return;
    }

    match &app.focus {
        FocusState::Normal => normal::handle_normal_key(app, key),
        FocusState::CommandPalette { .. } => command::handle_command_palette_key(app, key),
        FocusState::Search { .. } => search::handle_search_key(app, key),
        FocusState::DateFilter { .. } => date_filter::handle_date_filter_key(app, key),
        FocusState::FileOpen { .. } => file_open::handle_file_open_key(app, key),
        FocusState::Detail { .. } => detail::handle_detail_key(app, key),
        FocusState::Help { .. } => help::handle_help_key(app, key),
        FocusState::ExcludeManager { .. } => {
            exclude_manager::handle_exclude_manager_key(app, key)
        }
    }
}

/// Clean a pasted/dropped path: trim whitespace, strip surrounding quotes,
/// strip trailing newlines that terminals sometimes append.
fn clean_pasted_path(text: &str) -> String {
    let mut s = text.trim().to_string();
    // Strip surrounding single or double quotes
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        s = s[1..s.len() - 1].to_string();
    }
    // Tilde expansion
    if s == "~" {
        if let Ok(home) = std::env::var("HOME") {
            return home;
        }
    } else if let Some(rest) = s.strip_prefix("~/")
        && let Ok(home) = std::env::var("HOME")
    {
        return format!("{}/{}", home, rest);
    }
    s
}

/// Handle paste events (drag-and-drop sends paste in most terminals)
fn handle_paste(app: &mut App, text: String) {
    match &mut app.focus {
        FocusState::Normal | FocusState::Detail { .. } | FocusState::Help { .. } => {
            // Treat paste as a file path drop (replaces current file)
            let path = clean_pasted_path(&text);
            if !path.is_empty() && std::path::Path::new(&path).is_file() {
                app.file_path = path;
            } else {
                app.status_message = Some(format!("Not a file: {}", text.trim()));
            }
        }
        FocusState::FileOpen {
            path,
            cursor_pos,
            error,
            ..
        } => {
            let cleaned = clean_pasted_path(&text);
            // Replace entire path with cleaned paste (typical drag-and-drop)
            *path = cleaned;
            *cursor_pos = path.chars().count();
            *error = None;
        }
        FocusState::Search { query, .. } => {
            query.push_str(text.trim());
            app.search_dirty = Some(Instant::now());
        }
        FocusState::ExcludeManager { input, .. } => {
            input.push_str(text.trim());
        }
        FocusState::CommandPalette { input, selected } => {
            input.push_str(text.trim());
            *selected = 0;
        }
        FocusState::DateFilter { .. } => {
            // Not useful for date filter
        }
    }
}
