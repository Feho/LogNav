use crate::app::{App, FocusState};
use crossterm::event::{Event, KeyEvent, KeyEventKind};
use std::time::Instant;

mod clusters;
mod command;
mod date_filter;
mod detail;
mod export;
mod file_open;
mod filter_manager;
mod help;
mod mouse;
mod normal;
mod search;
mod search_panel;
mod stats;
mod theme_picker;

pub use search::flush_search;

/// Handle crossterm events
pub fn handle_event(app: &mut App, event: Event) {
    // Clear status message on next user action
    app.status_message = None;

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

    let was_normal = matches!(app.focus, FocusState::Normal);

    match &app.focus {
        FocusState::Normal => normal::handle_normal_key(app, key),
        FocusState::CommandPalette { .. } => command::handle_command_palette_key(app, key),
        FocusState::Search { .. } => search::handle_search_key(app, key),
        FocusState::DateFilter { .. } => date_filter::handle_date_filter_key(app, key),
        FocusState::FileOpen { .. } => file_open::handle_file_open_key(app, key),
        FocusState::Detail { .. } => detail::handle_detail_key(app, key),
        FocusState::Help { .. } => help::handle_help_key(app, key),
        FocusState::FilterManager { .. } => filter_manager::handle_filter_manager_key(app, key),
        FocusState::ExportDialog { .. } => export::handle_export_key(app, key),
        FocusState::Clusters { .. } => clusters::handle_clusters_key(app, key),
        FocusState::ThemePicker { .. } => theme_picker::handle_theme_picker_key(app, key),
        FocusState::Stats { .. } => stats::handle_stats_key(app, key),
    }

    // Clear visual selection when leaving normal mode
    if was_normal && !matches!(app.focus, FocusState::Normal) {
        app.visual_anchor = None;
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
    // Tilde expansion: check HOME then USERPROFILE (Windows)
    let home_dir = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok();
    if let Some(ref home) = home_dir {
        if s == "~" {
            return home.clone();
        } else if let Some(rest) = s.strip_prefix("~/").or_else(|| s.strip_prefix("~\\")) {
            let sep = std::path::MAIN_SEPARATOR;
            return format!("{}{}{}", home, sep, rest);
        }
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
        FocusState::FileOpen { input, error, .. } => {
            let cleaned = clean_pasted_path(&text);
            input.set_text(cleaned);
            *error = None;
        }
        FocusState::Search { input, .. } => {
            for c in text.trim().chars() {
                input.insert_char(c);
            }
            app.search_dirty = Some(Instant::now());
        }
        FocusState::FilterManager { input, .. } => {
            for c in text.trim().chars() {
                input.insert_char(c);
            }
        }
        FocusState::CommandPalette { input, selected } => {
            for c in text.trim().chars() {
                input.insert_char(c);
            }
            *selected = 0;
        }
        FocusState::DateFilter { .. }
        | FocusState::Clusters { .. }
        | FocusState::ThemePicker { .. }
        | FocusState::Stats { .. } => {
            // Not useful for these overlays
        }
        FocusState::ExportDialog { input, error, .. } => {
            let cleaned = clean_pasted_path(&text);
            input.set_text(cleaned);
            *error = None;
        }
    }
}
