use crate::app::{App, DateFilterFocus, FocusState, QUICK_FILTERS};
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
        (_, KeyCode::Char('q')) => {
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

        // Expand/collapse entry
        (_, KeyCode::Enter) => app.toggle_expand(),
        (_, KeyCode::Char('a')) | (_, KeyCode::Char('A')) => {
            app.toggle_expand_all();
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

/// Compute search matches based on query without applying filter
fn update_search_matches(
    entries: &[crate::log_entry::LogEntry],
    query: &str,
    match_indices: &mut Vec<usize>,
    current_match: &mut usize,
) {
    match_indices.clear();

    if query.is_empty() {
        *current_match = 0;
        return;
    }

    // Try to compile regex from query
    if let Ok(regex) = regex::Regex::new(&format!("(?i){}", regex::escape(query))) {
        *match_indices = entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| regex.is_match(entry.searchable_text()))
            .map(|(idx, _)| idx)
            .collect();
    }

    if !match_indices.is_empty() && *current_match >= match_indices.len() {
        *current_match = 0;
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
            // Apply search filter from focus state and close
            let query = match &app.focus {
                FocusState::Search { query, .. } => query.clone(),
                _ => return,
            };
            app.set_search(&query);
            app.update_search_matches();
            app.close_overlay();
        }

        KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            // Navigate to next match in search results without filtering
            match &mut app.focus {
                FocusState::Search {
                    match_indices,
                    current_match,
                    ..
                } => {
                    if !match_indices.is_empty() {
                        *current_match = (*current_match + 1) % match_indices.len();
                        let target = match_indices[*current_match];
                        app.selected_index = target;
                        app.ensure_selected_visible();
                    }
                }
                _ => {}
            }
        }

        KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            // Navigate to previous match in search results without filtering
            match &mut app.focus {
                FocusState::Search {
                    match_indices,
                    current_match,
                    ..
                } => {
                    if !match_indices.is_empty() {
                        *current_match = if *current_match == 0 {
                            match_indices.len() - 1
                        } else {
                            *current_match - 1
                        };
                        let target = match_indices[*current_match];
                        app.selected_index = target;
                        app.ensure_selected_visible();
                    }
                }
                _ => {}
            }
        }

        KeyCode::Char(c) => {
            // Update query and recompute match_indices without filtering
            let query = match &mut app.focus {
                FocusState::Search { query, .. } => {
                    query.push(c);
                    query.clone()
                }
                _ => return,
            };
            // Recompute matches as user types
            if let FocusState::Search {
                match_indices,
                current_match,
                ..
            } = &mut app.focus
            {
                update_search_matches(&app.entries, &query, match_indices, current_match);
            }
        }

        KeyCode::Backspace => {
            let query = match &mut app.focus {
                FocusState::Search { query, .. } => {
                    query.pop();
                    query.clone()
                }
                _ => return,
            };
            // Recompute matches as user deletes
            if let FocusState::Search {
                match_indices,
                current_match,
                ..
            } = &mut app.focus
            {
                update_search_matches(&app.entries, &query, match_indices, current_match);
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
            return;
        }

        KeyCode::Tab => {
            if let FocusState::DateFilter { focus, .. } = &mut app.focus {
                *focus = match focus {
                    DateFilterFocus::QuickFilter => DateFilterFocus::From,
                    DateFilterFocus::From => DateFilterFocus::To,
                    DateFilterFocus::To => DateFilterFocus::QuickFilter,
                };
            }
            return;
        }

        KeyCode::BackTab => {
            if let FocusState::DateFilter { focus, .. } = &mut app.focus {
                *focus = match focus {
                    DateFilterFocus::QuickFilter => DateFilterFocus::To,
                    DateFilterFocus::From => DateFilterFocus::QuickFilter,
                    DateFilterFocus::To => DateFilterFocus::From,
                };
            }
            return;
        }

        _ => {}
    }

    let current_focus = match &app.focus {
        FocusState::DateFilter { focus, .. } => *focus,
        _ => return,
    };

    match current_focus {
        DateFilterFocus::QuickFilter => handle_quick_filter_key(app, key),
        DateFilterFocus::From | DateFilterFocus::To => handle_custom_date_key(app, key),
    }
}

