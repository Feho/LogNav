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

/// What kind of export the ExportDialog is performing.
#[derive(Debug, Clone)]
pub enum ExportKind {
    /// Normal filtered-log export (Ctrl+S).
    FilteredLog,
    /// Stats dashboard as self-contained HTML with Chart.js.
    StatsHtml(Box<StatsData>),
}

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
        kind: ExportKind,
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
    Stats {
        data: Box<StatsData>,
        /// Index into ZOOM_LEVELS for current zoom
        zoom_idx: usize,
        /// Pan offset in raw (1-min) buckets from the left
        pan_offset: usize,
    },
}

/// Per-bucket level breakdown for the stacked event rate chart.
#[derive(Debug, Clone, Copy, Default)]
pub struct BucketCounts {
    pub error: u64,
    pub warn: u64,
    pub other: u64,
}

impl BucketCounts {
    pub fn total(&self) -> u64 {
        self.error + self.warn + self.other
    }
}

/// Zoom level definitions: (bucket_ms, label).
/// Each level merges raw 1-min buckets into coarser display buckets.
pub const ZOOM_LEVELS: &[(i64, &str)] = &[
    (60_000, "1 min"),
    (5 * 60_000, "5 min"),
    (15 * 60_000, "15 min"),
    (30 * 60_000, "30 min"),
    (3_600_000, "1 hr"),
    (6 * 3_600_000, "6 hr"),
    (86_400_000, "1 day"),
];

/// Pick default zoom index based on time span.
pub fn default_zoom_idx(span_ms: i64) -> usize {
    if span_ms <= 60 * 60_000 {
        0 // 1 min
    } else if span_ms <= 6 * 3_600_000 {
        1 // 5 min
    } else if span_ms <= 24 * 3_600_000 {
        2 // 15 min
    } else if span_ms <= 3 * 86_400_000 {
        4 // 1 hr
    } else {
        6 // 1 day
    }
}

/// Pre-computed statistics for the stats dashboard overlay.
#[derive(Debug, Clone)]
pub struct StatsData {
    pub total_entries: usize,
    pub filtered_count: usize,
    /// Per-level count indexed by LogLevel::filter_bit_index() (0=ERR…5=PRF, 6=Unknown)
    pub level_counts: [u64; 7],
    /// (min_ms, max_ms) over filtered entries that have timestamps
    pub time_range: Option<(i64, i64)>,
    /// Error rate as percentage; None if no entries
    pub error_rate: Option<f64>,
    /// Raw 1-minute buckets (oldest to newest), starting from bucket_base_ms
    pub buckets: Vec<BucketCounts>,
    /// Epoch ms of raw bucket 0 (t_min floored to local midnight)
    pub bucket_base_ms: i64,
    /// True if any filtered entry had a timestamp
    pub has_timestamps: bool,
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

/// A filter pattern used by exclude, include, and alert filters
#[derive(Clone)]
pub struct FilterPattern {
    pub query: String,
    pub regex: Regex,
    pub regex_mode: bool,
}

impl fmt::Debug for FilterPattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FilterPattern")
            .field("query", &self.query)
            .finish()
    }
}

/// Whether a filter manager is for exclude, include, or alert patterns
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterKind {
    Exclude,
    Include,
    Alert,
}

