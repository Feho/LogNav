mod app;
mod config;
mod events;
mod log_entry;
mod log_tailer;
mod ui;

use app::App;
use config::Config;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use log_tailer::{LogTailer, TailerEvent};
use ratatui::{backend::CrosstermBackend, Terminal};
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
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new();
    app.recent_files = config.recent_files.clone();

    // Create tailer channel
    let (tailer_tx, mut tailer_rx) = mpsc::channel::<TailerEvent>(100);

    // Load initial file if provided
    let mut tailer: Option<LogTailer> = None;
    if let Some(ref path) = initial_file {
        app.file_path = path.clone();
        let mut t = LogTailer::new(path, tailer_tx.clone());
        if let Err(e) = t.load_initial().await {
            app.status_message = Some(format!("Error: {}", e));
        } else {
            config.add_recent_file(path);
            app.recent_files = config.recent_files.clone();
        }
        tailer = Some(t);
    }

    // Main event loop
    let result = run_app(&mut terminal, &mut app, &mut tailer_rx, &mut tailer, tailer_tx, &mut config).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    // Save config
    let _ = config.save();

    if let Err(e) = result {
        eprintln!("Error: {}", e);
    }

    Ok(())
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    tailer_rx: &mut mpsc::Receiver<TailerEvent>,
    tailer: &mut Option<LogTailer>,
    tailer_tx: mpsc::Sender<TailerEvent>,
    config: &mut Config,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        // Draw UI
        terminal.draw(|frame| ui::draw(frame, app))?;

        // Check if we should quit
        if app.should_quit {
            break;
        }

        // Poll for events
        tokio::select! {
            // Handle tailer events
            Some(event) = tailer_rx.recv() => {
                match event {
                    TailerEvent::InitialLoad(entries) => {
                        app.set_entries(entries);
                    }
                    TailerEvent::NewEntries(entries) => {
                        app.append_entries(entries);
                    }
                    TailerEvent::Error(e) => {
                        app.status_message = Some(format!("Tail error: {}", e));
                    }
                    TailerEvent::FileReset => {
                        // File was truncated, reload
                        app.set_entries(Vec::new());
                        app.status_message = Some("File was reset".to_string());
                    }
                }
            }

            // Handle terminal events
            _ = async {
                if event::poll(Duration::from_millis(50)).unwrap_or(false) {
                    if let Ok(evt) = event::read() {
                        // Check for file open request
                        let previous_path = app.file_path.clone();
                        
                        events::handle_event(app, evt);
                        
                        // If file path changed, load new file
                        if app.file_path != previous_path && !app.file_path.is_empty() {
                            let path = app.file_path.clone();
                            
                            // Stop existing tailer
                            if let Some(t) = tailer {
                                t.stop_watching();
                            }
                            
                            // Create new tailer
                            let mut new_tailer = LogTailer::new(&path, tailer_tx.clone());
                            match new_tailer.load_initial().await {
                                Ok(()) => {
                                    config.add_recent_file(&path);
                                    app.recent_files = config.recent_files.clone();
                                    
                                    // Start watching if tail is enabled
                                    if app.tail_enabled {
                                        let _ = new_tailer.start_watching();
                                    }
                                    
                                    *tailer = Some(new_tailer);
                                }
                                Err(e) => {
                                    app.status_message = Some(format!("Error: {}", e));
                                    app.file_path = previous_path;
                                }
                            }
                        }
                        
                        // Handle tail toggle
                        if let Some(t) = tailer {
                            if app.tail_enabled {
                                let _ = t.start_watching();
                            } else {
                                t.stop_watching();
                            }
                        }
                    }
                }
            } => {}
        }
    }

    Ok(())
}
