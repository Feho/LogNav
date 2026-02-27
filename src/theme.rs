use crate::log_entry::LogLevel;
use ratatui::style::{Color, Modifier, Style};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// User-facing theme configuration (serializable)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThemeConfig {
    /// Theme preset name: "dark" (default), "light"
    #[serde(default = "default_theme_name")]
    pub theme: String,
    /// Per-color overrides: key = color slot name, value = color string
    #[serde(default)]
    pub theme_overrides: HashMap<String, String>,
}

fn default_theme_name() -> String {
    "dark".to_string()
}

/// Runtime theme with resolved Color values
#[derive(Debug, Clone)]
pub struct Theme {
    pub name: String,

    // Core UI
    pub fg: Color,
    pub bg: Color,
    pub border: Color,
    pub accent: Color,
    pub muted: Color,
    pub error_text: Color,
    pub warning_text: Color,
    pub hint: Color,

    // Log levels
    pub level_error: Color,
    pub level_warn: Color,
    pub level_info: Color,
    pub level_debug: Color,
    pub level_trace: Color,
    pub level_profile: Color,
    pub level_badge_fg: Color,

    // Syntax highlighting
    pub syntax_url: Color,
    pub syntax_string: Color,
    pub syntax_key_value: Color,
    pub syntax_path: Color,
    pub syntax_number: Color,
    pub syntax_error_keyword: Color,
    pub syntax_boolean: Color,
    pub syntax_hex: Color,
    pub syntax_uuid: Color,
    pub syntax_ip: Color,

    // Highlights
    pub search_match_fg: Color,
    pub search_match_bg: Color,
    pub cursor_fg: Color,
    pub visual_select_fg: Color,
    pub visual_select_bg: Color,
    pub input_cursor_fg: Color,
    pub input_cursor_bg: Color,

    // Indicators
    pub bookmark: Color,
    pub expand_indicator: Color,
    pub expand_match_hint: Color,

    // Source file gutter
    pub source_colors: [Color; 4],

    // Cluster
    pub cluster_gutter: Color,
    pub cluster_sequence: Color,
    pub cluster_single: Color,
}

impl Theme {
    /// Dark theme — matches the original hardcoded colors exactly
    pub fn dark() -> Self {
        Self {
            name: "dark".to_string(),
            fg: Color::White,
            bg: Color::Black,
            border: Color::Cyan,
            accent: Color::Cyan,
            muted: Color::DarkGray,
            error_text: Color::Red,
            warning_text: Color::Yellow,
            hint: Color::Cyan,

            level_error: Color::Red,
            level_warn: Color::Yellow,
            level_info: Color::White,
            level_debug: Color::Cyan,
            level_trace: Color::DarkGray,
            level_profile: Color::Rgb(140, 140, 140),
            level_badge_fg: Color::Black,

            syntax_url: Color::LightBlue,
            syntax_string: Color::Green,
            syntax_key_value: Color::LightYellow,
            syntax_path: Color::LightGreen,
            syntax_number: Color::LightCyan,
            syntax_error_keyword: Color::LightRed,
            syntax_boolean: Color::LightMagenta,
            syntax_hex: Color::LightCyan,
            syntax_uuid: Color::Gray,
            syntax_ip: Color::Gray,

            search_match_fg: Color::Black,
            search_match_bg: Color::Yellow,
            cursor_fg: Color::Black,
            visual_select_fg: Color::White,
            visual_select_bg: Color::Indexed(238),
            input_cursor_fg: Color::Black,
            input_cursor_bg: Color::White,

            bookmark: Color::Yellow,
            expand_indicator: Color::Cyan,
            expand_match_hint: Color::Yellow,

            source_colors: [Color::Green, Color::Magenta, Color::Blue, Color::Yellow],

            cluster_gutter: Color::DarkGray,
            cluster_sequence: Color::Magenta,
            cluster_single: Color::Yellow,
        }
    }

