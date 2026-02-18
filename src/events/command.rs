use crate::app::{App, FocusState};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Handle keys in command palette
pub fn handle_command_palette_key(app: &mut App, key: KeyEvent) {
    match (key.modifiers, key.code) {
        (_, KeyCode::Esc) => {
            app.close_overlay();
        }

        (_, KeyCode::Enter) => {
            let (query, selected) = match &app.focus {
                FocusState::CommandPalette { input, selected } => {
                    (input.text().to_string(), *selected)
                }
                _ => return,
            };
            let commands = app.get_filtered_commands(&query);
            if let Some((_, cmd, _)) = commands.get(selected) {
                let action = cmd.action;
                app.close_overlay();
                app.execute_command(action);
            }
        }

        (_, KeyCode::Up) => {
            if let FocusState::CommandPalette { selected, .. } = &mut app.focus
                && *selected > 0
            {
                *selected -= 1;
            }
        }

        (_, KeyCode::Down) => {
            let (query, current_selected) = match &app.focus {
                FocusState::CommandPalette { input, selected } => {
                    (input.text().to_string(), *selected)
                }
                _ => return,
            };
            let count = app.get_filtered_commands(&query).len();
            if let FocusState::CommandPalette { selected, .. } = &mut app.focus
                && current_selected + 1 < count
            {
                *selected = current_selected + 1;
            }
        }

        (_, KeyCode::PageUp) | (KeyModifiers::CONTROL, KeyCode::Char('u')) | (_, KeyCode::Home) => {
            if let FocusState::CommandPalette { selected, .. } = &mut app.focus {
                *selected = 0;
            }
        }

        (_, KeyCode::PageDown)
        | (KeyModifiers::CONTROL, KeyCode::Char('d'))
        | (_, KeyCode::End) => {
            let query = match &app.focus {
                FocusState::CommandPalette { input, .. } => input.text().to_string(),
                _ => return,
            };
            let count = app.get_filtered_commands(&query).len();
            if let FocusState::CommandPalette { selected, .. } = &mut app.focus
                && count > 0
            {
                *selected = count - 1;
            }
        }

        // Cursor movement
        (_, KeyCode::Left) => {
            if let FocusState::CommandPalette { input, .. } = &mut app.focus {
                input.move_left();
            }
        }
        (_, KeyCode::Right) => {
            if let FocusState::CommandPalette { input, .. } = &mut app.focus {
                input.move_right();
            }
        }

        // Ctrl+W: delete word
        (KeyModifiers::CONTROL, KeyCode::Char('w')) => {
            if let FocusState::CommandPalette { input, selected } = &mut app.focus {
                input.delete_word_back();
                *selected = 0;
            }
        }

        // Delete key
        (_, KeyCode::Delete) => {
            if let FocusState::CommandPalette { input, selected } = &mut app.focus {
                input.delete_forward();
                *selected = 0;
            }
        }

        (_, KeyCode::Char(c)) => {
            if let FocusState::CommandPalette { input, selected } = &mut app.focus {
                input.insert_char(c);
                *selected = 0;
            }
        }

        (_, KeyCode::Backspace) => {
            if let FocusState::CommandPalette { input, selected } = &mut app.focus {
                input.delete_back();
                *selected = 0;
            }
        }

        _ => {}
    }
}
