use crate::app::App;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Handle keys when the search matches panel is focused
pub fn handle_search_panel_key(app: &mut App, key: KeyEvent) {
    match (key.modifiers, key.code) {
        // Quit
        (_, KeyCode::Char('q')) => {
            app.should_quit = true;
        }

        // Close panel
        (_, KeyCode::Esc) => {
            app.close_search_panel();
        }

        // Switch focus back to main view
        (_, KeyCode::Tab) => {
            app.search_panel_focused = false;
        }

        // Navigation within panel
        (_, KeyCode::Down) | (_, KeyCode::Char('j')) | (_, KeyCode::Char('n')) => {
            app.panel_scroll_down(1);
        }

        (_, KeyCode::Up) | (_, KeyCode::Char('k')) => {
            app.panel_scroll_up(1);
        }

        (KeyModifiers::SHIFT, KeyCode::Char('N')) => {
            app.prev_match();
        }

        // Page up/down
        (_, KeyCode::PageDown) => {
            let amount = app.search_panel_height.saturating_sub(3).max(1);
            app.panel_scroll_down(amount);
        }

        (_, KeyCode::PageUp) | (KeyModifiers::CONTROL, KeyCode::Char('u')) => {
            let amount = app.search_panel_height.saturating_sub(3).max(1);
            app.panel_scroll_up(amount);
        }

        // Top/bottom of matches
        (_, KeyCode::Home) | (_, KeyCode::Char('g')) => {
            app.panel_scroll_to_top();
        }

        (_, KeyCode::End) | (KeyModifiers::SHIFT, KeyCode::Char('G')) => {
            app.panel_scroll_to_bottom();
        }
        (_, KeyCode::Char('G')) => {
            app.panel_scroll_to_bottom();
        }

        // Open search overlay (re-search)
        (KeyModifiers::CONTROL, KeyCode::Char('f')) | (_, KeyCode::Char('/')) => {
            app.open_search();
        }

        // Help
        (_, KeyCode::Char('?')) | (_, KeyCode::F(1)) => {
            app.open_help();
        }

        // Enter: no-op (already selected/synced)
        (_, KeyCode::Enter) => {}

        _ => {
            super::normal::handle_normal_key(app, key);
        }
    }
}
