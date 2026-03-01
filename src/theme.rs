use crate::log_entry::LogLevel;
use ratatui::style::{Color, Modifier, Style};
use std::collections::HashMap;

/// User-facing theme configuration (serializable)
#[derive(Debug, Clone, Default)]
pub struct ThemeConfig {
    /// Theme preset name: "dark" (default), "light"
    pub theme: String,
    /// Per-theme overrides
    pub dark_overrides: HashMap<String, String>,
    pub light_overrides: HashMap<String, String>,
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
            syntax_ip: Color::Rgb(209, 154, 102),

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

    /// Dracula — purple-gray bg, pink/cyan/green pastels
    pub fn dracula() -> Self {
        Self {
            name: "dracula".to_string(),
            fg: Color::Rgb(248, 248, 242),
            bg: Color::Rgb(40, 42, 54),
            border: Color::Rgb(98, 114, 164),
            accent: Color::Rgb(139, 233, 253),
            muted: Color::Rgb(98, 114, 164),
            error_text: Color::Rgb(255, 85, 85),
            warning_text: Color::Rgb(255, 184, 108),
            hint: Color::Rgb(139, 233, 253),

            level_error: Color::Rgb(255, 85, 85),
            level_warn: Color::Rgb(255, 184, 108),
            level_info: Color::Rgb(248, 248, 242),
            level_debug: Color::Rgb(139, 233, 253),
            level_trace: Color::Rgb(98, 114, 164),
            level_profile: Color::Rgb(140, 140, 140),
            level_badge_fg: Color::Rgb(40, 42, 54),

            syntax_url: Color::Rgb(139, 233, 253),
            syntax_string: Color::Rgb(80, 250, 123),
            syntax_key_value: Color::Rgb(241, 250, 140),
            syntax_path: Color::Rgb(80, 250, 123),
            syntax_number: Color::Rgb(189, 147, 249),
            syntax_error_keyword: Color::Rgb(255, 85, 85),
            syntax_boolean: Color::Rgb(255, 121, 198),
            syntax_hex: Color::Rgb(189, 147, 249),
            syntax_uuid: Color::Rgb(98, 114, 164),
            syntax_ip: Color::Rgb(98, 114, 164),

            search_match_fg: Color::Rgb(40, 42, 54),
            search_match_bg: Color::Rgb(241, 250, 140),
            cursor_fg: Color::Rgb(40, 42, 54),
            visual_select_fg: Color::Rgb(248, 248, 242),
            visual_select_bg: Color::Rgb(68, 71, 90),
            input_cursor_fg: Color::Rgb(40, 42, 54),
            input_cursor_bg: Color::Rgb(248, 248, 242),

            bookmark: Color::Rgb(255, 184, 108),
            expand_indicator: Color::Rgb(139, 233, 253),
            expand_match_hint: Color::Rgb(241, 250, 140),

            source_colors: [
                Color::Rgb(80, 250, 123),
                Color::Rgb(255, 121, 198),
                Color::Rgb(139, 233, 253),
                Color::Rgb(255, 184, 108),
            ],

            cluster_gutter: Color::Rgb(98, 114, 164),
            cluster_sequence: Color::Rgb(189, 147, 249),
            cluster_single: Color::Rgb(241, 250, 140),
        }
    }

