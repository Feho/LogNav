use crate::app::{App, FocusState, HoverWord};
use crate::text_utils::wrap_text;
use crate::ui::extract_message;
use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

/// Handle mouse events
pub fn handle_mouse(app: &mut App, mouse: MouseEvent) {
    match mouse.kind {
        MouseEventKind::ScrollUp => {
            app.hover_word = None;
            match &mut app.focus {
                FocusState::Help { scroll_offset } => {
                    *scroll_offset = scroll_offset.saturating_sub(3);
                    return;
                }
                FocusState::Detail { scroll_offset } => {
                    *scroll_offset = scroll_offset.saturating_sub(3);
                    return;
                }
                FocusState::CommandPalette { selected, .. } => {
                    *selected = selected.saturating_sub(3);
                    return;
                }
                FocusState::FileOpen {
                    selected_recent, ..
                } => {
                    *selected_recent = selected_recent.saturating_sub(3);
                    return;
                }
                FocusState::ExcludeManager {
                    selected, focus, ..
                } => {
                    if *focus == crate::app::ExcludeManagerFocus::List {
                        *selected = selected.saturating_sub(3);
                    }
                    return;
                }
                FocusState::Clusters { selected, .. } => {
                    *selected = selected.saturating_sub(3);
                    return;
                }
                _ => {}
            }
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
            app.hover_word = None;
            // Compute max index for lists before mutable borrow
            let cmd_count = if let FocusState::CommandPalette { input, .. } = &app.focus {
                app.get_filtered_commands(input.text()).len()
            } else {
                0
            };
            let recent_count = app.recent_files.len();
            let exclude_count = app.exclude_patterns.len();
            let cluster_count = app.clusters.len();
            match &mut app.focus {
                FocusState::Help { scroll_offset } => {
                    *scroll_offset += 3;
                    return;
                }
                FocusState::Detail { scroll_offset } => {
                    *scroll_offset += 3;
                    return;
                }
                FocusState::CommandPalette { selected, .. } => {
                    if cmd_count > 0 {
                        *selected = (*selected + 3).min(cmd_count - 1);
                    }
                    return;
                }
                FocusState::FileOpen {
                    selected_recent, ..
                } => {
                    if recent_count > 0 {
                        *selected_recent = (*selected_recent + 3).min(recent_count - 1);
                    }
                    return;
                }
                FocusState::ExcludeManager {
                    selected, focus, ..
                } => {
                    if *focus == crate::app::ExcludeManagerFocus::List && exclude_count > 0 {
                        *selected = (*selected + 3).min(exclude_count - 1);
                    }
                    return;
                }
                FocusState::Clusters { selected, .. } => {
                    if cluster_count > 0 {
                        *selected = (*selected + 3).min(cluster_count - 1);
                    }
                    return;
                }
                _ => {}
            }
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
            app.hover_word = None;

            if !matches!(app.focus, FocusState::Normal) {
                return;
            }

            let clicked_row = mouse.row as usize;
            let clicked_col = mouse.column as usize;
            let ctrl_held = mouse.modifiers.contains(KeyModifiers::CONTROL);
            let alt_held = mouse.modifiers.contains(KeyModifiers::ALT);

            // Check if click is in the search panel area
            if app.search_panel_open && app.search_panel_height > 0 {
                let terminal_height = app.viewport_height + app.search_panel_height + 1;
                let panel_start = terminal_height.saturating_sub(app.search_panel_height + 1);

                if clicked_row >= panel_start && clicked_row < panel_start + app.search_panel_height
                {
                    app.search_panel_focused = true;
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

            // Find which entry was clicked
            let mut visual_row = 0;
            let mut entry_idx = app.scroll_offset;
            while entry_idx < app.filtered_indices.len() {
                let lines = app.visual_lines_for_entry(entry_idx, app.viewport_width);
                if visual_row + lines > clicked_row {
                    app.selected_index = entry_idx;
                    break;
                }
                visual_row += lines;
                entry_idx += 1;
            }

            // Ctrl+Click: extract word under cursor and search
            if ctrl_held
                && let Some((word, _, _)) =
                    word_at_click(app, entry_idx, clicked_row, visual_row, clicked_col)
                && !word.is_empty()
            {
                app.search_history.retain(|h| h != &word);
                app.search_history.push(word.clone());
                app.commit_search_to_panel(&word, false);
            }

            // Alt+Click: extract word under cursor and add as exclude filter
            if alt_held
                && let Some((word, _, _)) =
                    word_at_click(app, entry_idx, clicked_row, visual_row, clicked_col)
                && !word.is_empty()
            {
                if let Some(err) = app.add_exclude(&word, false) {
                    app.status_message = Some(format!("Invalid exclude: {}", err));
                } else {
                    app.status_message = Some(format!("Exclude filter added: '{}'", word));
                }
            }
        }

        MouseEventKind::Moved => {
            let ctrl_held = mouse.modifiers.contains(KeyModifiers::CONTROL);
            let alt_held = mouse.modifiers.contains(KeyModifiers::ALT);
            if (ctrl_held || alt_held) && matches!(app.focus, FocusState::Normal) {
                let row = mouse.row as usize;
                let col = mouse.column as usize;

                // Find which entry the cursor is over
                let mut visual_row = 0;
                let mut entry_idx = app.scroll_offset;
                while entry_idx < app.filtered_indices.len() {
                    let lines = app.visual_lines_for_entry(entry_idx, app.viewport_width);
                    if visual_row + lines > row {
                        break;
                    }
                    visual_row += lines;
                    entry_idx += 1;
                }

                if let Some((_, char_start, char_end)) =
                    word_at_click(app, entry_idx, row, visual_row, col)
                {
                    app.hover_word = Some(HoverWord {
                        row,
                        char_start,
                        char_end,
                    });
                    return;
                }
                app.hover_word = None;
            } else {
                app.hover_word = None;
            }
        }

        _ => {
            app.hover_word = None;
        }
    }
}

/// Extract the word under the cursor for a given click position.
/// Returns (word, char_start, char_end) where start/end are char offsets in the display text.
/// Accounts for gutter widths and word-wrapping.
fn word_at_click(
    app: &App,
    entry_idx: usize,
    clicked_row: usize,
    entry_visual_start: usize,
    clicked_col: usize,
) -> Option<(String, usize, usize)> {
    if entry_idx >= app.filtered_indices.len() {
        return None;
    }
    let real_idx = app.filtered_indices[entry_idx];
    let entry = &app.entries[real_idx];
    let prefix_width = app.full_prefix_width();
    let row_within_entry = clicked_row.saturating_sub(entry_visual_start);

    if app.wrap_enabled && app.viewport_width > 0 {
        let msg_width = app.viewport_width.saturating_sub(prefix_width);
        if msg_width == 0 {
            return None;
        }

        // Wrap the main message into visual segments
        let message = extract_message(&entry.raw_line);
        let wrapped = wrap_text(&message, msg_width);
        let main_visual_rows = wrapped.len();

        if row_within_entry < main_visual_rows {
            // Click is on a wrapped segment of the main message
            let segment = &wrapped[row_within_entry];
            if clicked_col < prefix_width {
                return None;
            }
            let col_in_segment = clicked_col - prefix_width;
            // Map back to char offset in the full message
            let chars_before: usize = wrapped[..row_within_entry]
                .iter()
                .map(|s| s.chars().count())
                .sum();
            let result = extract_word_at(segment, col_in_segment)?;
            return Some((result.0, result.1 + chars_before, result.2 + chars_before));
        }

        // Click is on an expanded continuation line (also wrapped)
        if app.expanded_entries.contains(&real_idx) {
            let mut vis_row = main_visual_rows;
            for cont_line in entry.display_continuation() {
                let wrapped_cont = wrap_text(cont_line, msg_width);
                let cont_rows = wrapped_cont.len();
                if row_within_entry < vis_row + cont_rows {
                    let seg_idx = row_within_entry - vis_row;
                    let segment = &wrapped_cont[seg_idx];
                    if clicked_col < prefix_width {
                        return None;
                    }
                    let col_in_segment = clicked_col - prefix_width;
                    return extract_word_at(segment, col_in_segment);
                }
                vis_row += cont_rows;
            }
        }
        None
    } else {
        // No-wrap mode: row 0 is main line, rest are continuation lines
        if row_within_entry == 0 {
            let msg = extract_message(&entry.raw_line);
            if clicked_col < prefix_width {
                return None;
            }
            let char_offset = (clicked_col - prefix_width) + app.horizontal_scroll;
            return extract_word_at(&msg, char_offset);
        }

        // Continuation lines (expanded)
        if app.expanded_entries.contains(&real_idx) {
            let cont_idx = row_within_entry - 1;
            let display = entry.display_continuation();
            if cont_idx < display.len() {
                if clicked_col < prefix_width {
                    return None;
                }
                let char_offset = (clicked_col - prefix_width) + app.horizontal_scroll;
                return extract_word_at(&display[cont_idx], char_offset);
            }
        }
        None
    }
}

/// Extract a word at the given character offset within a string.
/// Returns (word, char_start, char_end) in char-offset coordinates.
///
/// Priority: quoted strings > hyphenated words > punctuation runs
fn extract_word_at(text: &str, char_offset: usize) -> Option<(String, usize, usize)> {
    let chars: Vec<char> = text.chars().collect();
    if char_offset >= chars.len() {
        return None;
    }

    let target = chars[char_offset];
    if target.is_whitespace() {
        return None;
    }

    // 1. Check if cursor is inside a single-quoted string
    if let Some(result) = find_quoted_string(&chars, char_offset) {
        return Some(result);
    }

    // 2. Hyphenated word: alphanumeric, underscore, hyphen
    let is_word_char = |c: char| c.is_alphanumeric() || c == '_' || c == '-';
    let target_is_word = is_word_char(target);

    let mut start = char_offset;
    while start > 0
        && (if target_is_word {
            is_word_char(chars[start - 1])
        } else {
            !is_word_char(chars[start - 1]) && !chars[start - 1].is_whitespace()
        })
    {
        start -= 1;
    }

    let mut end = char_offset;
    while end < chars.len()
        && (if target_is_word {
            is_word_char(chars[end])
        } else {
            !is_word_char(chars[end]) && !chars[end].is_whitespace()
        })
    {
        end += 1;
    }

    let word: String = chars[start..end].iter().collect();
    if word.is_empty() {
        None
    } else {
        Some((word, start, end))
    }
}

/// Check if char_offset falls inside a `'...'` quoted string.
/// Returns the inner content (without quotes) and the range of the inner content.
fn find_quoted_string(chars: &[char], char_offset: usize) -> Option<(String, usize, usize)> {
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '\'' {
            // Find closing quote
            let open = i;
            let mut close = None;
            let mut j = i + 1;
            while j < chars.len() {
                if chars[j] == '\'' {
                    close = Some(j);
                    break;
                }
                j += 1;
            }
            if let Some(close_idx) = close {
                // Check if char_offset is within this quoted region (inclusive of quotes)
                if char_offset >= open && char_offset <= close_idx {
                    let inner_start = open + 1;
                    let inner_end = close_idx;
                    if inner_start < inner_end {
                        let word: String = chars[inner_start..inner_end].iter().collect();
                        return Some((word, inner_start, inner_end));
                    }
                }
                i = close_idx + 1;
            } else {
                break;
            }
        } else {
            i += 1;
        }
    }
    None
}
