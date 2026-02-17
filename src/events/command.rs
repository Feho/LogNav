use crate::app::{App, FocusState};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Handle keys in command palette
pub fn handle_command_palette_key(app: &mut App, key: KeyEvent) {
    match (key.modifiers, key.code) {
        (_, KeyCode::Esc) => {
            app.close_overlay();
        }

        (_, KeyCode::Enter) => {
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

        (_, KeyCode::Up) => {
            if let FocusState::CommandPalette { selected, .. } = &mut app.focus
                && *selected > 0
            {
                *selected -= 1;
            }
        }

        (_, KeyCode::Down) => {
            let (input_clone, current_selected) = match &app.focus {
                FocusState::CommandPalette { input, selected } => (input.clone(), *selected),
                _ => return,
            };
            let count = app.get_filtered_commands(&input_clone).len();
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
            let (input_clone, _) = match &app.focus {
                FocusState::CommandPalette { input, selected } => (input.clone(), *selected),
                _ => return,
            };
            let count = app.get_filtered_commands(&input_clone).len();
            if let FocusState::CommandPalette { selected, .. } = &mut app.focus
                && count > 0
            {
                *selected = count - 1;
            }
        }

        (_, KeyCode::Char(c)) => {
            if let FocusState::CommandPalette { input, selected } = &mut app.focus {
                input.push(c);
                *selected = 0;
            }
        }

        (_, KeyCode::Backspace) => {
            if let FocusState::CommandPalette { input, selected } = &mut app.focus {
                input.pop();
                *selected = 0;
            }
        }

        _ => {}
    }
}
