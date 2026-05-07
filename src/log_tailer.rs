use crate::log_entry::LogEntry;
use crate::parsers::{self, LogParser};
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

const BATCH_SIZE: usize = 10_000;

fn complete_line_prefix_len(bytes: &[u8]) -> usize {
    bytes
        .iter()
        .rposition(|&b| b == b'\n')
        .map(|idx| idx + 1)
        .unwrap_or(0)
}

pub enum TailerEvent {
    /// A batch of entries from initial load (may be partial)
    LoadBatch {
        source_idx: u8,
        entries: Vec<LogEntry>,
        done: bool,
        /// Sent with final batch so caller can configure tailer for tailing
        parser: Option<Arc<dyn LogParser>>,
        /// Byte offset of the last complete (newline-terminated) line parsed
        loaded_through: Option<u64>,
    },
    /// New entries detected during tailing
    NewEntries {
        source_idx: u8,
        entries: Vec<LogEntry>,
    },
    /// Error occurred
    Error { source_idx: u8, message: String },
    /// File was truncated/rotated
    FileReset { source_idx: u8 },
}

/// Fingerprint identifying a specific file instance, used to detect rotation
/// where the new file may be same size or larger than the old one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FileFingerprint {
    #[cfg(unix)]
    dev: u64,
    #[cfg(unix)]
    inode: u64,
    #[cfg(windows)]
    created: Option<std::time::SystemTime>,
}

impl FileFingerprint {
    fn from_metadata(_meta: &std::fs::Metadata) -> Option<Self> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            Some(Self {
                dev: _meta.dev(),
                inode: _meta.ino(),
            })
        }
        #[cfg(windows)]
        {
            Some(Self {
                created: _meta.created().ok(),
            })
        }
        #[cfg(not(any(unix, windows)))]
        {
            None
        }
    }
}

pub struct LogTailer {
    path: PathBuf,
    source_idx: u8,
    parser: Arc<dyn LogParser>,
    last_position: u64,
    last_size: u64,
    last_fingerprint: Option<FileFingerprint>,
    entry_count: usize,
    watcher: Option<RecommendedWatcher>,
    event_tx: mpsc::Sender<TailerEvent>,
    cancel_token: Option<CancellationToken>,
    load_cancel_token: Option<CancellationToken>,
}