/// Handle keys when quick filter list is focused
fn handle_quick_filter_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Up => {
            if let FocusState::DateFilter { selected_quick, .. } = &mut app.focus {
                *selected_quick = selected_quick.saturating_sub(1);
            }
        }

        KeyCode::Down => {
            if let FocusState::DateFilter { selected_quick, .. } = &mut app.focus
                && *selected_quick + 1 < QUICK_FILTERS.len()
            {
                *selected_quick += 1;
            }
        }

        KeyCode::Enter => {
            let selected = match &app.focus {
                FocusState::DateFilter { selected_quick, .. } => *selected_quick,
                _ => return,
            };
            apply_quick_filter(app, selected);
        }

        // Direct number selection: 1-6
        KeyCode::Char(c @ '1'..='6') => {
            let idx = (c as usize) - ('1' as usize);
            if idx < QUICK_FILTERS.len() {
                apply_quick_filter(app, idx);
            }
        }

        _ => {}
    }
}

/// Apply a quick filter by index
fn apply_quick_filter(app: &mut App, index: usize) {
    let now = chrono::Local::now().naive_local();
    let today_start = now.date().and_hms_opt(0, 0, 0).unwrap();

    match index {
        0 => {
            // Last hour
            app.date_from = Some(now - chrono::Duration::hours(1));
            app.date_to = Some(now);
        }
        1 => {
            // Last 24 hours
            app.date_from = Some(now - chrono::Duration::hours(24));
            app.date_to = Some(now);
        }
        2 => {
            // Today
            app.date_from = Some(today_start);
            app.date_to = Some(now);
        }
        3 => {
            // Yesterday
            let yesterday_start = today_start - chrono::Duration::days(1);
            app.date_from = Some(yesterday_start);
            app.date_to = Some(today_start);
        }
        4 => {
            // Last 7 days
            app.date_from = Some(now - chrono::Duration::days(7));
            app.date_to = Some(now);
        }
        5 => {
            // Clear filter
            app.date_from = None;
            app.date_to = None;
        }
        _ => return,
    }

    app.apply_filters();
    app.close_overlay();
}

/// Handle keys when custom date fields are focused
fn handle_custom_date_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Enter => {
            let (from, to) = match &app.focus {
                FocusState::DateFilter { from, to, .. } => (from.clone(), to.clone()),
                _ => return,
            };

            let parsed_from = parse_date_input(&from, false);
            let parsed_to = parse_date_input(&to, true);

            let from_err = !from.trim().is_empty() && parsed_from.is_none();
            let to_err = !to.trim().is_empty() && parsed_to.is_none();

            if from_err || to_err {
                if let FocusState::DateFilter { error, .. } = &mut app.focus {
                    *error = Some(
                        match (from_err, to_err) {
                            (true, true) => "Invalid 'From' and 'To' format",
                            (true, false) => "Invalid 'From' format",
                            (false, true) => "Invalid 'To' format",
                            _ => unreachable!(),
                        }
                        .to_string(),
                    );
                }
                return;
            }

            app.date_from = parsed_from;
            app.date_to = parsed_to;
            app.apply_filters();
            app.close_overlay();
        }

        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if let FocusState::DateFilter {
                from,
                to,
                focus,
                error,
                ..
            } = &mut app.focus
            {
                *error = None;
                match focus {
                    DateFilterFocus::From => from.clear(),
                    DateFilterFocus::To => to.clear(),
                    _ => {}
                }
            }
        }

        KeyCode::Char(c) => {
            if let FocusState::DateFilter {
                from,
                to,
                focus,
                error,
                ..
            } = &mut app.focus
            {
                *error = None;
                match focus {
                    DateFilterFocus::From => from.push(c),
                    DateFilterFocus::To => to.push(c),
                    _ => {}
                }
            }
        }

        KeyCode::Backspace => {
            if let FocusState::DateFilter {
                from,
                to,
                focus,
                error,
                ..
            } = &mut app.focus
            {
                *error = None;
                match focus {
                    DateFilterFocus::From => {
                        from.pop();
                    }
                    DateFilterFocus::To => {
                        to.pop();
                    }
                    _ => {}
                }
            }
        }

        _ => {}
    }
}

