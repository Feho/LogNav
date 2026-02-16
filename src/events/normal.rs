use crate::app::App;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Handle keys in normal mode
pub fn handle_normal_key(app: &mut App, key: KeyEvent) {
    match (key.modifiers, key.code) {
        // Quit
        (_, KeyCode::Char('q')) => {
            app.should_quit = true;
        }

        // Esc: close search panel if open
        (_, KeyCode::Esc) => {
            if app.search_panel_open {
                app.close_search_panel();
            }
        }

        // Help
        (_, KeyCode::Char('?')) | (_, KeyCode::F(1)) => {
            app.open_help();
        }

        // Command palette
        (KeyModifiers::CONTROL, KeyCode::Char('p')) => {
            app.open_command_palette();
        }

        // Open file
        (KeyModifiers::CONTROL, KeyCode::Char('o')) | (_, KeyCode::Char('o')) => {
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

        // Tab: toggle search panel focus
        (_, KeyCode::Tab) => {
            if app.search_panel_open {
                app.search_panel_focused = !app.search_panel_focused;
            }
        }

        // n: next search match (vim-style, redo last search if panel closed)
        (_, KeyCode::Char('n')) => {
            if app.search_panel_matches.is_empty() && !app.search.query.is_empty() {
                let q = app.search.query.clone();
                let m = app.search.regex_mode;
                app.commit_search_to_panel(&q, m);
            }
            if !app.search_panel_matches.is_empty() {
                app.next_match();
            }
        }

        // N: previous search match (vim-style, redo last search if panel closed)
        (KeyModifiers::SHIFT, KeyCode::Char('N')) => {
            if app.search_panel_matches.is_empty() && !app.search.query.is_empty() {
                let q = app.search.query.clone();
                let m = app.search.regex_mode;
                app.commit_search_to_panel(&q, m);
            }
            if !app.search_panel_matches.is_empty() {
                app.prev_match();
            }
        }

        // e: next error
        (_, KeyCode::Char('e')) => {
            app.next_error();
        }

        // E: previous error
        (KeyModifiers::SHIFT, KeyCode::Char('E')) => {
            app.prev_error();
        }

        // Toggle tail
        (KeyModifiers::CONTROL, KeyCode::Char('t')) | (_, KeyCode::Char('t')) => {
            app.toggle_tail();
            app.status_message = Some(format!(
                "Tail mode {}",
                if app.tail_enabled { "ON" } else { "OFF" }
            ));
        }

        // Ctrl+w: toggle wrap
        (KeyModifiers::CONTROL, KeyCode::Char('w')) => {
            app.toggle_wrap();
            app.status_message = Some(format!(
                "Word wrap {}",
                if app.wrap_enabled { "ON" } else { "OFF" }
            ));
        }

        // w: next warning
        (_, KeyCode::Char('w')) => {
            app.next_warning();
        }

        // W: previous warning
        (KeyModifiers::SHIFT, KeyCode::Char('W')) => {
            app.prev_warning();
        }

        // Toggle syntax highlighting
        (_, KeyCode::Char('s')) => {
            app.toggle_syntax_highlight();
            app.status_message = Some(format!(
                "Syntax highlight {}",
                if app.syntax_highlight { "ON" } else { "OFF" }
            ));
        }

        // Level toggles (1-6) with status messages
        (_, KeyCode::Char(c @ '1'..='6')) => {
            const LEVEL_NAMES: [&str; 6] = ["ERR", "WRN", "INF", "DBG", "TRC", "PRF"];
            let idx = (c as u8 - b'1') as usize;
            app.toggle_level(idx);
            let state = if app.level_filters[idx] { "ON" } else { "OFF" };
            app.status_message = Some(format!("Level {} {}", LEVEL_NAMES[idx], state));
        }
        (_, KeyCode::Char('0')) => {
            app.reset_level_filters();
            app.status_message = Some("Levels reset to default".to_string());
        }

        // Bookmarks
        (_, KeyCode::Char('m')) => app.toggle_bookmark(),
        (_, KeyCode::Char('b')) => app.next_bookmark(),
        (KeyModifiers::SHIFT, KeyCode::Char('B')) => app.prev_bookmark(),

        // Navigation
        (_, KeyCode::Up) | (_, KeyCode::Char('k')) => app.scroll_up(1),
        (_, KeyCode::Down) | (_, KeyCode::Char('j')) => app.scroll_down(1),
        (_, KeyCode::PageUp) | (KeyModifiers::CONTROL, KeyCode::Char('u')) => {
            let amount = app.viewport_height.saturating_sub(1).max(1);
            app.scroll_viewport_up(amount, app.viewport_height);
        }
        (_, KeyCode::PageDown) => {
            let amount = app.viewport_height.saturating_sub(1).max(1);
            app.scroll_viewport_down(amount, app.viewport_height);
        }
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

        // Clear exclude filters
        (_, KeyCode::Char('x')) => {
            let count = app.exclude_patterns.len();
            app.clear_excludes();
            app.status_message = Some(format!("Cleared {} exclude filter(s)", count));
        }

        _ => {}
    }
}