    /// Light theme — high-contrast colors for light terminal backgrounds
    pub fn light() -> Self {
        Self {
            name: "light".to_string(),
            fg: Color::Rgb(30, 30, 30),
            bg: Color::Rgb(255, 255, 255),
            border: Color::Rgb(80, 120, 160),  // steel blue (4.6:1)
            accent: Color::Rgb(0, 95, 175),    // strong blue
            muted: Color::Rgb(100, 100, 100),  // medium gray (5.9:1)
            error_text: Color::Rgb(180, 0, 0), // dark red
            warning_text: Color::Rgb(175, 95, 0), // dark orange
            hint: Color::Rgb(0, 95, 175),

            level_error: Color::Rgb(180, 0, 0), // dark red (5.6:1)
            level_warn: Color::Rgb(175, 95, 0), // dark orange (4.7:1)
            level_info: Color::Black,
            level_debug: Color::Rgb(0, 115, 115), // teal (5.7:1)
            level_trace: Color::Rgb(100, 100, 100), // medium gray (5.9:1)
            level_profile: Color::Rgb(140, 140, 140), // medium gray
            level_badge_fg: Color::White,

            syntax_url: Color::Rgb(0, 70, 170),       // dark blue
            syntax_string: Color::Rgb(0, 120, 0),     // dark green
            syntax_key_value: Color::Rgb(160, 80, 0), // burnt orange
            syntax_path: Color::Rgb(0, 110, 50),      // forest green
            syntax_number: Color::Rgb(0, 120, 130),   // teal
            syntax_error_keyword: Color::Rgb(200, 0, 0),
            syntax_boolean: Color::Rgb(140, 0, 140), // dark magenta
            syntax_hex: Color::Rgb(0, 120, 130),     // teal
            syntax_uuid: Color::Rgb(100, 100, 100),  // muted gray (5.9:1)
            syntax_ip: Color::Rgb(100, 100, 100),

            search_match_fg: Color::Black,
            search_match_bg: Color::Rgb(255, 220, 50), // warm gold
            cursor_fg: Color::White,
            visual_select_fg: Color::Black,
            visual_select_bg: Color::Rgb(210, 225, 245), // light blue tint
            input_cursor_fg: Color::White,
            input_cursor_bg: Color::Rgb(30, 30, 30),

            bookmark: Color::Rgb(170, 95, 0), // amber (4.8:1)
            expand_indicator: Color::Rgb(0, 95, 175),
            expand_match_hint: Color::Rgb(170, 95, 0),

            source_colors: [
                Color::Rgb(0, 130, 60),  // green
                Color::Rgb(150, 0, 150), // magenta
                Color::Rgb(0, 70, 170),  // blue
                Color::Rgb(175, 95, 0),  // orange
            ],

            cluster_gutter: Color::Rgb(100, 100, 100),
            cluster_sequence: Color::Rgb(140, 0, 140),
            cluster_single: Color::Rgb(175, 95, 0),
        }
    }

    /// Create theme from preset name
    pub fn from_name(name: &str) -> Self {
        match name {
            "light" => Self::light(),
            _ => Self::dark(),
        }
    }

    /// Load theme from config: resolve preset then apply overrides
    pub fn from_config(config: &ThemeConfig) -> Self {
        let mut theme = Self::from_name(&config.theme);
        theme.apply_overrides(&config.theme_overrides);
        theme
    }

