You are tasked with rebuilding LogViewer—a TUI log file viewer—from C# .NET into Rust using the Ratatui framework.

## Project Overview

LogViewer is a terminal UI application that displays, filters, and live-tails custom log files. It supports two proprietary log formats and provides real-time filtering, searching, and log level visualization.

## Architecture Requirements

### 1. Log Entry Parser (equivalent to LogEntry.cs)
- Parse two log formats:
  - **wd.log**: Lines starting with `[*!#]` prefix, followed by log level token (TRACE/INFO/WARN/ERROR/=====/~~~~~) and timestamp `MM-dd HH:mm:ss.fff`
  - **wpc.log**: Lines starting with level token (DBG/INF/WRN/ERR) and timestamp `MM-dd HH:mm:ss.fff`
- Handle continuation lines (lines beginning with whitespace, `{`, `}`, `]`, or empty lines) that attach to the previous entry
- Parse timestamps in `MM-dd HH:mm:ss.fff` format (no year)
- Map level tokens to enum: Trace, Debug, Info, Warn, Error, Unknown
- Support comment lines starting with `#`
- Return parsed entries as a `Vec<LogEntry>` with index, level, timestamp, raw text, and continuation lines

### 2. Log File Tailer (equivalent to LogTailer.cs)
- Load full log file on initialization
- Support live-tail mode via file watching (use `notify` crate for cross-platform file watching)
- Implement 500ms polling fallback if file watch events are missed
- Maintain thread-safe entry list (`Arc<Mutex<Vec<LogEntry>>>`)
- Expose callback/channel for new entries detected during tailing
- Handle file reopening on tail resume

### 3. TUI Layout (equivalent to LogViewerWindow.cs)
Build a 3-row toolbar + scrollable list + status bar:

**Row 0 (File controls):**
- "File:" label + text input (file path)
- "_Open" button (Ctrl+O)
- "_Tail" checkbox (Ctrl+T, toggle live-tail)

**Row 1 (Search & date range):**
- "Search:" label + text input (regex search, case-insensitive)
- "From:" label + text input (date range filter, format: MM-dd HH:mm or yyyy-MM-dd HH:mm or MM-dd HH:mm:ss)
- "To:" label + text input (date range filter)

**Row 2 (Severity filters & word wrap):**
- Checkboxes: ERR, WRN, INF, DBG, TRC (ERR/WRN/INF checked by default)
- "_Wrap" checkbox (toggle word wrapping)

**Main area (scrollable list):**
- ListView showing filtered log entries
- Color coding by log level:
  - Error: Red
  - Warn: Yellow
  - Info: White
  - Debug: Cyan
  - Unknown: Dark Gray
- Highlight selected row (invert colors)
- Support horizontal scrolling for long lines
- Word wrap support (wrap text to viewport width when enabled)

**Status bar:**
- Display: "X total | Y shown | tail: ON/OFF"

### 4. Filtering Logic
Apply filters in this order:
1. Log level severity (checkbox states)
2. Regex search on full text (first line + continuation lines)
3. Timestamp range (if from/to dates provided)
- Re-filter when any filter control changes
- Update list view with filtered results
- Auto-scroll to newest entry when new entries arrive in tail mode

### 5. Keyboard Shortcuts
- Ctrl+O: Focus file path input and open file
- Ctrl+F: Focus search field
- Ctrl+T: Toggle tail mode
- Esc: Quit application
- Arrow keys: Navigate list
- Page Up/Down: Scroll list
- Home/End: Jump to top/bottom

### 6. File I/O & Async
- Use tokio for async file operations
- Load initial file path from CLI argument (first arg)
- Open file picker or just validate path input
- Handle file not found gracefully
- Support FileShare.ReadWrite equivalent (allow reading while file is being written)

## Technical Stack

- **Language**: Rust (latest stable)
- **TUI Framework**: Ratatui (latest stable)
- **Async Runtime**: tokio
- **File Watching**: notify crate
- **Regex**: regex crate
- **Project Structure**: Single binary crate, modules for parser, tailer, UI

## Code Organization

src/
  main.rs           — App setup, event loop, state management
  log_entry.rs      — LogEntry struct, parsing logic (wd.log + wpc.log)
  log_tailer.rs     — LogTailer, file watching, live tail
  ui/
    app.rs          — App state, filtering logic
    ui.rs           — Draw layout, widget rendering
    events.rs       — Keyboard/file watch event handling

## Acceptance Criteria

1. Parse both wd.log and wpc.log formats correctly
2. Live-tail works reliably (300ms+ latency acceptable)
3. All filters (severity, regex search, date range) work and update instantly
4. Keyboard shortcuts respond immediately
5. Word wrap toggles and displays correctly
6. Color coding matches severity levels
7. Status bar updates accurately
8. No crashes on invalid input or missing files
9. Memory efficient for large log files (stream processing where possible)
10. Quit (Esc) cleans up resources properly

## Notes

- Focus on correctness and maintainability over feature completeness
- Rust's type system will prevent many bugs; use it to your advantage
- Consider using channels for communication between tailer thread and UI
- Ratatui is immediate-mode drawing; redraw full UI each frame (no state persistence between draws)
- Test parsing with both log format samples if available