impl LogTailer {
    /// Create a new tailer for the given file path
    pub fn new(
        path: impl AsRef<Path>,
        source_idx: u8,
        event_tx: mpsc::Sender<TailerEvent>,
    ) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            source_idx,
            parser: parsers::fallback_parser(),
            last_position: 0,
            last_size: 0,
            last_fingerprint: None,
            entry_count: 0,
            watcher: None,
            event_tx,
            cancel_token: None,
            load_cancel_token: None,
        }
    }

    pub fn source_idx(&self) -> u8 {
        self.source_idx
    }

    /// Configure tailer state after streaming load completes
    pub fn configure_for_tailing(
        &mut self,
        parser: Arc<dyn LogParser>,
        loaded_through: u64,
        entry_count: usize,
    ) {
        self.parser = parser;
        self.last_position = loaded_through;
        self.last_size = loaded_through;
        self.entry_count = entry_count;
        self.last_fingerprint = std::fs::metadata(&self.path)
            .ok()
            .and_then(|m| FileFingerprint::from_metadata(&m));
    }

    /// Start loading the file in the background (fire-and-forget).
    /// Entries arrive via the event channel as LoadBatch events.
    pub fn start_loading(&mut self) {
        let cancel = CancellationToken::new();
        self.load_cancel_token = Some(cancel.clone());

        let path = self.path.clone();
        let source_idx = self.source_idx;
        let tx = self.event_tx.clone();

        tokio::task::spawn_blocking(move || {
            if let Err(e) = Self::load_initial_blocking(&path, source_idx, &tx, cancel) {
                let _ = tx.blocking_send(TailerEvent::Error {
                    source_idx,
                    message: e,
                });
            }
        });
    }

    /// Cancel an in-progress background load, if any.
    pub fn cancel_loading(&mut self) {
        if let Some(token) = self.load_cancel_token.take() {
            token.cancel();
        }
    }

    /// Blocking implementation of streaming batch load
    fn load_initial_blocking(
        path: &Path,
        source_idx: u8,
        tx: &mpsc::Sender<TailerEvent>,
        cancel: CancellationToken,
    ) -> Result<(), String> {
        let file = File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;

        let mut reader = BufReader::with_capacity(1 << 20, file);

        // Read sample lines for parser detection
        let mut sample = String::new();
        let mut sample_count = 0;
        let mut raw_line: Vec<u8> = Vec::new();
        loop {
            raw_line.clear();
            match reader.read_until(b'\n', &mut raw_line) {
                Ok(0) => break,
                Ok(_) => {
                    let line = String::from_utf8_lossy(&raw_line);
                    let trimmed = line.trim();
                    if !trimmed.is_empty() && !trimmed.starts_with('#') {
                        sample.push_str(&line);
                        sample_count += 1;
                        if sample_count >= 20 {
                            break;
                        }
                    }
                }
                Err(e) => return Err(format!("Failed to read file: {}", e)),
            }
        }

        let parser = parsers::detect_parser(&sample).unwrap_or_else(parsers::fallback_parser);

        // Seek back to start for full parse
        reader
            .seek(SeekFrom::Start(0))
            .map_err(|e| format!("Failed to seek: {}", e))?;

        let mut batch: Vec<LogEntry> = Vec::with_capacity(BATCH_SIZE);
        let mut pending: Option<LogEntry> = None;
        let mut index: usize = 0;
        let mut in_header = true;
        let mut line_buf: Vec<u8> = Vec::new();
        let mut last_complete_position = 0u64;

        let mut lines_since_cancel_check: usize = 0;
        loop {
            line_buf.clear();
            let line_start = reader
                .stream_position()
                .map_err(|e| format!("Failed to get position: {}", e))?;
            match reader.read_until(b'\n', &mut line_buf) {
                Ok(0) => break, // EOF
                Ok(_) => {}
                Err(e) => return Err(format!("Failed to read: {}", e)),
            }

            if !line_buf.ends_with(b"\n") {
                last_complete_position = line_start;
                break;
            }
            last_complete_position = reader
                .stream_position()
                .map_err(|e| format!("Failed to get position: {}", e))?;

            lines_since_cancel_check += 1;
            if lines_since_cancel_check >= 1024 {
                if cancel.is_cancelled() {
                    return Ok(());
                }
                lines_since_cancel_check = 0;
            }

            let line_cow = String::from_utf8_lossy(&line_buf);
            let line = line_cow.trim_end_matches(['\n', '\r']);

            // Skip header comment lines
            if in_header && line.starts_with('#') {
                continue;
            }
            in_header = false;

            if let Some((level, timestamp)) = parser.parse_line(line) {
                // New entry detected — flush pending into batch
                if let Some(mut p) = pending.take() {
                    p.ensure_search_cache();
                    batch.push(p);

                    // Send batch if full
                    if batch.len() >= BATCH_SIZE {
                        if cancel.is_cancelled() {
                            return Ok(());
                        }
                        tx.blocking_send(TailerEvent::LoadBatch {
                            source_idx,
                            entries: std::mem::take(&mut batch),
                            done: false,
                            parser: None,
                            loaded_through: None,
                        })
                        .map_err(|e| format!("Failed to send batch: {}", e))?;
                    }
                }

                let clean = parser.clean_line(line);
                let msg_off = parser.message_start(&clean);
                pending = Some(LogEntry {
                    index,
                    level,
                    timestamp,
                    raw_line: clean.into_owned(),
                    continuation_lines: Vec::new(),
                    cached_full_text: None,
                    pretty_continuation: None,
                    source_idx,
                    source_local_idx: index,
                    message_offset: msg_off,
                });
                index += 1;
            } else if let Some(ref mut p) = pending {
                p.add_continuation(parser.clean_line(line).into_owned());
            }
        }

        // Flush pending entry
        if let Some(mut p) = pending.take() {
            p.ensure_search_cache();
            batch.push(p);
        }

        // Send final batch (skip if cancelled)
        if cancel.is_cancelled() {
            return Ok(());
        }
        tx.blocking_send(TailerEvent::LoadBatch {
            source_idx,
            entries: batch,
            done: true,
            parser: Some(parser),
            loaded_through: Some(last_complete_position),
        })
        .map_err(|e| format!("Failed to send final batch: {}", e))?;

        Ok(())
    }

    /// Check if currently watching the file
    pub fn is_watching(&self) -> bool {
        self.cancel_token.is_some()
    }

    /// Start watching the file for changes
    pub fn start_watching(&mut self) -> Result<(), String> {
        // Don't start if already watching
        if self.is_watching() {
            return Ok(());
        }

        let path = self.path.clone();
        let tx = self.event_tx.clone();

        // Create cancellation token
        let cancel_token = CancellationToken::new();
        let cancel_clone = cancel_token.clone();
        self.cancel_token = Some(cancel_token);

        // Create channel for file events
        let (notify_tx, mut notify_rx) = mpsc::channel::<Event>(100);

        // Create watcher
        let mut watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    let _ = notify_tx.blocking_send(event);
                }
            },
            notify::Config::default(),
        )
        .map_err(|e| format!("Failed to create watcher: {}", e))?;

        if let Err(e) = watcher.watch(&path, RecursiveMode::NonRecursive) {
            // Don't stash a watcher that never started.
            self.cancel_token = None;
            return Err(format!("Failed to watch file: {}", e));
        }
        self.watcher = Some(watcher);

        // Spawn task to handle file events
        let path_clone = self.path.clone();
        let parser = Arc::clone(&self.parser);
        let mut last_position = self.last_position;
        let mut last_size = self.last_size;
        let mut last_fingerprint = self.last_fingerprint;
        let mut entry_count = self.entry_count;
        let source_idx = self.source_idx;

        tokio::spawn(async move {
            let mut poll_interval = tokio::time::interval(std::time::Duration::from_millis(500));

            loop {
                tokio::select! {
                    _ = cancel_clone.cancelled() => {
                        // Stop watching
                        break;
                    }
                    Some(_event) = notify_rx.recv() => {
                        // File change detected
                        if let Err(e) = Self::check_for_changes(
                            &path_clone,
                            &*parser,
                            source_idx,
                            &mut last_position,
                            &mut last_size,
                            &mut last_fingerprint,
                            &mut entry_count,
                            &tx,
                        ).await {
                            let _ = tx.send(TailerEvent::Error { source_idx, message: e }).await;
                        }
                    }
                    _ = poll_interval.tick() => {
                        // Fallback polling
                        if let Err(e) = Self::check_for_changes(
                            &path_clone,
                            &*parser,
                            source_idx,
                            &mut last_position,
                            &mut last_size,
                            &mut last_fingerprint,
                            &mut entry_count,
                            &tx,
                        ).await {
                            let _ = tx.send(TailerEvent::Error { source_idx, message: e }).await;
                        }
                    }
                }
            }
        });

        Ok(())
    }

    /// Check for new content in the file
    #[allow(clippy::too_many_arguments)]
    async fn check_for_changes(
        path: &Path,
        parser: &dyn LogParser,
        source_idx: u8,
        last_position: &mut u64,
        last_size: &mut u64,
        last_fingerprint: &mut Option<FileFingerprint>,
        entry_count: &mut usize,
        tx: &mpsc::Sender<TailerEvent>,
    ) -> Result<(), String> {
        let path = path.to_path_buf();
        let pos = *last_position;
        let prev_size = *last_size;
        let prev_fp = *last_fingerprint;

        let result = tokio::task::spawn_blocking(move || {
            let file = File::open(&path).map_err(|e| format!("Failed to open file: {}", e))?;
            let metadata = file
                .metadata()
                .map_err(|e| format!("Failed to get metadata: {}", e))?;
            let current_size = metadata.len();
            let current_fp = FileFingerprint::from_metadata(&metadata);

            // Detect rotation: shrink OR fingerprint changed (same/larger new file).
            let rotated = current_size < prev_size
                || match (prev_fp, current_fp) {
                    (Some(a), Some(b)) => a != b,
                    _ => false,
                };
            if rotated {
                return Ok::<_, String>((None, 0, current_size, current_fp, true));
            }

            // No new content
            if current_size == pos {
                return Ok((None, pos, current_size, current_fp, false));
            }

            // Read new content
            let mut file = file;
            file.seek(SeekFrom::Start(pos))
                .map_err(|e| format!("Failed to seek: {}", e))?;

            // Cap bytes read per tick so a large burst doesn't balloon memory.
            // Any remainder is picked up on the next poll/event.
            const MAX_CHUNK_BYTES: usize = 1 << 20; // 1 MiB
            let mut reader = BufReader::new(file);
            let mut new_content = Vec::new();
            let mut raw_line: Vec<u8> = Vec::new();
            let mut bytes_read: usize = 0;

            while bytes_read < MAX_CHUNK_BYTES {
                raw_line.clear();
                match reader.read_until(b'\n', &mut raw_line) {
                    Ok(0) => break,
                    Ok(n) => {
                        bytes_read += n;
                        new_content.extend_from_slice(&raw_line);
                    }
                    Err(e) => return Err(format!("Failed to read: {}", e)),
                }
            }

            let complete_len = complete_line_prefix_len(&new_content);
            let advanced_to = pos + complete_len as u64;
            let content = if complete_len > 0 {
                Some(String::from_utf8_lossy(&new_content[..complete_len]).into_owned())
            } else {
                None
            };
            Ok((content, advanced_to, current_size, current_fp, false))
        })
        .await
        .map_err(|e| format!("Task failed: {}", e))??;

        let (content, new_pos, new_size, new_fp, truncated) = result;

        if truncated {
            *last_position = 0;
            *last_size = new_size;
            *last_fingerprint = new_fp;
            *entry_count = 0;
            tx.send(TailerEvent::FileReset { source_idx })
                .await
                .map_err(|e| format!("Failed to send: {}", e))?;
            return Ok(());
        }

        if last_fingerprint.is_none() {
            *last_fingerprint = new_fp;
        }

        if let Some(content) = content
            && !content.is_empty()
        {
            let mut entries =
                parsers::parse_incremental_with_parser(&content, parser, *entry_count, None);

            if !entries.is_empty() {
                for entry in &mut entries {
                    entry.source_idx = source_idx;
                }
                *entry_count += entries.len();
                tx.send(TailerEvent::NewEntries {
                    source_idx,
                    entries,
                })
                .await
                .map_err(|e| format!("Failed to send: {}", e))?;
            }
        }

        *last_position = new_pos;
        *last_size = new_size;

        Ok(())
    }

    /// Stop watching the file and cancel any in-progress background load.
    pub fn stop_watching(&mut self) {
        if let Some(token) = self.cancel_token.take() {
            token.cancel();
        }
        self.watcher = None;
        self.cancel_loading();
    }
}

#[cfg(test)]
mod tests {
    use super::complete_line_prefix_len;

    #[test]
    fn complete_line_prefix_keeps_full_lines_only() {
        let content = b"[ts] first\n[ts] second\n[ts] partial";
        assert_eq!(complete_line_prefix_len(content), 23);
    }

    #[test]
    fn complete_line_prefix_returns_zero_without_newline() {
        assert_eq!(complete_line_prefix_len(b"[ts] partial"), 0);
    }

    #[test]
    fn complete_line_prefix_accepts_trailing_newline() {
        let content = b"[ts] first\n";
        assert_eq!(complete_line_prefix_len(content), content.len());
    }
}
