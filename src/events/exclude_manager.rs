use crate::app::{App, ExcludeManagerFocus, FocusState};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Handle keys in exclude manager overlay
pub fn handle_exclude_manager_key(app: &mut App, key: KeyEvent) {
    let (input, selected, regex_mode, regex_error, focus) = match &mut app.focus {
        FocusState::ExcludeManager {
            input,
            selected,
            regex_mode,
            regex_error,
            focus,
        } => (input, selected, regex_mode, regex_error, focus),
        _ => return,
    };

    match key.code {
        KeyCode::Esc => {
            app.close_overlay();
        }

        KeyCode::Tab => {
            *focus = match *focus {
                ExcludeManagerFocus::Input => {
                    if app.exclude_patterns.is_empty() {
                        ExcludeManagerFocus::Input // stay on input if no patterns
                    } else {
                        ExcludeManagerFocus::List
                    }
                }
                ExcludeManagerFocus::List => ExcludeManagerFocus::Input,
            };
        }

        KeyCode::Enter => {
            if *focus == ExcludeManagerFocus::Input {
                // Add new exclude from input
                let query = input.clone();
                let rm = *regex_mode;
                if !query.is_empty() {
                    if let Some(err) = app.add_exclude(&query, rm) {
                        // Set error on the overlay
                        if let FocusState::ExcludeManager { regex_error, .. } = &mut app.focus {
                            *regex_error = Some(err);
                        }
                    } else {
                        // Success: clear input, show status
                        app.status_message = Some(format!("Exclude filter added: '{}'", query));
                        if let FocusState::ExcludeManager {
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

        // Delete selected exclude pattern
        KeyCode::Char('d') | KeyCode::Delete | KeyCode::Backspace
            if *focus == ExcludeManagerFocus::List =>
        {
            let idx = *selected;
            if idx < app.exclude_patterns.len() {
                let name = app.exclude_patterns[idx].query.clone();
                app.remove_exclude(idx);
                app.status_message = Some(format!("Removed exclude: '{}'", name));
                // Re-extract focus state after mutation
                if let FocusState::ExcludeManager {
                    selected, focus, ..
                } = &mut app.focus
                {
                    if app.exclude_patterns.is_empty() {
                        *focus = ExcludeManagerFocus::Input;
                        *selected = 0;
                    } else if *selected >= app.exclude_patterns.len() {
                        *selected = app.exclude_patterns.len() - 1;
                    }
                }
            }
        }

        // Navigation in list
        KeyCode::Up | KeyCode::Char('k') if *focus == ExcludeManagerFocus::List => {
            *selected = selected.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') if *focus == ExcludeManagerFocus::List => {
            if !app.exclude_patterns.is_empty() {
                *selected = (*selected + 1).min(app.exclude_patterns.len() - 1);
            }
        }

        // Text input (only when input is focused)
        KeyCode::Char(c) if *focus == ExcludeManagerFocus::Input => {
            input.push(c);
            *regex_error = None;
        }
        KeyCode::Backspace if *focus == ExcludeManagerFocus::Input => {
            input.pop();
            *regex_error = None;
        }

        _ => {}
    }
}
