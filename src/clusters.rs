use crate::log_entry::LogEntry;
use crate::ui::extract_message;
use regex::Regex;
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::LazyLock;

/// A detected cluster of consecutive similar log entries
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Cluster {
    /// Templatized message (single-line) or first line of sequence
    pub template: String,
    /// Index into filtered_indices where first occurrence starts
    pub start_filtered_idx: usize,
    /// Number of repetitions (single-line: consecutive count, sequence: number of occurrences)
    pub count: usize,
    /// Length of the repeating sequence (1 for single-line clusters)
    pub sequence_len: usize,
    /// All templates in the sequence (empty for single-line clusters)
    pub sequence_templates: Vec<String>,
    /// All occurrence positions: (start_filtered_idx, length) per occurrence
    pub occurrences: Vec<(usize, usize)>,
}

const MIN_SEQ_LEN: usize = 3;
const MAX_SEQ_LEN: usize = 50;
const MIN_SINGLE_RUN: usize = 3;
const MIN_SEQ_REPEATS: usize = 2;

static UUID_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}")
        .unwrap()
});

static HEX_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b[0-9a-fA-F]{8,}\b").unwrap());

static QUOTED_DOUBLE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#""[^"]*""#).unwrap());

static QUOTED_SINGLE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"'[^']*'").unwrap());

static NUMBER_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b\d+\b").unwrap());

/// Replace variable parts of a message with placeholders.
/// Applied in order: UUID → HEX → quoted strings → numbers (most specific first).
pub fn templatize(message: &str) -> String {
    let s = UUID_RE.replace_all(message, "{UUID}");
    let s = HEX_RE.replace_all(&s, "{HEX}");
    let s = QUOTED_DOUBLE_RE.replace_all(&s, "{STR}");
    let s = QUOTED_SINGLE_RE.replace_all(&s, "{STR}");
    let s = NUMBER_RE.replace_all(&s, "{N}");
    s.into_owned()
}

/// Hash a subsequence of templates into a fingerprint
fn fingerprint(templates: &[String], start: usize, len: usize) -> u64 {
    let mut hasher = DefaultHasher::new();
    for t in &templates[start..start + len] {
        t.hash(&mut hasher);
    }
    hasher.finish()
}

/// A candidate sequence cluster before greedy selection
struct Candidate {
    seq_len: usize,
    positions: Vec<usize>,  // non-overlapping start positions
    templates: Vec<String>, // the sequence templates
    coverage: usize,        // positions.len() * seq_len
}

/// Detect multi-line sequence clusters.
/// Collects all candidate sequences across all lengths, then greedily selects
/// by total coverage (count × seq_len) to avoid suboptimal greedy-by-length.
fn detect_sequence_clusters(templates: &[String], used: &mut [bool]) -> Vec<Cluster> {
    let n = templates.len();
    let max_len = MAX_SEQ_LEN.min(n / MIN_SEQ_REPEATS);

    // Phase 1: collect all candidates
    let mut candidates: Vec<Candidate> = Vec::new();

    for seq_len in MIN_SEQ_LEN..=max_len {
        let mut fp_map: HashMap<u64, Vec<usize>> = HashMap::new();
        for start in 0..=n - seq_len {
            let fp = fingerprint(templates, start, seq_len);
            fp_map.entry(fp).or_default().push(start);
        }

        for positions in fp_map.values() {
            if positions.len() < MIN_SEQ_REPEATS {
                continue;
            }

            // Verify actual match and collect non-overlapping positions
            let reference = &templates[positions[0]..positions[0] + seq_len];
            let mut valid: Vec<usize> = Vec::new();
            let mut last_end: usize = 0;

            for &pos in positions {
                if pos < last_end {
                    continue;
                }
                if templates[pos..pos + seq_len] == *reference {
                    valid.push(pos);
                    last_end = pos + seq_len;
                }
            }

            if valid.len() >= MIN_SEQ_REPEATS {
                // Skip sequences where all lines share the same template —
                // those are better handled as single-line runs
                let all_same = reference.iter().all(|t| t == &reference[0]);
                if all_same {
                    continue;
                }
                let coverage = valid.len() * seq_len;
                candidates.push(Candidate {
                    seq_len,
                    positions: valid,
                    templates: reference.to_vec(),
                    coverage,
                });
            }
        }
    }

    // Phase 2: greedily select by coverage (most entries covered first)
    candidates.sort_by(|a, b| b.coverage.cmp(&a.coverage));

    let mut clusters = Vec::new();
    for cand in candidates {
        // Re-check positions against used entries
        let mut valid: Vec<usize> = Vec::new();
        let mut last_end: usize = 0;
        for &pos in &cand.positions {
            if pos < last_end {
                continue;
            }
            if used[pos..pos + cand.seq_len].iter().any(|&u| u) {
                continue;
            }
            valid.push(pos);
            last_end = pos + cand.seq_len;
        }

        if valid.len() >= MIN_SEQ_REPEATS {
            for &pos in &valid {
                for u in used.iter_mut().skip(pos).take(cand.seq_len) {
                    *u = true;
                }
            }

            let occurrences: Vec<(usize, usize)> =
                valid.iter().map(|&pos| (pos, cand.seq_len)).collect();
            clusters.push(Cluster {
                template: cand.templates[0].clone(),
                start_filtered_idx: valid[0],
                count: valid.len(),
                sequence_len: cand.seq_len,
                sequence_templates: cand.templates,
                occurrences,
            });
        }
    }

    clusters
}

