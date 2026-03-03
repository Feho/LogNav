use crate::app::{App, FocusState};
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle_stats_key(app: &mut App, key: KeyEvent) {
    if !matches!(app.focus, FocusState::Stats { .. }) {
        return;
    }

    match key.code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::F(2) => {
            app.close_overlay();
        }
        _ => {}
    }
}
