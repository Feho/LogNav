# AGENTS.md - LogViewerRust

TUI log viewer for proprietary log formats (wd.log, wpc.log). Rust + ratatui + tokio.

## Build & Run

```bash
cargo build              # Debug build
cargo build --release    # Release build
cargo run                # Run debug
cargo run -- /path/log   # Run with file
```

## Test Commands

```bash
cargo test                           # All tests
cargo test <pattern>                 # Tests matching pattern
cargo test test_detect               # Tests containing "test_detect"
cargo test log_entry::tests::test_parse_wd_log -- --exact  # Single exact test
```

Tests are inline in `src/log_entry.rs`. 12 tests total covering log format detection and parsing.

## Lint & Format

```bash
cargo fmt                # Format code
cargo fmt -- --check     # Verify formatting
cargo clippy             # Lint
cargo clippy -- -D warnings  # Lint, treat warnings as errors
```

No custom rustfmt.toml or clippy.toml - use defaults.

## Project Structure

```
src/
  main.rs              # Entry point, async event loop
  config.rs            # Config persistence (~/.config/logviewer/)
  log_entry.rs         # Log parsing, format detection, tests
  log_tailer.rs        # File watching for live tail
  app/
    mod.rs             # App struct definition, core state, Default impl
    commands.rs        # Command struct, CommandAction enum
    filtering.rs       # Filtering logic (apply_filters, set_search, etc.)
    navigation.rs      # Scrolling and navigation methods
  events/
    mod.rs             # handle_event(), dispatch to submodules
    normal.rs          # Normal mode key handling
    command.rs         # Command palette key handling
    search.rs          # Search mode key handling
    date_filter.rs     # Date filter dialog keys, parse_date_input()
    file_open.rs       # File open dialog keys
    mouse.rs           # Mouse event handling
  ui/
    mod.rs             # draw(), level_color(), level_style(), helpers
    log_view.rs        # Log view rendering (wrap/nowrap modes)
    status_bar.rs      # Status bar rendering
    overlays.rs        # Command palette, search bar, dialogs
```

## Code Style

### Imports

Group imports in order:
1. `crate::` (local modules)
2. External crates (alphabetical)
3. `std::`

```rust
use crate::app::{App, FocusState};
use chrono::NaiveDateTime;
use regex::Regex;
use std::sync::LazyLock;
```

### Naming

- Types: `PascalCase` (e.g., `LogEntry`, `FocusState`)
- Functions/methods: `snake_case` (e.g., `parse_log`, `scroll_to_bottom`)
- Constants: `SCREAMING_SNAKE_CASE` (e.g., `MAX_ENTRIES`, `TIMESTAMP_FORMAT`)
- Enums: `PascalCase` variants (e.g., `LogLevel::Error`, `FocusState::Normal`)

### Types

- Use `Option<T>` for nullable fields (e.g., `timestamp: Option<NaiveDateTime>`)
- Use `Result<T, E>` for fallible operations, `Box<dyn std::error::Error>` for main
- Prefer concrete types over trait objects when possible
- Use `&str` for params, return `String` when ownership needed

### Structs

- Derive common traits: `#[derive(Debug, Clone)]` minimum
- Add `Copy` for small types (enums, simple structs)
- Add `PartialEq, Eq` for comparisons
- Add `Default` via `impl Default` or derive when sensible

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel { ... }

#[derive(Debug, Clone)]
pub struct LogEntry { ... }
```

### Error Handling

- Use `?` for propagation in functions returning Result
- Use `.ok()` to convert Result to Option when error is ignorable
- Use `unwrap_or_default()` for safe fallbacks
- Log errors to `status_message` field for user visibility

```rust
// Propagate
let content = fs::read_to_string(&path)?;

// Ignore error, use default
let config = serde_json::from_str(&content).unwrap_or_default();

// Show error to user
if let Err(e) = operation() {
    app.status_message = Some(format!("Error: {}", e));
}
```

### Pattern Matching

- Exhaustive match on enums
- Use `_` for catch-all only when appropriate
- Prefer `if let` for single-variant checks

```rust
match level {
    LogLevel::Error => ...,
    LogLevel::Warn => ...,
    // all variants
}

if let FocusState::Search { query, .. } = &app.focus {
    // handle search state
}
```

### Async

- Use `tokio::select!` for concurrent event handling
- Use `mpsc` channels for communication between tasks
- Mark async functions with `async fn`

### Documentation

- Use `///` doc comments for public items
- Keep comments brief, focus on "why" not "what"
- No need to document obvious methods

```rust
/// Parse log content into entries
pub fn parse_log(content: &str) -> Vec<LogEntry> { ... }
```

### Constants

- Define at module level with `const` for simple values
- Use `LazyLock` for complex initialization (e.g., compiled regex)

```rust
const MAX_ENTRIES: usize = 500_000;

static WD_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"...").unwrap()
});
```

### UI Patterns (ratatui)

- Main `draw` function dispatches to sub-functions
- Use `Layout` for splitting areas
- Use `Clear` widget before rendering overlays
- Render overlays last (on top)

### Key Event Handling

- Match on `(KeyModifiers, KeyCode)` tuple
- Handle Ctrl+key as `(KeyModifiers::CONTROL, KeyCode::Char('x'))`
- Fall through with `_ => {}` for unhandled keys

## Dependencies

| Crate | Purpose |
|-------|---------|
| ratatui | TUI framework |
| crossterm | Terminal backend |
| tokio | Async runtime |
| notify | File watching |
| regex | Search filtering |
| chrono | Timestamp parsing |
| serde/serde_json | Config persistence |
| fuzzy-matcher | Command palette fuzzy search |
| directories | XDG config paths |

## Edition

Rust Edition 2024, requires rustc 1.93.0+