    /// Apply per-color overrides from config
    pub fn apply_overrides(&mut self, overrides: &HashMap<String, String>) {
        for (key, value) in overrides {
            if let Some(color) = parse_color(value) {
                match key.as_str() {
                    "fg" => self.fg = color,
                    "bg" => self.bg = color,
                    "border" => self.border = color,
                    "accent" => self.accent = color,
                    "muted" => self.muted = color,
                    "error_text" => self.error_text = color,
                    "warning_text" => self.warning_text = color,
                    "hint" => self.hint = color,
                    "level_error" => self.level_error = color,
                    "level_warn" => self.level_warn = color,
                    "level_info" => self.level_info = color,
                    "level_debug" => self.level_debug = color,
                    "level_trace" => self.level_trace = color,
                    "level_profile" => self.level_profile = color,
                    "level_badge_fg" => self.level_badge_fg = color,
                    "syntax_url" => self.syntax_url = color,
                    "syntax_string" => self.syntax_string = color,
                    "syntax_key_value" => self.syntax_key_value = color,
                    "syntax_path" => self.syntax_path = color,
                    "syntax_number" => self.syntax_number = color,
                    "syntax_error_keyword" => self.syntax_error_keyword = color,
                    "syntax_boolean" => self.syntax_boolean = color,
                    "syntax_hex" => self.syntax_hex = color,
                    "syntax_uuid" => self.syntax_uuid = color,
                    "syntax_ip" => self.syntax_ip = color,
                    "search_match_fg" => self.search_match_fg = color,
                    "search_match_bg" => self.search_match_bg = color,
                    "cursor_fg" => self.cursor_fg = color,
                    "visual_select_fg" => self.visual_select_fg = color,
                    "visual_select_bg" => self.visual_select_bg = color,
                    "input_cursor_fg" => self.input_cursor_fg = color,
                    "input_cursor_bg" => self.input_cursor_bg = color,
                    "bookmark" => self.bookmark = color,
                    "expand_indicator" => self.expand_indicator = color,
                    "expand_match_hint" => self.expand_match_hint = color,
                    "cluster_gutter" => self.cluster_gutter = color,
                    "cluster_sequence" => self.cluster_sequence = color,
                    "cluster_single" => self.cluster_single = color,
                    _ => {} // Unknown keys silently ignored
                }
            }
        }
    }

    // --- Convenience methods ---

    /// Get color for log level
    pub fn level_color(&self, level: LogLevel) -> Color {
        match level {
            LogLevel::Error => self.level_error,
            LogLevel::Warn => self.level_warn,
            LogLevel::Info => self.level_info,
            LogLevel::Debug => self.level_debug,
            LogLevel::Trace => self.level_trace,
            LogLevel::Profile => self.level_profile,
            LogLevel::Unknown => self.muted,
        }
    }

    /// Get style for level badge (inverted: fg on bg)
    pub fn level_style(&self, level: LogLevel) -> Style {
        Style::default()
            .fg(self.level_badge_fg)
            .bg(self.level_color(level))
            .add_modifier(Modifier::BOLD)
    }

    /// Search match highlight style
    pub fn search_highlight_style(&self) -> Style {
        Style::new()
            .fg(self.search_match_fg)
            .bg(self.search_match_bg)
    }

    /// Text input cursor style
    pub fn cursor_style(&self) -> Style {
        Style::new()
            .fg(self.input_cursor_fg)
            .bg(self.input_cursor_bg)
    }

    /// Cursor line style (selected entry)
    pub fn cursor_line_style(&self, level: LogLevel) -> Style {
        Style::default()
            .bg(self.level_color(level))
            .fg(self.cursor_fg)
            .add_modifier(Modifier::BOLD)
    }

    /// Visual selection style
    pub fn visual_select_style(&self) -> Style {
        Style::default()
            .bg(self.visual_select_bg)
            .fg(self.visual_select_fg)
    }

    /// Status bar base style
    pub fn status_bar_style(&self) -> Style {
        Style::default().bg(self.bg).fg(self.fg)
    }

    /// Border style for overlays
    pub fn border_style(&self) -> Style {
        Style::default().fg(self.border)
    }

    /// Accent style (selected items in lists)
    pub fn selected_style(&self) -> Style {
        Style::default().bg(self.accent).fg(self.level_badge_fg)
    }

    /// Source gutter color for given index
    pub fn source_color(&self, idx: u8) -> Color {
        self.source_colors[idx as usize % self.source_colors.len()]
    }
}