/// Max gap of non-matching (or used) entries tolerated inside a single-line run.
/// The condition `gap < MAX_SINGLE_GAP` means up to MAX_SINGLE_GAP consecutive
/// non-matching entries are allowed between two matching entries before the run
/// is split.
pub const MAX_SINGLE_GAP: usize = 3;

/// Detect single-line run clusters on entries not already used by sequence detection.
/// Tolerates small gaps of non-matching/used entries within a run.
fn detect_single_clusters(templates: &[String], used: &[bool]) -> Vec<Cluster> {
    let mut clusters = Vec::new();
    let n = templates.len();
    if n == 0 {
        return clusters;
    }

    let mut run_template: Option<&str> = None;
    let mut matching_indices: Vec<usize> = Vec::new();
    let mut gap = 0usize;

    for i in 0..=n {
        let matches = i < n && !used[i] && run_template.is_some_and(|t| templates[i] == t);

        if matches {
            matching_indices.push(i);
            gap = 0;
        } else if i < n && !used[i] && run_template.is_none() {
            // Start new run
            run_template = Some(&templates[i]);
            matching_indices.push(i);
            gap = 0;
        } else if run_template.is_some() && i < n && gap < MAX_SINGLE_GAP {
            // Non-matching entry within tolerance
            gap += 1;
        } else {
            // End of run: emit cluster if enough matching entries
            if matching_indices.len() >= MIN_SINGLE_RUN {
                let tmpl = run_template.unwrap();
                let start = matching_indices[0];
                // Each matching entry is its own length-1 occurrence
                // so gap entries are NOT included in cluster_map
                let occurrences: Vec<(usize, usize)> =
                    matching_indices.iter().map(|&idx| (idx, 1)).collect();
                clusters.push(Cluster {
                    template: tmpl.to_string(),
                    start_filtered_idx: start,
                    count: matching_indices.len(),
                    sequence_len: 1,
                    sequence_templates: Vec::new(),
                    occurrences,
                });
            }
            // Start new run if current entry is usable
            matching_indices.clear();
            gap = 0;
            if i < n && !used[i] {
                run_template = Some(&templates[i]);
                matching_indices.push(i);
            } else {
                run_template = None;
            }
        }
    }

    clusters
}

