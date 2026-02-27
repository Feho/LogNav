use crate::app::{App, FocusState};
use crate::theme::{LIGHT_START_INDEX, THEME_PRESETS};
use crossterm::event::{KeyCode, KeyEvent};

/// Handle key events in theme picker overlay
pub fn handle_theme_picker_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            // Restore original theme
            if let FocusState::ThemePicker {
                original_theme, ..
            } = &app.focus
            {
                app.theme = original_theme.clone();
                recolor_sources(app);
            }
            app.close_overlay();
        }
        KeyCode::Enter => {
            // Confirm current preview
            app.status_message = Some(format!("Theme: {}", app.theme.name));
            app.close_overlay();
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if let FocusState::ThemePicker { selected, .. } = &mut app.focus {
                *selected = selected.saturating_sub(1);
            }
            apply_preview(app);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if let FocusState::ThemePicker { selected, .. } = &mut app.focus {
                let max = THEME_PRESETS.len().saturating_sub(1);
                if *selected < max {
                    *selected += 1;
                }
            }
            apply_preview(app);
        }
        _ => {}
    }
}

/// Apply the currently selected theme as a live preview
fn apply_preview(app: &mut App) {
    let selected = match &app.focus {
        FocusState::ThemePicker { selected, .. } => *selected,
        _ => return,
    };

    let (preset_id, _, constructor) = &THEME_PRESETS[selected];
    let mut theme = constructor();

    // Apply per-theme overrides (dark vs light)
    let overrides = if selected < LIGHT_START_INDEX {
        &app.dark_overrides
    } else {
        &app.light_overrides
    };
    theme.apply_overrides(overrides);
    // Keep the preset name even after overrides
    theme.name = preset_id.to_string();

    app.theme = theme;
    recolor_sources(app);
}

/// Re-color source file gutter colors after theme change
fn recolor_sources(app: &mut App) {
    for (i, source) in app.sources.iter_mut().enumerate() {
        source.color = app.theme.source_color(i as u8);
    }
}
