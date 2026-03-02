mod app;
mod clusters;
mod config;
mod events;
mod log_entry;
mod log_tailer;
mod parsers;
mod text_input;
mod text_utils;
mod theme;
mod tips;
mod ui;

use app::{App, SourceFile};
use config::Config;
use crossterm::{
    event::{
        self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
        Event,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use log_tailer::{LogTailer, TailerEvent};
use parsers::LogParser;
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

/// Metadata returned when a streaming load completes
struct LoadComplete {
    source_idx: u8,
    parser: Arc<dyn LogParser>,
    file_size: u64,
    entry_count: usize,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse CLI args
    let args: Vec<String> = std::env::args().collect();
    let initial_file = args.get(1).cloned();

    // Load config
    let mut config = Config::load();

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        EnableMouseCapture,
        EnableBracketedPaste
    )?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new();
    app.recent_files = config.recent_files.clone();
    app.syntax_highlight = config.syntax_highlight.unwrap_or(true);
    app.dark_overrides = config.dark_overrides.clone();
    app.light_overrides = config.light_overrides.clone();
    app.theme = theme::Theme::from_config(&theme::ThemeConfig {
        theme: config.theme.clone(),
        dark_overrides: config.dark_overrides.clone(),
        light_overrides: config.light_overrides.clone(),
    });

    // Create tailer channel (shared across all tailers)
    let (tailer_tx, mut tailer_rx) = mpsc::channel::<TailerEvent>(100);

    // Create cluster detection channel
    let (cluster_tx, mut cluster_rx) = mpsc::channel(1);
    app.cluster_tx = Some(cluster_tx);

    // Load initial file if provided (fire-and-forget streaming load)
    let mut tailers: Vec<LogTailer> = Vec::new();
    if let Some(ref path) = initial_file {
        app.file_path = path.clone();
        app.sources
            .push(SourceFile::new(path, app.theme.source_color(0)));
        app.source_entry_counts.push(0);
        app.loading_sources.insert(0);

        let t = LogTailer::new(path, 0, tailer_tx.clone());
        t.start_loading();

        config.add_recent_file(path);
        app.recent_files = config.recent_files.clone();
        // Store bookmark stable IDs; actual bookmarks rebuilt as entries arrive
        let loaded_bookmarks = config.load_bookmarks(path);
        app.bookmark_stable_ids = loaded_bookmarks.iter().map(|&idx| (0u8, idx)).collect();

        tailers.push(t);
    }

    // Main event loop
    let result = run_app(
        &mut terminal,
        &mut app,
        &mut tailer_rx,
        &mut cluster_rx,
        &mut tailers,
        tailer_tx,
        &mut config,
    )
    .await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture,
        DisableBracketedPaste
    )?;
    terminal.show_cursor()?;

    // Save config — save per-source bookmarks
    config.syntax_highlight = Some(app.syntax_highlight);
    config.theme = app.theme.name.clone();
    config.dark_overrides = app.dark_overrides.clone();
    config.light_overrides = app.light_overrides.clone();
    save_bookmarks_for_sources(&app, &mut config);
    let _ = config.save();

    if let Err(e) = result {
        eprintln!("Error: {}", e);
    }

    Ok(())
}

