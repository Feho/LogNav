use rayon::prelude::*;

use super::App;

impl App {
    /// Compute a u8 bitmask from level_filters[0..6].
    /// Unknown (bit 6) always passes.
    fn level_filters_as_mask(&self) -> u8 {
        let mut mask = 0u8;
        for (i, &enabled) in self.level_filters.iter().enumerate() {
            if enabled {
                mask |= 1 << i;
            }
        }
        // Unknown level (bit 6) always passes
        mask |= 1 << 6;
        mask
    }

    /// Apply all filters and update filtered_indices.
    ///
    /// Uses a tiered approach:
    /// - Fast path (no regex): scan only entry_meta with bitmask + integer compare
    /// - Full path (regex active): metadata pre-check then regex on candidates,
    ///   parallelized with rayon when candidates > 50K
    pub fn apply_filters(&mut self) {
        let level_mask = self.level_filters_as_mask();
        let has_regex = !self.exclude_patterns.is_empty() || !self.include_patterns.is_empty();

        // Convert date bounds to millis for fast integer comparison
        let from_ms = self.date_from.map(|d| d.and_utc().timestamp_millis());
        let to_ms = self.date_to.map(|d| d.and_utc().timestamp_millis());

        // Fast metadata scan — touches only entry_meta, not entries
        let candidates: Vec<usize> = self
            .entry_meta
            .iter()
            .enumerate()
            .filter(|(_, m)| {
                // Level check via bitmask
                if m.level_bit & level_mask == 0 {
                    return false;
                }
                // Date range check (entries without timestamps always pass)
                if let Some(from) = from_ms
                    && m.timestamp_ms != i64::MIN
                    && m.timestamp_ms < from
                {
                    return false;
                }
                if let Some(to) = to_ms
                    && m.timestamp_ms != i64::MIN
                    && m.timestamp_ms > to
                {
                    return false;
                }
                true
            })
            .map(|(i, _)| i)
            .collect();

        if has_regex {
            if candidates.len() > 50_000 {
                // Parallel regex check
                let exclude_refs: Vec<&regex::Regex> =
                    self.exclude_patterns.iter().map(|p| &p.regex).collect();
                let include_refs: Vec<&regex::Regex> =
                    self.include_patterns.iter().map(|p| &p.regex).collect();
                let entries = &self.entries;

                self.filtered_indices = candidates
                    .into_par_iter()
                    .filter(|&idx| {
                        let text = entries[idx].searchable_text();
                        if exclude_refs.iter().any(|r| r.is_match(text)) {
                            return false;
                        }
                        if !include_refs.is_empty()
                            && !include_refs.iter().any(|r| r.is_match(text))
                        {
                            return false;
                        }
                        true
                    })
                    .collect();
            } else {
                // Sequential regex check
                self.filtered_indices = candidates
                    .into_iter()
                    .filter(|&idx| {
                        let text = self.entries[idx].searchable_text();
                        if self.exclude_patterns.iter().any(|p| p.regex.is_match(text)) {
                            return false;
                        }
                        if !self.include_patterns.is_empty()
                            && !self.include_patterns.iter().any(|p| p.regex.is_match(text))
                        {
                            return false;
                        }
                        true
                    })
                    .collect();
            }
        } else {
            // Fast path: no regex, candidates are the final result
            self.filtered_indices = candidates;
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
        self.clusters.clear();
        self.cluster_map.clear();
        self.folded_clusters.clear();
        self.clusters_dirty = true;
        self.clamp_scroll();
    }

    /// Apply filters only to newly appended entries (starting from start_idx).
    /// Uses the same bitmask + millis path as apply_filters.
    pub fn apply_filters_incremental(&mut self, start_idx: usize) {
        let level_mask = self.level_filters_as_mask();
        let from_ms = self.date_from.map(|d| d.and_utc().timestamp_millis());
        let to_ms = self.date_to.map(|d| d.and_utc().timestamp_millis());
        let has_regex = !self.exclude_patterns.is_empty() || !self.include_patterns.is_empty();

        for idx in start_idx..self.entries.len() {
            let m = &self.entry_meta[idx];
            if m.level_bit & level_mask == 0 {
                continue;
            }
            if let Some(from) = from_ms
                && m.timestamp_ms != i64::MIN
                && m.timestamp_ms < from
            {
                continue;
            }
            if let Some(to) = to_ms
                && m.timestamp_ms != i64::MIN
                && m.timestamp_ms > to
            {
                continue;
            }
            if has_regex {
                let text = self.entries[idx].searchable_text();
                if self.exclude_patterns.iter().any(|p| p.regex.is_match(text)) {
                    continue;
                }
                if !self.include_patterns.is_empty()
                    && !self.include_patterns.iter().any(|p| p.regex.is_match(text))
                {
                    continue;
                }
            }
            self.filtered_indices.push(idx);
        }
        self.clusters_dirty = true;
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
    /// Parallelized with rayon when filtered_indices > 50K.
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
        if self.filtered_indices.len() > 50_000 {
            let entries = &self.entries;
            self.search_panel_matches = self
                .filtered_indices
                .par_iter()
                .enumerate()
                .filter(|&(_, &entry_idx)| regex.is_match(entries[entry_idx].searchable_text()))
                .map(|(pos, _)| pos)
                .collect();
        } else {
            self.search_panel_matches = self
                .filtered_indices
                .iter()
                .enumerate()
                .filter(|&(_, &entry_idx)| {
                    regex.is_match(self.entries[entry_idx].searchable_text())
                })
                .map(|(pos, _)| pos)
                .collect();
        }

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
