use crate::clusters::Cluster;
use crate::log_entry::LogEntry;
use crate::text_input::TextInput;
use crate::theme::Theme;
use crate::tips::TipsManager;
use chrono::NaiveDateTime;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use ratatui::style::Color;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::time::Instant;
use tokio::sync::mpsc;

/// A source file in a merged view
#[derive(Debug, Clone)]
pub struct SourceFile {
    pub path: String,
    #[allow(dead_code)]
    pub color: Color,
    pub label: String, // basename
}

impl SourceFile {
    pub fn new(path: &str, color: Color) -> Self {
        let label = std::path::Path::new(path)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string());
        Self {
            path: path.to_string(),
            color,
            label,
        }
    }
}

/// Unified search state (replaces separate search_regex/highlight_regex systems)
#[derive(Debug, Clone, Default)]
pub struct SearchState {
    pub query: String,
    pub regex_mode: bool,
    pub regex: Option<Regex>,
}

impl SearchState {
    /// Compile query into regex
    pub fn compile(&mut self) {
        if self.query.is_empty() {
            self.regex = None;
            return;
        }
        let pattern = if self.regex_mode {
            format!("(?i){}", self.query)
        } else {
            format!("(?i){}", regex::escape(&self.query))
        };
        self.regex = Regex::new(&pattern).ok();
    }

    pub fn clear(&mut self) {
        self.query.clear();
        self.regex = None;
    }
}

pub mod commands;
pub mod filtering;
pub mod navigation;

#[derive(Debug, Clone)]
pub enum FocusState {
    Normal,
    CommandPalette {
        input: TextInput,
        selected: usize,
    },
    Search {
        input: TextInput,
        match_indices: Vec<usize>,
        current_match: usize,
        regex_mode: bool,
        regex_error: Option<String>,
    },
    DateFilter {
        from: TextInput,
        to: TextInput,
        focus: DateFilterFocus,
        selected_quick: usize,
        error: Option<String>,
    },
    FileOpen {
        input: TextInput,
        selected_recent: usize,
        error: Option<String>,
        is_merge: bool,
        completions: Vec<String>,
        completion_index: Option<usize>,
    },
    Detail {
        scroll_offset: usize,
    },
    Help {
        scroll_offset: usize,
    },
    FilterManager {
        kind: FilterKind,
        input: TextInput,
        selected: usize,
        regex_mode: bool,
        regex_error: Option<String>,
        focus: FilterManagerFocus,
    },
    ExportDialog {
        input: TextInput,
        error: Option<String>,
    },
    Clusters {
        selected: usize,
        scroll_offset: usize,
    },
    ThemePicker {
        selected: usize,
        original_theme: Theme,
        original_name: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateFilterFocus {
    QuickFilter,
    From,
    To,
}

pub const QUICK_FILTERS: &[&str] = &[
    "Last hour",
    "Last 24 hours",
    "Today",
    "Yesterday",
    "Last 7 days",
    "Clear filter",
];

#[derive(Debug, Clone)]
pub struct HoverWord {
    pub row: usize,        // terminal row
    pub char_start: usize, // char offset in display text (after prefix)
    pub char_end: usize,   // exclusive end
}

/// A filter pattern used by both exclude and include filters
#[derive(Clone)]
pub struct FilterPattern {
    pub query: String,
    pub regex: Regex,
}

impl fmt::Debug for FilterPattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FilterPattern")
            .field("query", &self.query)
            .finish()
    }
}

/// Whether a filter manager is for exclude or include patterns
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterKind {
    Exclude,
    Include,
}