/// Detect all clusters (both multi-line sequences and single-line runs).
/// Sequence detection runs first (greedy, longest first), then single-line on remainder.
/// Results sorted by position.
pub fn detect_clusters(
    entries: &[LogEntry],
    filtered_indices: &[usize],
    _min_size: usize,
) -> Vec<Cluster> {
    if filtered_indices.is_empty() {
        return Vec::new();
    }

    // Templatize all filtered entries
    let templates: Vec<String> = filtered_indices
        .iter()
        .map(|&idx| {
            let msg = extract_message(&entries[idx].raw_line);
            templatize(&msg)
        })
        .collect();

    let mut used = vec![false; templates.len()];

    // Phase 1: detect multi-line sequences
    let mut clusters = detect_sequence_clusters(&templates, &mut used);

    // Phase 2: detect single-line runs on remaining entries
    clusters.extend(detect_single_clusters(&templates, &used));

    // Merge clusters with identical templates (same template + sequence_templates)
    clusters = merge_duplicate_clusters(clusters);

    // Sort by total count descending
    clusters.sort_by(|a, b| b.count.cmp(&a.count));

    clusters
}

/// Strip component prefix for display in the clusters panel.
pub fn display_template(template: &str) -> &str {
    strip_component_prefix(template)
}

/// Strip component prefix from a templatized message for dedup.
/// Handles patterns like `[{N}] ComponentName   actual message`
/// and `[{N}] ComponentName   actual message` (WPC format),
/// or `LVL|Component actual message` (WD format).
fn strip_component_prefix(template: &str) -> &str {
    let s = template.trim_start();

    // WPC: "[{N}] ComponentName   message" or "[ {N}] ComponentName   message"
    if s.starts_with('[') || s.starts_with("{N}]") {
        // Skip past "] "
        if let Some(bracket_end) = s.find("] ") {
            let after_bracket = s[bracket_end + 2..].trim_start();
            // Skip component name (non-whitespace) then whitespace
            if let Some(space_pos) = after_bracket.find(|c: char| c.is_whitespace()) {
                return after_bracket[space_pos..].trim_start();
            }
        }
    }

    // WD: "LVL|Component message" — skip past first space after pipe
    if let Some(pipe) = s.find('|') {
        let after_pipe = &s[pipe + 1..];
        if let Some(space_pos) = after_pipe.find(' ') {
            return after_pipe[space_pos..].trim_start();
        }
    }

    s
}

/// Build a dedup key for a cluster, stripping component prefixes.
fn dedup_key(template: &str) -> String {
    strip_component_prefix(template).to_string()
}

