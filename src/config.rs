use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const MAX_RECENT_FILES: usize = 10;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub recent_files: Vec<String>,
}

impl Config {
    /// Get the config file path
    fn config_path() -> Option<PathBuf> {
        ProjectDirs::from("", "", "logviewer").map(|dirs| dirs.config_dir().join("config.json"))
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