impl FilterKind {
    pub fn label(self) -> &'static str {
        match self {
            FilterKind::Exclude => "Exclude",
            FilterKind::Include => "Include",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterManagerFocus {
    Input,
    List,
}

/// Compact per-entry metadata for fast filter passes (parallel to `entries`)
#[derive(Clone, Copy)]
pub(crate) struct EntryMeta {
    pub level_bit: u8,     // 1 << LogLevel::filter_bit_index()
    pub timestamp_ms: i64, // milliseconds since epoch; i64::MIN = no timestamp
}

impl EntryMeta {
    pub fn from_entry(entry: &LogEntry) -> Self {
        Self {
            level_bit: 1u8 << entry.level.filter_bit_index(),
            timestamp_ms: entry
                .timestamp
                .map(|ts| ts.and_utc().timestamp_millis())
                .unwrap_or(i64::MIN),
        }
    }
}

pub struct App {
    // Log data
    pub entries: Vec<LogEntry>,
    pub(crate) entry_meta: Vec<EntryMeta>,
    pub filtered_indices: Vec<usize>,

    // Filter state
    pub level_filters: [bool; 6], // ERR, WRN, INF, DBG, TRC, PRF
    pub date_from: Option<NaiveDateTime>,
    pub date_to: Option<NaiveDateTime>,
    pub exclude_patterns: Vec<FilterPattern>,
    pub include_patterns: Vec<FilterPattern>,

    // UI state
    pub scroll_offset: usize,
    pub selected_index: usize,
    pub focus: FocusState,
    pub tail_enabled: bool,
    pub wrap_enabled: bool,
    pub syntax_highlight: bool,
    pub horizontal_scroll: usize,
    pub expanded_entries: HashSet<usize>, // Entry indices that are expanded
    pub bookmarks: HashSet<usize>,        // Entry indices that are bookmarked
    pub visual_anchor: Option<usize>,     // Anchor index for visual selection mode
    pub viewport_height: usize,           // Last known viewport height for mouse scroll
    pub viewport_width: usize,            // Last known viewport width for mouse scroll

    // File state
    pub file_path: String,
    pub recent_files: Vec<String>,

    // Multi-file merged view
    pub sources: Vec<SourceFile>,
    pub pending_merge_path: Option<String>,
    /// Per-source entry counts for assigning source_local_idx
    pub source_entry_counts: Vec<usize>,
    /// Stable bookmark IDs: (source_idx, source_local_idx)
    pub bookmark_stable_ids: HashSet<(u8, usize)>,

    // Loading state (streaming batch load)
    pub loading_sources: HashSet<u8>,
    pub loading_entry_count: usize,

    // Status
    pub status_message: Option<String>,
    pub should_quit: bool,

    // Fuzzy matcher for command palette
    fuzzy_matcher: SkimMatcherV2,

    // Clipboard for copying (kept alive to prevent drop issues)
    clipboard: Option<arboard::Clipboard>,

    // Debounce: when set, search needs recomputing after this instant
    pub search_dirty: Option<Instant>,

    // Search results panel (split-screen)
    pub search_panel_open: bool,
    pub search_panel_focused: bool,
    pub search_panel_matches: Vec<usize>, // positions within filtered_indices that match
    pub search_panel_selected: usize,
    pub search_panel_scroll: usize,
    pub search_panel_height: usize,
    pub search: SearchState,

    // Search history (most recent last)
    pub search_history: Vec<String>,
    pub search_history_index: Option<usize>, // None = typing new query, Some(i) = browsing history

    /// Ctrl+hover: word to underline
    pub hover_word: Option<HoverWord>,

    // Cluster detection results
    pub clusters: Vec<Cluster>,
    pub clusters_dirty: bool,
    pub cluster_tx: Option<mpsc::Sender<Vec<Cluster>>>,
    /// Per-entry cluster lookup: filtered_idx → (cluster_id, offset, group_len)
    pub cluster_map: HashMap<usize, (usize, usize, usize)>,
    /// Cluster IDs currently folded in main view
    pub folded_clusters: HashSet<usize>,
    /// Whether async cluster detection is in progress
    pub clusters_loading: bool,

    // Tips management
    pub tips_manager: TipsManager,

    // Theme
    pub theme: Theme,
    pub dark_overrides: HashMap<String, String>,
    pub light_overrides: HashMap<String, String>,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            entry_meta: Vec::new(),
            filtered_indices: Vec::new(),
            level_filters: [true, true, true, true, true, true], // All levels on by default
            date_from: None,
            date_to: None,
            exclude_patterns: Vec::new(),
            include_patterns: Vec::new(),
            scroll_offset: 0,
            selected_index: 0,
            focus: FocusState::Normal,
            tail_enabled: true,
            wrap_enabled: false,
            syntax_highlight: true,
            horizontal_scroll: 0,
            expanded_entries: HashSet::new(),
            bookmarks: HashSet::new(),
            visual_anchor: None,
            viewport_height: 25, // Default viewport height
            viewport_width: 80,  // Default viewport width
            file_path: String::new(),
            recent_files: Vec::new(),
            sources: Vec::new(),
            pending_merge_path: None,
            source_entry_counts: Vec::new(),
            bookmark_stable_ids: HashSet::new(),
            loading_sources: HashSet::new(),
            loading_entry_count: 0,
            status_message: None,
            should_quit: false,
            fuzzy_matcher: SkimMatcherV2::default(),
            clipboard: arboard::Clipboard::new().ok(),
            search_dirty: None,
            search_panel_open: false,
            search_panel_focused: false,
            search_panel_matches: Vec::new(),
            search_panel_selected: 0,
            search_panel_scroll: 0,
            search_panel_height: 0,
            search: SearchState::default(),
            search_history: Vec::new(),
            search_history_index: None,
            hover_word: None,
            clusters: Vec::new(),
            clusters_dirty: true,
            cluster_tx: None,
            cluster_map: HashMap::new(),
            folded_clusters: HashSet::new(),
            clusters_loading: false,
            tips_manager: TipsManager::new(),
            theme: Theme::dark(),
            dark_overrides: HashMap::new(),
            light_overrides: HashMap::new(),
        }
    }

