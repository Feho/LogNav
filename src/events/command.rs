use crate::app::{App, FocusState};
use crossterm::event::{KeyCode, KeyEvent};

/// Handle keys in command palette
pub fn handle_command_palette_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.close_overlay();
        }

        KeyCode::Enter => {
            // Get input and selected from focus state
            let (input, selected) = match &app.focus {
                FocusState::CommandPalette { input, selected } => (input.clone(), *selected),
                _ => return,
            };
            let commands = app.get_filtered_commands(&input);
            if let Some((_, cmd, _)) = commands.get(selected) {
                let action = cmd.action;
                app.close_overlay();
                app.execute_command(action);
            }
        }

        KeyCode::Up => {
            if let FocusState::CommandPalette { selected, .. } = &mut app.focus {
                if *selected > 0 {
                    *selected -= 1;
                }
            }
        }

        KeyCode::Down => {
            let (input_clone, current_selected) = match &app.focus {
                FocusState::CommandPalette { input, selected } => (input.clone(), *selected),
                _ => return,
            };
            let count = app.get_filtered_commands(&input_clone).len();
            if let FocusState::CommandPalette { selected, .. } = &mut app.focus {
                if current_selected + 1 < count {
                    *selected = current_selected + 1;
                }
            }
        }

        KeyCode::Char(c) => {
            if let FocusState::CommandPalette { input, selected } = &mut app.focus {
                input.push(c);
                *selected = 0;
            }
        }

        KeyCode::Backspace => {
            if let FocusState::CommandPalette { input, selected } = &mut app.focus {
                input.pop();
                *selected = 0;
            }
        }

        _ => {}
    }
}
