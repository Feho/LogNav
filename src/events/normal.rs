use crate::app::App;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Handle keys in normal mode
pub fn handle_normal_key(app: &mut App, key: KeyEvent) {
    match (key.modifiers, key.code) {
        // Quit
        (_, KeyCode::Char('q')) => {
            app.should_quit = true;
        }

        // Command palette
        (KeyModifiers::CONTROL, KeyCode::Char('p')) => {
            app.open_command_palette();
        }

        // Open file
        (KeyModifiers::CONTROL, KeyCode::Char('o')) => {
            app.open_file_dialog();
        }

        // Search
        (KeyModifiers::CONTROL, KeyCode::Char('f')) | (_, KeyCode::Char('/')) => {
            app.open_search();
        }

        // Date filter
        (KeyModifiers::CONTROL, KeyCode::Char('d')) => {
            app.open_date_filter();
        }

        // Toggle tail
        (KeyModifiers::CONTROL, KeyCode::Char('t')) | (_, KeyCode::Char('t')) => {
            app.toggle_tail();
        }

        // Toggle wrap
        (KeyModifiers::CONTROL, KeyCode::Char('w')) | (_, KeyCode::Char('w')) => {
            app.toggle_wrap();
        }

        // Level toggles (1-6)
        (_, KeyCode::Char('1')) => app.toggle_level(0),
        (_, KeyCode::Char('2')) => app.toggle_level(1),
        (_, KeyCode::Char('3')) => app.toggle_level(2),
        (_, KeyCode::Char('4')) => app.toggle_level(3),
        (_, KeyCode::Char('5')) => app.toggle_level(4),
        (_, KeyCode::Char('6')) => app.toggle_level(5),

        // Navigation
        (_, KeyCode::Up) | (_, KeyCode::Char('k')) => app.scroll_up(1),
        (_, KeyCode::Down) | (_, KeyCode::Char('j')) => app.scroll_down(1),
        (_, KeyCode::PageUp) | (KeyModifiers::CONTROL, KeyCode::Char('u')) => app.scroll_up(20),
        (_, KeyCode::PageDown) => app.scroll_down(20),
        (_, KeyCode::Home) | (_, KeyCode::Char('g')) => app.scroll_to_top(),
        (_, KeyCode::End) | (KeyModifiers::SHIFT, KeyCode::Char('G')) => app.scroll_to_bottom(),
        (_, KeyCode::Char('G')) => app.scroll_to_bottom(),

        // Horizontal scroll
        (_, KeyCode::Left) | (_, KeyCode::Char('h')) => app.scroll_left(4),
        (_, KeyCode::Right) | (_, KeyCode::Char('l')) => app.scroll_right(4),

        // Expand/collapse entry
        (_, KeyCode::Enter) => app.toggle_expand(),
        (_, KeyCode::Char('a')) | (_, KeyCode::Char('A')) => {
            app.toggle_expand_all();
        }

        // Detail popup
        (_, KeyCode::Char('d')) => {
            app.open_detail_popup();
        }

        // Copy current line
        (_, KeyCode::Char('c')) => {
            app.copy_current_line();
        }

        _ => {}
    }
}
