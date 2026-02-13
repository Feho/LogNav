use crate::log_entry::LogEntry;
use chrono::NaiveDateTime;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use regex::Regex;
use std::collections::HashSet;
use std::time::Instant;

/// Unified search state (replaces separate search_regex/highlight_regex systems)
#[derive(Debug, Clone, Default)]
pub struct SearchState {
    pub query: String,
    pub regex_mode: bool,
    pub regex: Option<Regex>,
}

impl SearchState {
    /// Compile query into regex
    pub fn compile(&mut self) {
        if self.query.is_empty() {
            self.regex = None;
            return;
        }
        let pattern = if self.regex_mode {
            format!("(?i){}", self.query)
        } else {
            format!("(?i){}", regex::escape(&self.query))
        };
        self.regex = Regex::new(&pattern).ok();
    }

    pub fn clear(&mut self) {
        self.query.clear();
        self.regex = None;
    }
}

pub mod commands;
pub mod filtering;
pub mod navigation;

const MAX_ENTRIES: usize = 500_000;

#[derive(Debug, Clone)]
pub enum FocusState {
    Normal,
    CommandPalette {
        input: String,
        selected: usize,
    },
    Search {
        query: String,
        match_indices: Vec<usize>,
        current_match: usize,
        regex_mode: bool,
        regex_error: Option<String>,
    },
    DateFilter {
        from: String,
        to: String,
        focus: DateFilterFocus,
        selected_quick: usize,
        error: Option<String>,
    },
    FileOpen {
        path: String,
        selected_recent: usize,
        cursor_pos: usize,
        error: Option<String>,
    },
    Detail {
        scroll_offset: usize,
    },
    Help {
        scroll_offset: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateFilterFocus {
    QuickFilter,
    From,
    To,
}

pub const QUICK_FILTERS: &[&str] = &[
    "Last hour",
    "Last 24 hours",
    "Today",
    "Yesterday",
    "Last 7 days",
    "Clear filter",
];

#[derive(Debug, Clone)]
pub struct HoverWord {
    pub row: usize,        // terminal row
    pub char_start: usize, // char offset in display text (after prefix)
    pub char_end: usize,   // exclusive end
}

pub struct App {
    // Log data
    pub entries: Vec<LogEntry>,
    pub filtered_indices: Vec<usize>,

    // Filter state
    pub level_filters: [bool; 6], // ERR, WRN, INF, DBG, TRC, PRF
    pub date_from: Option<NaiveDateTime>,
    pub date_to: Option<NaiveDateTime>,

    // UI state
    pub scroll_offset: usize,
    pub selected_index: usize,
    pub focus: FocusState,
    pub tail_enabled: bool,
    pub wrap_enabled: bool,
    pub syntax_highlight: bool,
    pub horizontal_scroll: usize,
    pub expanded_entries: HashSet<usize>, // Entry indices that are expanded
    pub viewport_height: usize,           // Last known viewport height for mouse scroll

    // File state
    pub file_path: String,
    pub recent_files: Vec<String>,

    // Status
    pub status_message: Option<String>,
    pub should_quit: bool,

    // Fuzzy matcher for command palette
    fuzzy_matcher: SkimMatcherV2,

    // Clipboard for copying (kept alive to prevent drop issues)
    clipboard: Option<arboard::Clipboard>,

    // Debounce: when set, search needs recomputing after this instant
    pub search_dirty: Option<Instant>,

    // Search results panel (split-screen)
    pub search_panel_open: bool,
    pub search_panel_focused: bool,
    pub search_panel_matches: Vec<usize>, // positions within filtered_indices that match
    pub search_panel_selected: usize,
    pub search_panel_scroll: usize,
    pub search_panel_height: usize,
    pub search: SearchState,

    // Search history (most recent last)
    pub search_history: Vec<String>,
    pub search_history_index: Option<usize>, // None = typing new query, Some(i) = browsing history

    /// Ctrl+hover: word to underline
    pub hover_word: Option<HoverWord>,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            filtered_indices: Vec::new(),
            level_filters: [true, true, true, true, false, false], // ERR, WRN, INF, DBG on by default
            date_from: None,
            date_to: None,
            scroll_offset: 0,
            selected_index: 0,
            focus: FocusState::Normal,
            tail_enabled: true,
            wrap_enabled: false,
            syntax_highlight: true,
            horizontal_scroll: 0,
            expanded_entries: HashSet::new(),
            viewport_height: 25, // Default viewport height
            file_path: String::new(),
            recent_files: Vec::new(),
            status_message: None,
            should_quit: false,
            fuzzy_matcher: SkimMatcherV2::default(),
            clipboard: arboard::Clipboard::new().ok(),
            search_dirty: None,
            search_panel_open: false,
            search_panel_focused: false,
            search_panel_matches: Vec::new(),
            search_panel_selected: 0,
            search_panel_scroll: 0,
            search_panel_height: 0,
            search: SearchState::default(),
            search_history: Vec::new(),
            search_history_index: None,
            hover_word: None,
        }
    }

