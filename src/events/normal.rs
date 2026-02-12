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
            if app.search_panel_matches.is_empty() && !app.highlight_query.is_empty() {
                let q = app.highlight_query.clone();
                let m = app.highlight_regex_mode;
                app.commit_search_to_panel(&q, m);
            }
            if !app.search_panel_matches.is_empty() {
                app.next_match();
            }
        }

        // N: previous search match (vim-style, redo last search if panel closed)
        (KeyModifiers::SHIFT, KeyCode::Char('N')) => {
            if app.search_panel_matches.is_empty() && !app.highlight_query.is_empty() {
                let q = app.highlight_query.clone();
                let m = app.highlight_regex_mode;
                app.commit_search_to_panel(&q, m);
            }
            if !app.search_panel_matches.is_empty() {
                app.prev_match();
            }
        }

        // Toggle tail
        (KeyModifiers::CONTROL, KeyCode::Char('t')) | (_, KeyCode::Char('t')) => {
            app.toggle_tail();
            app.status_message = Some(format!(
                "Tail mode {}",
                if app.tail_enabled { "ON" } else { "OFF" }
            ));
        }

        // Toggle wrap
        (KeyModifiers::CONTROL, KeyCode::Char('w')) | (_, KeyCode::Char('w')) => {
            app.toggle_wrap();
            app.status_message = Some(format!(
                "Word wrap {}",
                if app.wrap_enabled { "ON" } else { "OFF" }
            ));
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
        (_, KeyCode::Char('1')) => {
            app.toggle_level(0);
            let level_name = ["ERR", "WRN", "INF", "DBG", "TRC", "PRF"][0];
            let state = if app.level_filters[0] { "ON" } else { "OFF" };
            app.status_message = Some(format!("Level {} {}", level_name, state));
        }
        (_, KeyCode::Char('2')) => {
            app.toggle_level(1);
            let level_name = ["ERR", "WRN", "INF", "DBG", "TRC", "PRF"][1];
            let state = if app.level_filters[1] { "ON" } else { "OFF" };
            app.status_message = Some(format!("Level {} {}", level_name, state));
        }
        (_, KeyCode::Char('3')) => {
            app.toggle_level(2);
            let level_name = ["ERR", "WRN", "INF", "DBG", "TRC", "PRF"][2];
            let state = if app.level_filters[2] { "ON" } else { "OFF" };
            app.status_message = Some(format!("Level {} {}", level_name, state));
        }
        (_, KeyCode::Char('4')) => {
            app.toggle_level(3);
            let level_name = ["ERR", "WRN", "INF", "DBG", "TRC", "PRF"][3];
            let state = if app.level_filters[3] { "ON" } else { "OFF" };
            app.status_message = Some(format!("Level {} {}", level_name, state));
        }
        (_, KeyCode::Char('5')) => {
            app.toggle_level(4);
            let level_name = ["ERR", "WRN", "INF", "DBG", "TRC", "PRF"][4];
            let state = if app.level_filters[4] { "ON" } else { "OFF" };
            app.status_message = Some(format!("Level {} {}", level_name, state));
        }
        (_, KeyCode::Char('6')) => {
            app.toggle_level(5);
            let level_name = ["ERR", "WRN", "INF", "DBG", "TRC", "PRF"][5];
            let state = if app.level_filters[5] { "ON" } else { "OFF" };
            app.status_message = Some(format!("Level {} {}", level_name, state));
        }
        (_, KeyCode::Char('0')) => {
            app.reset_level_filters();
            app.status_message = Some("Levels reset to default".to_string());
        }

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

        _ => {}
    }
}
