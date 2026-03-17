use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

const MAX_RECENT_FILES: usize = 10;
pub const DATETIME_FMT: &str = "%Y-%m-%dT%H:%M:%S";

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AlertKeyword {
    pub query: String,
    #[serde(default)]
    pub regex_mode: bool,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SessionState {
    /// Ordered source file paths; index 0 is the primary file
    pub sources: Vec<String>,
    pub scroll_offset: usize,
    pub selected_index: usize,
    /// Level filter toggles: [ERR, WRN, INF, DBG, TRC, PRF]
    pub level_filters: [bool; 6],
    /// ISO 8601 date-range bounds
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub exclude_patterns: Vec<AlertKeyword>,
    pub include_patterns: Vec<AlertKeyword>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub recent_files: Vec<String>,
    #[serde(default)]
    pub syntax_highlight: Option<bool>,
    /// Per-file bookmarks: file path -> list of entry indices
    #[serde(default)]
    pub bookmarks: HashMap<String, Vec<usize>>,
    /// Keywords that trigger a terminal bell when matched in live mode
    #[serde(default)]
    pub alert_keywords: Vec<AlertKeyword>,
    /// Theme preset name: "dark" (default), "light"
    #[serde(default = "default_theme_name")]
    pub theme: String,
    #[serde(default)]
    pub dark_overrides: HashMap<String, String>,
    #[serde(default)]
    pub light_overrides: HashMap<String, String>,
    /// Last session state, restored when no CLI file argument is given
    #[serde(default)]
    pub session: Option<SessionState>,
    /// Path to a network folder or local directory to check for updates
    #[serde(default)]
    pub update_path: Option<String>,
}

fn default_theme_name() -> String {
    "dark".to_string()
}

impl Config {
    /// Get the config file path
    fn config_path() -> Option<PathBuf> {
        ProjectDirs::from("", "", "lognav").map(|dirs| dirs.config_dir().join("config.json"))
    }

    /// Load config from disk
    pub fn load() -> Self {
        let Some(path) = Self::config_path() else {
            return Self::default();
        };

        if !path.exists() {
            return Self::default();
        }

        match fs::read_to_string(&path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// Save config to disk
    pub fn save(&self) -> Result<(), String> {
        let Some(path) = Self::config_path() else {
            return Err("Could not determine config directory".to_string());
        };

        // Create directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config dir: {}", e))?;
        }

        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize: {}", e))?;

        fs::write(&path, content).map_err(|e| format!("Failed to write config: {}", e))
    }

    /// Save bookmarks for a file (clears entry if empty)
    pub fn save_bookmarks(&mut self, path: &str, bookmarks: &std::collections::HashSet<usize>) {
        if bookmarks.is_empty() {
            self.bookmarks.remove(path);
        } else {
            let mut sorted: Vec<usize> = bookmarks.iter().copied().collect();
            sorted.sort_unstable();
            self.bookmarks.insert(path.to_string(), sorted);
        }
    }

    /// Load bookmarks for a file
    pub fn load_bookmarks(&self, path: &str) -> std::collections::HashSet<usize> {
        self.bookmarks
            .get(path)
            .map(|v| v.iter().copied().collect())
            .unwrap_or_default()
    }

    /// Capture current app state into session for next startup
    pub fn save_session(&mut self, app: &crate::app::App) {
        if app.sources.is_empty() {
            self.session = None;
            return;
        }
        self.session = Some(SessionState {
            sources: app.sources.iter().map(|s| s.path.clone()).collect(),
            scroll_offset: app.scroll_offset,
            selected_index: app.selected_index,
            level_filters: app.level_filters,
            date_from: app
                .date_from
                .map(|dt| dt.format(DATETIME_FMT).to_string()),
            date_to: app
                .date_to
                .map(|dt| dt.format(DATETIME_FMT).to_string()),
            exclude_patterns: app
                .exclude_patterns
                .iter()
                .map(|p| AlertKeyword {
                    query: p.query.clone(),
                    regex_mode: p.regex_mode,
                })
                .collect(),
            include_patterns: app
                .include_patterns
                .iter()
                .map(|p| AlertKeyword {
                    query: p.query.clone(),
                    regex_mode: p.regex_mode,
                })
                .collect(),
        });
    }

    /// Add a file to recent files list
    pub fn add_recent_file(&mut self, path: &str) {
        // Remove if already exists
        self.recent_files.retain(|p| p != path);

        // Add to front
        self.recent_files.insert(0, path.to_string());

        // Trim to max
        self.recent_files.truncate(MAX_RECENT_FILES);
    }
}
