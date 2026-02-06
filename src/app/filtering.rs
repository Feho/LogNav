use super::App;
use crate::log_entry::LogLevel;

impl App {
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
    pub fn apply_filters_incremental(&mut self, start_idx: usize) {
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

    /// Reset level filters to defaults (ERR/WRN/INF/DBG on, TRC/PRF off)
    pub fn reset_level_filters(&mut self) {
        self.level_filters = [true, true, true, true, false, false];
        self.apply_filters();
    }

    /// Toggle a level filter by index (0-5)
    pub fn toggle_level(&mut self, level_idx: usize) {
        if level_idx < 6 {
            self.level_filters[level_idx] = !self.level_filters[level_idx];
            self.apply_filters();
        }
    }

    /// Set search query and compile regex (literal mode)
    pub fn set_search(&mut self, query: &str) {
        self.set_search_with_mode(query, false);
    }

    /// Set search query and compile regex with optional regex mode
    pub fn set_search_with_mode(&mut self, query: &str, regex_mode: bool) {
        self.search_query = query.to_string();
        if query.is_empty() {
            self.search_regex = None;
        } else {
            let pattern = if regex_mode {
                format!("(?i){}", query)
            } else {
                format!("(?i){}", regex::escape(query))
            };
            self.search_regex = regex::Regex::new(&pattern).ok();
        }
        self.apply_filters();
    }

    /// Update search match indices for navigation
    /// When search is active, all filtered entries are matches
    pub fn update_search_matches(&mut self) {
        if let super::FocusState::Search {
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

    fn clamp_scroll(&mut self) {
        if self.filtered_indices.is_empty() {
            self.scroll_offset = 0;
        }
    }
}