    /// Add entries from initial load
    pub fn set_entries(&mut self, entries: Vec<LogEntry>) {
        self.entries = entries;
        self.apply_entry_cap();
        self.apply_filters();
        if self.tail_enabled {
            self.scroll_to_bottom();
        }
    }

    /// Add new entries from tailing - uses incremental filtering
    pub fn append_entries(&mut self, mut new_entries: Vec<LogEntry>) {
        if new_entries.is_empty() {
            return;
        }

        // Re-index new entries
        let start_idx = self.entries.len();
        for (i, entry) in new_entries.iter_mut().enumerate() {
            entry.index = start_idx + i;
        }

        self.entries.append(&mut new_entries);

        // Check if we need to cap entries
        let needs_cap = self.entries.len() > MAX_ENTRIES;
        if needs_cap {
            self.apply_entry_cap();
            // After capping, indices are invalidated - must refilter
            self.apply_filters();
        } else {
            // Incremental filter - only process new entries
            self.apply_filters_incremental(start_idx);
        }

        if self.tail_enabled && matches!(self.focus, FocusState::Normal) {
            self.scroll_to_bottom();
        }
    }

    /// Apply entry cap, removing oldest entries if needed
    fn apply_entry_cap(&mut self) {
        if self.entries.len() > MAX_ENTRIES {
            let skip = self.entries.len() - MAX_ENTRIES;
            // Drop oldest entries (more efficient than creating new Vec)
            self.entries.drain(..skip);
            // Re-index
            for (i, entry) in self.entries.iter_mut().enumerate() {
                entry.index = i;
            }
        }
    }

    /// Get filtered commands based on fuzzy search
    pub fn get_filtered_commands(&self, query: &str) -> Vec<(usize, &commands::Command, i64)> {
        if query.is_empty() {
            return commands::Command::ALL
                .iter()
                .enumerate()
                .map(|(i, c)| (i, c, 0))
                .collect();
        }

        let mut results: Vec<_> = commands::Command::ALL
            .iter()
            .enumerate()
            .filter_map(|(i, cmd)| {
                self.fuzzy_matcher
                    .fuzzy_match(cmd.name, query)
                    .map(|score| (i, cmd, score))
            })
            .collect();

        results.sort_by(|a, b| b.2.cmp(&a.2));
        results
    }

    /// Open command palette
    pub fn open_command_palette(&mut self) {
        self.focus = FocusState::CommandPalette {
            input: String::new(),
            selected: 0,
        };
    }

    /// Open search overlay
    pub fn open_search(&mut self) {
        let query = self.search.query.clone();
        let regex_mode = self.search.regex_mode;

        let match_indices = if let Some(regex) = self.search.regex.as_ref() {
            self.filtered_indices
                .iter()
                .enumerate()
                .filter(|&(_, &entry_idx)| {
                    regex.is_match(self.entries[entry_idx].searchable_text())
                })
                .map(|(pos, _)| pos)
                .collect()
        } else {
            Vec::new()
        };

        self.search_history_index = None;
        self.focus = FocusState::Search {
            query,
            match_indices,
            current_match: 0,
            regex_mode,
            regex_error: None,
        };
    }

    /// Open date filter dialog
    pub fn open_date_filter(&mut self) {
        self.focus = FocusState::DateFilter {
            from: self
                .date_from
                .map(|d| d.format("%m-%d %H:%M").to_string())
                .unwrap_or_default(),
            to: self
                .date_to
                .map(|d| d.format("%m-%d %H:%M").to_string())
                .unwrap_or_default(),
            focus: DateFilterFocus::QuickFilter,
            selected_quick: 0,
            error: None,
        };
    }

    /// Open file open dialog
    pub fn open_file_dialog(&mut self) {
        self.focus = FocusState::FileOpen {
            path: String::new(),
            selected_recent: 0,
            cursor_pos: 0,
            error: None,
        };
    }

    /// Open detail popup for selected entry
    pub fn open_detail_popup(&mut self) {
        self.focus = FocusState::Detail { scroll_offset: 0 };
    }

    /// Open help dialog
    pub fn open_help(&mut self) {
        self.focus = FocusState::Help { scroll_offset: 0 };
    }

    /// Close any overlay and return to normal
    pub fn close_overlay(&mut self) {
        self.focus = FocusState::Normal;
    }

    /// Close the search results panel and clear highlight regex.
    /// Keeps search.query/regex_mode so n/N can redo last search.
    pub fn close_search_panel(&mut self) {
        self.search_panel_open = false;
        self.search_panel_focused = false;
        self.search_panel_matches.clear();
        self.search_panel_selected = 0;
        self.search_panel_scroll = 0;
        self.search.regex = None;
    }

    /// Toggle tail mode
    pub fn toggle_tail(&mut self) {
        self.tail_enabled = !self.tail_enabled;
        if self.tail_enabled {
            self.scroll_to_bottom();
        }
    }

