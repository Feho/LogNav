# LogNav

A fast, keyboard-driven terminal log viewer built in Rust.

## Why LogNav?

Debugging means digging through logs. Traditional tools force you to juggle multiple programs — `tail -f` for live output, `grep` for patterns, `less` for scrolling. LogNav combines all of this into one fast binary with vim-style navigation. Errors are colored red so they jump out. Filters update instantly. New lines stream in without losing your place.

## Installation

```bash
cargo install --path .
```

Or build manually:

```bash
cargo build --release
# Binary: target/release/lognav
```

Requires Rust 1.93+ (Edition 2024).

## Getting Started

```bash
lognav /path/to/app.log   # Open a file directly
lognav                     # Launch and open with 'o'
```

LogNav auto-detects the log format and starts displaying entries.

## Features

### Navigation

Navigate logs like vim:

| Key | Action |
|-----|--------|
| `j` / `k` | Move down / up |
| `g` / `G` | Top / bottom |
| `h` / `l` | Scroll left / right (wrap off) |
| `PageUp` / `PageDown` | Page navigation |

Mouse scrolling and clicking also work.

### Filter by Severity

Log entries are color-coded by level. Press a number key to toggle that level:

| Level | Key |
|-------|-----|
| Error | `1` |
| Warn | `2` |
| Info | `3` |
| Debug | `4` |
| Trace | `5` |
| Profile | `6` |

Press `0` to reset all levels to visible.

### Search

Press `/` or `Ctrl+F` to open the search bar. Type a term, press `Enter` to show all matches in a side panel. Use `n` / `N` to jump between matches. Press `Ctrl+R` to toggle regex mode.

### Date Filter

Press `Ctrl+D` to open the date filter dialog.

**Quick filters** (press 1–6):
1. Last hour
2. Last 24 hours
3. Today
4. Yesterday
5. Last 7 days
6. Clear filter

**Custom range**: Tab to the From/To fields and enter relative times (`-2h`, `-30m`, `-7d`), specific dates (`2024-03-15 14:00`), or keywords (`today`, `yesterday`, `now`).

### Exclude Filters

Press `x` to open the exclude filter manager. Type a pattern and press `Enter` to hide all matching entries. `Ctrl+R` toggles regex mode. Press `X` to clear all exclude filters at once.

**Quick exclude**: `Alt+Click` on any word to instantly exclude it.

### Multi-line Entries

Some entries span multiple lines (stack traces, JSON payloads). These appear collapsed by default.

| Key | Action |
|-----|--------|
| `Enter` | Expand/collapse selected entry |
| `a` | Toggle expand all |
| `d` | Detail popup (full-screen view) |

Embedded JSON is automatically pretty-printed when expanded.

### Bookmarks

| Key | Action |
|-----|--------|
| `m` | Toggle bookmark on current line |
| `b` / `B` | Next / previous bookmark |

Bookmarks persist across sessions.

### Jump to Errors and Warnings

| Key | Action |
|-----|--------|
| `e` / `E` | Next / previous error |
| `w` / `W` | Next / previous warning |

### Live Mode

Press `t` to toggle live mode — new entries appear and the view auto-scrolls. Live mode pauses automatically when you scroll up and resumes when you return to the bottom.

### Merge Multiple Files

Press `M` to merge an additional file into the current view. Entries are sorted by timestamp; each source file gets a distinct color indicator.

### Cluster Detection

Open via the command palette (`Ctrl+P` → "Find repeating patterns"). LogNav scans the filtered log for repeating patterns — identical lines or multi-line sequences that appear 3+ times. Variable parts (UUIDs, hex strings, numbers, quoted strings) are replaced with placeholders so near-identical entries cluster together. Useful for spotting noisy repeated errors or identifying startup sequences in a busy log.

### Statistics Dashboard

Press `F2` to open a statistics overlay showing entry counts, error rate, level distribution, and an event rate timeline.

