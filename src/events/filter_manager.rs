use crate::app::{App, FilterKind, FilterManagerFocus, FocusState};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Handle keys in filter manager overlay (shared by exclude and include)
pub fn handle_filter_manager_key(app: &mut App, key: KeyEvent) {
    let (kind, input, selected, regex_mode, regex_error, focus) = match &mut app.focus {
        FocusState::FilterManager {
            kind,
            input,
            selected,
            regex_mode,
            regex_error,
            focus,
        } => (*kind, input, selected, regex_mode, regex_error, focus),
        _ => return,
    };

    let label = kind.label();
    // Direct reference to the patterns vec (avoids borrow conflict with &mut app.focus).
    // NOTE: this ref must not be used after calls that mutate the vec (add_filter,
    // remove_filter) — those arms re-extract state from app after the mutation.
    let patterns = match kind {
        FilterKind::Exclude => &app.exclude_patterns,
        FilterKind::Include => &app.include_patterns,
    };

    match key.code {
        KeyCode::Esc => {
            app.close_overlay();
        }

        KeyCode::Tab => {
            *focus = match *focus {
                FilterManagerFocus::Input => {
                    if patterns.is_empty() {
                        FilterManagerFocus::Input
                    } else {
                        FilterManagerFocus::List
                    }
                }
                FilterManagerFocus::List => FilterManagerFocus::Input,
            };
        }

        KeyCode::Enter => {
            if *focus == FilterManagerFocus::Input {
                let query = input.text().to_string();
                let rm = *regex_mode;
                if !query.is_empty() {
                    if let Some(err) = app.add_filter(kind, &query, rm) {
                        if let FocusState::FilterManager { regex_error, .. } = &mut app.focus {
                            *regex_error = Some(err);
                        }
                    } else {
                        app.status_message = Some(format!("{} filter added: '{}'", label, query));
                        if let FocusState::FilterManager {
                            input, regex_error, ..
                        } = &mut app.focus
                        {
                            input.clear();
                            *regex_error = None;
                        }
                    }
                }
            }
        }

        // Toggle regex mode
        KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            *regex_mode = !*regex_mode;
            *regex_error = None;
        }

        // Delete selected pattern
        KeyCode::Char('d') | KeyCode::Delete | KeyCode::Backspace
            if *focus == FilterManagerFocus::List =>
        {
            let idx = *selected;
            if idx < patterns.len() {
                let name = patterns[idx].query.clone();
                app.remove_filter(kind, idx);
                app.status_message = Some(format!("Removed {}: '{}'", label.to_lowercase(), name));
                // Re-extract focus state after mutation
                let patterns_len = app.filter_patterns(kind).len();
                if let FocusState::FilterManager {
                    selected, focus, ..
                } = &mut app.focus
                {
                    if patterns_len == 0 {
                        *focus = FilterManagerFocus::Input;
                        *selected = 0;
                    } else if *selected >= patterns_len {
                        *selected = patterns_len - 1;
                    }
                }
            }
        }

        // Navigation in list
        KeyCode::Up | KeyCode::Char('k') if *focus == FilterManagerFocus::List => {
            *selected = selected.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') if *focus == FilterManagerFocus::List => {
            if !patterns.is_empty() {
                *selected = (*selected + 1).min(patterns.len() - 1);
            }
        }

        // Cursor movement (input focused)
        KeyCode::Left if *focus == FilterManagerFocus::Input => {
            input.move_left();
        }
        KeyCode::Right if *focus == FilterManagerFocus::Input => {
            input.move_right();
        }
        KeyCode::Home if *focus == FilterManagerFocus::Input => {
            input.home();
        }
        KeyCode::End if *focus == FilterManagerFocus::Input => {
            input.end();
        }

        // Ctrl+U: clear line
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if *focus == FilterManagerFocus::Input {
                input.clear();
                *regex_error = None;
            }
        }

        // Ctrl+W: delete word
        KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if *focus == FilterManagerFocus::Input {
                input.delete_word_back();
                *regex_error = None;
            }
        }

        // Forward delete
        KeyCode::Delete if *focus == FilterManagerFocus::Input => {
            input.delete_forward();
            *regex_error = None;
        }

        // Text input (only when input is focused)
        KeyCode::Char(c) if *focus == FilterManagerFocus::Input => {
            input.insert_char(c);
            *regex_error = None;
        }
        KeyCode::Backspace if *focus == FilterManagerFocus::Input => {
            input.delete_back();
            *regex_error = None;
        }

        _ => {}
    }
}
