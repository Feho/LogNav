use crate::app::{App, DateFilterFocus, FocusState, QUICK_FILTERS};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Handle keys in date filter dialog
pub fn handle_date_filter_key(app: &mut App, key: KeyEvent) {
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

/// Get active TextInput for the current date field focus
fn with_active_input(
    app: &mut App,
    f: impl FnOnce(&mut crate::text_input::TextInput, &mut Option<String>),
) {
    if let FocusState::DateFilter {
        from,
        to,
        focus,
        error,
        ..
    } = &mut app.focus
    {
        match focus {
            DateFilterFocus::From => f(from, error),
            DateFilterFocus::To => f(to, error),
            _ => {}
        }
    }
}

/// Handle keys when custom date fields are focused
fn handle_custom_date_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Enter => {
            let (from, to) = match &app.focus {
                FocusState::DateFilter { from, to, .. } => {
                    (from.text().to_string(), to.text().to_string())
                }
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
            with_active_input(app, |input, error| {
                input.clear();
                *error = None;
            });
        }

        KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            with_active_input(app, |input, error| {
                input.delete_word_back();
                *error = None;
            });
        }

        KeyCode::Left => {
            with_active_input(app, |input, _| input.move_left());
        }

        KeyCode::Right => {
            with_active_input(app, |input, _| input.move_right());
        }

        KeyCode::Home => {
            with_active_input(app, |input, _| input.home());
        }

        KeyCode::End => {
            with_active_input(app, |input, _| input.end());
        }

        KeyCode::Delete => {
            with_active_input(app, |input, error| {
                input.delete_forward();
                *error = None;
            });
        }

        KeyCode::Char(c) => {
            with_active_input(app, |input, error| {
                input.insert_char(c);
                *error = None;
            });
        }

        KeyCode::Backspace => {
            with_active_input(app, |input, error| {
                input.delete_back();
                *error = None;
            });
        }

        _ => {}
    }
}

/// Parse date input string into NaiveDateTime
///
/// When `is_end` is true, date-only inputs resolve to 23:59:59 (end of day).
/// When false, they resolve to 00:00:00 (start of day).
pub fn parse_date_input(input: &str, is_end: bool) -> Option<chrono::NaiveDateTime> {
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
        ("%Y-%m-%d %H:%M:%S", format!("{}-{}", current_year, input)),
        ("%Y-%m-%d %H:%M", format!("{}-{}", current_year, input)),
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
