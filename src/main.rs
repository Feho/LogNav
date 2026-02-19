mod app;
mod config;
mod events;
mod log_entry;
mod log_tailer;
mod parsers;
mod text_input;
mod text_utils;
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
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io;
use std::time::Duration;
use tokio::sync::mpsc;

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

    // Create tailer channel (shared across all tailers)
    let (tailer_tx, mut tailer_rx) = mpsc::channel::<TailerEvent>(100);

    // Load initial file if provided
    let mut tailers: Vec<LogTailer> = Vec::new();
    if let Some(ref path) = initial_file {
        app.file_path = path.clone();
        app.sources.push(SourceFile::new(path, 0));
        app.source_entry_counts.push(0);
        let mut t = LogTailer::new(path, 0, tailer_tx.clone());
        if let Err(e) = t.load_initial().await {
            app.status_message = Some(format!("Error: {}", e));
        } else {
            config.add_recent_file(path);
            app.recent_files = config.recent_files.clone();
            app.bookmarks = config.load_bookmarks(path);
            // Populate bookmark_stable_ids from loaded bookmarks
            // (source_idx=0, source_local_idx=entry_idx for single file)
            app.bookmark_stable_ids = app.bookmarks.iter().map(|&idx| (0u8, idx)).collect();
            if app.tail_enabled {
                let _ = t.start_watching();
            }
        }
        tailers.push(t);
    }

    // Main event loop
    let result = run_app(
        &mut terminal,
        &mut app,
        &mut tailer_rx,
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
                handle_tailer_event(app, event);
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
        // wait for more to catch drag-and-drop bursts on terminals
        // without bracketed paste (chars arrive individually with tiny gaps).
        // 30 ms is well below human typing speed (~100 ms) but enough to
        // capture rapid terminal paste/drop sequences.
        let mut coalesce_chars = is_plain_char_event(pending.first());
        loop {
            let timeout = if coalesce_chars {
                Duration::from_millis(30)
            } else {
                Duration::ZERO
            };
            if !event::poll(timeout).unwrap_or(false) {
                break;
            }
            if let Ok(evt) = event::read() {
                // Release events for plain chars don't break a coalescing burst
                let is_release = matches!(&evt, Event::Key(k) if k.kind != crossterm::event::KeyEventKind::Press && matches!(k.code, crossterm::event::KeyCode::Char(_)));
                if !is_release {
                    coalesce_chars = coalesce_chars && is_plain_char_event(Some(&evt));
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
            handle_tailer_event(app, event);
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
            app.set_primary_source(&path);

            let mut new_tailer = LogTailer::new(&path, 0, tailer_tx.clone());
            match new_tailer.load_initial().await {
                Ok(()) => {
                    config.add_recent_file(&path);
                    app.recent_files = config.recent_files.clone();
                    // Load bookmarks for this file
                    let loaded = config.load_bookmarks(&path);
                    app.bookmark_stable_ids = loaded.iter().map(|&idx| (0u8, idx)).collect();
                    app.bookmarks = loaded;
                    if app.tail_enabled {
                        let _ = new_tailer.start_watching();
                    }
                    tailers.push(new_tailer);
                }
                Err(e) => {
                    app.status_message = Some(format!("Error: {}", e));
                    app.file_path = previous_path;
                    app.sources.clear();
                }
            }
        }

        // Handle merge file request
        if let Some(merge_path) = app.pending_merge_path.take() {
            let source_idx = app.sources.len() as u8;
            app.sources.push(SourceFile::new(&merge_path, source_idx));
            while app.source_entry_counts.len() <= source_idx as usize {
                app.source_entry_counts.push(0);
            }

            config.add_recent_file(&merge_path);
            app.recent_files = config.recent_files.clone();

            let mut new_tailer = LogTailer::new(&merge_path, source_idx, tailer_tx.clone());
            match new_tailer.load_initial().await {
                Ok(()) => {
                    // Load bookmarks for this source
                    let loaded = config.load_bookmarks(&merge_path);
                    for &local_idx in &loaded {
                        app.bookmark_stable_ids.insert((source_idx, local_idx));
                    }
                    if app.tail_enabled {
                        let _ = new_tailer.start_watching();
                    }
                    tailers.push(new_tailer);
                    app.status_message = Some(format!(
                        "Merged: {}",
                        app.sources
                            .iter()
                            .map(|s| s.label.as_str())
                            .collect::<Vec<_>>()
                            .join(" + ")
                    ));
                }
                Err(e) => {
                    app.status_message = Some(format!("Error merging: {}", e));
                    app.sources.pop();
                }
            }
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

fn is_plain_char_event(evt: Option<&Event>) -> bool {
    use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};
    matches!(
        evt,
        Some(Event::Key(k))
            if k.kind == KeyEventKind::Press
            && (k.modifiers.is_empty() || k.modifiers == KeyModifiers::SHIFT)
            && matches!(k.code, KeyCode::Char(_))
    )
}


/// Coalesce runs of plain char key-presses into Paste events.
/// Terminals without bracketed paste (e.g. Windows drag-and-drop) send
/// pasted/dropped text as individual KeyCode::Char events. When we see
/// a burst of 4+ consecutive char keys (no ctrl/alt modifier), we merge
/// them into a single Event::Paste so the app treats it as dropped text.
fn coalesce_char_events(events: Vec<Event>) -> Vec<Event> {
    use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};

    let mut result: Vec<Event> = Vec::with_capacity(events.len());
    let mut char_buf = String::new();

    for evt in events {
        let is_plain_char_press = matches!(
            &evt,
            Event::Key(k)
                if k.kind == KeyEventKind::Press
                && (k.modifiers.is_empty() || k.modifiers == KeyModifiers::SHIFT)
                && matches!(k.code, KeyCode::Char(_))
        );
        let is_plain_char_release = matches!(
            &evt,
            Event::Key(k)
                if k.kind != KeyEventKind::Press
                && (k.modifiers.is_empty() || k.modifiers == KeyModifiers::SHIFT)
                && matches!(k.code, KeyCode::Char(_))
        );
        if is_plain_char_press {
            if let Event::Key(k) = &evt
                && let KeyCode::Char(c) = k.code
            {
                char_buf.push(c);
            }
        } else if is_plain_char_release && !char_buf.is_empty() {
            // Skip release events while accumulating a char burst — terminals
            // with keyboard enhancement protocols send Press+Release pairs,
            // and we don't want the Release to break a drag-and-drop sequence.
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
        // Looks like a paste / drag-and-drop
        out.push(Event::Paste(std::mem::take(buf)));
    } else {
        // Re-emit as individual key events
        for c in buf.drain(..).collect::<Vec<_>>() {
            use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
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

fn handle_tailer_event(app: &mut App, event: TailerEvent) {
    match event {
        TailerEvent::InitialLoad {
            source_idx,
            entries,
        } => app.merge_entries_from_source(source_idx, entries),
        TailerEvent::NewEntries {
            source_idx,
            entries,
        } => app.merge_entries_from_source(source_idx, entries),
        TailerEvent::Error {
            source_idx,
            message,
        } => {
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
        }
        TailerEvent::FileReset { source_idx } => {
            app.reset_source(source_idx);
            app.status_message = Some(format!("Source {} was reset", source_idx));
        }
    }
}