    pub fn is_loading(&self) -> bool {
        !self.loading_sources.is_empty()
    }

    /// Add entries from initial load
    pub fn set_entries(&mut self, entries: Vec<LogEntry>) {
        self.entries = entries;
        self.entry_meta = self.entries.iter().map(EntryMeta::from_entry).collect();
        self.apply_filters();
        if self.tail_enabled {
            self.scroll_to_bottom();
        }
    }

    /// Add new entries from tailing - uses incremental filtering
    pub fn append_entries(&mut self, mut new_entries: Vec<LogEntry>) {
        if new_entries.is_empty() {
            return;
        }

        let was_at_bottom = self.is_bottom_visible();

        // Re-index new entries
        let start_idx = self.entries.len();
        for (i, entry) in new_entries.iter_mut().enumerate() {
            entry.index = start_idx + i;
        }

        self.entries.append(&mut new_entries);

        // Build metadata for new entries
        for entry in &self.entries[start_idx..] {
            self.entry_meta.push(EntryMeta::from_entry(entry));
        }

        // Incremental filter - only process new entries
        self.apply_filters_incremental(start_idx);

        if self.tail_enabled
            && matches!(self.focus, FocusState::Normal)
            && was_at_bottom
        {
            self.visual_anchor = None;
            self.scroll_to_bottom();
        }
    }

    /// Get filtered commands based on fuzzy search
    pub fn get_filtered_commands(&self, query: &str) -> Vec<(usize, &commands::Command, i64)> {
        if query.is_empty() {
            return commands::Command::ALL
                .iter()
                .enumerate()
                .map(|(i, c)| (i, c, 0))
                .collect();
        }

        let mut results: Vec<_> = commands::Command::ALL
            .iter()
            .enumerate()
            .filter_map(|(i, cmd)| {
                self.fuzzy_matcher
                    .fuzzy_match(cmd.name, query)
                    .map(|score| (i, cmd, score))
            })
            .collect();

        results.sort_by(|a, b| b.2.cmp(&a.2));
        results
    }

    /// Open command palette
    pub fn open_command_palette(&mut self) {
        self.focus = FocusState::CommandPalette {
            input: TextInput::new(),
            selected: 0,
        };
    }

    /// Open search overlay
    pub fn open_search(&mut self) {
        let query = self.search.query.clone();
        let regex_mode = self.search.regex_mode;

        let match_indices = if let Some(regex) = self.search.regex.as_ref() {
            self.filtered_indices
                .iter()
                .enumerate()
                .filter(|&(_, &entry_idx)| {
                    regex.is_match(self.entries[entry_idx].searchable_text())
                })
                .map(|(pos, _)| pos)
                .collect()
        } else {
            Vec::new()
        };

        self.search_history_index = None;
        self.focus = FocusState::Search {
            input: TextInput::with_text(query),
            match_indices,
            current_match: 0,
            regex_mode,
            regex_error: None,
        };
    }

    /// Open date filter dialog
    pub fn open_date_filter(&mut self) {
        self.focus = FocusState::DateFilter {
            from: TextInput::with_text(
                self.date_from
                    .map(|d| d.format("%m-%d %H:%M").to_string())
                    .unwrap_or_default(),
            ),
            to: TextInput::with_text(
                self.date_to
                    .map(|d| d.format("%m-%d %H:%M").to_string())
                    .unwrap_or_default(),
            ),
            focus: DateFilterFocus::QuickFilter,
            selected_quick: 0,
            error: None,
        };
    }

    /// Open file open dialog
    pub fn open_file_dialog(&mut self) {
        self.focus = FocusState::FileOpen {
            input: TextInput::new(),
            selected_recent: 0,
            error: None,
            is_merge: false,
            completions: Vec::new(),
            completion_index: None,
        };
    }

    /// Open file dialog in merge mode (add file to merged view)
    pub fn open_merge_file_dialog(&mut self) {
        self.focus = FocusState::FileOpen {
            input: TextInput::new(),
            selected_recent: 0,
            error: None,
            is_merge: true,
            completions: Vec::new(),
            completion_index: None,
        };
    }

    /// Open detail popup for selected entry
    pub fn open_detail_popup(&mut self) {
        if let Some(&entry_idx) = self.filtered_indices.get(self.selected_index) {
            self.entries[entry_idx].ensure_pretty_continuation();
        }
        self.focus = FocusState::Detail { scroll_offset: 0 };
    }

    /// Open help dialog
    pub fn open_help(&mut self) {
        self.focus = FocusState::Help { scroll_offset: 0 };
    }

