/// Wrap text to fit within a given width, returning wrapped lines
pub fn wrap_text(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![text.to_string()];
    }

    let mut result = Vec::new();
    let mut current_line = String::new();
    let mut current_width = 0;

    for word in text.split_inclusive(|c: char| c.is_whitespace()) {
        let word_width = word.chars().count();

        if current_width + word_width <= width {
            current_line.push_str(word);
            current_width += word_width;
        } else if word_width > width {
            // Word is longer than width, need to split it
            if !current_line.is_empty() {
                result.push(current_line);
                current_line = String::new();
                current_width = 0;
            }
            // Split long word
            let mut chars = word.chars().peekable();
            while chars.peek().is_some() {
                let chunk: String = chars.by_ref().take(width).collect();
                if chars.peek().is_some() {
                    result.push(chunk);
                } else {
                    current_line = chunk;
                    current_width = current_line.chars().count();
                }
            }
        } else {
            // Start new line
            if !current_line.is_empty() {
                result.push(current_line);
            }
            current_line = word.to_string();
            current_width = word_width;
        }
    }

    if !current_line.is_empty() {
        result.push(current_line);
    }

    if result.is_empty() {
        result.push(String::new());
    }

    result
}

/// Count how many lines text will wrap to at given width (no allocation)
pub fn wrap_text_line_count(text: &str, width: usize) -> usize {
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
            if current_width > 0 {
                lines += 1;
            }
            let mut remaining = word_width;
            while remaining > width {
                lines += 1;
                remaining -= width;
            }
            current_width = remaining;
        } else {
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
