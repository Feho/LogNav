use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

const MAX_RECENT_FILES: usize = 10;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub recent_files: Vec<String>,
    #[serde(default)]
    pub syntax_highlight: Option<bool>,
    /// Per-file bookmarks: file path -> list of entry indices
    #[serde(default)]
    pub bookmarks: HashMap<String, Vec<usize>>,
    /// Theme preset name: "dark" (default), "light"
    #[serde(default = "default_theme_name")]
    pub theme: String,
    #[serde(default)]
    pub dark_overrides: HashMap<String, String>,
    #[serde(default)]
    pub light_overrides: HashMap<String, String>,
    #[serde(default)]
    pub max_entries: Option<usize>,
}

fn default_theme_name() -> String {
    "dark".to_string()
}

impl Config {
    pub fn max_entries(&self) -> usize {
        self.max_entries.unwrap_or(5_000_000)
    }

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