    /// Nord — deep blue-gray bg, frost blues, arctic greens
    pub fn nord() -> Self {
        Self {
            name: "nord".to_string(),
            fg: Color::Rgb(216, 222, 233),
            bg: Color::Rgb(46, 52, 64),
            border: Color::Rgb(136, 192, 208),
            accent: Color::Rgb(136, 192, 208),
            muted: Color::Rgb(76, 86, 106),
            error_text: Color::Rgb(191, 97, 106),
            warning_text: Color::Rgb(235, 203, 139),
            hint: Color::Rgb(136, 192, 208),

            level_error: Color::Rgb(191, 97, 106),
            level_warn: Color::Rgb(235, 203, 139),
            level_info: Color::Rgb(216, 222, 233),
            level_debug: Color::Rgb(136, 192, 208),
            level_trace: Color::Rgb(76, 86, 106),
            level_profile: Color::Rgb(140, 140, 140),
            level_badge_fg: Color::Rgb(46, 52, 64),

            syntax_url: Color::Rgb(129, 161, 193),
            syntax_string: Color::Rgb(163, 190, 140),
            syntax_key_value: Color::Rgb(235, 203, 139),
            syntax_path: Color::Rgb(163, 190, 140),
            syntax_number: Color::Rgb(180, 142, 173),
            syntax_error_keyword: Color::Rgb(191, 97, 106),
            syntax_boolean: Color::Rgb(180, 142, 173),
            syntax_hex: Color::Rgb(143, 188, 187),
            syntax_uuid: Color::Rgb(76, 86, 106),
            syntax_ip: Color::Rgb(76, 86, 106),

            search_match_fg: Color::Rgb(46, 52, 64),
            search_match_bg: Color::Rgb(235, 203, 139),
            cursor_fg: Color::Rgb(46, 52, 64),
            visual_select_fg: Color::Rgb(216, 222, 233),
            visual_select_bg: Color::Rgb(67, 76, 94),
            input_cursor_fg: Color::Rgb(46, 52, 64),
            input_cursor_bg: Color::Rgb(216, 222, 233),

            bookmark: Color::Rgb(235, 203, 139),
            expand_indicator: Color::Rgb(136, 192, 208),
            expand_match_hint: Color::Rgb(235, 203, 139),

            source_colors: [
                Color::Rgb(163, 190, 140),
                Color::Rgb(180, 142, 173),
                Color::Rgb(129, 161, 193),
                Color::Rgb(208, 135, 112),
            ],

            cluster_gutter: Color::Rgb(76, 86, 106),
            cluster_sequence: Color::Rgb(180, 142, 173),
            cluster_single: Color::Rgb(235, 203, 139),
        }
    }

    /// Gruvbox Dark — dark brown bg, warm retro oranges/greens
    pub fn gruvbox_dark() -> Self {
        Self {
            name: "gruvbox_dark".to_string(),
            fg: Color::Rgb(235, 219, 178),
            bg: Color::Rgb(40, 40, 40),
            border: Color::Rgb(168, 153, 132),
            accent: Color::Rgb(131, 165, 152),
            muted: Color::Rgb(146, 131, 116),
            error_text: Color::Rgb(251, 73, 52),
            warning_text: Color::Rgb(250, 189, 47),
            hint: Color::Rgb(131, 165, 152),

            level_error: Color::Rgb(251, 73, 52),
            level_warn: Color::Rgb(250, 189, 47),
            level_info: Color::Rgb(235, 219, 178),
            level_debug: Color::Rgb(131, 165, 152),
            level_trace: Color::Rgb(146, 131, 116),
            level_profile: Color::Rgb(140, 140, 140),
            level_badge_fg: Color::Rgb(40, 40, 40),

            syntax_url: Color::Rgb(131, 165, 152),
            syntax_string: Color::Rgb(184, 187, 38),
            syntax_key_value: Color::Rgb(250, 189, 47),
            syntax_path: Color::Rgb(184, 187, 38),
            syntax_number: Color::Rgb(211, 134, 155),
            syntax_error_keyword: Color::Rgb(251, 73, 52),
            syntax_boolean: Color::Rgb(211, 134, 155),
            syntax_hex: Color::Rgb(142, 192, 124),
            syntax_uuid: Color::Rgb(146, 131, 116),
            syntax_ip: Color::Rgb(146, 131, 116),

            search_match_fg: Color::Rgb(40, 40, 40),
            search_match_bg: Color::Rgb(250, 189, 47),
            cursor_fg: Color::Rgb(40, 40, 40),
            visual_select_fg: Color::Rgb(235, 219, 178),
            visual_select_bg: Color::Rgb(80, 73, 69),
            input_cursor_fg: Color::Rgb(40, 40, 40),
            input_cursor_bg: Color::Rgb(235, 219, 178),

            bookmark: Color::Rgb(254, 128, 25),
            expand_indicator: Color::Rgb(131, 165, 152),
            expand_match_hint: Color::Rgb(250, 189, 47),

            source_colors: [
                Color::Rgb(184, 187, 38),
                Color::Rgb(211, 134, 155),
                Color::Rgb(131, 165, 152),
                Color::Rgb(254, 128, 25),
            ],

            cluster_gutter: Color::Rgb(146, 131, 116),
            cluster_sequence: Color::Rgb(211, 134, 155),
            cluster_single: Color::Rgb(250, 189, 47),
        }
    }