    /// Toggle word wrap
    pub fn toggle_wrap(&mut self) {
        self.wrap_enabled = !self.wrap_enabled;
    }

    /// Toggle syntax highlighting
    pub fn toggle_syntax_highlight(&mut self) {
        self.syntax_highlight = !self.syntax_highlight;
    }

    /// Toggle expand/collapse of selected entry's continuation lines
    pub fn toggle_expand(&mut self) {
        if let Some(&entry_idx) = self.filtered_indices.get(self.selected_index) {
            // Only toggle if entry has continuation lines
            if !self.entries[entry_idx].continuation_lines.is_empty() {
                if self.expanded_entries.contains(&entry_idx) {
                    self.expanded_entries.remove(&entry_idx);
                } else {
                    self.expanded_entries.insert(entry_idx);
                }
            }
        }
    }

    /// Expand all entries that have continuation lines; collapse all if already fully expanded
    pub fn toggle_expand_all(&mut self) {
        let expandable: Vec<usize> = self
            .filtered_indices
            .iter()
            .copied()
            .filter(|&idx| !self.entries[idx].continuation_lines.is_empty())
            .collect();

        let all_expanded = expandable
            .iter()
            .all(|idx| self.expanded_entries.contains(idx));
        if all_expanded {
            self.expanded_entries.clear();
        } else {
            self.expanded_entries.extend(expandable);
        }
    }

    /// Check if an entry is expanded
    pub fn is_expanded(&self, entry_idx: usize) -> bool {
        self.expanded_entries.contains(&entry_idx)
    }

    /// Execute a command action
    pub fn execute_command(&mut self, action: commands::CommandAction) {
        use commands::CommandAction;
        match action {
            CommandAction::OpenFile => self.open_file_dialog(),
            CommandAction::Search => self.open_search(),
            CommandAction::DateFilter => self.open_date_filter(),
            CommandAction::ToggleError => self.toggle_level(0),
            CommandAction::ToggleWarn => self.toggle_level(1),
            CommandAction::ToggleInfo => self.toggle_level(2),
            CommandAction::ToggleDebug => self.toggle_level(3),
            CommandAction::ToggleTrace => self.toggle_level(4),
            CommandAction::ToggleProfile => self.toggle_level(5),
            CommandAction::ToggleTail => self.toggle_tail(),
            CommandAction::ToggleWrap => self.toggle_wrap(),
            CommandAction::ToggleSyntax => self.toggle_syntax_highlight(),
            CommandAction::GoToTop => self.scroll_to_top(),
            CommandAction::GoToBottom => self.scroll_to_bottom(),
            CommandAction::NextError => self.next_error(),
            CommandAction::PrevError => self.prev_error(),
            CommandAction::NextWarning => self.next_warning(),
            CommandAction::PrevWarning => self.prev_warning(),
            CommandAction::Quit => self.should_quit = true,
        }
    }

    /// Get active date filter display for status bar
    pub fn date_filter_display(&self) -> Option<String> {
        match (self.date_from, self.date_to) {
            (Some(from), Some(to)) => Some(format!(
                "{} -> {}",
                from.format("%m-%d %H:%M"),
                to.format("%m-%d %H:%M")
            )),
            (Some(from), None) => Some(format!("From {}", from.format("%m-%d %H:%M"))),
            (None, Some(to)) => Some(format!("To {}", to.format("%m-%d %H:%M"))),
            (None, None) => None,
        }
    }

    /// Get active level filter names for status bar
    pub fn active_levels_display(&self) -> String {
        let levels = ["ERR", "WRN", "INF", "DBG", "TRC", "PRF"];
        levels
            .iter()
            .enumerate()
            .filter(|(i, _)| self.level_filters[*i])
            .map(|(_, l)| *l)
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Copy the current entry to clipboard (includes continuation lines)
    pub fn copy_current_line(&mut self) {
        // Build full text with continuation lines
        let text_to_copy = self.selected_entry().map(|e| {
            if e.continuation_lines.is_empty() {
                e.raw_line.clone()
            } else {
                let mut text = e.raw_line.clone();
                for line in &e.continuation_lines {
                    text.push('\n');
                    text.push_str(line);
                }
                text
            }
        });

        if let Some(text) = text_to_copy {
            // Try to use existing clipboard or create new one if needed
            let result = if let Some(ref mut clipboard) = self.clipboard {
                clipboard.set_text(&text)
            } else {
                // Try to create clipboard if it doesn't exist
                match arboard::Clipboard::new() {
                    Ok(mut clipboard) => {
                        let result = clipboard.set_text(&text);
                        self.clipboard = Some(clipboard);
                        result
                    }
                    Err(_) => {
                        self.status_message = Some("Clipboard unavailable".to_string());
                        return;
                    }
                }
            };

            if result.is_ok() {
                self.status_message = Some("Copied!".to_string());
            } else {
                self.status_message = Some("Failed to copy".to_string());
            }
        } else {
            self.status_message = Some("No line selected".to_string());
        }
    }
}
