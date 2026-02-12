use super::App;

/// Count how many lines text will wrap to at given width
fn wrap_text_lines(text: &str, width: usize) -> usize {
    if width == 0 {
        return 1;
    }

    let mut lines = 0;
    let mut current_width = 0;

    for word in text.split_inclusive(|c: char| c.is_whitespace()) {
        let word_width = word.chars().count();

        if current_width + word_width <= width {
            current_width += word_width;
        } else if word_width > width {
            // Word is longer than width, need to split it
            if current_width > 0 {
                lines += 1;
            }
            // Split long word across multiple lines
            let mut remaining = word_width;
            while remaining > width {
                lines += 1;
                remaining -= width;
            }
            current_width = remaining;
        } else {
            // Start new line
            if current_width > 0 {
                lines += 1;
            }
            current_width = word_width;
        }
    }

    if current_width > 0 || lines == 0 {
        lines += 1;
    }

    lines
}

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

    /// Scroll viewport up without moving cursor (mouse wheel style)
    /// When cursor would go off-screen, it jumps to bottom of viewport
    pub fn scroll_viewport_up(&mut self, amount: usize, viewport_height: usize) {
        if self.filtered_indices.is_empty() || viewport_height == 0 {
            return;
        }

        let new_offset = self.scroll_offset.saturating_sub(amount);

        // If cursor would go below viewport, move it to bottom of new viewport
        if self.selected_index >= new_offset + viewport_height {
            self.selected_index =
                (new_offset + viewport_height - 1).min(self.filtered_indices.len() - 1);
        }

        self.scroll_offset = new_offset;
    }

    /// Scroll viewport down without moving cursor (mouse wheel style)
    /// When cursor would go off-screen, it jumps to top of viewport
    pub fn scroll_viewport_down(&mut self, amount: usize, viewport_height: usize) {
        if self.filtered_indices.is_empty() || viewport_height == 0 {
            return;
        }

        let max_offset = self.filtered_indices.len().saturating_sub(1);
        let new_offset = (self.scroll_offset + amount).min(max_offset);

        // If cursor would go above viewport, move it to top of new viewport
        if self.selected_index < new_offset {
            self.selected_index = new_offset;
        }

        self.scroll_offset = new_offset;
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

    /// Center the selected line in the viewport
    pub fn center_selected(&mut self) {
        if self.viewport_height == 0 {
            self.ensure_selected_visible();
            return;
        }
        self.scroll_offset = self.selected_index.saturating_sub(self.viewport_height / 2);
    }

    pub fn ensure_selected_visible_with_height(
        &mut self,
        viewport_height: usize,
        viewport_width: usize,
    ) {
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
            visual_lines += self.visual_lines_for_entry(idx, viewport_width);
        }

        // If selected entry extends beyond viewport, increase scroll_offset
        while visual_lines > viewport_height && self.scroll_offset < self.selected_index {
            visual_lines -= self.visual_lines_for_entry(self.scroll_offset, viewport_width);
            self.scroll_offset += 1;
        }
    }

    /// Calculate how many visual lines an entry occupies
    /// Accounts for continuation lines (when expanded) and word wrapping
    pub fn visual_lines_for_entry(&self, filtered_idx: usize, viewport_width: usize) -> usize {
        let entry_idx = match self.filtered_indices.get(filtered_idx) {
            Some(&idx) => idx,
            None => return 1,
        };
        let entry = match self.entries.get(entry_idx) {
            Some(e) => e,
            None => return 1,
        };

        // Calculate lines for the main message
        let lines = if self.wrap_enabled && viewport_width > 0 {
            let prefix_width = 20; // timestamp + level badge + space
            let available_width = viewport_width.saturating_sub(prefix_width);
            let message = crate::ui::extract_message(&entry.raw_line);
            wrap_text_lines(&message, available_width)
        } else {
            1
        };

        // Add continuation lines (also wrapped if needed)
        if self.expanded_entries.contains(&entry_idx) {
            if self.wrap_enabled && viewport_width > 0 {
                let prefix_width = 20;
                let available_width = viewport_width.saturating_sub(prefix_width);
                let cont_lines: usize = entry
                    .continuation_lines
                    .iter()
                    .map(|line| wrap_text_lines(line, available_width))
                    .sum();
                lines + cont_lines
            } else {
                lines + entry.continuation_lines.len()
            }
        } else {
            lines
        }
    }

    /// Jump to next search match relative to current cursor position (vim-style)
    pub fn next_match(&mut self) {
        if self.search_panel_matches.is_empty() {
            return;
        }
        // Find first match whose filtered_pos is strictly after selected_index
        let found = self
            .search_panel_matches
            .iter()
            .position(|&pos| pos > self.selected_index);
        self.search_panel_selected = found.unwrap_or(0); // wrap to first match
        self.sync_main_to_panel_selection();
    }

    /// Jump to previous search match relative to current cursor position (vim-style)
    pub fn prev_match(&mut self) {
        if self.search_panel_matches.is_empty() {
            return;
        }
        // Find last match whose filtered_pos is strictly before selected_index
        let found = self
            .search_panel_matches
            .iter()
            .rposition(|&pos| pos < self.selected_index);
        self.search_panel_selected = match found {
            Some(idx) => idx,
            None => self.search_panel_matches.len() - 1, // wrap to last match
        };
        self.sync_main_to_panel_selection();
    }

    /// Sync the main view cursor to the currently selected panel match
    pub fn sync_main_to_panel_selection(&mut self) {
        if let Some(&filtered_pos) = self.search_panel_matches.get(self.search_panel_selected) {
            self.selected_index = filtered_pos;
            self.center_selected();
        }
        self.ensure_panel_selected_visible();
    }

    /// Ensure search_panel_selected is visible within the panel's scroll window
    pub fn ensure_panel_selected_visible(&mut self) {
        if self.search_panel_height == 0 {
            return;
        }
        // Account for border (2 rows: top + bottom)
        let inner_height = self.search_panel_height.saturating_sub(2);
        if inner_height == 0 {
            return;
        }
        if self.search_panel_selected < self.search_panel_scroll {
            self.search_panel_scroll = self.search_panel_selected;
        } else if self.search_panel_selected >= self.search_panel_scroll + inner_height {
            self.search_panel_scroll = self.search_panel_selected - inner_height + 1;
        }
    }

    /// Move panel selection down by n
    pub fn panel_scroll_down(&mut self, n: usize) {
        if self.search_panel_matches.is_empty() {
            return;
        }
        self.search_panel_selected =
            (self.search_panel_selected + n).min(self.search_panel_matches.len() - 1);
        self.sync_main_to_panel_selection();
    }

    /// Move panel selection up by n
    pub fn panel_scroll_up(&mut self, n: usize) {
        self.search_panel_selected = self.search_panel_selected.saturating_sub(n);
        self.sync_main_to_panel_selection();
    }

    /// Jump to top of panel matches
    pub fn panel_scroll_to_top(&mut self) {
        self.search_panel_selected = 0;
        self.sync_main_to_panel_selection();
    }

    /// Jump to bottom of panel matches
    pub fn panel_scroll_to_bottom(&mut self) {
        if !self.search_panel_matches.is_empty() {
            self.search_panel_selected = self.search_panel_matches.len() - 1;
        }
        self.sync_main_to_panel_selection();
    }

    /// Get the currently selected entry
    pub fn selected_entry(&self) -> Option<&crate::log_entry::LogEntry> {
        self.filtered_indices
            .get(self.selected_index)
            .and_then(|&idx| self.entries.get(idx))
    }
}