    /// Open theme picker overlay
    pub fn open_theme_picker(&mut self) {
        use crate::theme::THEME_PRESETS;
        let current_name = self.theme.name.clone();
        let selected = THEME_PRESETS
            .iter()
            .position(|(id, _, _)| *id == current_name)
            .unwrap_or(0);
        self.focus = FocusState::ThemePicker {
            selected,
            original_theme: self.theme.clone(),
            original_name: current_name,
        };
    }

    /// Run cluster detection on filtered entries and open overlay.
    /// Uses cached results if filters haven't changed; spawns async task otherwise.
    pub fn open_clusters(&mut self) {
        if !self.clusters_dirty && !self.clusters.is_empty() {
            // Use cached results
            self.status_message =
                Some(format!("{} cluster(s) found (cached)", self.clusters.len()));
            self.focus = FocusState::Clusters {
                selected: 0,
                scroll_offset: 0,
            };
            return;
        }

        // Build lightweight snapshot: only raw_line + message_offset
        let snapshots: Vec<(String, Option<usize>)> = self
            .filtered_indices
            .iter()
            .map(|&idx| {
                let e = &self.entries[idx];
                (e.raw_line.clone(), e.message_offset)
            })
            .collect();

        let Some(tx) = self.cluster_tx.clone() else {
            // Fallback: synchronous detection (no channel wired)
            self.clusters = crate::clusters::detect_clusters(&snapshots, 3);
            self.clusters_dirty = false;
            self.build_cluster_map();
            if self.clusters.is_empty() {
                self.status_message = Some("No clusters detected".to_string());
                return;
            }
            self.status_message = Some(format!("{} cluster(s) found", self.clusters.len()));
            self.focus = FocusState::Clusters {
                selected: 0,
                scroll_offset: 0,
            };
            return;
        };

        self.clusters_loading = true;
        self.status_message = Some("Detecting clusters...".to_string());
        self.focus = FocusState::Clusters {
            selected: 0,
            scroll_offset: 0,
        };

        tokio::spawn(async move {
            let result = crate::clusters::detect_clusters(&snapshots, 3);
            let _ = tx.send(result).await;
        });
    }

    /// Handle async cluster detection result
    pub fn receive_clusters(&mut self, clusters: Vec<Cluster>) {
        self.clusters = clusters;
        self.clusters_dirty = false;
        self.clusters_loading = false;
        self.build_cluster_map();
        if self.clusters.is_empty() {
            self.status_message = Some("No clusters detected".to_string());
            return;
        }
        self.status_message = Some(format!("{} cluster(s) found", self.clusters.len()));
        self.focus = FocusState::Clusters {
            selected: 0,
            scroll_offset: 0,
        };
    }

    /// Build per-entry cluster lookup map from cluster occurrences.
    /// For single-line clusters (sequence_len==1), consecutive occurrences are
    /// grouped visually with sequential offsets so the gutter renders correctly.
    fn build_cluster_map(&mut self) {
        self.cluster_map.clear();
        self.folded_clusters.clear();
        for (cluster_id, cluster) in self.clusters.iter().enumerate() {
            if cluster.sequence_len == 1 && cluster.occurrences.len() > 1 {
                // Group consecutive single-entry occurrences into visual runs.
                // Two passes: first collect runs, then assign offsets with group_len.
                let mut sorted_starts: Vec<usize> =
                    cluster.occurrences.iter().map(|&(s, _)| s).collect();
                sorted_starts.sort_unstable();

                // Split into visual runs separated by large gaps
                let mut runs: Vec<Vec<usize>> = Vec::new();
                let mut current_run: Vec<usize> = Vec::new();
                for &idx in &sorted_starts {
                    if let Some(&last) = current_run.last()
                        && idx - last > crate::clusters::MAX_SINGLE_GAP + 1
                    {
                        runs.push(std::mem::take(&mut current_run));
                    }
                    current_run.push(idx);
                }
                if !current_run.is_empty() {
                    runs.push(current_run);
                }

                for run in &runs {
                    let group_len = run.len();
                    for (offset, &idx) in run.iter().enumerate() {
                        self.cluster_map
                            .entry(idx)
                            .or_insert((cluster_id, offset, group_len));
                    }
                }
            } else {
                for &(start, len) in &cluster.occurrences {
                    for offset in 0..len {
                        self.cluster_map
                            .entry(start + offset)
                            .or_insert((cluster_id, offset, len));
                    }
                }
            }
        }
    }

    /// Toggle fold for a cluster by ID
    pub fn toggle_fold_cluster(&mut self, cluster_id: usize) {
        if self.folded_clusters.contains(&cluster_id) {
            self.folded_clusters.remove(&cluster_id);
        } else {
            self.folded_clusters.insert(cluster_id);
            // Snap cursor to occurrence start if it would be hidden
            if let Some(&(cid, off, _)) = self.cluster_map.get(&self.selected_index)
                && cid == cluster_id
                && off > 0
            {
                self.selected_index -= off;
            }
        }
    }

