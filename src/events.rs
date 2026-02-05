use crate::app::{App, DateFilterField, FocusState};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};

/// Handle crossterm events
pub fn handle_event(app: &mut App, event: Event) {
    match event {
        Event::Key(key) => handle_key(app, key),
        Event::Mouse(mouse) => handle_mouse(app, mouse),
        Event::Resize(_, _) => {} // Ratatui handles this
        _ => {}
    }
}

/// Handle keyboard events
fn handle_key(app: &mut App, key: KeyEvent) {
    match &app.focus {
        FocusState::Normal => handle_normal_key(app, key),
        FocusState::CommandPalette { .. } => handle_command_palette_key(app, key),
        FocusState::Search { .. } => handle_search_key(app, key),
        FocusState::DateFilter { .. } => handle_date_filter_key(app, key),
        FocusState::FileOpen { .. } => handle_file_open_key(app, key),
    }
}

/// Handle keys in normal mode
fn handle_normal_key(app: &mut App, key: KeyEvent) {
    match (key.modifiers, key.code) {
        // Quit
        (_, KeyCode::Esc) | (_, KeyCode::Char('q')) => {
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

        // Search navigation (when search was applied)
        (_, KeyCode::Char('n')) => {
            // Jump to next match if search is active
            if app.search_regex.is_some() {
                app.open_search();
                app.next_search_match();
            }
        }
        (KeyModifiers::SHIFT, KeyCode::Char('N')) => {
            if app.search_regex.is_some() {
                app.open_search();
                app.prev_search_match();
            }
        }

        _ => {}
    }
}

/// Handle keys in command palette
fn handle_command_palette_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.close_overlay();
        }

        KeyCode::Enter => {
            // Get input and selected from focus state
            let (input, selected) = match &app.focus {
                FocusState::CommandPalette { input, selected } => (input.clone(), *selected),
                _ => return,
            };
            let commands = app.get_filtered_commands(&input);
            if let Some((_, cmd, _)) = commands.get(selected) {
                let action = cmd.action;
                app.close_overlay();
                app.execute_command(action);
            }
        }

        KeyCode::Up => {
            if let FocusState::CommandPalette { selected, .. } = &mut app.focus {
                if *selected > 0 {
                    *selected -= 1;
                }
            }
        }

        KeyCode::Down => {
            let (input_clone, current_selected) = match &app.focus {
                FocusState::CommandPalette { input, selected } => (input.clone(), *selected),
                _ => return,
            };
            let count = app.get_filtered_commands(&input_clone).len();
            if let FocusState::CommandPalette { selected, .. } = &mut app.focus {
                if current_selected + 1 < count {
                    *selected = current_selected + 1;
                }
            }
        }

        KeyCode::Char(c) => {
            if let FocusState::CommandPalette { input, selected } = &mut app.focus {
                input.push(c);
                *selected = 0;
            }
        }

        KeyCode::Backspace => {
            if let FocusState::CommandPalette { input, selected } = &mut app.focus {
                input.pop();
                *selected = 0;
            }
        }

        _ => {}
    }
}

/// Handle keys in search mode
fn handle_search_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            // Clear search and close
            app.set_search("");
            app.close_overlay();
        }

        KeyCode::Enter => {
            // Apply search from focus state and close
            let query = match &app.focus {
                FocusState::Search { query, .. } => query.clone(),
                _ => return,
            };
            app.set_search(&query);
            app.update_search_matches();
            app.close_overlay();
        }

        KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            // Apply search first if not yet applied, then navigate
            let query = match &app.focus {
                FocusState::Search { query, .. } => query.clone(),
                _ => return,
            };
            if app.search_query != query {
                app.set_search(&query);
                app.update_search_matches();
            }
            app.next_search_match();
        }

        KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            let query = match &app.focus {
                FocusState::Search { query, .. } => query.clone(),
                _ => return,
            };
            if app.search_query != query {
                app.set_search(&query);
                app.update_search_matches();
            }
            app.prev_search_match();
        }

        KeyCode::Char(c) => {
            // Just update the focus state query, don't apply filter yet
            if let FocusState::Search { query, .. } = &mut app.focus {
                query.push(c);
            }
        }

        KeyCode::Backspace => {
            if let FocusState::Search { query, .. } = &mut app.focus {
                query.pop();
            }
        }

        _ => {}
    }
}

