use crate::app::{App, FocusState};
use crate::log_entry::LogEntry;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::time::Instant;

/// Compute search matches scoped to currently visible (filtered) entries.
/// Stores filtered positions so Ctrl+N/P can use them directly as selected_index.
fn update_search_matches(
    entries: &[LogEntry],
    filtered_indices: &[usize],
    query: &str,
    match_indices: &mut Vec<usize>,
    current_match: &mut usize,
) {
    match_indices.clear();

    if query.is_empty() {
        *current_match = 0;
        return;
    }

    if let Ok(regex) = regex::Regex::new(&format!("(?i){}", regex::escape(query))) {
        *match_indices = filtered_indices
            .iter()
            .enumerate()
            .filter(|&(_, &entry_idx)| regex.is_match(entries[entry_idx].searchable_text()))
            .map(|(pos, _)| pos)
            .collect();
    }

    if !match_indices.is_empty() && *current_match >= match_indices.len() {
        *current_match = 0;
    }
}

/// Jump to the first match at or after current position (vim-style incremental search)
fn jump_to_nearest_match(app: &mut App) {
    if let FocusState::Search {
        ref match_indices,
        ref mut current_match,
        ..
    } = app.focus
    {
        if match_indices.is_empty() {
            return;
        }
        // Find first match at or after current selected_index
        let pos = match_indices
            .iter()
            .position(|&m| m >= app.selected_index)
            .unwrap_or(0);
        *current_match = pos;
        app.selected_index = match_indices[pos];
        app.ensure_selected_visible();
    }
}

/// Flush pending search: recompute matches and jump to nearest
pub fn flush_search(app: &mut App) {
    let query = match &app.focus {
        FocusState::Search { query, .. } => query.clone(),
        _ => {
            app.search_dirty = None;
            return;
        }
    };
    app.search_dirty = None;
    if let FocusState::Search {
        match_indices,
        current_match,
        ..
    } = &mut app.focus
    {
        update_search_matches(
            &app.entries,
            &app.filtered_indices,
            &query,
            match_indices,
            current_match,
        );
    }
    jump_to_nearest_match(app);
}

/// Handle keys in search mode
pub fn handle_search_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            // Clear search and close
            app.set_search("");
            app.close_overlay();
        }

        KeyCode::Enter => {
            // Apply search filter from focus state and close
            let query = match &app.focus {
                FocusState::Search { query, .. } => query.clone(),
                _ => return,
            };
            app.set_search(&query);
            app.update_search_matches();
            app.close_overlay();
        }

        KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            // Navigate to next match
            match &mut app.focus {
                FocusState::Search {
                    match_indices,
                    current_match,
                    ..
                } => {
                    if !match_indices.is_empty() {
                        *current_match = (*current_match + 1) % match_indices.len();
                        app.selected_index = match_indices[*current_match];
                        app.ensure_selected_visible();
                    }
                }
                _ => {}
            }
        }

        KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            // Navigate to previous match
            match &mut app.focus {
                FocusState::Search {
                    match_indices,
                    current_match,
                    ..
                } => {
                    if !match_indices.is_empty() {
                        *current_match = if *current_match == 0 {
                            match_indices.len() - 1
                        } else {
                            *current_match - 1
                        };
                        app.selected_index = match_indices[*current_match];
                        app.ensure_selected_visible();
                    }
                }
                _ => {}
            }
        }

        KeyCode::Char(c) => {
            if let FocusState::Search { ref mut query, .. } = app.focus {
                query.push(c);
            }
            app.search_dirty = Some(Instant::now());
        }

        KeyCode::Backspace => {
            if let FocusState::Search { ref mut query, .. } = app.focus {
                query.pop();
            }
            app.search_dirty = Some(Instant::now());
        }

        _ => {}
    }
}
