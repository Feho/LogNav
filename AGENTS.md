# LogNav

TUI log viewer for proprietary log formats. Rust + ratatui + tokio.

## Build & Run

```bash
cargo build              # Debug build
cargo build --release    # Release build
cargo run                # Run debug
cargo run -- /path/log   # Run with file
```

## Test Commands

```bash
cargo test                           # All tests (~76)
cargo test <pattern>                 # Tests matching pattern
cargo test test_detect               # Tests containing "test_detect"
cargo test log_entry::tests::test_parse_wd_log -- --exact  # Single exact test
```

Tests are spread across modules: `log_entry.rs`, `parsers/`, `clusters.rs`, `text_utils.rs`, `tips.rs`, etc.

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
  config.rs            # Config persistence (~/.config/lognav/)
  log_entry.rs         # LogEntry struct, format detection, tests
  log_tailer.rs        # File watching for live tail
  clusters.rs          # Repeating pattern detection
  text_input.rs        # Reusable text input widget
  text_utils.rs        # Unicode-aware text processing
  theme.rs             # Theme system (colors, styles, TOML themes)
  tips.rs              # Rotating tips display
  parsers/
    mod.rs             # LogParser trait, detect_parser(), all_parsers()
    wd.rs              # WdParser — wd.log format
    wpc.rs             # WpcParser — wpc.log format
    qconsole.rs        # QConsoleParser — quake console logs
    generic.rs         # GenericParser — fallback, learns from sample lines
    custom.rs          # CustomParser — loaded from ~/.config/lognav/formats/*.toml
    common.rs          # Shared parser utilities
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
    search_panel.rs    # Search results panel keys
    date_filter.rs     # Date filter dialog keys, parse_date_input()
    file_open.rs       # File open dialog keys
    filter_manager.rs  # Filter manager dialog keys
    detail.rs          # Detail view keys
    export.rs          # Export dialog keys
    clusters.rs        # Cluster view keys
    help.rs            # Help overlay keys
    theme_picker.rs    # Theme picker keys
    mouse.rs           # Mouse event handling
  ui/
    mod.rs             # draw(), level_color(), level_style(), helpers
    log_view.rs        # Log view rendering (wrap/nowrap modes)
    status_bar.rs      # Status bar rendering
    overlays.rs        # Command palette, search bar, dialogs
    clusters_panel.rs  # Cluster results rendering
    matches_panel.rs   # Search matches panel rendering
    syntax.rs          # Syntax highlighting
```

## Parser Architecture

Pluggable parser system via `LogParser` trait (`src/parsers/mod.rs`):
- `detect(first_line) -> f64` — confidence score 0.0–1.0
- `parse_line(line) -> Option<(LogLevel, Option<NaiveDateTime>)>`
- `message_start(line) -> Option<usize>` — message byte offset
- `clean_line(line) -> String` — optional stripping (e.g., color codes)

**Detection pipeline** (`detect_parser`): try all parsers on first line → scan 20 sample lines if no confident match → fall back to GenericParser.

**Custom formats**: TOML files in `~/.config/lognav/formats/*.toml` with regex named groups (`(?P<level>...)`, `(?P<timestamp>...)`), custom level mappings, chrono timestamp formats. Loaded automatically via `load_custom_parsers()`.

## Code Style

### Imports

Group in order: `crate::` → external crates (alphabetical) → `std::`

### Key Event Handling

- Match on `(KeyModifiers, KeyCode)` tuple
- Ctrl+key: `(KeyModifiers::CONTROL, KeyCode::Char('x'))`
- Fall through with `_ => {}`

### UI Patterns (ratatui)

- `draw()` dispatches to sub-functions
- `Clear` widget before overlays; overlays render last
- `Layout` for splitting areas

### Error Handling

- `?` for propagation; `.ok()` to discard errors; `unwrap_or_default()` for fallbacks
- Show errors to user via `app.status_message`

## Dependencies

| Crate | Purpose |
|-------|---------|
| ratatui | TUI framework |
| crossterm | Terminal backend |
| tokio / tokio-util | Async runtime |
| notify | File watching |
| regex | Search filtering |
| chrono | Timestamp parsing |
| serde / serde_json / toml | Config & format persistence |
| fuzzy-matcher | Command palette fuzzy search |
| directories | XDG config paths |
| arboard | Clipboard access |

## Custom Log Formats

Prefer adding TOML files over creating new Rust parser modules.
- Linux/macOS: `~/.config/lognav/formats/*.toml`
- Windows: `C:\Users\<user>\AppData\Roaming\lognav\config\formats\*.toml`

Pattern uses named regex groups: `(?P<level>...)`, `(?P<timestamp>...)`, optional `(?P<message>...)`.
See `src/parsers/custom.rs` for implementation. Custom parsers detect at 0.9 confidence.

## Testing Notes

`cargo run` requires a real terminal (TTY) — fails in non-interactive contexts.
To validate parser regex changes without launching the TUI, use `cargo test` or test regex with a script.

## Edition

Rust Edition 2024, requires rustc 1.93.0+
