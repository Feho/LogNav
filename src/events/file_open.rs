use crate::app::{App, FocusState};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Convert a char index to a byte index within a string
pub fn char_to_byte_index(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(i, _)| i)
        .unwrap_or(s.len())
}

/// Handle keys in file open dialog
pub fn handle_file_open_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.close_overlay();
        }

        KeyCode::Enter => {
            let file_path = match &app.focus {
                FocusState::FileOpen {
                    path,
                    selected_recent,
                    ..
                } => {
                    if path.is_empty() && !app.recent_files.is_empty() {
                        app.recent_files.get(*selected_recent).cloned()
                    } else {
                        // Tilde expansion
                        let expanded = if path == "~" {
                            std::env::var("HOME").unwrap_or_else(|_| path.clone())
                        } else if let Some(rest) = path.strip_prefix("~/") {
                            match std::env::var("HOME") {
                                Ok(home) => format!("{}/{}", home, rest),
                                Err(_) => path.clone(),
                            }
                        } else {
                            path.clone()
                        };
                        Some(expanded)
                    }
                }
                _ => return,
            };

            if let Some(path) = file_path {
                if !std::path::Path::new(&path).is_file() {
                    if let FocusState::FileOpen { error, .. } = &mut app.focus {
                        *error = Some(format!("File not found: {}", path));
                    }
                    return;
                }
                app.file_path = path;
            }
            app.close_overlay();
        }

        KeyCode::Up => {
            if let FocusState::FileOpen {
                selected_recent, ..
            } = &mut app.focus
                && *selected_recent > 0
            {
                *selected_recent -= 1;
            }
        }

        KeyCode::Down => {
            if let FocusState::FileOpen {
                selected_recent, ..
            } = &mut app.focus
                && *selected_recent + 1 < app.recent_files.len()
            {
                *selected_recent += 1;
            }
        }

        KeyCode::PageUp => {
            if let FocusState::FileOpen {
                selected_recent, ..
            } = &mut app.focus
            {
                *selected_recent = 0;
            }
        }

        KeyCode::PageDown => {
            let len = app.recent_files.len();
            if let FocusState::FileOpen {
                selected_recent, ..
            } = &mut app.focus
                && len > 0
            {
                *selected_recent = len - 1;
            }
        }

        KeyCode::Left => {
            if let FocusState::FileOpen { cursor_pos, .. } = &mut app.focus {
                *cursor_pos = cursor_pos.saturating_sub(1);
            }
        }

        KeyCode::Right => {
            if let FocusState::FileOpen {
                path, cursor_pos, ..
            } = &mut app.focus
            {
                let char_count = path.chars().count();
                if *cursor_pos < char_count {
                    *cursor_pos += 1;
                }
            }
        }

        KeyCode::Home => {
            if let FocusState::FileOpen { cursor_pos, .. } = &mut app.focus {
                *cursor_pos = 0;
            }
        }

        KeyCode::End => {
            if let FocusState::FileOpen {
                path, cursor_pos, ..
            } = &mut app.focus
            {
                *cursor_pos = path.chars().count();
            }
        }

        KeyCode::Tab => {
            // Select from recent files
            let recent_path = match &app.focus {
                FocusState::FileOpen {
                    selected_recent, ..
                } => app.recent_files.get(*selected_recent).cloned(),
                _ => return,
            };
            if let Some(recent) = recent_path
                && let FocusState::FileOpen {
                    path,
                    cursor_pos,
                    error,
                    ..
                } = &mut app.focus
            {
                *path = recent;
                *cursor_pos = path.chars().count();
                *error = None;
            }
        }

        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if let FocusState::FileOpen {
                path,
                cursor_pos,
                error,
                ..
            } = &mut app.focus
            {
                path.clear();
                *cursor_pos = 0;
                *error = None;
            }
        }

        KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if let FocusState::FileOpen {
                path,
                cursor_pos,
                error,
                ..
            } = &mut app.focus
                && *cursor_pos > 0
            {
                *error = None;
                let byte_end = char_to_byte_index(path, *cursor_pos);
                let mut new_pos = *cursor_pos;

                // Skip trailing '/' separators
                while new_pos > 0 {
                    let bi = char_to_byte_index(path, new_pos - 1);
                    if path[bi..].starts_with('/') {
                        new_pos -= 1;
                    } else {
                        break;
                    }
                }

                // Delete back to previous '/' or start
                while new_pos > 0 {
                    let bi = char_to_byte_index(path, new_pos - 1);
                    if path[bi..].starts_with('/') {
                        break;
                    }
                    new_pos -= 1;
                }

                let byte_start = char_to_byte_index(path, new_pos);
                path.drain(byte_start..byte_end);
                *cursor_pos = new_pos;
            }
        }

        KeyCode::Char(c) => {
            if let FocusState::FileOpen {
                path,
                cursor_pos,
                error,
                ..
            } = &mut app.focus
            {
                let byte_idx = char_to_byte_index(path, *cursor_pos);
                path.insert(byte_idx, c);
                *cursor_pos += 1;
                *error = None;
            }
        }

        KeyCode::Backspace => {
            if let FocusState::FileOpen {
                path,
                cursor_pos,
                error,
                ..
            } = &mut app.focus
                && *cursor_pos > 0
            {
                let byte_idx = char_to_byte_index(path, *cursor_pos - 1);
                path.remove(byte_idx);
                *cursor_pos -= 1;
                *error = None;
            }
        }

        _ => {}
    }
}