    /// Catppuccin Latte — warm cream bg, soft pastels
    pub fn catppuccin_latte() -> Self {
        Self {
            name: "catppuccin_latte".to_string(),
            fg: Color::Rgb(76, 79, 105),
            bg: Color::Rgb(239, 241, 245),
            border: Color::Rgb(140, 143, 161),
            accent: Color::Rgb(30, 102, 245),
            muted: Color::Rgb(140, 143, 161),
            error_text: Color::Rgb(210, 15, 57),
            warning_text: Color::Rgb(223, 142, 29),
            hint: Color::Rgb(30, 102, 245),

            level_error: Color::Rgb(210, 15, 57),
            level_warn: Color::Rgb(223, 142, 29),
            level_info: Color::Rgb(76, 79, 105),
            level_debug: Color::Rgb(4, 165, 229),
            level_trace: Color::Rgb(140, 143, 161),
            level_profile: Color::Rgb(140, 140, 140),
            level_badge_fg: Color::Rgb(239, 241, 245),

            syntax_url: Color::Rgb(30, 102, 245),
            syntax_string: Color::Rgb(64, 160, 43),
            syntax_key_value: Color::Rgb(223, 142, 29),
            syntax_path: Color::Rgb(64, 160, 43),
            syntax_number: Color::Rgb(136, 57, 239),
            syntax_error_keyword: Color::Rgb(210, 15, 57),
            syntax_boolean: Color::Rgb(234, 118, 203),
            syntax_hex: Color::Rgb(23, 146, 153),
            syntax_uuid: Color::Rgb(140, 143, 161),
            syntax_ip: Color::Rgb(140, 143, 161),

            search_match_fg: Color::Rgb(76, 79, 105),
            search_match_bg: Color::Rgb(223, 142, 29),
            cursor_fg: Color::Rgb(239, 241, 245),
            visual_select_fg: Color::Rgb(76, 79, 105),
            visual_select_bg: Color::Rgb(204, 208, 218),
            input_cursor_fg: Color::Rgb(239, 241, 245),
            input_cursor_bg: Color::Rgb(76, 79, 105),

            bookmark: Color::Rgb(223, 142, 29),
            expand_indicator: Color::Rgb(30, 102, 245),
            expand_match_hint: Color::Rgb(223, 142, 29),

            source_colors: [
                Color::Rgb(64, 160, 43),
                Color::Rgb(234, 118, 203),
                Color::Rgb(30, 102, 245),
                Color::Rgb(223, 142, 29),
            ],

            cluster_gutter: Color::Rgb(140, 143, 161),
            cluster_sequence: Color::Rgb(136, 57, 239),
            cluster_single: Color::Rgb(223, 142, 29),
        }
    }

    /// Rosé Pine Dawn — muted rose/gold on warm white
    pub fn rose_pine_dawn() -> Self {
        Self {
            name: "rose_pine_dawn".to_string(),
            fg: Color::Rgb(87, 82, 121),
            bg: Color::Rgb(250, 244, 237),
            border: Color::Rgb(144, 140, 170),
            accent: Color::Rgb(40, 105, 131),
            muted: Color::Rgb(144, 140, 170),
            error_text: Color::Rgb(180, 99, 122),
            warning_text: Color::Rgb(234, 157, 52),
            hint: Color::Rgb(40, 105, 131),

            level_error: Color::Rgb(180, 99, 122),
            level_warn: Color::Rgb(234, 157, 52),
            level_info: Color::Rgb(87, 82, 121),
            level_debug: Color::Rgb(40, 105, 131),
            level_trace: Color::Rgb(144, 140, 170),
            level_profile: Color::Rgb(140, 140, 140),
            level_badge_fg: Color::Rgb(250, 244, 237),

            syntax_url: Color::Rgb(40, 105, 131),
            syntax_string: Color::Rgb(40, 105, 131),
            syntax_key_value: Color::Rgb(234, 157, 52),
            syntax_path: Color::Rgb(86, 148, 159),
            syntax_number: Color::Rgb(144, 122, 169),
            syntax_error_keyword: Color::Rgb(180, 99, 122),
            syntax_boolean: Color::Rgb(215, 130, 126),
            syntax_hex: Color::Rgb(86, 148, 159),
            syntax_uuid: Color::Rgb(144, 140, 170),
            syntax_ip: Color::Rgb(144, 140, 170),

            search_match_fg: Color::Rgb(87, 82, 121),
            search_match_bg: Color::Rgb(242, 213, 156),
            cursor_fg: Color::Rgb(250, 244, 237),
            visual_select_fg: Color::Rgb(87, 82, 121),
            visual_select_bg: Color::Rgb(242, 233, 222),
            input_cursor_fg: Color::Rgb(250, 244, 237),
            input_cursor_bg: Color::Rgb(87, 82, 121),

            bookmark: Color::Rgb(234, 157, 52),
            expand_indicator: Color::Rgb(40, 105, 131),
            expand_match_hint: Color::Rgb(234, 157, 52),

            source_colors: [
                Color::Rgb(86, 148, 159),
                Color::Rgb(215, 130, 126),
                Color::Rgb(40, 105, 131),
                Color::Rgb(234, 157, 52),
            ],

            cluster_gutter: Color::Rgb(144, 140, 170),
            cluster_sequence: Color::Rgb(144, 122, 169),
            cluster_single: Color::Rgb(234, 157, 52),
        }
    }