impl FilterKind {
    pub fn label(self) -> &'static str {
        match self {
            FilterKind::Exclude => "Exclude",
            FilterKind::Include => "Include",
            FilterKind::Alert => "Alert",
        }
    }

    /// Whether this filter kind affects the visible entry list (requires apply_filters).
    /// Alert keywords only trigger a bell and do not filter entries.
    pub fn affects_filtering(self) -> bool {
        !matches!(self, FilterKind::Alert)
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
    /// Keywords that trigger a terminal bell when matched in live entries
    pub alert_patterns: Vec<FilterPattern>,

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

    // Toast notification (auto-dismissing tip)
    pub toast: Option<(String, Instant)>,

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

    /// Scroll position to restore after initial load completes (scroll_offset, selected_index)
    pub pending_scroll: Option<(usize, usize)>,
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
            alert_patterns: Vec::new(),
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
            toast: None,
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
            pending_scroll: None,
        }
    }

    #[cfg(test)]
    fn push_test_entry(&mut self, level: crate::log_entry::LogLevel, ts_ms: Option<i64>) {
        let ts = ts_ms
            .and_then(chrono::DateTime::from_timestamp_millis)
            .map(|dt| dt.naive_utc());
        let entry = crate::log_entry::LogEntry {
            index: self.entries.len(),
            level,
            timestamp: ts,
            raw_line: String::new(),
            continuation_lines: Vec::new(),
            cached_full_text: None,
            pretty_continuation: None,
            source_idx: 0,
            source_local_idx: self.entries.len(),
            message_offset: None,
        };
        let meta = EntryMeta::from_entry(&entry);
        let idx = self.entries.len();
        self.entries.push(entry);
        self.entry_meta.push(meta);
        self.filtered_indices.push(idx);
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

        if self.tail_enabled && matches!(self.focus, FocusState::Normal) && was_at_bottom {
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

    pub fn open_stats(&mut self) {
        let total_entries = self.entries.len();
        let filtered_count = self.filtered_indices.len();

        let mut level_counts = [0u64; 7];
        let mut min_ms = i64::MAX;
        let mut max_ms = i64::MIN;

        for &idx in &self.filtered_indices {
            let meta = &self.entry_meta[idx];
            let bit_idx = meta.level_bit.trailing_zeros() as usize;
            if bit_idx < 7 {
                level_counts[bit_idx] += 1;
            }
            if meta.timestamp_ms != i64::MIN {
                min_ms = min_ms.min(meta.timestamp_ms);
                max_ms = max_ms.max(meta.timestamp_ms);
            }
        }

        let has_timestamps = min_ms != i64::MAX;
        let time_range = if has_timestamps {
            Some((min_ms, max_ms))
        } else {
            None
        };

        let error_rate = if filtered_count > 0 {
            Some(level_counts[0] as f64 / filtered_count as f64 * 100.0)
        } else {
            None
        };

        // Always compute raw 1-min buckets for zoom/pan support.
        // Floor t_min to the minute so the first bar aligns to the first log entry.
        let (buckets, bucket_base_ms, zoom_idx) = if let Some((t_min, t_max)) = time_range {
            let base_ms = (t_min / 60_000) * 60_000;

            let span_ms = (t_max - base_ms).max(1);
            let bms: i64 = 60_000; // 1-min raw buckets
            let n_buckets = ((span_ms + bms - 1) / bms).max(1) as usize;
            let mut bkts = vec![BucketCounts::default(); n_buckets];

            for &idx in &self.filtered_indices {
                let meta = &self.entry_meta[idx];
                let ts = meta.timestamp_ms;
                if ts != i64::MIN {
                    let bi = (((ts - base_ms) / bms) as usize).min(n_buckets - 1);
                    match meta.level_bit.trailing_zeros() as usize {
                        0 => bkts[bi].error += 1,
                        1 => bkts[bi].warn += 1,
                        _ => bkts[bi].other += 1,
                    }
                }
            }

            (bkts, base_ms, default_zoom_idx(t_max - t_min))
        } else {
            (Vec::new(), 0, 0)
        };

        self.focus = FocusState::Stats {
            data: Box::new(StatsData {
                total_entries,
                filtered_count,
                level_counts,
                time_range,
                error_rate,
                buckets,
                bucket_base_ms,
                has_timestamps,
            }),
            zoom_idx,
            pan_offset: 0,
        };
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

    /// Open the alert keywords manager
    pub fn open_alert_manager(&mut self) {
        self.open_filter_manager(FilterKind::Alert);
    }

    /// Build a default export path from the current file path.
    fn default_export_path(&self, suffix: &str, ext: &str) -> String {
        if self.file_path.is_empty() {
            format!("{}.{}", suffix, ext)
        } else {
            let p = std::path::Path::new(&self.file_path);
            let stem = p
                .file_stem()
                .map(|s| s.to_string_lossy())
                .unwrap_or("output".into());
            let dir = p
                .parent()
                .map(|d| d.to_string_lossy())
                .unwrap_or(".".into());
            format!("{}/{}_{}.{}", dir, stem, suffix, ext)
        }
    }

    /// Open export dialog with default path
    pub fn open_export_dialog(&mut self) {
        let default_path = if self.file_path.is_empty() {
            "filtered.log".to_string()
        } else {
            let p = std::path::Path::new(&self.file_path);
            let ext = p
                .extension()
                .map(|s| s.to_string_lossy())
                .unwrap_or("log".into());
            self.default_export_path("filtered", &ext)
        };
        self.focus = FocusState::ExportDialog {
            input: TextInput::with_text(default_path),
            error: None,
            kind: ExportKind::FilteredLog,
        };
    }

    /// Open export dialog for stats HTML export, carrying stats data forward.
    pub fn open_stats_export(&mut self) {
        let data = match &self.focus {
            FocusState::Stats { data, .. } => data.clone(),
            _ => return,
        };
        let default_path = self.default_export_path("stats", "html");
        self.focus = FocusState::ExportDialog {
            input: TextInput::with_text(default_path),
            error: None,
            kind: ExportKind::StatsHtml(data),
        };
    }

    /// Export filtered entries to a file
    pub fn export_filtered(&mut self, path: &str) -> Result<usize, String> {
        use std::io::Write;
        let expanded = Self::expand_path(path);

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

    /// Expand ~ in paths to $HOME (or %USERPROFILE% on Windows).
    fn expand_path(path: &str) -> String {
        let home = || {
            std::env::var("HOME")
                .or_else(|_| std::env::var("USERPROFILE"))
                .ok()
        };
        if path == "~" {
            home().unwrap_or_else(|| path.to_string())
        } else if let Some(rest) = path.strip_prefix("~/") {
            match home() {
                Some(h) => format!("{}/{}", h, rest),
                None => path.to_string(),
            }
        } else {
            path.to_string()
        }
    }

    /// Export stats dashboard as self-contained HTML with Chart.js.
    /// Returns the expanded path on success.
    pub fn export_stats_html(data: &StatsData, path: &str) -> Result<String, String> {
        use std::io::Write;
        let expanded = Self::expand_path(path);

        let file_name = std::path::Path::new(&expanded)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "Stats".to_string());

        // Build JSON payload for the HTML template
        let level_labels = ["Error", "Warn", "Info", "Debug", "Trace", "Profile"];
        let level_values: Vec<u64> = data.level_counts[..6].to_vec();

        let buckets_json: Vec<serde_json::Value> = data
            .buckets
            .iter()
            .map(|b| {
                serde_json::json!({
                    "error": b.error,
                    "warn": b.warn,
                    "other": b.other
                })
            })
            .collect();

        let stats_json = serde_json::json!({
            "totalEntries": data.total_entries,
            "filteredCount": data.filtered_count,
            "errorRate": data.error_rate,
            "timeRange": data.time_range.map(|(a, b)| vec![a, b]),
            "hasTimestamps": data.has_timestamps,
            "levelLabels": level_labels,
            "levelCounts": level_values,
            "buckets": buckets_json,
            "bucketBaseMs": data.bucket_base_ms,
        });

        let stats_str =
            serde_json::to_string(&stats_json).map_err(|e| format!("JSON error: {}", e))?;

        let html = Self::stats_html_template(&file_name, &stats_str);

        let file = std::fs::File::create(&expanded).map_err(|e| e.to_string())?;
        let mut writer = std::io::BufWriter::new(file);
        writer
            .write_all(html.as_bytes())
            .map_err(|e| e.to_string())?;
        writer.flush().map_err(|e| e.to_string())?;
        Ok(expanded)
    }

    /// Build the full HTML string for stats export.
    /// Template lives in `stats_template.html`; uses `__TITLE__` / `__STATS_JSON__` placeholders.
    fn stats_html_template(title: &str, stats_json: &str) -> String {
        let escaped = title
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;");
        include_str!("stats_template.html")
            .replace("__TITLE__", &escaped)
            .replace("__STATS_JSON__", stats_json)
    }

    /// Open the exported file in the default browser.
    pub fn open_in_browser(path: &str) {
        #[cfg(target_os = "windows")]
        {
            // On Windows, use cmd /c start to open with default browser
            let _ = std::process::Command::new("cmd")
                .args(["/c", "start", "", path])
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn();
        }
        #[cfg(target_os = "macos")]
        {
            let _ = std::process::Command::new("open")
                .arg(path)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn();
        }
        #[cfg(target_os = "linux")]
        {
            let _ = std::process::Command::new("xdg-open")
                .arg(path)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn();
        }
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
            FilterKind::Alert => &mut self.alert_patterns,
        }
    }

    /// Get reference to filter patterns by kind
    pub fn filter_patterns(&self, kind: FilterKind) -> &[FilterPattern] {
        match kind {
            FilterKind::Exclude => &self.exclude_patterns,
            FilterKind::Include => &self.include_patterns,
            FilterKind::Alert => &self.alert_patterns,
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
                    regex_mode,
                });
                if kind.affects_filtering() {
                    self.apply_filters();
                }
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
            if kind.affects_filtering() {
                self.apply_filters();
            }
        }
    }

    /// Clear all filters of a given kind
    pub fn clear_filters(&mut self, kind: FilterKind) {
        let patterns = self.filter_patterns_mut(kind);
        if !patterns.is_empty() {
            patterns.clear();
            if kind.affects_filtering() {
                self.apply_filters();
            }
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
            CommandAction::AlertManager => self.open_alert_manager(),
            CommandAction::ClearAlerts => {
                let count = self.alert_patterns.len();
                self.clear_filters(FilterKind::Alert);
                self.status_message = Some(format!("Cleared {} alert keyword(s)", count));
            }
            CommandAction::ClearIncludes => {
                let count = self.include_patterns.len();
                self.clear_filters(FilterKind::Include);
                self.status_message = Some(format!("Cleared {} include filter(s)", count));
            }
            CommandAction::MergeFile => self.open_merge_file_dialog(),
            CommandAction::ExportFiltered => self.open_export_dialog(),
            CommandAction::Clusters => self.open_clusters(),
            CommandAction::ThemePicker => self.open_theme_picker(),
            CommandAction::Stats => self.open_stats(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log_entry::LogLevel;

    #[test]
    fn bucket_counts_total() {
        let bc = BucketCounts {
            error: 3,
            warn: 5,
            other: 10,
        };
        assert_eq!(bc.total(), 18);
        assert_eq!(BucketCounts::default().total(), 0);
    }

    fn stats_data(app: &App) -> &StatsData {
        match &app.focus {
            FocusState::Stats { data, .. } => data,
            _ => panic!("expected Stats focus"),
        }
    }

    fn stats_zoom_idx(app: &App) -> usize {
        match &app.focus {
            FocusState::Stats { zoom_idx, .. } => *zoom_idx,
            _ => panic!("expected Stats focus"),
        }
    }

    #[test]
    fn open_stats_empty() {
        let mut app = App::new();
        app.open_stats();
        let d = stats_data(&app);
        assert_eq!(d.total_entries, 0);
        assert_eq!(d.filtered_count, 0);
        assert!(d.time_range.is_none());
        assert!(d.error_rate.is_none());
        assert!(d.buckets.is_empty());
        assert!(!d.has_timestamps);
    }

    #[test]
    fn open_stats_level_counts() {
        let mut app = App::new();
        app.push_test_entry(LogLevel::Error, Some(1_000_000));
        app.push_test_entry(LogLevel::Error, Some(2_000_000));
        app.push_test_entry(LogLevel::Warn, Some(3_000_000));
        app.push_test_entry(LogLevel::Info, Some(4_000_000));
        app.push_test_entry(LogLevel::Info, Some(5_000_000));
        app.push_test_entry(LogLevel::Info, Some(6_000_000));
        app.open_stats();
        let d = stats_data(&app);
        assert_eq!(d.total_entries, 6);
        assert_eq!(d.filtered_count, 6);
        assert_eq!(d.level_counts[0], 2); // Error
        assert_eq!(d.level_counts[1], 1); // Warn
        assert_eq!(d.level_counts[2], 3); // Info
        assert!(d.has_timestamps);
    }

    #[test]
    fn open_stats_error_rate() {
        let mut app = App::new();
        // 1 error out of 4 entries = 25%
        app.push_test_entry(LogLevel::Error, Some(1_000));
        app.push_test_entry(LogLevel::Info, Some(2_000));
        app.push_test_entry(LogLevel::Info, Some(3_000));
        app.push_test_entry(LogLevel::Info, Some(4_000));
        app.open_stats();
        let d = stats_data(&app);
        let rate = d.error_rate.unwrap();
        assert!((rate - 25.0).abs() < 0.01);
    }

    #[test]
    fn open_stats_time_range() {
        let mut app = App::new();
        app.push_test_entry(LogLevel::Info, Some(100_000));
        app.push_test_entry(LogLevel::Info, Some(500_000));
        app.open_stats();
        let d = stats_data(&app);
        let (tmin, tmax) = d.time_range.unwrap();
        assert_eq!(tmin, 100_000);
        assert_eq!(tmax, 500_000);
    }

    #[test]
    fn open_stats_no_timestamps() {
        let mut app = App::new();
        app.push_test_entry(LogLevel::Info, None);
        app.push_test_entry(LogLevel::Warn, None);
        app.open_stats();
        let d = stats_data(&app);
        assert!(!d.has_timestamps);
        assert!(d.time_range.is_none());
        assert!(d.buckets.is_empty());
    }

    #[test]
    fn open_stats_default_zoom_selection() {
        let mut app = App::new();
        let base = 1_700_000_000_000i64; // ~2023
        // Span < 1 hour → zoom idx 0 (1 min)
        app.push_test_entry(LogLevel::Info, Some(base));
        app.push_test_entry(LogLevel::Info, Some(base + 30 * 60_000));
        app.open_stats();
        assert_eq!(stats_zoom_idx(&app), 0);
        assert_eq!(ZOOM_LEVELS[0].1, "1 min");

        // Span ~6 hours → zoom idx 1 (5 min)
        let mut app = App::new();
        app.push_test_entry(LogLevel::Info, Some(base));
        app.push_test_entry(LogLevel::Info, Some(base + 6 * 3_600_000));
        app.open_stats();
        assert_eq!(stats_zoom_idx(&app), 1);
        assert_eq!(ZOOM_LEVELS[1].1, "5 min");

        // Span ~2 days → zoom idx 4 (1 hr)
        let mut app = App::new();
        app.push_test_entry(LogLevel::Info, Some(base));
        app.push_test_entry(LogLevel::Info, Some(base + 2 * 86_400_000));
        app.open_stats();
        assert_eq!(stats_zoom_idx(&app), 4);
        assert_eq!(ZOOM_LEVELS[4].1, "1 hr");

        // Span ~10 days → zoom idx 6 (1 day)
        let mut app = App::new();
        app.push_test_entry(LogLevel::Info, Some(base));
        app.push_test_entry(LogLevel::Info, Some(base + 10 * 86_400_000));
        app.open_stats();
        assert_eq!(stats_zoom_idx(&app), 6);
        assert_eq!(ZOOM_LEVELS[6].1, "1 day");
    }

    #[test]
    fn open_stats_buckets_assign_levels_correctly() {
        let mut app = App::new();
        let base = 1_700_000_000_000i64;
        // All in same bucket (span < 1 min bucket)
        app.push_test_entry(LogLevel::Error, Some(base));
        app.push_test_entry(LogLevel::Warn, Some(base + 1000));
        app.push_test_entry(LogLevel::Info, Some(base + 2000));
        app.push_test_entry(LogLevel::Debug, Some(base + 3000));
        app.open_stats();
        let d = stats_data(&app);
        assert!(!d.buckets.is_empty());
        let totals: u64 = d.buckets.iter().map(|b| b.total()).sum();
        assert_eq!(totals, 4);
        let errs: u64 = d.buckets.iter().map(|b| b.error).sum();
        let warns: u64 = d.buckets.iter().map(|b| b.warn).sum();
        let others: u64 = d.buckets.iter().map(|b| b.other).sum();
        assert_eq!(errs, 1);
        assert_eq!(warns, 1);
        assert_eq!(others, 2); // Info + Debug → other
    }
}
