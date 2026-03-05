use crate::app::{App, FocusState, ZOOM_LEVELS};
use crossterm::event::{KeyCode, KeyEvent};

/// Number of raw 1-min buckets per display bar at zoom level `z`.
fn raw_per_bar(z: usize) -> usize {
    (ZOOM_LEVELS[z].0 / 60_000) as usize
}

pub fn handle_stats_key(app: &mut App, key: KeyEvent) {
    let (zoom_idx, total_raw) = match &app.focus {
        FocusState::Stats { data, zoom_idx, .. } => (*zoom_idx, data.buckets.len()),
        _ => return,
    };

    match key.code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::F(2) => {
            app.close_overlay();
        }
        // Zoom in (finer granularity)
        KeyCode::Char('+') | KeyCode::Char('=') if zoom_idx > 0 => {
            mutate_zoom_pan(&mut app.focus, |z, p, _| {
                *z -= 1;
                *p = (*p).min(total_raw.saturating_sub(raw_per_bar(*z)));
            });
        }
        // Zoom out (coarser granularity)
        KeyCode::Char('-') if zoom_idx < ZOOM_LEVELS.len() - 1 => {
            mutate_zoom_pan(&mut app.focus, |z, p, _| {
                *z += 1;
                *p = (*p).min(total_raw.saturating_sub(raw_per_bar(*z)));
            });
        }
        // Pan right
        KeyCode::Right | KeyCode::Char('l') => {
            mutate_zoom_pan(&mut app.focus, |z, p, _| {
                let step = raw_per_bar(*z).max(1);
                *p = (*p + step).min(total_raw.saturating_sub(raw_per_bar(*z)));
            });
        }
        // Pan left
        KeyCode::Left | KeyCode::Char('h') => {
            mutate_zoom_pan(&mut app.focus, |z, p, _| {
                let step = raw_per_bar(*z).max(1);
                *p = p.saturating_sub(step);
            });
        }
        // Pan to start
        KeyCode::Home => {
            mutate_zoom_pan(&mut app.focus, |_, p, _| {
                *p = 0;
            });
        }
        // Pan to end
        KeyCode::End => {
            mutate_zoom_pan(&mut app.focus, |z, p, _| {
                *p = total_raw.saturating_sub(raw_per_bar(*z));
            });
        }
        // Reset zoom/pan to default
        KeyCode::Char('0') => {
            mutate_zoom_pan(&mut app.focus, |z, p, data| {
                if let Some((t_min, t_max)) = data.time_range {
                    *z = crate::app::default_zoom_idx(t_max - t_min);
                }
                *p = 0;
            });
        }
        // Export as HTML
        KeyCode::Char('e') => {
            app.open_stats_export();
        }
        _ => {}
    }
}

fn mutate_zoom_pan(
    focus: &mut FocusState,
    f: impl FnOnce(&mut usize, &mut usize, &crate::app::StatsData),
) {
    if let FocusState::Stats {
        data,
        zoom_idx,
        pan_offset,
    } = focus
    {
        f(zoom_idx, pan_offset, data);
    }
}
