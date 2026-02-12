use crate::app::{App, FocusState};
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

/// Handle mouse events
pub fn handle_mouse(app: &mut App, mouse: MouseEvent) {
    match mouse.kind {
        MouseEventKind::ScrollUp => {
            if app.search_panel_open && app.search_panel_height > 0 {
                let terminal_height = app.viewport_height + app.search_panel_height + 1;
                let panel_start = terminal_height.saturating_sub(app.search_panel_height + 1);
                if (mouse.row as usize) >= panel_start {
                    app.panel_scroll_up(3);
                    return;
                }
            }
            app.scroll_viewport_up(3, app.viewport_height);
        }

        MouseEventKind::ScrollDown => {
            if app.search_panel_open && app.search_panel_height > 0 {
                let terminal_height = app.viewport_height + app.search_panel_height + 1;
                let panel_start = terminal_height.saturating_sub(app.search_panel_height + 1);
                if (mouse.row as usize) >= panel_start {
                    app.panel_scroll_down(3);
                    return;
                }
            }
            app.scroll_viewport_down(3, app.viewport_height);
        }

        MouseEventKind::Down(MouseButton::Left) => {
            if !matches!(app.focus, FocusState::Normal) {
                return;
            }

            let clicked_row = mouse.row as usize;

            // Check if click is in the search panel area
            if app.search_panel_open && app.search_panel_height > 0 {
                // The panel is at the bottom of the content area (above status bar).
                // Total terminal height minus 1 (status bar) = content area.
                // Panel occupies the last search_panel_height rows of content area.
                let terminal_height = app.viewport_height + app.search_panel_height + 1; // +1 for status bar
                let panel_start = terminal_height.saturating_sub(app.search_panel_height + 1);

                if clicked_row >= panel_start && clicked_row < panel_start + app.search_panel_height
                {
                    // Click is in the panel area
                    app.search_panel_focused = true;

                    // Account for border (1 row top border)
                    let inner_row = clicked_row.saturating_sub(panel_start + 1);
                    let match_idx = app.search_panel_scroll + inner_row;

                    if match_idx < app.search_panel_matches.len() {
                        app.search_panel_selected = match_idx;
                        app.sync_main_to_panel_selection();
                    }
                    return;
                }
            }

            // Click in main log view
            if app.search_panel_open {
                app.search_panel_focused = false;
            }

            let mut visual_row = 0;
            let mut entry_idx = app.scroll_offset;
            while entry_idx < app.filtered_indices.len() {
                let lines = app.visual_lines_for_entry(entry_idx, 0);
                if visual_row + lines > clicked_row {
                    app.selected_index = entry_idx;
                    break;
                }
                visual_row += lines;
                entry_idx += 1;
            }
        }

        _ => {}
    }
}