/// Merge clusters that share the same message body (ignoring component prefix).
/// Sums counts and keeps the earliest start position.
fn merge_duplicate_clusters(clusters: Vec<Cluster>) -> Vec<Cluster> {
    let mut merged: Vec<Cluster> = Vec::new();
    let mut seen: HashMap<String, usize> = HashMap::new(); // key → index in merged

    for c in clusters {
        // Build a dedup key stripping component prefixes
        let key = if c.sequence_len > 1 {
            c.sequence_templates
                .iter()
                .map(|t| dedup_key(t))
                .collect::<Vec<_>>()
                .join("\x00")
        } else {
            dedup_key(&c.template)
        };

        if let Some(&idx) = seen.get(&key) {
            merged[idx].count += c.count;
            merged[idx].occurrences.extend(c.occurrences);
            if c.start_filtered_idx < merged[idx].start_filtered_idx {
                merged[idx].start_filtered_idx = c.start_filtered_idx;
            }
        } else {
            seen.insert(key, merged.len());
            merged.push(c);
        }
    }

    merged
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_templatize_uuid() {
        let input = "Loaded user 550e8400-e29b-41d4-a716-446655440000 from cache";
        assert_eq!(templatize(input), "Loaded user {UUID} from cache");
    }

    #[test]
    fn test_templatize_hex() {
        let input = "Object handle DEADBEEF01 released";
        assert_eq!(templatize(input), "Object handle {HEX} released");
    }

    #[test]
    fn test_templatize_numbers() {
        let input = "Processed 42 items in 100 ms";
        assert_eq!(templatize(input), "Processed {N} items in {N} ms");
    }

    #[test]
    fn test_templatize_quoted() {
        let input = r#"FindDN(DOXENSE,"$NOCOLOR",group,True)"#;
        let result = templatize(input);
        assert_eq!(result, "FindDN(DOXENSE,{STR},group,True)");
    }

    #[test]
    fn test_templatize_mixed() {
        let input = r#"User 123 logged in from "192.168.1.1" at session 0A1B2C3D"#;
        let result = templatize(input);
        assert_eq!(result, "User {N} logged in from {STR} at session {HEX}");
    }

    #[test]
    fn test_detect_clusters_empty() {
        let clusters = detect_clusters(&[], &[], 3);
        assert!(clusters.is_empty());
    }

    fn make_entry(index: usize, raw_line: &str) -> LogEntry {
        use crate::log_entry::LogLevel;
        LogEntry {
            index,
            level: LogLevel::Info,
            timestamp: None,
            raw_line: raw_line.to_string(),
            continuation_lines: Vec::new(),
            cached_full_text: None,
            pretty_continuation: None,
            source_idx: 0,
            source_local_idx: index,
        }
    }

    #[test]
    fn test_single_line_clusters() {
        // Use only 4 identical entries — too short for sequence detection (needs 3+ lines
        // of *different* templates), so these will be detected as a single-line run
        let entries: Vec<LogEntry> = (0..4)
            .map(|i| {
                make_entry(
                    i,
                    &format!("01-01 00:00:00.000 INF|Comp \"Processing item {}\"", i),
                )
            })
            .collect();

        let filtered: Vec<usize> = (0..4).collect();
        let clusters = detect_clusters(&entries, &filtered, 3);

        // All 4 entries templatize to same template → sequence detector sees them as
        // a 1-line sequence repeated, but single-line runs catch them since sequence
        // detection requires seq_len >= 3. Result: 1 single-line cluster
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].count, 4);
        assert_eq!(clusters[0].sequence_len, 1);
        assert!(clusters[0].sequence_templates.is_empty());
    }

    #[test]
    fn test_single_line_min_size() {
        let entries: Vec<LogEntry> = (0..2)
            .map(|i| make_entry(i, &format!("01-01 00:00:00.000 INF|Comp \"Item {}\"", i)))
            .collect();

        let filtered: Vec<usize> = (0..2).collect();
        let clusters = detect_clusters(&entries, &filtered, 3);
        assert!(clusters.is_empty());
    }

    #[test]
    fn test_sequence_cluster_basic() {
        // Create a 3-line sequence that repeats twice
        let pattern = [
            "01-01 00:00:00.000 INF|Auth \"FindDN(DOXENSE,user1,person,False)\"",
            "01-01 00:00:00.000 INF|Auth \"GetFromCache(dn_cache, User:user1)\"",
            "01-01 00:00:00.000 INF|Auth \"Aliased to DOXENSE\\user1\"",
        ];

        let mut entries = Vec::new();
        for rep in 0..2 {
            for (j, line) in pattern.iter().enumerate() {
                let idx = rep * 3 + j;
                entries.push(make_entry(idx, line));
            }
        }

        let filtered: Vec<usize> = (0..6).collect();
        let clusters = detect_clusters(&entries, &filtered, 2);

        // Should detect 1 sequence cluster of length 3, repeated 2x
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].sequence_len, 3);
        assert_eq!(clusters[0].count, 2);
        assert_eq!(clusters[0].sequence_templates.len(), 3);
    }

    #[test]
    fn test_sequence_with_varying_params() {
        // Sequence where parameters vary but templates match
        let mut entries = Vec::new();
        let users = ["olive", "admin"];
        for (rep, user) in users.iter().enumerate() {
            let base = rep * 3;
            entries.push(make_entry(
                base,
                &format!(
                    "01-01 00:00:00.000 INF|Auth \"FindDN(DOXENSE,{},person,False)\"",
                    user
                ),
            ));
            entries.push(make_entry(
                base + 1,
                &format!(
                    "01-01 00:00:00.000 INF|Auth \"GetFromCache(dn_cache, User:{})\"",
                    user
                ),
            ));
            entries.push(make_entry(
                base + 2,
                &format!(
                    "01-01 00:00:00.000 INF|Auth \"Aliased to DOXENSE\\{}\"",
                    user
                ),
            ));
        }

        let filtered: Vec<usize> = (0..6).collect();
        let clusters = detect_clusters(&entries, &filtered, 2);

        // Templates won't match because user names aren't quoted/numeric
        // "olive" and "admin" won't be templatized to same placeholder
        // So this should NOT form a sequence cluster
        // (This tests that we don't over-match)
        let seq_clusters: Vec<_> = clusters.iter().filter(|c| c.sequence_len > 1).collect();
        assert!(seq_clusters.is_empty());
    }

    #[test]
    fn test_sequence_with_quoted_params() {
        // Sequence where varying params are quoted → templatize matches
        let mut entries = Vec::new();
        let values = ["$NOCOLOR", "$NOPRINT", "$NOCOPY"];
        for (rep, val) in values.iter().enumerate() {
            let base = rep * 3;
            entries.push(make_entry(
                base,
                &format!(
                    "01-01 00:00:00.000 INF|Auth \"FindDN(DOXENSE,\"{}\",group,True)\"",
                    val
                ),
            ));
            entries.push(make_entry(
                base + 1,
                &format!(
                    "01-01 00:00:00.000 INF|Dir \"GetFromCache(dn_cache, \"{}\")\"",
                    val
                ),
            ));
            entries.push(make_entry(
                base + 2,
                &format!(
                    "01-01 00:00:00.000 INF|Dir \"GetFromCache(notfound_cache, \"{}\")\"",
                    val
                ),
            ));
        }

        let filtered: Vec<usize> = (0..9).collect();
        let clusters = detect_clusters(&entries, &filtered, 2);

        // All 3 repetitions should templatize identically → 1 sequence cluster
        let seq_clusters: Vec<_> = clusters.iter().filter(|c| c.sequence_len > 1).collect();
        assert_eq!(seq_clusters.len(), 1);
        assert_eq!(seq_clusters[0].sequence_len, 3);
        assert_eq!(seq_clusters[0].count, 3);
    }

    #[test]
    fn test_mixed_sequence_and_single() {
        // 3-line sequence repeated 2x, then 4 identical single lines
        let mut entries = Vec::new();
        // Sequence part
        for rep in 0..2 {
            let base = rep * 3;
            entries.push(make_entry(base, "01-01 00:00:00.000 INF|A \"step1\""));
            entries.push(make_entry(base + 1, "01-01 00:00:00.000 INF|A \"step2\""));
            entries.push(make_entry(base + 2, "01-01 00:00:00.000 INF|A \"step3\""));
        }
        // Single-line run part
        for i in 0..4 {
            entries.push(make_entry(
                6 + i,
                &format!("01-01 00:00:00.000 INF|B \"repeat {}\"", i),
            ));
        }

        let filtered: Vec<usize> = (0..10).collect();
        let clusters = detect_clusters(&entries, &filtered, 2);

        // Should have: 1 sequence cluster (3 lines × 2) + 1 single-line cluster (4x)
        assert_eq!(clusters.len(), 2);

        let seq = clusters.iter().find(|c| c.sequence_len > 1).unwrap();
        assert_eq!(seq.sequence_len, 3);
        assert_eq!(seq.count, 2);

        let single = clusters.iter().find(|c| c.sequence_len == 1).unwrap();
        assert_eq!(single.count, 4);
    }

    #[test]
    fn test_duplicate_single_clusters_merged() {
        // Two separate runs of identical-template lines, separated by different lines.
        // Without merging, these would be 2 separate clusters.
        let mut entries = Vec::new();
        // Run 1: 4x "Processing item {N}"
        for i in 0..4 {
            entries.push(make_entry(
                i,
                &format!("01-01 00:00:00.000 INF|Comp \"Processing item {}\"", i),
            ));
        }
        // Gap: 2 different lines
        entries.push(make_entry(
            4,
            "01-01 00:00:00.000 INF|Other Something else A",
        ));
        entries.push(make_entry(
            5,
            "01-01 00:00:00.000 INF|Other Something else B",
        ));
        // Run 2: 3x "Processing item {N}" again
        for i in 0..3 {
            entries.push(make_entry(
                6 + i,
                &format!("01-01 00:00:00.000 INF|Comp \"Processing item {}\"", 10 + i),
            ));
        }

        let filtered: Vec<usize> = (0..9).collect();
        let clusters = detect_clusters(&entries, &filtered, 3);

        // Both runs share same template → merged into 1 cluster
        let matching: Vec<_> = clusters
            .iter()
            .filter(|c| c.template.contains("Processing"))
            .collect();
        assert_eq!(matching.len(), 1);
        // Total count = sum of both runs' counts
        assert!(
            matching[0].count >= 2,
            "merged count should combine both runs"
        );
    }

    #[test]
    fn test_strip_component_prefix_wpc() {
        // WPC format: "[{N}] ComponentName   message"
        assert_eq!(
            strip_component_prefix(
                "[{N}] SpoolerMonitorActor    Actor {STR} changed IDLE => CHECKING"
            ),
            "Actor {STR} changed IDLE => CHECKING"
        );
        assert_eq!(
            strip_component_prefix("[ {N}] UserActor    Sending UI state {N}"),
            "Sending UI state {N}"
        );
    }

    #[test]
    fn test_strip_component_prefix_wd() {
        // WD format: "LVL|Component message"
        assert_eq!(
            strip_component_prefix("INF|Auth FindDN(DOXENSE,{STR},person,False)"),
            "FindDN(DOXENSE,{STR},person,False)"
        );
    }

    #[test]
    fn test_strip_component_prefix_passthrough() {
        // No recognizable prefix
        assert_eq!(
            strip_component_prefix("plain message with no prefix"),
            "plain message with no prefix"
        );
    }
}

