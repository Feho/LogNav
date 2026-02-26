use std::collections::HashSet;
use std::hash::{DefaultHasher, Hasher};
use std::time::SystemTime;

/// Default tips bundled with the app
const DEFAULT_TIPS: &[&str] = &[
    "Press 'o' to open a log file, or 'M' to merge multiple files together",
    "Use Ctrl+P to open the command palette — fuzzy search all commands",
    "Press '/' to search with regex, then n/N to jump between matches",
    "Filter by level with 1-5 (Error, Warn, Info, Debug, Trace), 0 to reset",
    "Press Ctrl+D to filter by date range — supports relative times like -1h, -30m",
    "Press 'e' to jump to next error, 'E' for previous — same with 'w'/'W' for warnings",
    "Toggle bookmarks with 'm', then jump between them with 'b' and 'B'",
    "Ctrl+Click a word in the log to search for it, Alt+Click to exclude it",
    "Press 't' to toggle tail mode — auto-follows new log entries in real time",
    "Press 'v' to start visual select, move to extend range, then 'c' to copy",
    "Press Ctrl+W to toggle word wrap for long lines",
    "Press 's' to toggle syntax highlighting for better readability",
    "Press 'x' to manage exclude filters — hide lines matching patterns",
    "Press Ctrl+S to export the currently filtered log entries to a file",
    "Use the up/down arrows in search to recall previous searches",
    "Press 'd' to view the full details of a selected log entry",
    "Press 'a' to expand or collapse all log entries at once",
    "Tab completion works in the file open dialog — press Tab to cycle matches",
    "Use the command palette to detect repeated log clusters and patterns",
    "Press Space on a cluster annotation to fold/unfold repeated entries",
];

#[derive(Debug, Clone)]
pub struct TipsManager {
    all_tips: Vec<String>,
    seen_tips: HashSet<usize>,
    current_tip_idx: usize,
    counter: u64,
}

impl TipsManager {
    /// Create a new tips manager with default tips
    pub fn new() -> Self {
        let all_tips: Vec<String> = DEFAULT_TIPS.iter().map(|s| s.to_string()).collect();
        let seed = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        let initial_idx = Self::pseudo_random(seed, all_tips.len());
        let mut seen_tips = HashSet::new();
        seen_tips.insert(initial_idx);
        Self {
            all_tips,
            seen_tips,
            current_tip_idx: initial_idx,
            counter: seed,
        }
    }

    /// Get the current tip text
    pub fn get_current_tip(&self) -> &str {
        &self.all_tips[self.current_tip_idx]
    }

    /// Advance to the next random tip (avoiding repeats within cycle)
    pub fn next_tip(&mut self) {
        if self.seen_tips.len() >= self.all_tips.len() {
            self.seen_tips.clear();
        }

        let available: Vec<usize> = (0..self.all_tips.len())
            .filter(|i| !self.seen_tips.contains(i))
            .collect();

        self.counter = self.counter.wrapping_add(1);
        let pick = Self::pseudo_random(self.counter, available.len());
        let idx = available[pick];
        self.seen_tips.insert(idx);
        self.current_tip_idx = idx;
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

    #[test]
    fn test_no_repeat_tips_in_cycle() {
        let mut tips = TipsManager::new();
        let mut seen = HashSet::new();
        seen.insert(tips.get_current_tip().to_string());

        for _ in 1..DEFAULT_TIPS.len() {
            tips.next_tip();
            let tip = tips.get_current_tip().to_string();
            assert!(
                seen.insert(tip.clone()),
                "Tip repeated within a cycle: {}",
                tip
            );
        }
    }

    #[test]
    fn test_cycle_resets_after_all_seen() {
        let mut tips = TipsManager::new();
        // Exhaust all tips
        for _ in 1..DEFAULT_TIPS.len() {
            tips.next_tip();
        }
        assert_eq!(tips.seen_tips.len(), DEFAULT_TIPS.len());
        // Next tip should reset and succeed
        tips.next_tip();
        assert!(tips.seen_tips.len() <= 2); // cleared then picked one
    }
}
