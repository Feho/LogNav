use super::App;

impl App {
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

    /// Jump to next search match
    pub fn next_search_match(&mut self) {
        if let super::FocusState::Search {
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
        if let super::FocusState::Search {
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

    /// Get the currently selected entry
    pub fn selected_entry(&self) -> Option<&crate::log_entry::LogEntry> {
        self.filtered_indices
            .get(self.selected_index)
            .and_then(|&idx| self.entries.get(idx))
    }
}
