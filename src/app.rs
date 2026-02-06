use crate::log_entry::{LogEntry, LogLevel};
use chrono::NaiveDateTime;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use regex::Regex;
use std::collections::HashSet;

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
    },
    DateFilter {
        from: String,
        to: String,
        focused_field: DateFilterField,
    },
    FileOpen {
        path: String,
        selected_recent: usize,
        cursor_pos: usize,
        error: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateFilterField {
    From,
    To,
}

#[derive(Debug, Clone)]
pub struct Command {
    pub name: &'static str,
    pub shortcut: &'static str,
    pub action: CommandAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandAction {
    OpenFile,
    Search,
    DateFilter,
    ToggleError,
    ToggleWarn,
    ToggleInfo,
    ToggleDebug,
    ToggleTrace,
    ToggleProfile,
    ToggleTail,
    ToggleWrap,
    GoToTop,
    GoToBottom,
    Quit,
}

impl Command {
    pub const ALL: &'static [Command] = &[
        Command {
            name: "Open file...",
            shortcut: "Ctrl+O",
            action: CommandAction::OpenFile,
        },
        Command {
            name: "Search logs...",
            shortcut: "/",
            action: CommandAction::Search,
        },
        Command {
            name: "Filter by date range...",
            shortcut: "Ctrl+D",
            action: CommandAction::DateFilter,
        },
        Command {
            name: "Toggle Error",
            shortcut: "1",
            action: CommandAction::ToggleError,
        },
        Command {
            name: "Toggle Warning",
            shortcut: "2",
            action: CommandAction::ToggleWarn,
        },
        Command {
            name: "Toggle Info",
            shortcut: "3",
            action: CommandAction::ToggleInfo,
        },
        Command {
            name: "Toggle Debug",
            shortcut: "4",
            action: CommandAction::ToggleDebug,
        },
        Command {
            name: "Toggle Trace",
            shortcut: "5",
            action: CommandAction::ToggleTrace,
        },
        Command {
            name: "Toggle Profile",
            shortcut: "6",
            action: CommandAction::ToggleProfile,
        },
        Command {
            name: "Toggle tail mode",
            shortcut: "t",
            action: CommandAction::ToggleTail,
        },
        Command {
            name: "Toggle word wrap",
            shortcut: "w",
            action: CommandAction::ToggleWrap,
        },
        Command {
            name: "Go to top",
            shortcut: "g",
            action: CommandAction::GoToTop,
        },
        Command {
            name: "Go to bottom",
            shortcut: "G",
            action: CommandAction::GoToBottom,
        },
        Command {
            name: "Quit",
            shortcut: "q/Esc",
            action: CommandAction::Quit,
        },
    ];
}

pub struct App {
    // Log data
    pub entries: Vec<LogEntry>,
    pub filtered_indices: Vec<usize>,

    // Filter state
    pub level_filters: [bool; 6], // ERR, WRN, INF, DBG, TRC, PRF
    pub search_regex: Option<Regex>,
    pub search_query: String,
    pub date_from: Option<NaiveDateTime>,
    pub date_to: Option<NaiveDateTime>,

    // UI state
    pub scroll_offset: usize,
    pub selected_index: usize,
    pub focus: FocusState,
    pub tail_enabled: bool,
    pub wrap_enabled: bool,
    pub horizontal_scroll: usize,
    pub expanded_entries: HashSet<usize>, // Entry indices that are expanded

    // File state
    pub file_path: String,
    pub recent_files: Vec<String>,

    // Status
    pub status_message: Option<String>,
    pub should_quit: bool,

    // Fuzzy matcher for command palette
    fuzzy_matcher: SkimMatcherV2,
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
            search_regex: None,
            search_query: String::new(),
            date_from: None,
            date_to: None,
            scroll_offset: 0,
            selected_index: 0,
            focus: FocusState::Normal,
            tail_enabled: true,
            wrap_enabled: false,
            horizontal_scroll: 0,
            expanded_entries: HashSet::new(),
            file_path: String::new(),
            recent_files: Vec::new(),
            status_message: None,
            should_quit: false,
            fuzzy_matcher: SkimMatcherV2::default(),
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

        if self.tail_enabled {
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

    /// Apply all filters and update filtered_indices
    pub fn apply_filters(&mut self) {
        self.filtered_indices.clear();

        for (idx, entry) in self.entries.iter().enumerate() {
            // Level filter
            if !self.passes_level_filter(entry.level) {
                continue;
            }

            // Search filter - use searchable_text for fast path
            if let Some(ref regex) = self.search_regex {
                if !regex.is_match(entry.searchable_text()) {
                    continue;
                }
            }

            // Date range filter
            if let Some(from) = self.date_from {
                if let Some(ts) = entry.timestamp {
                    if ts < from {
                        continue;
                    }
                }
            }
            if let Some(to) = self.date_to {
                if let Some(ts) = entry.timestamp {
                    if ts > to {
                        continue;
                    }
                }
            }

            self.filtered_indices.push(idx);
        }

        // Clamp selection to valid range
        if !self.filtered_indices.is_empty() {
            if self.selected_index >= self.filtered_indices.len() {
                self.selected_index = self.filtered_indices.len() - 1;
            }
        } else {
            self.selected_index = 0;
        }

        // Clamp scroll offset
        self.clamp_scroll();
    }

    /// Apply filters only to newly appended entries (starting from start_idx)
    fn apply_filters_incremental(&mut self, start_idx: usize) {
        for idx in start_idx..self.entries.len() {
            let entry = &self.entries[idx];

            // Level filter
            if !self.passes_level_filter(entry.level) {
                continue;
            }

            // Search filter - use searchable_text for fast path
            if let Some(ref regex) = self.search_regex {
                if !regex.is_match(entry.searchable_text()) {
                    continue;
                }
            }

            // Date range filter
            if let Some(from) = self.date_from {
                if let Some(ts) = entry.timestamp {
                    if ts < from {
                        continue;
                    }
                }
            }
            if let Some(to) = self.date_to {
                if let Some(ts) = entry.timestamp {
                    if ts > to {
                        continue;
                    }
                }
            }

            self.filtered_indices.push(idx);
        }
    }

    fn passes_level_filter(&self, level: LogLevel) -> bool {
        match level {
            LogLevel::Error => self.level_filters[0],
            LogLevel::Warn => self.level_filters[1],
            LogLevel::Info => self.level_filters[2],
            LogLevel::Debug => self.level_filters[3],
            LogLevel::Trace => self.level_filters[4],
            LogLevel::Profile => self.level_filters[5],
            LogLevel::Unknown => true, // Always show unknown
        }
    }

    /// Toggle a level filter by index (0-5)
    pub fn toggle_level(&mut self, level_idx: usize) {
        if level_idx < 6 {
            self.level_filters[level_idx] = !self.level_filters[level_idx];
            self.apply_filters();
        }
    }

    /// Set search query and compile regex
    pub fn set_search(&mut self, query: &str) {
        self.search_query = query.to_string();
        if query.is_empty() {
            self.search_regex = None;
        } else {
            // Case-insensitive search
            self.search_regex = Regex::new(&format!("(?i){}", regex::escape(query))).ok();
        }
        self.apply_filters();
    }

    /// Update search match indices for navigation
    /// When search is active, all filtered entries are matches
    pub fn update_search_matches(&mut self) {
        if let FocusState::Search {
            ref mut match_indices,
            ref mut current_match,
            ..
        } = self.focus
        {
            match_indices.clear();
            // All filtered entries are matches when search is applied
            if self.search_regex.is_some() {
                match_indices.extend(0..self.filtered_indices.len());
            }
            if !match_indices.is_empty() && *current_match >= match_indices.len() {
                *current_match = 0;
            }
        }
    }

    /// Jump to next search match
    pub fn next_search_match(&mut self) {
        if let FocusState::Search {
            ref match_indices,
            ref mut current_match,
            ..
        } = self.focus
        {
            if !match_indices.is_empty() {
                *current_match = (*current_match + 1) % match_indices.len();
                let target = match_indices[*current_match];
                self.selected_index = target;
                self.ensure_selected_visible();
            }
        }
    }

    /// Jump to previous search match
    pub fn prev_search_match(&mut self) {
        if let FocusState::Search {
            ref match_indices,
            ref mut current_match,
            ..
        } = self.focus
        {
            if !match_indices.is_empty() {
                *current_match = if *current_match == 0 {
                    match_indices.len() - 1
                } else {
                    *current_match - 1
                };
                let target = match_indices[*current_match];
                self.selected_index = target;
                self.ensure_selected_visible();
            }
        }
    }

    /// Get filtered commands based on fuzzy search
    pub fn get_filtered_commands(&self, query: &str) -> Vec<(usize, &Command, i64)> {
        if query.is_empty() {
            return Command::ALL
                .iter()
                .enumerate()
                .map(|(i, c)| (i, c, 0))
                .collect();
        }

        let mut results: Vec<_> = Command::ALL
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

    // Navigation
    pub fn scroll_up(&mut self, amount: usize) {
        self.selected_index = self.selected_index.saturating_sub(amount);
        self.ensure_selected_visible();
    }

    pub fn scroll_down(&mut self, amount: usize) {
        if !self.filtered_indices.is_empty() {
            self.selected_index =
                (self.selected_index + amount).min(self.filtered_indices.len() - 1);
        }
        self.ensure_selected_visible();
    }

    pub fn scroll_to_top(&mut self) {
        self.selected_index = 0;
        self.scroll_offset = 0;
    }

    pub fn scroll_to_bottom(&mut self) {
        if !self.filtered_indices.is_empty() {
            self.selected_index = self.filtered_indices.len() - 1;
        }
        self.ensure_selected_visible();
    }

    pub fn scroll_left(&mut self, amount: usize) {
        self.horizontal_scroll = self.horizontal_scroll.saturating_sub(amount);
    }

    pub fn scroll_right(&mut self, amount: usize) {
        self.horizontal_scroll += amount;
    }

    pub fn ensure_selected_visible(&mut self) {
        // This will be called with viewport height from UI
        // For now, just ensure scroll_offset is reasonable
        if self.selected_index < self.scroll_offset {
            self.scroll_offset = self.selected_index;
        }
    }

    pub fn ensure_selected_visible_with_height(&mut self, viewport_height: usize) {
        if viewport_height == 0 {
            return;
        }

        // Selected is above viewport - scroll up
        if self.selected_index < self.scroll_offset {
            self.scroll_offset = self.selected_index;
            return;
        }

        // Count visual lines from scroll_offset through selected_index (inclusive)
        let mut visual_lines = 0;
        for idx in self.scroll_offset..=self.selected_index {
            visual_lines += self.visual_lines_for_entry(idx);
        }

        // If selected entry extends beyond viewport, increase scroll_offset
        while visual_lines > viewport_height && self.scroll_offset < self.selected_index {
            visual_lines -= self.visual_lines_for_entry(self.scroll_offset);
            self.scroll_offset += 1;
        }
    }

    /// Calculate how many visual lines an entry occupies (1 if collapsed, 1+continuations if expanded)
    pub fn visual_lines_for_entry(&self, filtered_idx: usize) -> usize {
        if let Some(&entry_idx) = self.filtered_indices.get(filtered_idx) {
            if self.expanded_entries.contains(&entry_idx) {
                return 1 + self.entries[entry_idx].continuation_lines.len();
            }
        }
        1
    }

    fn clamp_scroll(&mut self) {
        if self.filtered_indices.is_empty() {
            self.scroll_offset = 0;
        }
    }

    /// Get the currently selected entry
    pub fn selected_entry(&self) -> Option<&LogEntry> {
        self.filtered_indices
            .get(self.selected_index)
            .and_then(|&idx| self.entries.get(idx))
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
        // Compute match_indices based on current search regex applied to ALL entries
        let match_indices = if let Some(ref regex) = self.search_regex {
            self.entries
                .iter()
                .enumerate()
                .filter(|(_, entry)| regex.is_match(entry.searchable_text()))
                .map(|(idx, _)| idx)
                .collect()
        } else {
            Vec::new()
        };

        self.focus = FocusState::Search {
            query: self.search_query.clone(),
            match_indices,
            current_match: 0,
        };
    }

    /// Open date filter dialog
    pub fn open_date_filter(&mut self) {
        self.focus = FocusState::DateFilter {
            from: String::new(),
            to: String::new(),
            focused_field: DateFilterField::From,
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

    /// Close any overlay and return to normal
    pub fn close_overlay(&mut self) {
        self.focus = FocusState::Normal;
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

    /// Check if an entry is expanded
    pub fn is_expanded(&self, entry_idx: usize) -> bool {
        self.expanded_entries.contains(&entry_idx)
    }

    /// Execute a command action
    pub fn execute_command(&mut self, action: CommandAction) {
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
            CommandAction::GoToTop => self.scroll_to_top(),
            CommandAction::GoToBottom => self.scroll_to_bottom(),
            CommandAction::Quit => self.should_quit = true,
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
}