    /// Open filter manager overlay for given kind
    pub fn open_filter_manager(&mut self, kind: FilterKind) {
        self.focus = FocusState::FilterManager {
            kind,
            input: TextInput::new(),
            selected: 0,
            regex_mode: false,
            regex_error: None,
            focus: FilterManagerFocus::Input,
        };
    }

    /// Open export dialog with default path
    pub fn open_export_dialog(&mut self) {
        let default_path = if self.file_path.is_empty() {
            "filtered.log".to_string()
        } else {
            let p = std::path::Path::new(&self.file_path);
            let stem = p
                .file_stem()
                .map(|s| s.to_string_lossy())
                .unwrap_or("output".into());
            let ext = p
                .extension()
                .map(|s| s.to_string_lossy())
                .unwrap_or("log".into());
            let dir = p
                .parent()
                .map(|d| d.to_string_lossy())
                .unwrap_or(".".into());
            format!("{}/{}_filtered.{}", dir, stem, ext)
        };
        self.focus = FocusState::ExportDialog {
            input: TextInput::with_text(default_path),
            error: None,
        };
    }

    /// Export filtered entries to a file
    pub fn export_filtered(&mut self, path: &str) -> Result<usize, String> {
        use std::io::Write;
        let expanded = if path == "~" {
            std::env::var("HOME").unwrap_or_else(|_| path.to_string())
        } else if let Some(rest) = path.strip_prefix("~/") {
            match std::env::var("HOME") {
                Ok(home) => format!("{}/{}", home, rest),
                Err(_) => path.to_string(),
            }
        } else {
            path.to_string()
        };

        let file = std::fs::File::create(&expanded).map_err(|e| e.to_string())?;
        let mut writer = std::io::BufWriter::new(file);
        let mut count = 0;
        for &idx in &self.filtered_indices {
            let entry = &self.entries[idx];
            writeln!(writer, "{}", entry.raw_line).map_err(|e| e.to_string())?;
            for line in &entry.continuation_lines {
                writeln!(writer, "{}", line).map_err(|e| e.to_string())?;
            }
            count += 1;
        }
        writer.flush().map_err(|e| e.to_string())?;
        Ok(count)
    }

    /// Close any overlay and return to normal
    pub fn close_overlay(&mut self) {
        self.focus = FocusState::Normal;
    }

    /// Close the search results panel and clear highlight regex.
    /// Keeps search.query/regex_mode so n/N can redo last search.
    pub fn close_search_panel(&mut self) {
        self.search_panel_open = false;
        self.search_panel_focused = false;
        self.search_panel_matches.clear();
        self.search_panel_selected = 0;
        self.search_panel_scroll = 0;
        self.search.regex = None;
    }

    /// Get mutable reference to filter patterns by kind
    pub fn filter_patterns_mut(&mut self, kind: FilterKind) -> &mut Vec<FilterPattern> {
        match kind {
            FilterKind::Exclude => &mut self.exclude_patterns,
            FilterKind::Include => &mut self.include_patterns,
        }
    }

    /// Get reference to filter patterns by kind
    pub fn filter_patterns(&self, kind: FilterKind) -> &[FilterPattern] {
        match kind {
            FilterKind::Exclude => &self.exclude_patterns,
            FilterKind::Include => &self.include_patterns,
        }
    }

    /// Add a filter pattern. Returns error string on invalid regex.
    pub fn add_filter(
        &mut self,
        kind: FilterKind,
        query: &str,
        regex_mode: bool,
    ) -> Option<String> {
        if query.is_empty() {
            return None;
        }
        let pattern = if regex_mode {
            format!("(?i){}", query)
        } else {
            format!("(?i){}", regex::escape(query))
        };
        match Regex::new(&pattern) {
            Ok(regex) => {
                self.filter_patterns_mut(kind).push(FilterPattern {
                    query: query.to_string(),
                    regex,
                });
                self.apply_filters();
                None
            }
            Err(e) => Some(e.to_string()),
        }
    }

    /// Remove a single filter by index
    pub fn remove_filter(&mut self, kind: FilterKind, index: usize) {
        let patterns = self.filter_patterns_mut(kind);
        if index < patterns.len() {
            patterns.remove(index);
            self.apply_filters();
        }
    }

    /// Clear all filters of a given kind
    pub fn clear_filters(&mut self, kind: FilterKind) {
        let patterns = self.filter_patterns_mut(kind);
        if !patterns.is_empty() {
            patterns.clear();
            self.apply_filters();
        }
    }

    /// Toggle tail mode
    pub fn toggle_tail(&mut self) {
        self.tail_enabled = !self.tail_enabled;
        if self.tail_enabled {
            self.scroll_to_bottom();
        }
    }

    /// Toggle word wrap
    pub fn toggle_wrap(&mut self) {
        self.wrap_enabled = !self.wrap_enabled;
    }

