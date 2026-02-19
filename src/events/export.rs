use crate::app::{App, FocusState};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Handle keys in export dialog
pub fn handle_export_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.close_overlay();
        }

        KeyCode::Enter => {
            let path = match &app.focus {
                FocusState::ExportDialog { input, .. } => input.text().to_string(),
                _ => return,
            };
            if path.is_empty() {
                if let FocusState::ExportDialog { error, .. } = &mut app.focus {
                    *error = Some("Path cannot be empty".to_string());
                }
                return;
            }
            match app.export_filtered(&path) {
                Ok(count) => {
                    app.close_overlay();
                    app.status_message = Some(format!("Exported {} entries to {}", count, path));
                }
                Err(e) => {
                    if let FocusState::ExportDialog { error, .. } = &mut app.focus {
                        *error = Some(e);
                    }
                }
            }
        }

        KeyCode::Left => {
            if let FocusState::ExportDialog { input, .. } = &mut app.focus {
                input.move_left();
            }
        }

        KeyCode::Right => {
            if let FocusState::ExportDialog { input, .. } = &mut app.focus {
                input.move_right();
            }
        }

        KeyCode::Home => {
            if let FocusState::ExportDialog { input, .. } = &mut app.focus {
                input.home();
            }
        }

        KeyCode::End => {
            if let FocusState::ExportDialog { input, .. } = &mut app.focus {
                input.end();
            }
        }

        KeyCode::Delete => {
            if let FocusState::ExportDialog { input, error, .. } = &mut app.focus {
                input.delete_forward();
                *error = None;
            }
        }

        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if let FocusState::ExportDialog { input, error, .. } = &mut app.focus {
                input.clear();
                *error = None;
            }
        }

        KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if let FocusState::ExportDialog { input, error, .. } = &mut app.focus {
                input.delete_path_segment_back();
                *error = None;
            }
        }

        KeyCode::Char(c) => {
            if let FocusState::ExportDialog { input, error, .. } = &mut app.focus {
                input.insert_char(c);
                *error = None;
            }
        }

        KeyCode::Backspace => {
            if let FocusState::ExportDialog { input, error, .. } = &mut app.focus {
                input.delete_back();
                *error = None;
            }
        }

        _ => {}
    }
}
