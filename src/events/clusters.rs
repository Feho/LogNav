use crate::app::{App, FocusState};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Handle keys in cluster overlay
pub fn handle_clusters_key(app: &mut App, key: KeyEvent) {
    let (selected, scroll_offset) = match &mut app.focus {
        FocusState::Clusters {
            selected,
            scroll_offset,
        } => (selected, scroll_offset),
        _ => return,
    };

    let total = app.clusters.len();
    if total == 0 {
        app.close_overlay();
        return;
    }

    match (key.modifiers, key.code) {
        // Close
        (_, KeyCode::Esc) => {
            app.close_overlay();
        }
        (_, KeyCode::Char('q')) => {
            app.should_quit = true;
        }

        // Navigate
        (_, KeyCode::Down) | (_, KeyCode::Char('j')) => {
            if *selected + 1 < total {
                *selected += 1;
            }
        }
        (_, KeyCode::Up) | (_, KeyCode::Char('k')) => {
            *selected = selected.saturating_sub(1);
        }
        (_, KeyCode::Home) | (_, KeyCode::Char('g')) => {
            *selected = 0;
            *scroll_offset = 0;
        }
        (_, KeyCode::End) | (KeyModifiers::SHIFT, KeyCode::Char('G')) => {
            *selected = total.saturating_sub(1);
        }
        (_, KeyCode::Char('G')) => {
            *selected = total.saturating_sub(1);
        }
        (_, KeyCode::PageDown) => {
            *selected = (*selected + 10).min(total.saturating_sub(1));
        }
        (_, KeyCode::PageUp) | (KeyModifiers::CONTROL, KeyCode::Char('u')) => {
            *selected = selected.saturating_sub(10);
        }

        // Jump to cluster start
        (_, KeyCode::Enter) => {
            let sel = *selected;
            if let Some(cluster) = app.clusters.get(sel) {
                let filtered_idx = cluster.start_filtered_idx;
                app.selected_index = filtered_idx;
                app.ensure_selected_visible();
            }
            app.close_overlay();
        }

        _ => {}
    }
}
