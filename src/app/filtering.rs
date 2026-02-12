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
            if let Some(ref regex) = self.search_regex
                && !regex.is_match(entry.searchable_text())
            {
                continue;
            }

            // Date range filter
            if let Some(from) = self.date_from
                && let Some(ts) = entry.timestamp
                && ts < from
            {
                continue;
            }
            if let Some(to) = self.date_to
                && let Some(ts) = entry.timestamp
                && ts > to
            {
                continue;
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
            if let Some(ref regex) = self.search_regex
                && !regex.is_match(entry.searchable_text())
            {
                continue;
            }

            // Date range filter
            if let Some(from) = self.date_from
                && let Some(ts) = entry.timestamp
                && ts < from
            {
                continue;
            }
            if let Some(to) = self.date_to
                && let Some(ts) = entry.timestamp
                && ts > to
            {
                continue;
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

    /// Commit search to the results panel (split-screen mode).
    /// Stores highlight regex, computes match indices, opens panel.
    /// Does NOT filter entries — all entries remain visible.
    pub fn commit_search_to_panel(&mut self, query: &str, regex_mode: bool) {
        if query.is_empty() {
            self.close_search_panel();
            return;
        }

        // Compile highlight regex
        let pattern = if regex_mode {
            format!("(?i){}", query)
        } else {
            format!("(?i){}", regex::escape(query))
        };
        let regex = match regex::Regex::new(&pattern) {
            Ok(r) => r,
            Err(_) => return,
        };

        // Clear any existing search filter so all entries are visible
        self.search_regex = None;
        self.search_query.clear();
        self.apply_filters();

        // Scan filtered_indices for matches
        self.search_panel_matches = self
            .filtered_indices
            .iter()
            .enumerate()
            .filter(|&(_, &entry_idx)| regex.is_match(self.entries[entry_idx].searchable_text()))
            .map(|(pos, _)| pos)
            .collect();

        self.highlight_regex = Some(regex);
        self.highlight_query = query.to_string();
        self.highlight_regex_mode = regex_mode;
        self.search_panel_open = true;
        self.search_panel_focused = false;
        self.search_panel_selected = 0;
        self.search_panel_scroll = 0;

        // Jump to first match
        if let Some(&first) = self.search_panel_matches.first() {
            self.selected_index = first;
            self.center_selected();
        }
    }

    fn clamp_scroll(&mut self) {
        if self.filtered_indices.is_empty() {
            self.scroll_offset = 0;
        }
    }
}
