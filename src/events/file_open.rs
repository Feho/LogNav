use crate::app::{App, FocusState};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Handle keys in file open dialog
pub fn handle_file_open_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.close_overlay();
        }

        KeyCode::Enter => {
            let (file_path, is_merge) = match &app.focus {
                FocusState::FileOpen {
                    input,
                    selected_recent,
                    is_merge,
                    ..
                } => {
                    let path = input.text();
                    let resolved = if path.is_empty() && !app.recent_files.is_empty() {
                        app.recent_files.get(*selected_recent).cloned()
                    } else {
                        // Tilde expansion
                        let expanded = if path == "~" {
                            std::env::var("HOME").unwrap_or_else(|_| path.to_string())
                        } else if let Some(rest) = path.strip_prefix("~/") {
                            match std::env::var("HOME") {
                                Ok(home) => format!("{}/{}", home, rest),
                                Err(_) => path.to_string(),
                            }
                        } else {
                            path.to_string()
                        };
                        Some(expanded)
                    };
                    (resolved, *is_merge)
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
                if is_merge {
                    app.pending_merge_path = Some(path);
                } else {
                    app.file_path = path;
                }
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
            if let FocusState::FileOpen { input, .. } = &mut app.focus {
                input.move_left();
            }
        }

        KeyCode::Right => {
            if let FocusState::FileOpen { input, .. } = &mut app.focus {
                input.move_right();
            }
        }

        KeyCode::Home => {
            if let FocusState::FileOpen { input, .. } = &mut app.focus {
                input.home();
            }
        }

        KeyCode::End => {
            if let FocusState::FileOpen { input, .. } = &mut app.focus {
                input.end();
            }
        }

        KeyCode::Delete => {
            if let FocusState::FileOpen { input, error, .. } = &mut app.focus {
                input.delete_forward();
                *error = None;
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
                && let FocusState::FileOpen { input, error, .. } = &mut app.focus
            {
                input.set_text(recent);
                *error = None;
            }
        }

        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if let FocusState::FileOpen { input, error, .. } = &mut app.focus {
                input.clear();
                *error = None;
            }
        }

        KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if let FocusState::FileOpen { input, error, .. } = &mut app.focus {
                input.delete_path_segment_back();
                *error = None;
            }
        }

        KeyCode::Char(c) => {
            if let FocusState::FileOpen { input, error, .. } = &mut app.focus {
                input.insert_char(c);
                *error = None;
            }
        }

        KeyCode::Backspace => {
            if let FocusState::FileOpen { input, error, .. } = &mut app.focus {
                input.delete_back();
                *error = None;
            }
        }

        _ => {}
    }
}
