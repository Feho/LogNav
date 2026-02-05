use crate::log_entry::{detect_format, parse_incremental, parse_log_with_format, LogEntry, LogFormat};
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

const MAX_ENTRIES: usize = 500_000;

#[derive(Debug)]
pub enum TailerEvent {
    /// Initial load complete with entries
    InitialLoad(Vec<LogEntry>),
    /// New entries detected during tailing
    NewEntries(Vec<LogEntry>),
    /// Error occurred
    Error(String),
    /// File was truncated/rotated
    FileReset,
}

pub struct LogTailer {
    path: PathBuf,
    format: LogFormat,
    last_position: u64,
    last_size: u64,
    entry_count: usize,
    watcher: Option<RecommendedWatcher>,
    event_tx: mpsc::Sender<TailerEvent>,
    cancel_token: Option<CancellationToken>,
}

impl LogTailer {
    /// Create a new tailer for the given file path
    pub fn new(path: impl AsRef<Path>, event_tx: mpsc::Sender<TailerEvent>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            format: LogFormat::Unknown,
            last_position: 0,
            last_size: 0,
            entry_count: 0,
            watcher: None,
            event_tx,
            cancel_token: None,
        }
    }

    /// Load the initial file contents
    pub async fn load_initial(&mut self) -> Result<(), String> {
        let path = self.path.clone();
        let tx = self.event_tx.clone();

        // Read file in blocking task
        let result = tokio::task::spawn_blocking(move || {
            let file = File::open(&path).map_err(|e| format!("Failed to open file: {}", e))?;
            let metadata = file.metadata().map_err(|e| format!("Failed to get metadata: {}", e))?;
            let size = metadata.len();

            let mut reader = BufReader::new(file);
            let mut content = String::new();
            reader
                .read_to_string(&mut content)
                .map_err(|e| format!("Failed to read file: {}", e))?;

            let format = detect_format(&content);
            let entries = parse_log_with_format(&content, format);

            Ok::<_, String>((entries, format, size))
        })
        .await
        .map_err(|e| format!("Task failed: {}", e))?;

        let (mut entries, format, size) = result?;

        // Apply entry cap
        if entries.len() > MAX_ENTRIES {
            let skip = entries.len() - MAX_ENTRIES;
            entries = entries.into_iter().skip(skip).collect();
            // Re-index
            for (i, entry) in entries.iter_mut().enumerate() {
                entry.index = i;
            }
        }

        self.format = format;
        self.last_position = size;
        self.last_size = size;
        self.entry_count = entries.len();

        tx.send(TailerEvent::InitialLoad(entries))
            .await
            .map_err(|e| format!("Failed to send event: {}", e))?;

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
        let format = self.format;
        let mut last_position = self.last_position;
        let mut last_size = self.last_size;
        let mut entry_count = self.entry_count;

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
                            format,
                            &mut last_position,
                            &mut last_size,
                            &mut entry_count,
                            &tx,
                        ).await {
                            let _ = tx.send(TailerEvent::Error(e)).await;
                        }
                    }
                    _ = poll_interval.tick() => {
                        // Fallback polling
                        if let Err(e) = Self::check_for_changes(
                            &path_clone,
                            format,
                            &mut last_position,
                            &mut last_size,
                            &mut entry_count,
                            &tx,
                        ).await {
                            let _ = tx.send(TailerEvent::Error(e)).await;
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
        format: LogFormat,
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
            let metadata = file.metadata().map_err(|e| format!("Failed to get metadata: {}", e))?;
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
            tx.send(TailerEvent::FileReset)
                .await
                .map_err(|e| format!("Failed to send: {}", e))?;
            return Ok(());
        }

        if let Some(content) = content {
            if !content.is_empty() {
                let entries = parse_incremental(&content, format, *entry_count, None);

                if !entries.is_empty() {
                    *entry_count += entries.len();
                    tx.send(TailerEvent::NewEntries(entries))
                        .await
                        .map_err(|e| format!("Failed to send: {}", e))?;
                }
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