    /// Solarized Light — cream bg, muted blue/cyan accents
    pub fn solarized_light() -> Self {
        Self {
            name: "solarized_light".to_string(),
            fg: Color::Rgb(101, 123, 131),
            bg: Color::Rgb(253, 246, 227),
            border: Color::Rgb(147, 161, 161),
            accent: Color::Rgb(38, 139, 210),
            muted: Color::Rgb(147, 161, 161),
            error_text: Color::Rgb(220, 50, 47),
            warning_text: Color::Rgb(181, 137, 0),
            hint: Color::Rgb(38, 139, 210),

            level_error: Color::Rgb(220, 50, 47),
            level_warn: Color::Rgb(181, 137, 0),
            level_info: Color::Rgb(101, 123, 131),
            level_debug: Color::Rgb(42, 161, 152),
            level_trace: Color::Rgb(147, 161, 161),
            level_profile: Color::Rgb(140, 140, 140),
            level_badge_fg: Color::Rgb(253, 246, 227),

            syntax_url: Color::Rgb(38, 139, 210),
            syntax_string: Color::Rgb(42, 161, 152),
            syntax_key_value: Color::Rgb(181, 137, 0),
            syntax_path: Color::Rgb(133, 153, 0),
            syntax_number: Color::Rgb(108, 113, 196),
            syntax_error_keyword: Color::Rgb(220, 50, 47),
            syntax_boolean: Color::Rgb(211, 54, 130),
            syntax_hex: Color::Rgb(42, 161, 152),
            syntax_uuid: Color::Rgb(147, 161, 161),
            syntax_ip: Color::Rgb(147, 161, 161),

            search_match_fg: Color::Rgb(253, 246, 227),
            search_match_bg: Color::Rgb(181, 137, 0),
            cursor_fg: Color::Rgb(253, 246, 227),
            visual_select_fg: Color::Rgb(101, 123, 131),
            visual_select_bg: Color::Rgb(238, 232, 213),
            input_cursor_fg: Color::Rgb(253, 246, 227),
            input_cursor_bg: Color::Rgb(101, 123, 131),

            bookmark: Color::Rgb(203, 75, 22),
            expand_indicator: Color::Rgb(38, 139, 210),
            expand_match_hint: Color::Rgb(181, 137, 0),

            source_colors: [
                Color::Rgb(133, 153, 0),
                Color::Rgb(211, 54, 130),
                Color::Rgb(38, 139, 210),
                Color::Rgb(203, 75, 22),
            ],

            cluster_gutter: Color::Rgb(147, 161, 161),
            cluster_sequence: Color::Rgb(108, 113, 196),
            cluster_single: Color::Rgb(181, 137, 0),
        }
    }