/// Save bookmarks for each source file
fn save_bookmarks_for_sources(app: &App, config: &mut Config) {
    if app.sources.is_empty() {
        return;
    }
    for (si, source) in app.sources.iter().enumerate() {
        let local_indices: std::collections::HashSet<usize> = app
            .bookmark_stable_ids
            .iter()
            .filter(|(s, _)| *s == si as u8)
            .map(|(_, li)| *li)
            .collect();
        config.save_bookmarks(&source.path, &local_indices);
    }
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    tailer_rx: &mut mpsc::Receiver<TailerEvent>,
    cluster_rx: &mut mpsc::Receiver<Vec<crate::clusters::Cluster>>,
    tailers: &mut Vec<LogTailer>,
    tailer_tx: mpsc::Sender<TailerEvent>,
    config: &mut Config,
) -> Result<(), Box<dyn std::error::Error>> {
    const SEARCH_DEBOUNCE: Duration = Duration::from_millis(150);

    loop {
        // Flush debounced search if deadline passed
        if let Some(dirty_at) = app.search_dirty
            && dirty_at.elapsed() >= SEARCH_DEBOUNCE
        {
            events::flush_search(app);
        }

        // Draw UI
        terminal.draw(|frame| ui::draw(frame, app))?;

        // Check if we should quit
        if app.should_quit {
            break;
        }

        // Snapshot state before handling events
        let previous_path = app.file_path.clone();
        let previous_tail_enabled = app.tail_enabled;

        // Wait for at least one event
        let mut pending: Vec<Event> = Vec::new();
        tokio::select! {
            Some(event) = tailer_rx.recv() => {
                if let Some(lc) = handle_tailer_event(app, event) {
                    finish_load(app, tailers, config, lc);
                }
            }
            Some(clusters) = cluster_rx.recv() => {
                app.receive_clusters(clusters);
            }
            _ = async {
                if event::poll(Duration::from_millis(50)).unwrap_or(false)
                    && let Ok(evt) = event::read()
                {
                    pending.push(evt);
                }
            } => {}
        }

        // Drain remaining events. If the first event is a plain char key,
        // wait longer to catch drag-and-drop bursts on terminals without
        // bracketed paste (chars arrive individually with tiny gaps).
        let mut coalescing = is_plain_char_press(pending.first());
        loop {
            let timeout = if coalescing {
                Duration::from_millis(5)
            } else {
                Duration::ZERO
            };
            if !event::poll(timeout).unwrap_or(false) {
                break;
            }
            if let Ok(evt) = event::read() {
                // Only plain-char events (press or release) keep the burst alive
                if let Event::Key(k) = &evt {
                    if !is_plain_char(k) {
                        coalescing = false;
                    }
                } else {
                    coalescing = false;
                }
                pending.push(evt);
            } else {
                break;
            }
        }

        // Coalesce rapid char-key bursts into Paste events
        // (handles drag-and-drop on terminals without bracketed paste)
        for evt in coalesce_char_events(pending) {
            events::handle_event(app, evt);
        }

        // Drain pending tailer events too
        while let Ok(event) = tailer_rx.try_recv() {
            if let Some(lc) = handle_tailer_event(app, event) {
                finish_load(app, tailers, config, lc);
            }
        }

        // Handle file path change — "Open File" replaces everything
        if app.file_path != previous_path && !app.file_path.is_empty() {
            // Save bookmarks for all previous sources
            save_bookmarks_for_sources(app, config);

            let path = app.file_path.clone();

            // Stop all tailers and clear merged state
            for t in tailers.iter_mut() {
                t.stop_watching();
            }
            tailers.clear();
            app.remove_all_sources();
            app.reset_all_filters();
            app.set_primary_source(&path);
            app.loading_sources.clear();
            app.loading_sources.insert(0);
            app.loading_entry_count = 0;

            let new_tailer = LogTailer::new(&path, 0, tailer_tx.clone());
            new_tailer.start_loading();

            config.add_recent_file(&path);
            app.recent_files = config.recent_files.clone();
            // Store bookmark stable IDs; actual bookmarks rebuilt as entries arrive
            let loaded = config.load_bookmarks(&path);
            app.bookmark_stable_ids = loaded.iter().map(|&idx| (0u8, idx)).collect();

            tailers.push(new_tailer);
        }

        // Handle merge file request
        if let Some(merge_path) = app.pending_merge_path.take() {
            let source_idx = app.sources.len() as u8;
            app.sources.push(SourceFile::new(
                &merge_path,
                app.theme.source_color(source_idx),
            ));
            while app.source_entry_counts.len() <= source_idx as usize {
                app.source_entry_counts.push(0);
            }

            config.add_recent_file(&merge_path);
            app.recent_files = config.recent_files.clone();
            app.loading_sources.insert(source_idx);
            app.loading_entry_count = 0;

            let new_tailer = LogTailer::new(&merge_path, source_idx, tailer_tx.clone());
            new_tailer.start_loading();

            // Store bookmark stable IDs for this source
            let loaded = config.load_bookmarks(&merge_path);
            for &local_idx in &loaded {
                app.bookmark_stable_ids.insert((source_idx, local_idx));
            }

            tailers.push(new_tailer);
        }

        // Handle tail toggle (once after all events)
        if app.tail_enabled != previous_tail_enabled {
            for t in tailers.iter_mut() {
                if app.tail_enabled {
                    let _ = t.start_watching();
                } else {
                    t.stop_watching();
                }
            }
        }
    }

    Ok(())
}

/// Handle load completion: configure tailer for tailing, rebuild bookmarks
fn finish_load(
    app: &mut App,
    tailers: &mut [LogTailer],
    config: &mut Config,
    lc: LoadComplete,
) {
    if let Some(tailer) = tailers.iter_mut().find(|t| t.source_idx() == lc.source_idx) {
        tailer.configure_for_tailing(lc.parser, lc.file_size, lc.entry_count);
        if app.tail_enabled {
            let _ = tailer.start_watching();
        }
    }

    // Rebuild bookmarks now that entries exist
    app.rebuild_bookmarks_from_stable();

    if app.sources.len() > 1 {
        let label = app
            .sources
            .get(lc.source_idx as usize)
            .map(|s| s.label.as_str())
            .unwrap_or("?");

        // Only show merge status for non-primary sources
        if lc.source_idx > 0 {
            app.status_message = Some(format!(
                "Merged: {}",
                app.sources
                    .iter()
                    .map(|s| s.label.as_str())
                    .collect::<Vec<_>>()
                    .join(" + ")
            ));
        }
        let _ = label;
    }

    // Save bookmarks path association
    if let Some(source) = app.sources.get(lc.source_idx as usize) {
        let _ = config.load_bookmarks(&source.path);
    }
}

