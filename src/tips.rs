use std::hash::{DefaultHasher, Hasher};
use std::time::SystemTime;

/// Default tips bundled with the app
const DEFAULT_TIPS: &[&str] = &[
    "Press 'o' to open a log file, or 'M' to merge multiple files together",
    "Use Ctrl+P to open the command palette — fuzzy search all commands",
    "Press '/' to search with regex, then n/N to jump between matches",
    "Filter by level with 1-6 (Error, Warn, Info, Debug, Trace, Profile), 0 to reset",
    "Press Ctrl+D to filter by date range — supports relative times like -1h, -30m",
    "Press 'e' to jump to next error, 'E' for previous — same with 'w'/'W' for warnings",
    "Toggle bookmarks with 'm', then jump between them with 'b' and 'B'",
    "Ctrl+Click a word in the log to search for it, Alt+Click to exclude it",
    "Press 't' to toggle live mode — auto-follows new log entries in real time",
    "Press 'v' to start visual select, move to extend range, then 'c' to copy",
    "Press Alt+W to toggle word wrap for long lines",
    "Press 's' to toggle syntax highlighting for better readability",
    "Press 'x' to manage exclude filters — hide lines matching patterns",
    "Press Ctrl+S to export the currently filtered log entries to a file",
    "Use the up/down arrows in search to recall previous searches",
    "Press 'd' to view the full details of a selected log entry",
    "Press 'a' to expand or collapse all log entries at once",
    "Tab completion works in the file open dialog — press Tab to cycle matches",
    "Use the command palette to detect repeated log clusters and patterns",
    "Press Space on a cluster annotation to fold/unfold repeated entries",
    "You can drag and drop a file onto the window to open it",
    "Press F2 to open the statistics dashboard — see event rates and level distribution",
    "In the stats dashboard, use +/- to zoom, h/l or arrows to pan left/right, 0 to reset",
    "Press 'e' in stats view to export as an interactive HTML report you can open in a browser",
];

#[derive(Debug, Clone)]
pub struct TipsManager {
    all_tips: Vec<String>,
    current_tip_idx: usize,
}

impl TipsManager {
    /// Create a new tips manager with default tips, starting at a random tip
    pub fn new() -> Self {
        let all_tips: Vec<String> = DEFAULT_TIPS.iter().map(|s| s.to_string()).collect();
        let seed = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        let current_tip_idx = Self::pseudo_random(seed, all_tips.len());
        Self {
            all_tips,
            current_tip_idx,
        }
    }

    /// Get the current tip text
    pub fn get_current_tip(&self) -> &str {
        &self.all_tips[self.current_tip_idx]
    }

    /// Simple hash-based pseudo-random index
    fn pseudo_random(seed: u64, len: usize) -> usize {
        let mut hasher = DefaultHasher::new();
        hasher.write_u64(seed);
        (hasher.finish() as usize) % len
    }
}

impl Default for TipsManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tips_manager_creation() {
        let tips = TipsManager::new();
        let tip = tips.get_current_tip();
        assert!(!tip.is_empty());
    }
}
