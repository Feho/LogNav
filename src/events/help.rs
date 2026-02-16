use crate::app::{App, FocusState};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Handle keys in help dialog
pub fn handle_help_key(app: &mut App, key: KeyEvent) {
    let scroll_offset = match &mut app.focus {
        FocusState::Help { scroll_offset } => scroll_offset,
        _ => return,
    };

    match (key.modifiers, key.code) {
        // Close popup
        (_, KeyCode::Esc)
        | (_, KeyCode::Char('q'))
        | (_, KeyCode::Char('?'))
        | (_, KeyCode::F(1)) => {
            app.close_overlay();
        }

        // Scroll within popup
        (_, KeyCode::Up) | (_, KeyCode::Char('k')) => {
            *scroll_offset = scroll_offset.saturating_sub(1);
        }
        (_, KeyCode::Down) | (_, KeyCode::Char('j')) => {
            *scroll_offset += 1;
        }
        (_, KeyCode::PageUp) | (KeyModifiers::CONTROL, KeyCode::Char('u')) => {
            *scroll_offset = scroll_offset.saturating_sub(10);
        }
        (_, KeyCode::PageDown)
        | (KeyModifiers::CONTROL, KeyCode::Char('f'))
        | (KeyModifiers::CONTROL, KeyCode::Char('d')) => {
            *scroll_offset += 10;
        }
        (_, KeyCode::Home) | (_, KeyCode::Char('g')) => {
            *scroll_offset = 0;
        }
        (_, KeyCode::End) | (_, KeyCode::Char('G')) => {
            // Will be clamped by draw function, just set to a large value
            *scroll_offset = usize::MAX;
        }

        _ => {}
    }
}