    /// Tokyo Night — cool blue-purple dark theme
    pub fn tokyo_night() -> Self {
        Self {
            name: "tokyo_night".to_string(),
            fg: Color::Rgb(169, 177, 214),
            bg: Color::Rgb(26, 27, 38),
            border: Color::Rgb(61, 89, 161),
            accent: Color::Rgb(122, 162, 247),
            muted: Color::Rgb(86, 95, 137),
            error_text: Color::Rgb(247, 118, 142),
            warning_text: Color::Rgb(224, 175, 104),
            hint: Color::Rgb(122, 162, 247),

            level_error: Color::Rgb(247, 118, 142),
            level_warn: Color::Rgb(224, 175, 104),
            level_info: Color::Rgb(169, 177, 214),
            level_debug: Color::Rgb(125, 207, 255),
            level_trace: Color::Rgb(86, 95, 137),
            level_profile: Color::Rgb(140, 140, 140),
            level_badge_fg: Color::Rgb(26, 27, 38),

            syntax_url: Color::Rgb(122, 162, 247),
            syntax_string: Color::Rgb(158, 206, 106),
            syntax_key_value: Color::Rgb(224, 175, 104),
            syntax_path: Color::Rgb(115, 218, 202),
            syntax_number: Color::Rgb(255, 158, 100),
            syntax_error_keyword: Color::Rgb(247, 118, 142),
            syntax_boolean: Color::Rgb(187, 154, 247),
            syntax_hex: Color::Rgb(125, 207, 255),
            syntax_uuid: Color::Rgb(86, 95, 137),
            syntax_ip: Color::Rgb(86, 95, 137),

            search_match_fg: Color::Rgb(26, 27, 38),
            search_match_bg: Color::Rgb(224, 175, 104),
            cursor_fg: Color::Rgb(26, 27, 38),
            visual_select_fg: Color::Rgb(169, 177, 214),
            visual_select_bg: Color::Rgb(41, 46, 66),
            input_cursor_fg: Color::Rgb(26, 27, 38),
            input_cursor_bg: Color::Rgb(169, 177, 214),

            bookmark: Color::Rgb(224, 175, 104),
            expand_indicator: Color::Rgb(122, 162, 247),
            expand_match_hint: Color::Rgb(224, 175, 104),

            source_colors: [
                Color::Rgb(158, 206, 106),
                Color::Rgb(187, 154, 247),
                Color::Rgb(125, 207, 255),
                Color::Rgb(255, 158, 100),
            ],

            cluster_gutter: Color::Rgb(86, 95, 137),
            cluster_sequence: Color::Rgb(187, 154, 247),
            cluster_single: Color::Rgb(224, 175, 104),
        }
    }

    /// One Dark — Atom's iconic dark theme
    pub fn one_dark() -> Self {
        Self {
            name: "one_dark".to_string(),
            fg: Color::Rgb(171, 178, 191),
            bg: Color::Rgb(40, 44, 52),
            border: Color::Rgb(76, 82, 99),
            accent: Color::Rgb(97, 175, 239),
            muted: Color::Rgb(92, 99, 112),
            error_text: Color::Rgb(224, 108, 117),
            warning_text: Color::Rgb(229, 192, 123),
            hint: Color::Rgb(97, 175, 239),

            level_error: Color::Rgb(224, 108, 117),
            level_warn: Color::Rgb(229, 192, 123),
            level_info: Color::Rgb(171, 178, 191),
            level_debug: Color::Rgb(86, 182, 194),
            level_trace: Color::Rgb(92, 99, 112),
            level_profile: Color::Rgb(140, 140, 140),
            level_badge_fg: Color::Rgb(40, 44, 52),

            syntax_url: Color::Rgb(97, 175, 239),
            syntax_string: Color::Rgb(152, 195, 121),
            syntax_key_value: Color::Rgb(229, 192, 123),
            syntax_path: Color::Rgb(152, 195, 121),
            syntax_number: Color::Rgb(209, 154, 102),
            syntax_error_keyword: Color::Rgb(224, 108, 117),
            syntax_boolean: Color::Rgb(198, 120, 221),
            syntax_hex: Color::Rgb(86, 182, 194),
            syntax_uuid: Color::Rgb(92, 99, 112),
            syntax_ip: Color::Rgb(92, 99, 112),

            search_match_fg: Color::Rgb(40, 44, 52),
            search_match_bg: Color::Rgb(229, 192, 123),
            cursor_fg: Color::Rgb(40, 44, 52),
            visual_select_fg: Color::Rgb(171, 178, 191),
            visual_select_bg: Color::Rgb(62, 68, 81),
            input_cursor_fg: Color::Rgb(40, 44, 52),
            input_cursor_bg: Color::Rgb(171, 178, 191),

            bookmark: Color::Rgb(209, 154, 102),
            expand_indicator: Color::Rgb(97, 175, 239),
            expand_match_hint: Color::Rgb(229, 192, 123),

            source_colors: [
                Color::Rgb(152, 195, 121),
                Color::Rgb(198, 120, 221),
                Color::Rgb(97, 175, 239),
                Color::Rgb(209, 154, 102),
            ],

            cluster_gutter: Color::Rgb(92, 99, 112),
            cluster_sequence: Color::Rgb(198, 120, 221),
            cluster_single: Color::Rgb(229, 192, 123),
        }
    }

