use crate::app::{App, FocusState};
use crossterm::event::{MouseEvent, MouseEventKind};

/// Handle mouse events
pub fn handle_mouse(app: &mut App, mouse: MouseEvent) {
    match mouse.kind {
        MouseEventKind::ScrollUp => {
            app.scroll_viewport_up(3, app.viewport_height);
        }

        MouseEventKind::ScrollDown => {
            app.scroll_viewport_down(3, app.viewport_height);
        }

        MouseEventKind::Down(_) => {
            // Click to select - only in normal mode
            if matches!(app.focus, FocusState::Normal) {
                let clicked_row = mouse.row as usize;
                // Walk through visible entries accounting for expanded entries
                let mut visual_row = 0;
                let mut entry_idx = app.scroll_offset;
                while entry_idx < app.filtered_indices.len() {
                    let lines = app.visual_lines_for_entry(entry_idx, 0); // Mouse uses nowrap logic for clicks
                    if visual_row + lines > clicked_row {
                        app.selected_index = entry_idx;
                        break;
                    }
                    visual_row += lines;
                    entry_idx += 1;
                }
            }
        }

        _ => {}
    }
}
