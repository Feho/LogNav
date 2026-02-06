use crate::app::{App, FocusState};
use crate::log_entry::LogEntry;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::time::Instant;

/// Build regex from query, respecting regex_mode
fn build_search_regex(query: &str, regex_mode: bool) -> Result<regex::Regex, String> {
    if query.is_empty() {
        return Err(String::new());
    }
    let pattern = if regex_mode {
        format!("(?i){}", query)
    } else {
        format!("(?i){}", regex::escape(query))
    };
    regex::Regex::new(&pattern).map_err(|e| e.to_string())
}

/// Compute search matches scoped to currently visible (filtered) entries.
/// Stores filtered positions so Ctrl+N/P can use them directly as selected_index.
fn update_search_matches(
    entries: &[LogEntry],
    filtered_indices: &[usize],
    query: &str,
    regex_mode: bool,
    match_indices: &mut Vec<usize>,
    current_match: &mut usize,
    regex_error: &mut Option<String>,
) {
    match_indices.clear();
    *regex_error = None;

    if query.is_empty() {
        *current_match = 0;
        return;
    }

    match build_search_regex(query, regex_mode) {
        Ok(regex) => {
            *match_indices = filtered_indices
                .iter()
                .enumerate()
                .filter(|&(_, &entry_idx)| regex.is_match(entries[entry_idx].searchable_text()))
                .map(|(pos, _)| pos)
                .collect();
        }
        Err(e) => {
            *regex_error = Some(e);
        }
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
        app.center_selected();
    }
}

/// Flush pending search: recompute matches and jump to nearest
pub fn flush_search(app: &mut App) {
    let (query, regex_mode) = match &app.focus {
        FocusState::Search {
            query, regex_mode, ..
        } => (query.clone(), *regex_mode),
        _ => {
            app.search_dirty = None;
            return;
        }
    };
    app.search_dirty = None;
    if let FocusState::Search {
        match_indices,
        current_match,
        regex_error,
        ..
    } = &mut app.focus
    {
        update_search_matches(
            &app.entries,
            &app.filtered_indices,
            &query,
            regex_mode,
            match_indices,
            current_match,
            regex_error,
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
            let (query, regex_mode) = match &app.focus {
                FocusState::Search {
                    query,
                    regex_mode,
                    regex_error,
                    ..
                } => {
                    if regex_error.is_some() {
                        return; // Don't apply invalid regex
                    }
                    (query.clone(), *regex_mode)
                }
                _ => return,
            };
            app.set_search_with_mode(&query, regex_mode);
            app.update_search_matches();
            app.close_overlay();
        }

        KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            // Toggle regex mode
            if let FocusState::Search {
                ref mut regex_mode, ..
            } = app.focus
            {
                *regex_mode = !*regex_mode;
            }
            app.search_dirty = Some(Instant::now());
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
                        app.center_selected();
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
                        app.center_selected();
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