    /// Gruvbox Light — warm retro light theme
    pub fn gruvbox_light() -> Self {
        Self {
            name: "gruvbox_light".to_string(),
            fg: Color::Rgb(60, 56, 54),
            bg: Color::Rgb(251, 241, 199),
            border: Color::Rgb(168, 153, 132),
            accent: Color::Rgb(69, 133, 136),
            muted: Color::Rgb(146, 131, 116),
            error_text: Color::Rgb(204, 36, 29),
            warning_text: Color::Rgb(181, 118, 20),
            hint: Color::Rgb(69, 133, 136),

            level_error: Color::Rgb(204, 36, 29),
            level_warn: Color::Rgb(181, 118, 20),
            level_info: Color::Rgb(60, 56, 54),
            level_debug: Color::Rgb(69, 133, 136),
            level_trace: Color::Rgb(146, 131, 116),
            level_profile: Color::Rgb(140, 140, 140),
            level_badge_fg: Color::Rgb(251, 241, 199),

            syntax_url: Color::Rgb(69, 133, 136),
            syntax_string: Color::Rgb(121, 116, 14),
            syntax_key_value: Color::Rgb(181, 118, 20),
            syntax_path: Color::Rgb(121, 116, 14),
            syntax_number: Color::Rgb(177, 98, 134),
            syntax_error_keyword: Color::Rgb(204, 36, 29),
            syntax_boolean: Color::Rgb(177, 98, 134),
            syntax_hex: Color::Rgb(69, 133, 136),
            syntax_uuid: Color::Rgb(146, 131, 116),
            syntax_ip: Color::Rgb(146, 131, 116),

            search_match_fg: Color::Rgb(60, 56, 54),
            search_match_bg: Color::Rgb(215, 153, 33),
            cursor_fg: Color::Rgb(251, 241, 199),
            visual_select_fg: Color::Rgb(60, 56, 54),
            visual_select_bg: Color::Rgb(235, 219, 178),
            input_cursor_fg: Color::Rgb(251, 241, 199),
            input_cursor_bg: Color::Rgb(60, 56, 54),

            bookmark: Color::Rgb(175, 58, 3),
            expand_indicator: Color::Rgb(69, 133, 136),
            expand_match_hint: Color::Rgb(181, 118, 20),

            source_colors: [
                Color::Rgb(121, 116, 14),
                Color::Rgb(177, 98, 134),
                Color::Rgb(69, 133, 136),
                Color::Rgb(175, 58, 3),
            ],

            cluster_gutter: Color::Rgb(146, 131, 116),
            cluster_sequence: Color::Rgb(177, 98, 134),
            cluster_single: Color::Rgb(181, 118, 20),
        }
    }