Timeline controls:

| Key | Action |
|-----|--------|
| `+` / `-` | Zoom in / out |
| `h` / `l` or `←` / `→` | Pan left / right |
| `Home` / `End` | Jump to start / end |
| `0` | Reset zoom and pan |
| `e` | Export as HTML |

The HTML export includes interactive Chart.js charts with drag-to-zoom and print-friendly styling.

### Export

Press `Ctrl+S` to export the currently filtered entries to a file. Only entries visible after all active filters are exported.

### Themes

Open via the command palette (`Ctrl+P` → "Change theme..."). 12 built-in Dark & Light themes. Selection persists across sessions.

For per-color overrides, edit `~/.config/lognav/config.toml` directly. Colors accept named values (`"Red"`), hex (`"#ff0000"`), or 256-color index (`"238"`).

### Command Palette

Press `Ctrl+P` to fuzzy-search all available commands. The fastest way to discover features.

---

## Supported Log Formats

LogNav auto-detects the format from file content. If no known format matches, it falls back to a generic parser that learns level tokens from a sample of lines.

### Custom Formats

Define your own format by adding a TOML file to:
- Linux/macOS: `~/.config/lognav/formats/`
- Windows: `C:\Users\<user>\AppData\Roaming\lognav\config\formats\`

```toml
name = "myapp"
pattern = '^(?P<level>VRB|DBG|INF|WRN|ERR)\s+(?P<timestamp>\d{4}-\d{2}-\d{2}\s+\d{2}:\d{2}:\d{2}\.\d{3})\s+\S+\s+(?P<message>.*)'
timestamp_format = "%Y-%m-%d %H:%M:%S%.3f"
timezone = "+01:00"

[level_map]
"VRB" = "trace"
"DBG" = "debug"
"INF" = "info"
"WRN" = "warn"
"ERR" = "error"
```

Named groups:
- `level` — matched against `level_map` first, then standard names
- `timestamp` — parsed with `timestamp_format`; time-only formats use today's date
- `message` — if present, used as the message start offset; otherwise message starts after the full match

Custom parsers are loaded automatically on startup and detected at 0.9 confidence.

---

## Key Reference

### Normal Mode

| Key | Action |
|-----|--------|
| `j` / `k` | Move down / up |
| `g` / `G` | Top / bottom |
| `h` / `l` | Scroll left / right |
| `PageUp` / `PageDown` | Page navigation |
| `/` or `Ctrl+F` | Search |
| `n` / `N` | Next / previous match |
| `1`–`6` | Toggle log levels |
| `0` | Reset level filters |
| `Ctrl+D` | Date filter |
| `x` | Exclude filter manager |
| `X` | Clear exclude filters |
| `t` | Toggle live mode |
| `Alt+W` | Toggle word wrap |
| `s` | Toggle syntax highlighting |
| `Enter` | Expand/collapse entry |
| `a` | Toggle expand all |
| `d` | Detail popup |
| `m` | Toggle bookmark |
| `b` / `B` | Next / previous bookmark |
| `e` / `E` | Next / previous error |
| `w` / `W` | Next / previous warning |
| `c` | Copy entry to clipboard |
| `o` or `Ctrl+O` | Open file |
| `M` | Merge file |
| `Ctrl+S` | Export filtered results |
| `F2` | Statistics dashboard |
| `Ctrl+P` | Command palette |
| `?` or `F1` | Help |
| `q` | Quit |

### Search Mode

| Key | Action |
|-----|--------|
| `Enter` | Apply search |
| `Esc` | Cancel |
| `Ctrl+R` | Toggle regex |
| `Up` / `Down` | Search history |
| `Ctrl+U` | Clear input |

### Mouse

| Action | Effect |
|--------|--------|
| Click | Select entry |
| Scroll | Navigate up/down |
| `Ctrl+Click` | Search for word under cursor |
| `Alt+Click` | Exclude word under cursor |