/// Convert a char index to a byte index within a string
fn char_to_byte_index(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(i, _)| i)
        .unwrap_or(s.len())
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
                    ..
                } => {
                    if path.is_empty() && !app.recent_files.is_empty() {
                        app.recent_files.get(*selected_recent).cloned()
                    } else {
                        // Tilde expansion
                        let expanded = if path == "~" {
                            std::env::var("HOME").unwrap_or_else(|_| path.clone())
                        } else if let Some(rest) = path.strip_prefix("~/") {
                            match std::env::var("HOME") {
                                Ok(home) => format!("{}/{}", home, rest),
                                Err(_) => path.clone(),
                            }
                        } else {
                            path.clone()
                        };
                        Some(expanded)
                    }
                }
                _ => return,
            };

            if let Some(path) = file_path {
                if !std::path::Path::new(&path).is_file() {
                    if let FocusState::FileOpen { error, .. } = &mut app.focus {
                        *error = Some(format!("File not found: {}", path));
                    }
                    return;
                }
                app.file_path = path;
            }
            app.close_overlay();
        }

        KeyCode::Up => {
            if let FocusState::FileOpen {
                selected_recent, ..
            } = &mut app.focus
                && *selected_recent > 0
            {
                *selected_recent -= 1;
            }
        }

        KeyCode::Down => {
            if let FocusState::FileOpen {
                selected_recent, ..
            } = &mut app.focus
                && *selected_recent + 1 < app.recent_files.len()
            {
                *selected_recent += 1;
            }
        }

        KeyCode::Left => {
            if let FocusState::FileOpen { cursor_pos, .. } = &mut app.focus {
                *cursor_pos = cursor_pos.saturating_sub(1);
            }
        }

        KeyCode::Right => {
            if let FocusState::FileOpen {
                path, cursor_pos, ..
            } = &mut app.focus
            {
                let char_count = path.chars().count();
                if *cursor_pos < char_count {
                    *cursor_pos += 1;
                }
            }
        }

        KeyCode::Home => {
            if let FocusState::FileOpen { cursor_pos, .. } = &mut app.focus {
                *cursor_pos = 0;
            }
        }

        KeyCode::End => {
            if let FocusState::FileOpen {
                path, cursor_pos, ..
            } = &mut app.focus
            {
                *cursor_pos = path.chars().count();
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
            if let Some(recent) = recent_path
                && let FocusState::FileOpen {
                    path,
                    cursor_pos,
                    error,
                    ..
                } = &mut app.focus
            {
                *path = recent;
                *cursor_pos = path.chars().count();
                *error = None;
            }
        }

        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if let FocusState::FileOpen {
                path,
                cursor_pos,
                error,
                ..
            } = &mut app.focus
            {
                path.clear();
                *cursor_pos = 0;
                *error = None;
            }
        }

        KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if let FocusState::FileOpen {
                path,
                cursor_pos,
                error,
                ..
            } = &mut app.focus
                && *cursor_pos > 0
            {
                *error = None;
                let byte_end = char_to_byte_index(path, *cursor_pos);
                let mut new_pos = *cursor_pos;

                // Skip trailing '/' separators
                while new_pos > 0 {
                    let bi = char_to_byte_index(path, new_pos - 1);
                    if path[bi..].starts_with('/') {
                        new_pos -= 1;
                    } else {
                        break;
                    }
                }

                // Delete back to previous '/' or start
                while new_pos > 0 {
                    let bi = char_to_byte_index(path, new_pos - 1);
                    if path[bi..].starts_with('/') {
                        break;
                    }
                    new_pos -= 1;
                }

                let byte_start = char_to_byte_index(path, new_pos);
                path.drain(byte_start..byte_end);
                *cursor_pos = new_pos;
            }
        }

        KeyCode::Char(c) => {
            if let FocusState::FileOpen {
                path,
                cursor_pos,
                error,
                ..
            } = &mut app.focus
            {
                let byte_idx = char_to_byte_index(path, *cursor_pos);
                path.insert(byte_idx, c);
                *cursor_pos += 1;
                *error = None;
            }
        }

        KeyCode::Backspace => {
            if let FocusState::FileOpen {
                path,
                cursor_pos,
                error,
                ..
            } = &mut app.focus
                && *cursor_pos > 0
            {
                let byte_idx = char_to_byte_index(path, *cursor_pos - 1);
                path.remove(byte_idx);
                *cursor_pos -= 1;
                *error = None;
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
                let clicked_row = mouse.row as usize;
                // Walk through visible entries accounting for expanded entries
                let mut visual_row = 0;
                let mut entry_idx = app.scroll_offset;
                while entry_idx < app.filtered_indices.len() {
                    let lines = app.visual_lines_for_entry(entry_idx);
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

/// Parse date input string into NaiveDateTime
///
/// When `is_end` is true, date-only inputs resolve to 23:59:59 (end of day).
/// When false, they resolve to 00:00:00 (start of day).
fn parse_date_input(input: &str, is_end: bool) -> Option<chrono::NaiveDateTime> {
    let input = input.trim();
    if input.is_empty() {
        return None;
    }

    let now = chrono::Local::now().naive_local();
    let current_year = now.format("%Y").to_string();

    // Keywords
    match input.to_lowercase().as_str() {
        "now" => return Some(now),
        "today" => {
            return if is_end {
                now.date().and_hms_opt(23, 59, 59)
            } else {
                now.date().and_hms_opt(0, 0, 0)
            };
        }
        "yesterday" => {
            let yesterday = now.date() - chrono::Duration::days(1);
            return if is_end {
                yesterday.and_hms_opt(23, 59, 59)
            } else {
                yesterday.and_hms_opt(0, 0, 0)
            };
        }
        _ => {}
    }

    // Relative times: -Nh, -Nm, -Nd
    if let Some(rel) = input.strip_prefix('-') {
        if let Some(num_str) = rel.strip_suffix('h')
            && let Ok(hours) = num_str.parse::<i64>()
        {
            return Some(now - chrono::Duration::hours(hours));
        }
        if let Some(num_str) = rel.strip_suffix('m')
            && let Ok(minutes) = num_str.parse::<i64>()
        {
            return Some(now - chrono::Duration::minutes(minutes));
        }
        if let Some(num_str) = rel.strip_suffix('d')
            && let Ok(days) = num_str.parse::<i64>()
        {
            return Some(now - chrono::Duration::days(days));
        }
    }

    // Full datetime formats
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

    // Date only: "YYYY-MM-dd"
    if let Ok(date) = chrono::NaiveDate::parse_from_str(input, "%Y-%m-%d") {
        return if is_end {
            date.and_hms_opt(23, 59, 59)
        } else {
            date.and_hms_opt(0, 0, 0)
        };
    }

    // Date only: "MM-dd" (assumes current year)
    let with_year = format!("{}-{}", current_year, input);
    if let Ok(date) = chrono::NaiveDate::parse_from_str(&with_year, "%Y-%m-%d") {
        return if is_end {
            date.and_hms_opt(23, 59, 59)
        } else {
            date.and_hms_opt(0, 0, 0)
        };
    }

    // Time only: "HH:mm:ss" (assumes today)
    if let Ok(time) = chrono::NaiveTime::parse_from_str(input, "%H:%M:%S") {
        return Some(now.date().and_time(time));
    }

    // Time only: "HH:mm" (assumes today)
    if let Ok(time) = chrono::NaiveTime::parse_from_str(input, "%H:%M") {
        return Some(now.date().and_time(time));
    }

    None
}
