# LogNav

A fast, keyboard-driven terminal log viewer built in Rust.

## Why LogNav?

Debugging means digging through logs. Traditional tools force you to juggle multiple programs:

- `tail -f` to watch live output
- `grep` to search for patterns
- `less` to scroll through history
- External editors to compare timestamps

LogNav combines all of this into one fast binary with vim-style navigation. Errors are colored red so they jump out. Filters update instantly. New lines stream in without losing your place.

## Getting Started

```bash
# Open a log file directly
lognav /path/to/app.log

# Or launch and open with Ctrl+O
lognav
```

That's it. LogNav auto-detects the log format and starts displaying entries.

## Walkthrough

### 1. Basic Navigation

Navigate logs like you would in vim:

| Key | Action |
|-----|--------|
| `j` / `k` | Move down / up |
| `g` | Jump to top |
| `G` | Jump to bottom |
| `PageUp` / `PageDown` | Scroll by page |
| `h` / `l` | Scroll left / right (when wrap is off) |

Mouse scrolling and clicking work too.

### 2. Filter by Severity

Log entries are color-coded by level:

| Level | Color | Keys to Toggle |
|-------|-------|----------------|
| Error | Red | `1` |
| Warn | Yellow | `2` |
| Info | White | `3` |
| Debug | Cyan | `4` |
| Trace | Gray | `5` |
| Profile | Magenta | `6` |

By default, all levels are visible. Press the number key to toggle any level on or off. Press `0` to reset all levels to visible.

### 3. Search

Press `/` or `Ctrl+F` to open the search bar:

1. Type your search term
2. Press `Enter` to see all matches in a side panel
3. Use `n` / `N` to jump between matches
4. Press `Ctrl+R` while typing to enable regex mode
5. Press `Esc` to close the search panel

The search panel shows all matching entries. Press `Tab` to focus it and navigate results directly.

### 4. Filter by Date

Press `Ctrl+D` to open the date filter dialog:

**Quick filters** (press 1-6):
1. Last hour
2. Last 24 hours
3. Today
4. Yesterday
5. Last 7 days
6. Clear filter

**Custom range**: Tab to the From/To fields and enter:
- Relative times: `-2h`, `-30m`, `-7d`
- Specific dates: `2024-03-15 14:00`
- Keywords: `today`, `yesterday`, `now`

### 5. Exclude Unwanted Entries

Press `x` to open the exclude filter manager:

1. Type a pattern (e.g., `healthcheck`)
2. Press `Enter` to add it
3. Use `Ctrl+R` to toggle regex mode
4. All matching entries disappear from view

**Quick exclude**: `Alt+Click` on any word to instantly exclude it.

Press `X` to clear all exclude filters at once.

### 6. Expand Multi-line Entries

Some log entries span multiple lines (stack traces, JSON payloads). These appear collapsed by default.

| Key | Action |
|-----|--------|
| `Enter` | Expand/collapse selected entry |
| `a` | Expand all entries |
| `A` | Collapse all entries |
| `d` | Show detail popup (full-screen view) |

Embedded JSON is automatically pretty-printed when expanded.

### 7. Bookmarks

Mark important entries to jump back to them later:

| Key | Action |
|-----|--------|
| `m` | Toggle bookmark on current line |
| `b` | Jump to next bookmark |
| `B` | Jump to previous bookmark |

Bookmarks are saved per file and persist across sessions.

### 8. Jump to Errors and Warnings

Quickly navigate between problems:

| Key | Action |
|-----|--------|
| `e` | Jump to next error |
| `E` | Jump to previous error |
| `w` | Jump to next warning |
| `W` | Jump to previous warning |

### 9. Tail Mode

Press `t` to toggle tail mode. When enabled:
- New entries appear at the bottom
- The view auto-scrolls to show them
- Great for monitoring live applications

Tail mode pauses automatically when you scroll up, and resumes when you return to the bottom.

### 10. Merge Multiple Files

Open additional files with `M` (Shift+m). LogNav merges them into a single view:

- Entries are sorted by timestamp
- Each source file gets a distinct color indicator
- Useful for correlating events across services

### 11. Command Palette

Press `Ctrl+P` to open the command palette. Type to fuzzy-search all available commands. This is the fastest way to discover features.

### 12. Mouse Support

| Action | Effect |
|--------|--------|
| Click | Select entry |
| Scroll | Navigate up/down |
| `Ctrl+Click` | Search for word under cursor |
| `Alt+Click` | Exclude word under cursor |

## Supported Log Formats

LogNav auto-detects the format from file content.

### wd.log

Detailed format with level tokens and timestamps:

```
  INFO  02-03 18:11:02.577 [Alarm] SPL|Context "Server starting"
  TRACE 02-03 18:10:39.720 [#10] HTTP|Server "Request received"
* ERROR 02-05 11:23:38.795 [#34] API|Controller "Auth failed"
! WARN  02-05 11:23:38.801 [#10] HTTP|Server "Rate limit exceeded"
```

### wpc.log

Simpler format with 3-letter level prefix:

```
INF 03-21 14:23:01.234 Application started
ERR 03-21 14:23:02.456 Connection failed
  stack trace here...
```

### qconsole.log

Game server console logs with bracketed timestamps:

```
[2026-01-09 18:48:38 UTC+1.000] Server initialized
[2026-01-09 19:05:01 UTC+1.000] Script Error: file not found
```

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
| `1`-`6` | Toggle log levels |
| `0` | Reset level filters |
| `Ctrl+D` | Date filter |
| `x` | Exclude filter manager |
| `X` | Clear exclude filters |
| `t` | Toggle tail mode |
| `Alt+W` | Toggle word wrap |
| `s` | Toggle syntax highlighting |
| `Enter` | Expand/collapse entry |
| `a` / `A` | Expand / collapse all |
| `d` | Detail popup |
| `m` | Toggle bookmark |
| `b` / `B` | Next / previous bookmark |
| `e` / `E` | Next / previous error |
| `w` / `W` | Next / previous warning |
| `c` | Copy entry to clipboard |
| `o` or `Ctrl+O` | Open file |
| `M` | Merge file |
| `Ctrl+S` | Export filtered results |
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

## Configuration

Settings are stored in `~/.config/lognav/config.json`:

- **Recent files**: Last 10 opened files
- **Bookmarks**: Saved per file
- **Syntax highlighting**: Preference persisted

## Building

```bash
cargo build --release
```

Requires Rust 1.93+ (uses Edition 2024). Binary output: `target/release/lognav` (~4MB).

## Design Principles

- **Minimal chrome**: 95% of screen shows logs
- **Keyboard-first**: Every action has a key binding
- **Memory efficient**: Streams entries in batches for fast loading
- **Fast startup**: Async loading with immediate UI response
- **Unix philosophy**: Does one thing well
