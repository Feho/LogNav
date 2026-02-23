use crate::app::{App, FocusState};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Handle keys in file open dialog
pub fn handle_file_open_key(app: &mut App, key: KeyEvent) {
    // Clear tab completions on any key that isn't Tab/BackTab
    if !matches!(key.code, KeyCode::Tab | KeyCode::BackTab)
        && let FocusState::FileOpen {
            completions,
            completion_index,
            ..
        } = &mut app.focus
        && !completions.is_empty()
    {
        completions.clear();
        *completion_index = None;
    }

    match key.code {
        KeyCode::Esc => {
            app.close_overlay();
        }

        KeyCode::Enter => {
            let (file_path, is_merge) = match &app.focus {
                FocusState::FileOpen {
                    input,
                    selected_recent,
                    is_merge,
                    ..
                } => {
                    let path = input.text();
                    let resolved = if path.is_empty() && !app.recent_files.is_empty() {
                        app.recent_files.get(*selected_recent).cloned()
                    } else {
                        // Tilde expansion
                        let expanded = if path == "~" {
                            std::env::var("HOME").unwrap_or_else(|_| path.to_string())
                        } else if let Some(rest) = path.strip_prefix("~/") {
                            match std::env::var("HOME") {
                                Ok(home) => format!("{}/{}", home, rest),
                                Err(_) => path.to_string(),
                            }
                        } else {
                            path.to_string()
                        };
                        Some(expanded)
                    };
                    (resolved, *is_merge)
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
                if is_merge {
                    app.pending_merge_path = Some(path);
                } else {
                    app.file_path = path;
                }
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

        KeyCode::PageUp => {
            if let FocusState::FileOpen {
                selected_recent, ..
            } = &mut app.focus
            {
                *selected_recent = 0;
            }
        }

        KeyCode::PageDown => {
            let len = app.recent_files.len();
            if let FocusState::FileOpen {
                selected_recent, ..
            } = &mut app.focus
                && len > 0
            {
                *selected_recent = len - 1;
            }
        }

        KeyCode::Left => {
            if let FocusState::FileOpen { input, .. } = &mut app.focus {
                input.move_left();
            }
        }

        KeyCode::Right => {
            if let FocusState::FileOpen { input, .. } = &mut app.focus {
                input.move_right();
            }
        }

        KeyCode::Home => {
            if let FocusState::FileOpen { input, .. } = &mut app.focus {
                input.home();
            }
        }

        KeyCode::End => {
            if let FocusState::FileOpen { input, .. } = &mut app.focus {
                input.end();
            }
        }

        KeyCode::Delete => {
            if let FocusState::FileOpen { input, error, .. } = &mut app.focus {
                input.delete_forward();
                *error = None;
            }
        }

        KeyCode::Tab | KeyCode::BackTab => {
            let input_text = match &app.focus {
                FocusState::FileOpen { input, .. } => input.text().to_string(),
                _ => return,
            };

            // Empty input: Tab fills from recent files
            if input_text.is_empty() {
                if key.code == KeyCode::Tab {
                    let recent_path = match &app.focus {
                        FocusState::FileOpen {
                            selected_recent, ..
                        } => app.recent_files.get(*selected_recent).cloned(),
                        _ => return,
                    };
                    if let Some(recent) = recent_path
                        && let FocusState::FileOpen { input, error, .. } = &mut app.focus
                    {
                        input.set_text(recent);
                        *error = None;
                    }
                }
                return;
            }

            // Filesystem tab completion
            let has_completions = match &app.focus {
                FocusState::FileOpen { completions, .. } => !completions.is_empty(),
                _ => false,
            };

            if has_completions {
                // Cycle through stored completions
                if let FocusState::FileOpen {
                    completions,
                    completion_index,
                    input,
                    error,
                    ..
                } = &mut app.focus
                {
                    let len = completions.len();
                    let next = match (*completion_index, key.code) {
                        (Some(i), KeyCode::BackTab) => (i + len - 1) % len,
                        (Some(i), _) => (i + 1) % len,
                        (None, KeyCode::BackTab) => len - 1,
                        (None, _) => 0,
                    };
                    *completion_index = Some(next);
                    input.set_text(completions[next].clone());
                    *error = None;
                }
            } else if key.code == KeyCode::Tab {
                // Compute new completions
                let matches = compute_path_completions(&input_text);
                if matches.is_empty() {
                    return;
                }
                if let FocusState::FileOpen {
                    completions,
                    completion_index,
                    input,
                    error,
                    ..
                } = &mut app.focus
                {
                    if matches.len() == 1 {
                        input.set_text(matches[0].clone());
                        *error = None;
                    } else {
                        let lcp = longest_common_prefix(&matches);
                        if lcp.len() > input_text.len() {
                            // Complete to common prefix, store for cycling
                            input.set_text(lcp);
                        } else {
                            // Already at common prefix, start cycling
                            input.set_text(matches[0].clone());
                            *completion_index = Some(0);
                        }
                        *completions = matches;
                        *error = None;
                    }
                }
            }
        }

        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if let FocusState::FileOpen { input, error, .. } = &mut app.focus {
                input.clear();
                *error = None;
            }
        }

        KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if let FocusState::FileOpen { input, error, .. } = &mut app.focus {
                input.delete_path_segment_back();
                *error = None;
            }
        }

        KeyCode::Char(c) => {
            if let FocusState::FileOpen { input, error, .. } = &mut app.focus {
                input.insert_char(c);
                *error = None;
            }
        }

        KeyCode::Backspace => {
            if let FocusState::FileOpen { input, error, .. } = &mut app.focus {
                input.delete_back();
                *error = None;
            }
        }

        _ => {}
    }
}

/// Compute filesystem path completions for tab completion
fn compute_path_completions(text: &str) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }

    // Tilde expansion
    let (expanded, tilde_home) = if text == "~" {
        match std::env::var("HOME") {
            Ok(home) => (home.clone(), Some(home)),
            Err(_) => (text.to_string(), None),
        }
    } else if let Some(rest) = text.strip_prefix("~/") {
        match std::env::var("HOME") {
            Ok(home) => (format!("{}/{}", home, rest), Some(home)),
            Err(_) => (text.to_string(), None),
        }
    } else {
        (text.to_string(), None)
    };

    let path = std::path::Path::new(&expanded);

    // Split into directory to list and prefix to filter by
    let (dir, prefix) = if expanded.ends_with('/') {
        (expanded.as_str(), "")
    } else {
        match path.file_name().and_then(|f| f.to_str()) {
            Some(fname) => {
                let parent = path.parent().and_then(|p| p.to_str()).unwrap_or(".");
                let parent = if parent.is_empty() { "." } else { parent };
                (parent, fname)
            }
            None => return Vec::new(),
        }
    };

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    // Don't prepend "./" for bare filenames (no path separator in input)
    let no_dir_prefix = dir == "." && !expanded.contains('/');

    let mut results: Vec<String> = Vec::new();
    for entry in entries.flatten() {
        let name = match entry.file_name().into_string() {
            Ok(n) => n,
            Err(_) => continue,
        };

        // Skip hidden files unless prefix starts with '.'
        if name.starts_with('.') && !prefix.starts_with('.') {
            continue;
        }

        if !prefix.is_empty() && !name.starts_with(prefix) {
            continue;
        }

        let is_dir = entry.path().is_dir();
        let mut full = if no_dir_prefix {
            name
        } else if dir.ends_with('/') {
            format!("{}{}", dir, name)
        } else {
            format!("{}/{}", dir, name)
        };

        if is_dir {
            full.push('/');
        }

        // Re-apply tilde prefix
        if let Some(ref home) = tilde_home
            && let Some(rest) = full.strip_prefix(home.as_str())
        {
            full = format!("~{}", rest);
        }

        results.push(full);
    }

    results.sort();
    results
}

/// Find the longest common prefix among a set of strings
fn longest_common_prefix(strings: &[String]) -> String {
    if strings.is_empty() {
        return String::new();
    }
    let first = &strings[0];
    let mut end = first.len();
    for s in &strings[1..] {
        let common = first
            .bytes()
            .zip(s.bytes())
            .take_while(|(a, b)| a == b)
            .count();
        end = end.min(common);
    }
    // Ensure we don't split a multi-byte character
    while end > 0 && !first.is_char_boundary(end) {
        end -= 1;
    }
    first[..end].to_string()
}