    /// GitHub Light — clean, minimal light theme
    pub fn github_light() -> Self {
        Self {
            name: "github_light".to_string(),
            fg: Color::Rgb(36, 41, 47),
            bg: Color::Rgb(255, 255, 255),
            border: Color::Rgb(208, 215, 222),
            accent: Color::Rgb(9, 105, 218),
            muted: Color::Rgb(101, 109, 118),
            error_text: Color::Rgb(207, 34, 46),
            warning_text: Color::Rgb(158, 106, 3),
            hint: Color::Rgb(9, 105, 218),

            level_error: Color::Rgb(207, 34, 46),
            level_warn: Color::Rgb(158, 106, 3),
            level_info: Color::Rgb(36, 41, 47),
            level_debug: Color::Rgb(2, 120, 138),
            level_trace: Color::Rgb(101, 109, 118),
            level_profile: Color::Rgb(140, 140, 140),
            level_badge_fg: Color::Rgb(255, 255, 255),

            syntax_url: Color::Rgb(9, 105, 218),
            syntax_string: Color::Rgb(17, 99, 41),
            syntax_key_value: Color::Rgb(158, 106, 3),
            syntax_path: Color::Rgb(17, 99, 41),
            syntax_number: Color::Rgb(5, 80, 174),
            syntax_error_keyword: Color::Rgb(207, 34, 46),
            syntax_boolean: Color::Rgb(130, 80, 223),
            syntax_hex: Color::Rgb(2, 120, 138),
            syntax_uuid: Color::Rgb(101, 109, 118),
            syntax_ip: Color::Rgb(101, 109, 118),

            search_match_fg: Color::Rgb(36, 41, 47),
            search_match_bg: Color::Rgb(255, 209, 92),
            cursor_fg: Color::Rgb(255, 255, 255),
            visual_select_fg: Color::Rgb(36, 41, 47),
            visual_select_bg: Color::Rgb(218, 230, 253),
            input_cursor_fg: Color::Rgb(255, 255, 255),
            input_cursor_bg: Color::Rgb(36, 41, 47),

            bookmark: Color::Rgb(191, 135, 0),
            expand_indicator: Color::Rgb(9, 105, 218),
            expand_match_hint: Color::Rgb(158, 106, 3),

            source_colors: [
                Color::Rgb(17, 99, 41),
                Color::Rgb(130, 80, 223),
                Color::Rgb(9, 105, 218),
                Color::Rgb(191, 135, 0),
            ],

            cluster_gutter: Color::Rgb(101, 109, 118),
            cluster_sequence: Color::Rgb(130, 80, 223),
            cluster_single: Color::Rgb(158, 106, 3),
        }
    }

    /// Create theme from preset name
    pub fn from_name(name: &str) -> Self {
        match name {
            "light" => Self::light(),
            "dracula" => Self::dracula(),
            "nord" => Self::nord(),
            "gruvbox_dark" => Self::gruvbox_dark(),
            "tokyo_night" => Self::tokyo_night(),
            "one_dark" => Self::one_dark(),
            "catppuccin_latte" => Self::catppuccin_latte(),
            "rose_pine_dawn" => Self::rose_pine_dawn(),
            "solarized_light" => Self::solarized_light(),
            "gruvbox_light" => Self::gruvbox_light(),
            "github_light" => Self::github_light(),
            _ => Self::dark(),
        }
    }

}

/// (id, display_name, constructor)
pub type ThemePreset = (&'static str, &'static str, fn() -> Theme);

/// Predefined theme presets ordered: dark group then light group
pub const THEME_PRESETS: &[ThemePreset] = &[
    ("dark", "Default Dark", Theme::dark),
    ("dracula", "Dracula", Theme::dracula),
    ("nord", "Nord", Theme::nord),
    ("gruvbox_dark", "Gruvbox Dark", Theme::gruvbox_dark),
    ("tokyo_night", "Tokyo Night", Theme::tokyo_night),
    ("one_dark", "One Dark", Theme::one_dark),
    ("light", "Default Light", Theme::light),
    ("catppuccin_latte", "Catppuccin Latte", Theme::catppuccin_latte),
    ("rose_pine_dawn", "Rosé Pine Dawn", Theme::rose_pine_dawn),
    ("solarized_light", "Solarized Light", Theme::solarized_light),
    ("gruvbox_light", "Gruvbox Light", Theme::gruvbox_light),
    ("github_light", "GitHub Light", Theme::github_light),
];

/// Index of the first light theme in THEME_PRESETS
pub const LIGHT_START_INDEX: usize = 6;

impl Theme {
    /// Load theme from config: resolve preset then apply per-theme overrides
    pub fn from_config(config: &ThemeConfig) -> Self {
        let mut theme = Self::from_name(&config.theme);
        let overrides = match config.theme.as_str() {
            "light" => &config.light_overrides,
            _ => &config.dark_overrides,
        };
        theme.apply_overrides(overrides);
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
            dark_overrides: HashMap::new(),
            light_overrides: HashMap::new(),
        };
        let t = Theme::from_config(&config);
        assert_eq!(t.name, "light");
    }
}
