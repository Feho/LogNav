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

pub enum TailerEvent {
    /// A batch of entries from initial load (may be partial)
    LoadBatch {
        source_idx: u8,
        entries: Vec<LogEntry>,
        done: bool,
        /// Sent with final batch so caller can configure tailer for tailing
        parser: Option<Arc<dyn LogParser>>,
        file_size: Option<u64>,
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

pub struct LogTailer {
    path: PathBuf,
    source_idx: u8,
    parser: Arc<dyn LogParser>,
    last_position: u64,
    last_size: u64,
    entry_count: usize,
    watcher: Option<RecommendedWatcher>,
    event_tx: mpsc::Sender<TailerEvent>,
    cancel_token: Option<CancellationToken>,
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
            entry_count: 0,
            watcher: None,
            event_tx,
            cancel_token: None,
        }
    }

    pub fn source_idx(&self) -> u8 {
        self.source_idx
    }

    /// Configure tailer state after streaming load completes
    pub fn configure_for_tailing(
        &mut self,
        parser: Arc<dyn LogParser>,
        file_size: u64,
        entry_count: usize,
    ) {
        self.parser = parser;
        self.last_position = file_size;
        self.last_size = file_size;
        self.entry_count = entry_count;
    }

    /// Start loading the file in the background (fire-and-forget).
    /// Entries arrive via the event channel as LoadBatch events.
    pub fn start_loading(&self, max_entries: usize) {
        let path = self.path.clone();
        let source_idx = self.source_idx;
        let tx = self.event_tx.clone();

        tokio::task::spawn_blocking(move || {
            if let Err(e) = Self::load_initial_blocking(&path, source_idx, &tx, max_entries) {
                let _ = tx.blocking_send(TailerEvent::Error {
                    source_idx,
                    message: e,
                });
            }
        });
    }

    /// Blocking implementation of streaming batch load
    fn load_initial_blocking(
        path: &Path,
        source_idx: u8,
        tx: &mpsc::Sender<TailerEvent>,
        max_entries: usize,
    ) -> Result<(), String> {
        let file = File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
        let metadata = file
            .metadata()
            .map_err(|e| format!("Failed to get metadata: {}", e))?;
        let file_size = metadata.len();

        let mut reader = BufReader::with_capacity(1 << 20, file);

        // Read sample lines for parser detection
        let mut sample = String::new();
        let mut sample_count = 0;
        loop {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => break,
                Ok(_) => {
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
        let mut total_sent: usize = 0;
        let mut in_header = true;
        let mut line_buf = String::new();

        loop {
            line_buf.clear();
            match reader.read_line(&mut line_buf) {
                Ok(0) => break, // EOF
                Ok(_) => {}
                Err(e) => return Err(format!("Failed to read: {}", e)),
            }

            let line = line_buf.trim_end_matches(['\n', '\r']);

            // Skip header comment lines
            if in_header && line.starts_with('#') {
                continue;
            }
            in_header = false;

            if let Some((level, timestamp)) = parser.parse_line(line) {
                // New entry detected — flush pending into batch
                if let Some(mut p) = pending.take() {
                    p.ensure_search_cache();
                    p.source_idx = source_idx;
                    batch.push(p);

                    // Send batch if full
                    if batch.len() >= BATCH_SIZE {
                        total_sent += batch.len();
                        tx.blocking_send(TailerEvent::LoadBatch {
                            source_idx,
                            entries: std::mem::take(&mut batch),
                            done: false,
                            parser: None,
                            file_size: None,
                        })
                        .map_err(|e| format!("Failed to send batch: {}", e))?;

                        // Check entry cap
                        if total_sent >= max_entries {
                            // Stop reading — we've sent enough
                            break;
                        }
                    }
                }

                let clean = parser.clean_line(line);
                let msg_off = parser.message_start(&clean);
                pending = Some(LogEntry {
                    index,
                    level,
                    timestamp,
                    raw_line: clean,
                    continuation_lines: Vec::new(),
                    cached_full_text: None,
                    pretty_continuation: None,
                    source_idx: 0,
                    source_local_idx: index,
                    message_offset: msg_off,
                });
                index += 1;
            } else if let Some(ref mut p) = pending {
                p.add_continuation(parser.clean_line(line));
            }
        }

        // Flush pending entry
        if let Some(mut p) = pending.take() {
            p.ensure_search_cache();
            p.source_idx = source_idx;
            batch.push(p);
        }

        total_sent += batch.len();

        // Send final batch
        tx.blocking_send(TailerEvent::LoadBatch {
            source_idx,
            entries: batch,
            done: true,
            parser: Some(parser),
            file_size: Some(file_size),
        })
        .map_err(|e| format!("Failed to send final batch: {}", e))?;

        let _ = total_sent; // suppress unused warning
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
        let watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    let _ = notify_tx.blocking_send(event);
                }
            },
            notify::Config::default(),
        )
        .map_err(|e| format!("Failed to create watcher: {}", e))?;

        self.watcher = Some(watcher);

        if let Some(ref mut w) = self.watcher {
            w.watch(&path, RecursiveMode::NonRecursive)
                .map_err(|e| format!("Failed to watch file: {}", e))?;
        }

        // Spawn task to handle file events
        let path_clone = self.path.clone();
        let parser = Arc::clone(&self.parser);
        let mut last_position = self.last_position;
        let mut last_size = self.last_size;
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
    async fn check_for_changes(
        path: &Path,
        parser: &dyn LogParser,
        source_idx: u8,
        last_position: &mut u64,
        last_size: &mut u64,
        entry_count: &mut usize,
        tx: &mpsc::Sender<TailerEvent>,
    ) -> Result<(), String> {
        let path = path.to_path_buf();
        let pos = *last_position;
        let prev_size = *last_size;

        let result = tokio::task::spawn_blocking(move || {
            let file = File::open(&path).map_err(|e| format!("Failed to open file: {}", e))?;
            let metadata = file
                .metadata()
                .map_err(|e| format!("Failed to get metadata: {}", e))?;
            let current_size = metadata.len();

            // Check for truncation (file rotation)
            if current_size < prev_size {
                return Ok::<_, String>((None, 0, current_size, true));
            }

            // No new content
            if current_size == pos {
                return Ok((None, pos, current_size, false));
            }

            // Read new content
            let mut file = file;
            file.seek(SeekFrom::Start(pos))
                .map_err(|e| format!("Failed to seek: {}", e))?;

            let mut reader = BufReader::new(file);
            let mut new_content = String::new();

            loop {
                let mut line = String::new();
                match reader.read_line(&mut line) {
                    Ok(0) => break,
                    Ok(_) => new_content.push_str(&line),
                    Err(e) => return Err(format!("Failed to read: {}", e)),
                }
            }

            Ok((Some(new_content), current_size, current_size, false))
        })
        .await
        .map_err(|e| format!("Task failed: {}", e))??;

        let (content, new_pos, new_size, truncated) = result;

        if truncated {
            *last_position = 0;
            *last_size = new_size;
            *entry_count = 0;
            tx.send(TailerEvent::FileReset { source_idx })
                .await
                .map_err(|e| format!("Failed to send: {}", e))?;
            return Ok(());
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

    /// Stop watching the file
    pub fn stop_watching(&mut self) {
        if let Some(token) = self.cancel_token.take() {
            token.cancel();
        }
        self.watcher = None;
    }
}