    /// Toggle syntax highlighting
    pub fn toggle_syntax_highlight(&mut self) {
        self.syntax_highlight = !self.syntax_highlight;
    }

    /// Toggle expand/collapse of selected entry's continuation lines
    pub fn toggle_expand(&mut self) {
        if let Some(&entry_idx) = self.filtered_indices.get(self.selected_index) {
            // Only toggle if entry has continuation lines
            if !self.entries[entry_idx].continuation_lines.is_empty() {
                if self.expanded_entries.contains(&entry_idx) {
                    self.expanded_entries.remove(&entry_idx);
                } else {
                    self.entries[entry_idx].ensure_pretty_continuation();
                    self.expanded_entries.insert(entry_idx);
                }
            }
        }
    }

    /// Expand all entries that have continuation lines; collapse all if already fully expanded
    pub fn toggle_expand_all(&mut self) {
        let expandable: Vec<usize> = self
            .filtered_indices
            .iter()
            .copied()
            .filter(|&idx| !self.entries[idx].continuation_lines.is_empty())
            .collect();

        let all_expanded = expandable
            .iter()
            .all(|idx| self.expanded_entries.contains(idx));
        if all_expanded {
            self.expanded_entries.clear();
        } else {
            for &idx in &expandable {
                self.entries[idx].ensure_pretty_continuation();
            }
            self.expanded_entries.extend(expandable);
        }
    }

    /// Check if an entry is expanded
    pub fn is_expanded(&self, entry_idx: usize) -> bool {
        self.expanded_entries.contains(&entry_idx)
    }

    /// Auto-expand selected entry if search match is hidden in continuation lines
    pub fn auto_expand_for_search(&mut self) {
        let regex = match self.search.regex.as_ref() {
            Some(r) => r.clone(),
            None => return,
        };
        let entry_idx = match self.filtered_indices.get(self.selected_index) {
            Some(&idx) => idx,
            None => return,
        };
        let entry = &self.entries[entry_idx];
        // Only act on entries with continuation lines that aren't already expanded
        if entry.continuation_lines.is_empty() || self.expanded_entries.contains(&entry_idx) {
            return;
        }
        // If the match is NOT on the main line, it must be in continuation lines
        if !regex.is_match(&entry.raw_line) {
            self.entries[entry_idx].ensure_pretty_continuation();
            self.expanded_entries.insert(entry_idx);
        }
    }

    /// Whether we're in merged multi-file view
    pub fn is_merged(&self) -> bool {
        self.sources.len() > 1
    }

    /// Compute the fixed gutter width for cluster annotations.
    /// Returns 0 when no clusters are active.
    pub fn cluster_gutter_width(&self) -> usize {
        if self.cluster_map.is_empty() {
            return 0;
        }
        // "│" (1) + max digits of count + "×" (1)
        let max_count = self.clusters.iter().map(|c| c.count).max().unwrap_or(1);
        let digits = max_count.max(1).ilog10() as usize + 1;
        1 + digits + 1
    }

    /// Total prefix width used when rendering log lines (timestamp/level + gutters).
    /// Must match the rendering logic in log_view.rs.
    pub fn full_prefix_width(&self) -> usize {
        use crate::ui::LINE_PREFIX_WIDTH;
        let source_gutter = if self.is_merged() { 1 } else { 0 };
        LINE_PREFIX_WIDTH + source_gutter + self.cluster_gutter_width()
    }

    /// Set the primary source file (single-file mode)
    pub fn set_primary_source(&mut self, path: &str) {
        self.sources.clear();
        self.sources
            .push(SourceFile::new(path, self.theme.source_color(0)));
        self.source_entry_counts = vec![0];
        self.file_path = path.to_string();
    }

    /// Remove all sources and clear entries (used when opening a new file while merged)
    pub fn remove_all_sources(&mut self) {
        self.sources.clear();
        self.source_entry_counts.clear();
        self.entries.clear();
        self.entry_meta.clear();
        self.filtered_indices.clear();
        self.expanded_entries.clear();
        self.bookmarks.clear();
        self.bookmark_stable_ids.clear();
        self.selected_index = 0;
        self.scroll_offset = 0;
    }