/// A "plain" char key: printable character with no modifier, Shift, or
/// AltGr (reported as Ctrl+Alt on non-US keyboard layouts like French).
fn is_plain_char(key: &crossterm::event::KeyEvent) -> bool {
    use crossterm::event::{KeyCode, KeyModifiers};
    let m = key.modifiers;
    (m.is_empty() || m == KeyModifiers::SHIFT || m == (KeyModifiers::ALT | KeyModifiers::CONTROL))
        && matches!(key.code, KeyCode::Char(_))
}

fn is_plain_char_press(evt: Option<&Event>) -> bool {
    matches!(evt, Some(Event::Key(k)) if k.kind == crossterm::event::KeyEventKind::Press && is_plain_char(k))
}

/// Coalesce runs of plain char key-presses into Paste events.
/// Terminals without bracketed paste (e.g. Windows drag-and-drop) send
/// pasted/dropped text as individual KeyCode::Char events. When we see
/// a burst of 4+ consecutive char keys we merge them into a single
/// Event::Paste so the app treats it as dropped text.
fn coalesce_char_events(events: Vec<Event>) -> Vec<Event> {
    let mut result: Vec<Event> = Vec::with_capacity(events.len());
    let mut char_buf = String::new();

    for evt in events {
        if let Event::Key(k) = &evt
            && is_plain_char(k)
        {
            if k.kind == crossterm::event::KeyEventKind::Press
                && let crossterm::event::KeyCode::Char(c) = k.code
            {
                char_buf.push(c);
            }
            // Release events are silently absorbed while coalescing —
            // terminals with keyboard enhancement protocols send
            // Press+Release pairs for each character.
        } else {
            flush_char_buf(&mut char_buf, &mut result);
            result.push(evt);
        }
    }
    flush_char_buf(&mut char_buf, &mut result);
    result
}

fn flush_char_buf(buf: &mut String, out: &mut Vec<Event>) {
    if buf.is_empty() {
        return;
    }
    if buf.len() >= 4 {
        out.push(Event::Paste(std::mem::take(buf)));
    } else {
        use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
        for c in buf.drain(..).collect::<Vec<_>>() {
            out.push(Event::Key(KeyEvent {
                code: KeyCode::Char(c),
                modifiers: if c.is_uppercase() {
                    KeyModifiers::SHIFT
                } else {
                    KeyModifiers::NONE
                },
                kind: KeyEventKind::Press,
                state: KeyEventState::NONE,
            }));
        }
    }
}

/// Handle a tailer event, returning LoadComplete if a streaming load finished
fn handle_tailer_event(app: &mut App, event: TailerEvent) -> Option<LoadComplete> {
    match event {
        TailerEvent::LoadBatch {
            source_idx,
            entries,
            done,
            parser,
            file_size,
        } => {
            let count = entries.len();
            app.loading_entry_count += count;
            app.merge_entries_from_source(source_idx, entries);
            if done {
                app.loading_sources.remove(&source_idx);
                let entry_count = app
                    .source_entry_counts
                    .get(source_idx as usize)
                    .copied()
                    .unwrap_or(0);
                return Some(LoadComplete {
                    source_idx,
                    parser: parser.expect("final LoadBatch must include parser"),
                    file_size: file_size.expect("final LoadBatch must include file_size"),
                    entry_count,
                });
            }
            None
        }
        TailerEvent::NewEntries {
            source_idx,
            entries,
        } => {
            app.merge_entries_from_source(source_idx, entries);
            None
        }
        TailerEvent::Error {
            source_idx,
            message,
        } => {
            // If error during loading, clear loading state for this source
            app.loading_sources.remove(&source_idx);
            if app.sources.len() > 1 {
                let label = app
                    .sources
                    .get(source_idx as usize)
                    .map(|s| s.label.as_str())
                    .unwrap_or("?");
                app.status_message = Some(format!("Tail error [{}]: {}", label, message));
            } else {
                app.status_message = Some(format!("Tail error: {}", message));
            }
            None
        }
        TailerEvent::FileReset { source_idx } => {
            app.reset_source(source_idx);
            app.status_message = Some(format!("Source {} was reset", source_idx));
            None
        }
    }
}
