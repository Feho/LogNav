use super::App;
use crate::log_entry::LogLevel;

impl App {
    /// Apply all filters and update filtered_indices
    pub fn apply_filters(&mut self) {
        self.filtered_indices.clear();

        for (idx, entry) in self.entries.iter().enumerate() {
            if self.passes_all_filters(entry) {
                self.filtered_indices.push(idx);
            }
        }

        // Clamp selection to valid range
        if !self.filtered_indices.is_empty() {
            if self.selected_index >= self.filtered_indices.len() {
                self.selected_index = self.filtered_indices.len() - 1;
            }
        } else {
            self.selected_index = 0;
        }

        self.visual_anchor = None;
        self.clusters_dirty = true;
        self.cluster_map.clear();
        self.folded_clusters.clear();
        self.clamp_scroll();
    }

    /// Apply filters only to newly appended entries (starting from start_idx)
    pub fn apply_filters_incremental(&mut self, start_idx: usize) {
        for idx in start_idx..self.entries.len() {
            let entry = &self.entries[idx];
            if self.passes_all_filters(entry) {
                self.filtered_indices.push(idx);
            }
        }
        self.clusters_dirty = true;
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

    fn passes_all_filters(&self, entry: &crate::log_entry::LogEntry) -> bool {
        if !self.passes_level_filter(entry.level) {
            return false;
        }

        if let Some(from) = self.date_from
            && let Some(ts) = entry.timestamp
            && ts < from
        {
            return false;
        }
        if let Some(to) = self.date_to
            && let Some(ts) = entry.timestamp
            && ts > to
        {
            return false;
        }

        if self
            .exclude_patterns
            .iter()
            .any(|ep| ep.regex.is_match(entry.searchable_text()))
        {
            return false;
        }

        if !self.include_patterns.is_empty()
            && !self
                .include_patterns
                .iter()
                .any(|ip| ip.regex.is_match(entry.searchable_text()))
        {
            return false;
        }

        true
    }

    /// Reset all filters (levels, date, search, exclude, include, search panel)
    pub fn reset_all_filters(&mut self) {
        self.level_filters = [true, true, true, true, true, true];
        self.date_from = None;
        self.date_to = None;
        self.exclude_patterns.clear();
        self.include_patterns.clear();
        self.search.clear();
        self.close_search_panel();
    }

    /// Reset level filters to defaults (all levels on)
    pub fn reset_level_filters(&mut self) {
        self.level_filters = [true, true, true, true, true, true];
        self.apply_filters();
    }

    /// Toggle a level filter by index (0-5)
    pub fn toggle_level(&mut self, level_idx: usize) {
        if level_idx < 6 {
            self.level_filters[level_idx] = !self.level_filters[level_idx];
            self.apply_filters();
        }
    }

    /// Commit search to the results panel (split-screen mode).
    /// Stores search state, computes match indices, opens panel.
    pub fn commit_search_to_panel(&mut self, query: &str, regex_mode: bool) {
        if query.is_empty() {
            self.close_search_panel();
            return;
        }

        self.search.query = query.to_string();
        self.search.regex_mode = regex_mode;
        self.search.compile();

        let regex = match self.search.regex.as_ref() {
            Some(r) => r,
            None => return,
        };

        // Scan filtered_indices for matches
        self.search_panel_matches = self
            .filtered_indices
            .iter()
            .enumerate()
            .filter(|&(_, &entry_idx)| regex.is_match(self.entries[entry_idx].searchable_text()))
            .map(|(pos, _)| pos)
            .collect();

        self.search_panel_open = true;
        self.search_panel_focused = false;
        self.search_panel_selected = 0;
        self.search_panel_scroll = 0;

        // Jump to nearest match at or after current position (stay close to where user was)
        if !self.search_panel_matches.is_empty() {
            let nearest = self
                .search_panel_matches
                .iter()
                .position(|&m| m >= self.selected_index)
                .unwrap_or(0);
            self.search_panel_selected = nearest;
            self.selected_index = self.search_panel_matches[nearest];
            self.auto_expand_for_search();
            self.center_selected();
        }
    }

    fn clamp_scroll(&mut self) {
        if self.filtered_indices.is_empty() {
            self.scroll_offset = 0;
        }
    }
}