#[cfg(test)]
mod gap_analysis_tests {
    use super::*;

    fn make_entry(index: usize, raw_line: &str) -> LogEntry {
        use crate::log_entry::LogLevel;
        LogEntry {
            index,
            level: LogLevel::Info,
            timestamp: None,
            raw_line: raw_line.to_string(),
            continuation_lines: Vec::new(),
            cached_full_text: None,
            pretty_continuation: None,
            source_idx: 0,
            source_local_idx: index,
        }
    }

    /// Test gap tolerance: occurrence span > count when gap entries included
    /// This demonstrates the core issue: detect_single_clusters creates an occurrence span
    /// that includes non-matching gap entries, so the span is wider than the actual matches.
    #[test]
    fn test_gap_tolerance_span_mismatch() {
        // Entries where indices 2,3 have DIFFERENT templates (not quoted) to force gap behavior
        let entries: Vec<LogEntry> = vec![
            make_entry(0, "01-01 00:00:00.000 INF|Comp \"Match 1\""), // {STR}
            make_entry(1, "01-01 00:00:00.000 INF|Comp \"Match 2\""), // {STR}
            make_entry(2, "01-01 00:00:00.000 INF|Comp Something"),   // NO {STR} - different!
            make_entry(3, "01-01 00:00:00.000 INF|Comp Else"),        // NO {STR} - different!
            make_entry(4, "01-01 00:00:00.000 INF|Comp \"Match 3\""), // {STR}
        ];

        let filtered: Vec<usize> = (0..5).collect();
        let clusters = detect_clusters(&entries, &filtered, 3);

        println!("Clusters:");
        for (i, c) in clusters.iter().enumerate() {
            println!(
                "  {}: template='{}', count={}, occurrences={:?}",
                i, c.template, c.count, c.occurrences
            );
        }

        let single_clusters: Vec<_> = clusters.iter().filter(|c| c.sequence_len == 1).collect();
        assert!(
            !single_clusters.is_empty(),
            "Should find at least one single-line cluster"
        );

        let cluster = &single_clusters[0];

        // Each matching entry is a separate (idx, 1) occurrence — no gap entries included
        assert_eq!(cluster.count, 3);
        assert_eq!(cluster.occurrences.len(), 3);
        for &(_, len) in &cluster.occurrences {
            assert_eq!(len, 1, "each occurrence should span exactly 1 entry");
        }
        // Gap entries (indices 2, 3) should NOT appear in any occurrence
        let occ_starts: Vec<usize> = cluster.occurrences.iter().map(|&(s, _)| s).collect();
        assert!(
            !occ_starts.contains(&2),
            "gap entry 2 should not be in occurrences"
        );
        assert!(
            !occ_starts.contains(&3),
            "gap entry 3 should not be in occurrences"
        );
    }
}