/// Handle keys in date filter dialog
fn handle_date_filter_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.close_overlay();
        }

        KeyCode::Tab => {
            if let FocusState::DateFilter { focused_field, .. } = &mut app.focus {
                *focused_field = match focused_field {
                    DateFilterField::From => DateFilterField::To,
                    DateFilterField::To => DateFilterField::From,
                };
            }
        }

        KeyCode::Enter => {
            let (from, to) = match &app.focus {
                FocusState::DateFilter { from, to, .. } => (from.clone(), to.clone()),
                _ => return,
            };
            // Parse and apply date filters
            app.date_from = parse_date_input(&from);
            app.date_to = parse_date_input(&to);
            app.apply_filters();
            app.close_overlay();
        }

        KeyCode::Char(c) => {
            if let FocusState::DateFilter {
                from,
                to,
                focused_field,
            } = &mut app.focus
            {
                match focused_field {
                    DateFilterField::From => from.push(c),
                    DateFilterField::To => to.push(c),
                }
            }
        }

        KeyCode::Backspace => {
            if let FocusState::DateFilter {
                from,
                to,
                focused_field,
            } = &mut app.focus
            {
                match focused_field {
                    DateFilterField::From => {
                        from.pop();
                    }
                    DateFilterField::To => {
                        to.pop();
                    }
                }
            }
        }

        _ => {}
    }
}

/// Handle keys in file open dialog
fn handle_file_open_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.close_overlay();
        }

        KeyCode::Enter => {
            let file_path = match &app.focus {
                FocusState::FileOpen {
                    path,
                    selected_recent,
                } => {
                    if path.is_empty() && !app.recent_files.is_empty() {
                        app.recent_files.get(*selected_recent).cloned()
                    } else {
                        Some(path.clone())
                    }
                }
                _ => return,
            };

            if let Some(path) = file_path {
                app.file_path = path;
            }
            app.close_overlay();
        }

        KeyCode::Up => {
            if let FocusState::FileOpen {
                selected_recent, ..
            } = &mut app.focus
            {
                if *selected_recent > 0 {
                    *selected_recent -= 1;
                }
            }
        }

        KeyCode::Down => {
            if let FocusState::FileOpen {
                selected_recent, ..
            } = &mut app.focus
            {
                if *selected_recent + 1 < app.recent_files.len() {
                    *selected_recent += 1;
                }
            }
        }

        KeyCode::Tab => {
            // Select from recent files
            let recent_path = match &app.focus {
                FocusState::FileOpen {
                    selected_recent, ..
                } => app.recent_files.get(*selected_recent).cloned(),
                _ => return,
            };
            if let Some(recent) = recent_path {
                if let FocusState::FileOpen { path, .. } = &mut app.focus {
                    *path = recent;
                }
            }
        }

        KeyCode::Char(c) => {
            if let FocusState::FileOpen { path, .. } = &mut app.focus {
                path.push(c);
            }
        }

        KeyCode::Backspace => {
            if let FocusState::FileOpen { path, .. } = &mut app.focus {
                path.pop();
            }
        }

        _ => {}
    }
}

/// Handle mouse events
fn handle_mouse(app: &mut App, mouse: MouseEvent) {
    match mouse.kind {
        MouseEventKind::ScrollUp => {
            app.scroll_up(3);
        }

        MouseEventKind::ScrollDown => {
            app.scroll_down(3);
        }

        MouseEventKind::Down(_) => {
            // Click to select - only in normal mode
            if matches!(app.focus, FocusState::Normal) {
                // Calculate which entry was clicked
                let clicked_row = mouse.row as usize;
                let target_index = app.scroll_offset + clicked_row;
                if target_index < app.filtered_indices.len() {
                    app.selected_index = target_index;
                }
            }
        }

        _ => {}
    }
}

/// Parse date input string into NaiveDateTime
fn parse_date_input(input: &str) -> Option<chrono::NaiveDateTime> {
    let input = input.trim();
    if input.is_empty() {
        return None;
    }

    let current_year = chrono::Local::now().format("%Y").to_string();

    // Try various formats
    let formats = [
        ("%Y-%m-%d %H:%M:%S", input.to_string()),
        ("%Y-%m-%d %H:%M", input.to_string()),
        ("%m-%d %H:%M:%S", format!("{}-{}", current_year, input)),
        ("%m-%d %H:%M", format!("{}-{}", current_year, input)),
    ];

    for (fmt, date_str) in &formats {
        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(date_str, fmt) {
            return Some(dt);
        }
    }

    None
}