/// Parse a color string. Supports:
/// - Named: "Red", "DarkGray", "LightBlue", etc.
/// - Hex: "#ff0000", "#FF5555"
/// - Indexed (256-color): "238", "130"
pub fn parse_color(s: &str) -> Option<Color> {
    let s = s.trim();

    // Hex color: #RRGGBB
    if let Some(hex) = s.strip_prefix('#') {
        if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            return Some(Color::Rgb(r, g, b));
        }
        return None;
    }

    // Pure number: indexed color
    if let Ok(n) = s.parse::<u8>() {
        return Some(Color::Indexed(n));
    }

    // Named colors (case-insensitive)
    match s.to_lowercase().as_str() {
        "black" => Some(Color::Black),
        "red" => Some(Color::Red),
        "green" => Some(Color::Green),
        "yellow" => Some(Color::Yellow),
        "blue" => Some(Color::Blue),
        "magenta" => Some(Color::Magenta),
        "cyan" => Some(Color::Cyan),
        "gray" | "grey" => Some(Color::Gray),
        "darkgray" | "darkgrey" | "dark_gray" | "dark_grey" => Some(Color::DarkGray),
        "lightred" | "light_red" => Some(Color::LightRed),
        "lightgreen" | "light_green" => Some(Color::LightGreen),
        "lightyellow" | "light_yellow" => Some(Color::LightYellow),
        "lightblue" | "light_blue" => Some(Color::LightBlue),
        "lightmagenta" | "light_magenta" => Some(Color::LightMagenta),
        "lightcyan" | "light_cyan" => Some(Color::LightCyan),
        "white" => Some(Color::White),
        "reset" => Some(Color::Reset),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_named_colors() {
        assert_eq!(parse_color("Red"), Some(Color::Red));
        assert_eq!(parse_color("darkgray"), Some(Color::DarkGray));
        assert_eq!(parse_color("LightBlue"), Some(Color::LightBlue));
        assert_eq!(parse_color("light_cyan"), Some(Color::LightCyan));
    }

    #[test]
    fn test_parse_hex_colors() {
        assert_eq!(parse_color("#ff0000"), Some(Color::Rgb(255, 0, 0)));
        assert_eq!(parse_color("#00FF00"), Some(Color::Rgb(0, 255, 0)));
        assert_eq!(parse_color("#0000ff"), Some(Color::Rgb(0, 0, 255)));
    }

    #[test]
    fn test_parse_indexed_colors() {
        assert_eq!(parse_color("238"), Some(Color::Indexed(238)));
        assert_eq!(parse_color("0"), Some(Color::Indexed(0)));
        assert_eq!(parse_color("255"), Some(Color::Indexed(255)));
    }

    #[test]
    fn test_parse_invalid() {
        assert_eq!(parse_color(""), None);
        assert_eq!(parse_color("#xyz"), None);
        assert_eq!(parse_color("notacolor"), None);
        assert_eq!(parse_color("256"), None); // u8 overflow
    }

    #[test]
    fn test_dark_theme_matches_original() {
        let t = Theme::dark();
        assert_eq!(t.level_color(LogLevel::Error), Color::Red);
        assert_eq!(t.level_color(LogLevel::Warn), Color::Yellow);
        assert_eq!(t.level_color(LogLevel::Info), Color::White);
        assert_eq!(t.level_color(LogLevel::Debug), Color::Cyan);
    }

    #[test]
    fn test_overrides() {
        let mut t = Theme::dark();
        let mut overrides = HashMap::new();
        overrides.insert("level_error".to_string(), "#ff5555".to_string());
        overrides.insert("border".to_string(), "Green".to_string());
        t.apply_overrides(&overrides);
        assert_eq!(t.level_error, Color::Rgb(255, 85, 85));
        assert_eq!(t.border, Color::Green);
    }

    #[test]
    fn test_from_config() {
        let config = ThemeConfig {
            theme: "light".to_string(),
            theme_overrides: HashMap::new(),
        };
        let t = Theme::from_config(&config);
        assert_eq!(t.name, "light");
    }
}
