use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

/// Reusable text input component with cursor tracking, mid-string editing,
/// and consistent rendering across all input bars.
#[derive(Debug, Clone)]
pub struct TextInput {
    pub text: String,
    pub cursor: usize, // char position (not byte)
}

impl Default for TextInput {
    fn default() -> Self {
        Self::new()
    }
}

impl TextInput {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            cursor: 0,
        }
    }

    /// Create with initial text, cursor at end
    pub fn with_text(text: String) -> Self {
        let cursor = text.chars().count();
        Self { text, cursor }
    }

    /// Get the text content
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Whether the input is empty
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// Set text and move cursor to end
    pub fn set_text(&mut self, text: String) {
        self.cursor = text.chars().count();
        self.text = text;
    }

    /// Clear text and reset cursor
    pub fn clear(&mut self) {
        self.text.clear();
        self.cursor = 0;
    }

    /// Insert a character at cursor position
    pub fn insert_char(&mut self, c: char) {
        let byte_idx = char_to_byte_index(&self.text, self.cursor);
        self.text.insert(byte_idx, c);
        self.cursor += 1;
    }

    /// Delete character before cursor (backspace)
    pub fn delete_back(&mut self) {
        if self.cursor > 0 {
            let byte_idx = char_to_byte_index(&self.text, self.cursor - 1);
            self.text.remove(byte_idx);
            self.cursor -= 1;
        }
    }

    /// Delete character at cursor (forward delete)
    pub fn delete_forward(&mut self) {
        let char_count = self.text.chars().count();
        if self.cursor < char_count {
            let byte_idx = char_to_byte_index(&self.text, self.cursor);
            self.text.remove(byte_idx);
        }
    }

    /// Move cursor left
    pub fn move_left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    /// Move cursor right
    pub fn move_right(&mut self) {
        let char_count = self.text.chars().count();
        if self.cursor < char_count {
            self.cursor += 1;
        }
    }

    /// Move cursor to start
    pub fn home(&mut self) {
        self.cursor = 0;
    }

    /// Move cursor to end
    pub fn end(&mut self) {
        self.cursor = self.text.chars().count();
    }

    /// Delete word backward (Ctrl+W). Uses whitespace boundaries by default.
    pub fn delete_word_back(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let byte_end = char_to_byte_index(&self.text, self.cursor);
        let mut new_pos = self.cursor;

        // Skip trailing whitespace
        while new_pos > 0 {
            let bi = char_to_byte_index(&self.text, new_pos - 1);
            if self.text[bi..].starts_with(char::is_whitespace) {
                new_pos -= 1;
            } else {
                break;
            }
        }

        // Delete back to previous whitespace or start
        while new_pos > 0 {
            let bi = char_to_byte_index(&self.text, new_pos - 1);
            if self.text[bi..].starts_with(char::is_whitespace) {
                break;
            }
            new_pos -= 1;
        }

        let byte_start = char_to_byte_index(&self.text, new_pos);
        self.text.drain(byte_start..byte_end);
        self.cursor = new_pos;
    }

    /// Delete path segment backward (Ctrl+W for file paths). Uses '/' boundaries.
    pub fn delete_path_segment_back(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let byte_end = char_to_byte_index(&self.text, self.cursor);
        let mut new_pos = self.cursor;

        // Skip trailing '/' separators
        while new_pos > 0 {
            let bi = char_to_byte_index(&self.text, new_pos - 1);
            if self.text[bi..].starts_with('/') {
                new_pos -= 1;
            } else {
                break;
            }
        }

        // Delete back to previous '/' or start
        while new_pos > 0 {
            let bi = char_to_byte_index(&self.text, new_pos - 1);
            if self.text[bi..].starts_with('/') {
                break;
            }
            new_pos -= 1;
        }

        let byte_start = char_to_byte_index(&self.text, new_pos);
        self.text.drain(byte_start..byte_end);
        self.cursor = new_pos;
    }

    /// Render the input text with block cursor into the given area.
    ///
    /// `prefix` is rendered before the text (e.g. "Path: ", "> ", " / ").
    /// `prefix_style` styles the prefix span.
    /// `cursor_style` styles the cursor block (inverted colors recommended).
    /// `active` controls whether the cursor is shown.
    pub fn render(
        &self,
        frame: &mut Frame,
        area: Rect,
        prefix: &str,
        prefix_style: Style,
        cursor_style: Style,
        active: bool,
    ) {
        let prefix_len = prefix.chars().count() as u16;
        let chars: Vec<char> = self.text.chars().collect();
        let available = area.width.saturating_sub(prefix_len + 1) as usize; // +1 for cursor block

        // Compute scroll offset so cursor stays visible
        let scroll_offset = if available == 0 {
            0
        } else if self.cursor >= available {
            self.cursor - available + 1
        } else {
            0
        };

        let visible_end = (scroll_offset + available).min(chars.len());

        let spans = if !active {
            // No cursor: just render text
            let visible: String = chars[scroll_offset..visible_end].iter().collect();
            vec![Span::styled(prefix, prefix_style), Span::raw(visible)]
        } else if self.cursor >= chars.len() {
            // Cursor at end
            let visible: String = chars[scroll_offset..visible_end].iter().collect();
            vec![
                Span::styled(prefix, prefix_style),
                Span::raw(visible),
                Span::styled(" ", cursor_style),
            ]
        } else {
            // Cursor in middle
            let before: String = chars[scroll_offset..self.cursor].iter().collect();
            let cursor_char: String = chars[self.cursor].to_string();
            let after: String = chars[self.cursor + 1..visible_end].iter().collect();
            vec![
                Span::styled(prefix, prefix_style),
                Span::raw(before),
                Span::styled(cursor_char, cursor_style),
                Span::raw(after),
            ]
        };

        frame.render_widget(Paragraph::new(Line::from(spans)), area);
    }

    /// Build spans for embedding in a larger Line (doesn't render directly).
    /// Returns Vec<Span> containing the text with cursor styling.
    pub fn to_spans(
        &self,
        available_width: u16,
        cursor_style: Style,
        active: bool,
    ) -> Vec<Span<'static>> {
        let chars: Vec<char> = self.text.chars().collect();
        let available = available_width.saturating_sub(1) as usize; // reserve 1 for cursor

        let scroll_offset = if available == 0 {
            0
        } else if self.cursor >= available {
            self.cursor - available + 1
        } else {
            0
        };

        let visible_end = (scroll_offset + available).min(chars.len());

        if !active {
            let visible: String = chars[scroll_offset..visible_end].iter().collect();
            vec![Span::raw(visible)]
        } else if self.cursor >= chars.len() {
            let visible: String = chars[scroll_offset..visible_end].iter().collect();
            vec![Span::raw(visible), Span::styled(" ", cursor_style)]
        } else {
            let before: String = chars[scroll_offset..self.cursor].iter().collect();
            let cursor_char: String = chars[self.cursor].to_string();
            let after: String = chars[self.cursor + 1..visible_end].iter().collect();
            vec![
                Span::raw(before),
                Span::styled(cursor_char, cursor_style),
                Span::raw(after),
            ]
        }
    }
}

/// Convert a char index to a byte index within a string
fn char_to_byte_index(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(i, _)| i)
        .unwrap_or(s.len())
}
