use crate::app::{App, FocusState};
use crate::log_entry::LogEntry;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Compute search matches based on query without applying filter
fn update_search_matches(
    entries: &[LogEntry],
    query: &str,
    match_indices: &mut Vec<usize>,
    current_match: &mut usize,
) {
    match_indices.clear();

    if query.is_empty() {
        *current_match = 0;
        return;
    }

    // Try to compile regex from query
    if let Ok(regex) = regex::Regex::new(&format!("(?i){}", regex::escape(query))) {
        *match_indices = entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| regex.is_match(entry.searchable_text()))
            .map(|(idx, _)| idx)
            .collect();
    }

    if !match_indices.is_empty() && *current_match >= match_indices.len() {
        *current_match = 0;
    }
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
            // Navigate to next match in search results without filtering
            match &mut app.focus {
                FocusState::Search {
                    match_indices,
                    current_match,
                    ..
                } => {
                    if !match_indices.is_empty() {
                        *current_match = (*current_match + 1) % match_indices.len();
                        let target = match_indices[*current_match];
                        app.selected_index = target;
                        app.ensure_selected_visible();
                    }
                }
                _ => {}
            }
        }

        KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            // Navigate to previous match in search results without filtering
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
                        let target = match_indices[*current_match];
                        app.selected_index = target;
                        app.ensure_selected_visible();
                    }
                }
                _ => {}
            }
        }

        KeyCode::Char(c) => {
            // Update query and recompute match_indices without filtering
            let query = match &mut app.focus {
                FocusState::Search { query, .. } => {
                    query.push(c);
                    query.clone()
                }
                _ => return,
            };
            // Recompute matches as user types
            if let FocusState::Search {
                match_indices,
                current_match,
                ..
            } = &mut app.focus
            {
                update_search_matches(&app.entries, &query, match_indices, current_match);
            }
        }

        KeyCode::Backspace => {
            let query = match &mut app.focus {
                FocusState::Search { query, .. } => {
                    query.pop();
                    query.clone()
                }
                _ => return,
            };
            // Recompute matches as user deletes
            if let FocusState::Search {
                match_indices,
                current_match,
                ..
            } = &mut app.focus
            {
                update_search_matches(&app.entries, &query, match_indices, current_match);
            }
        }

        _ => {}
    }
}