    /// Merge entries from a source into the main entries vec (sorted by timestamp)
    pub fn merge_entries_from_source(&mut self, source_idx: u8, mut new_entries: Vec<LogEntry>) {
        if new_entries.is_empty() {
            return;
        }

        // Single source fast path: use existing set_entries/append_entries
        if self.sources.len() <= 1 && source_idx == 0 {
            if self.entries.is_empty() {
                // Tag source_local_idx
                let count = &mut self.source_entry_counts;
                if count.is_empty() {
                    count.push(0);
                }
                for entry in &mut new_entries {
                    entry.source_local_idx = count[0];
                    count[0] += 1;
                }
                self.set_entries(new_entries);
            } else {
                let count = &mut self.source_entry_counts;
                if count.is_empty() {
                    count.push(self.entries.len());
                }
                for entry in &mut new_entries {
                    entry.source_local_idx = count[0];
                    count[0] += 1;
                }
                self.append_entries(new_entries);
            }
            return;
        }

        // Multi-source merge: tag entries and merge by timestamp
        let si = source_idx as usize;
        while self.source_entry_counts.len() <= si {
            self.source_entry_counts.push(0);
        }
        for entry in &mut new_entries {
            entry.source_idx = source_idx;
            entry.source_local_idx = self.source_entry_counts[si];
            self.source_entry_counts[si] += 1;
        }

        // Two-pointer merge: both vecs are already sorted by timestamp.
        // O(n+m) instead of O(n*m) from repeated Vec::insert.
        let existing = std::mem::take(&mut self.entries);
        let total = existing.len() + new_entries.len();
        let mut merged = Vec::with_capacity(total);
        let mut e_iter = existing.into_iter().peekable();
        let mut n_iter = new_entries.into_iter().peekable();
        loop {
            let take_existing = match (e_iter.peek(), n_iter.peek()) {
                (Some(e), Some(n)) => match (e.timestamp, n.timestamp) {
                    (Some(et), Some(nt)) => et <= nt,
                    (Some(_), None) => true,
                    (None, Some(_)) => false,
                    (None, None) => true,
                },
                (Some(_), None) => true,
                (None, Some(_)) => false,
                (None, None) => break,
            };
            if take_existing {
                merged.push(e_iter.next().unwrap());
            } else {
                merged.push(n_iter.next().unwrap());
            }
        }
        self.entries = merged;
        self.entry_meta = self.entries.iter().map(EntryMeta::from_entry).collect();

        // Re-index all entries
        for (i, entry) in self.entries.iter_mut().enumerate() {
            entry.index = i;
        }

        // Rebuild bookmarks from stable IDs
        self.rebuild_bookmarks_from_stable();

        let was_at_bottom = self.is_bottom_visible();
        self.apply_filters();
        if self.tail_enabled && was_at_bottom {
            self.scroll_to_bottom();
        }
    }

    /// Reset entries from a specific source (file truncated/rotated)
    pub fn reset_source(&mut self, source_idx: u8) {
        self.entries.retain(|e| e.source_idx != source_idx);
        self.entry_meta = self.entries.iter().map(EntryMeta::from_entry).collect();
        let si = source_idx as usize;
        if si < self.source_entry_counts.len() {
            self.source_entry_counts[si] = 0;
        }
        // Re-index
        for (i, entry) in self.entries.iter_mut().enumerate() {
            entry.index = i;
        }
        self.rebuild_bookmarks_from_stable();
        self.apply_filters();
    }

    /// Rebuild bookmarks HashSet from stable IDs by scanning entries
    pub fn rebuild_bookmarks_from_stable(&mut self) {
        self.bookmarks.clear();
        for entry in &self.entries {
            if self
                .bookmark_stable_ids
                .contains(&(entry.source_idx, entry.source_local_idx))
            {
                self.bookmarks.insert(entry.index);
            }
        }
    }

    /// Execute a command action
    pub fn execute_command(&mut self, action: commands::CommandAction) {
        use commands::CommandAction;
        match action {
            CommandAction::OpenFile => self.open_file_dialog(),
            CommandAction::Search => self.open_search(),
            CommandAction::DateFilter => self.open_date_filter(),
            CommandAction::ToggleError => self.toggle_level(0),
            CommandAction::ToggleWarn => self.toggle_level(1),
            CommandAction::ToggleInfo => self.toggle_level(2),
            CommandAction::ToggleDebug => self.toggle_level(3),
            CommandAction::ToggleTrace => self.toggle_level(4),
            CommandAction::ToggleProfile => self.toggle_level(5),
            CommandAction::ToggleTail => self.toggle_tail(),
            CommandAction::ToggleWrap => self.toggle_wrap(),
            CommandAction::ToggleSyntax => self.toggle_syntax_highlight(),
            CommandAction::GoToTop => self.scroll_to_top(),
            CommandAction::GoToBottom => self.scroll_to_bottom(),
            CommandAction::NextError => self.next_error(),
            CommandAction::PrevError => self.prev_error(),
            CommandAction::NextWarning => self.next_warning(),
            CommandAction::PrevWarning => self.prev_warning(),
            CommandAction::ToggleBookmark => self.toggle_bookmark(),
            CommandAction::NextBookmark => self.next_bookmark(),
            CommandAction::PrevBookmark => self.prev_bookmark(),
            CommandAction::ClearBookmarks => self.clear_bookmarks(),
            CommandAction::ClearExcludes => {
                let count = self.exclude_patterns.len();
                self.clear_filters(FilterKind::Exclude);
                self.status_message = Some(format!("Cleared {} exclude filter(s)", count));
            }
            CommandAction::VisualSelect => {
                self.visual_anchor = Some(self.selected_index);
            }
            CommandAction::ExcludeManager => self.open_filter_manager(FilterKind::Exclude),
            CommandAction::IncludeManager => self.open_filter_manager(FilterKind::Include),
            CommandAction::ClearIncludes => {
                let count = self.include_patterns.len();
                self.clear_filters(FilterKind::Include);
                self.status_message = Some(format!("Cleared {} include filter(s)", count));
            }
            CommandAction::MergeFile => self.open_merge_file_dialog(),
            CommandAction::ExportFiltered => self.open_export_dialog(),
            CommandAction::Clusters => self.open_clusters(),
            CommandAction::ThemePicker => self.open_theme_picker(),
            CommandAction::Quit => self.should_quit = true,
        }
    }

    /// Get active date filter display for status bar
    pub fn date_filter_display(&self) -> Option<String> {
        match (self.date_from, self.date_to) {
            (Some(from), Some(to)) => Some(format!(
                "{} -> {}",
                from.format("%m-%d %H:%M"),
                to.format("%m-%d %H:%M")
            )),
            (Some(from), None) => Some(format!("From {}", from.format("%m-%d %H:%M"))),
            (None, Some(to)) => Some(format!("To {}", to.format("%m-%d %H:%M"))),
            (None, None) => None,
        }
    }

    /// Get active level filter names for status bar
    pub fn active_levels_display(&self) -> String {
        let levels = ["ERR", "WRN", "INF", "DBG", "TRC", "PRF"];
        levels
            .iter()
            .enumerate()
            .filter(|(i, _)| self.level_filters[*i])
            .map(|(_, l)| *l)
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Copy the current entry to clipboard (includes continuation lines)
    /// Returns (min, max) range in filtered_indices space if visual mode is active
    pub fn visual_range(&self) -> Option<(usize, usize)> {
        self.visual_anchor.map(|anchor| {
            let a = anchor.min(self.selected_index);
            let b = anchor.max(self.selected_index);
            (a, b)
        })
    }

    /// Copy all entries in the visual selection range
    pub fn copy_selection(&mut self) {
        let Some((min, max)) = self.visual_range() else {
            return;
        };

        let mut parts = Vec::new();
        for pos in min..=max {
            if let Some(&entry_idx) = self.filtered_indices.get(pos)
                && let Some(entry) = self.entries.get(entry_idx)
            {
                parts.push(if entry.continuation_lines.is_empty() {
                    entry.raw_line.clone()
                } else {
                    let mut text = entry.raw_line.clone();
                    for line in &entry.continuation_lines {
                        text.push('\n');
                        text.push_str(line);
                    }
                    text
                });
            }
        }

        let text = parts.join("\n");
        let count = parts.len();

        let result = if let Some(ref mut clipboard) = self.clipboard {
            clipboard.set_text(&text)
        } else {
            match arboard::Clipboard::new() {
                Ok(mut clipboard) => {
                    let result = clipboard.set_text(&text);
                    self.clipboard = Some(clipboard);
                    result
                }
                Err(_) => {
                    self.status_message = Some("Clipboard unavailable".to_string());
                    return;
                }
            }
        };

        self.visual_anchor = None;
        if result.is_ok() {
            self.status_message = Some(format!("Copied {} lines!", count));
        } else {
            self.status_message = Some("Failed to copy".to_string());
        }
    }

    pub fn copy_current_line(&mut self) {
        if self.visual_anchor.is_some() {
            self.copy_selection();
            return;
        }
        // Build full text with continuation lines
        let text_to_copy = self.selected_entry().map(|e| {
            if e.continuation_lines.is_empty() {
                e.raw_line.clone()
            } else {
                let mut text = e.raw_line.clone();
                for line in &e.continuation_lines {
                    text.push('\n');
                    text.push_str(line);
                }
                text
            }
        });

        if let Some(text) = text_to_copy {
            // Try to use existing clipboard or create new one if needed
            let result = if let Some(ref mut clipboard) = self.clipboard {
                clipboard.set_text(&text)
            } else {
                // Try to create clipboard if it doesn't exist
                match arboard::Clipboard::new() {
                    Ok(mut clipboard) => {
                        let result = clipboard.set_text(&text);
                        self.clipboard = Some(clipboard);
                        result
                    }
                    Err(_) => {
                        self.status_message = Some("Clipboard unavailable".to_string());
                        return;
                    }
                }
            };

            if result.is_ok() {
                self.status_message = Some("Copied!".to_string());
            } else {
                self.status_message = Some("Failed to copy".to_string());
            }
        } else {
            self.status_message = Some("No line selected".to_string());
        }
    }
}
