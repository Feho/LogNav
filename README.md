# LogViewer

A fast, modern TUI log file viewer built in Rust.

## Why

Debugging applications often means sifting through massive log files. Traditional tools like `less`, `grep`, and `tail -f` work, but switching between them breaks flow. GUI log viewers are powerful but heavy—they don't belong in a terminal-first workflow.

LogViewer solves this by combining:
- **Live tailing** with instant filtering
- **Severity-based coloring** so errors jump out
- **Regex search** without leaving the viewer
- **Date range filtering** to zoom into incidents
- **Minimal UI** that maximizes log visibility

All in a single, fast binary with vim-style navigation.

## What

A terminal UI application that displays, filters, and live-tails log files. Supports two proprietary log formats used in internal tooling:

### Supported Formats

**wd.log** - Lines with prefix (2 spaces or marker `*`/`!`) followed by level token:
```
  ~~~~~ 02-03 18:10:37.564 [T32289|#6] HTTP|Connection  "Client disconnected"
  ===== 02-03 18:11:02.570 [Alarm] SCHED|Scheduler      "Processing 1 alarm(s)..."
  INFO  02-03 18:11:02.577 [Alarm] SPL|Context          "Server status: cpu=5.1%"
  TRACE 02-03 18:10:39.720 [#10] HTTP|Server            "200 POST /api/ping"
* ERROR 02-05 11:23:38.795 [#34] API|Controller         "Auth failed"
! WARN  02-05 11:23:38.801 [#10] HTTP|Server            "401 POST /api/list"
```

**wpc.log** - Lines starting with level token:
```
INF 03-21 14:23:01.234 Application started
ERR 03-21 14:23:02.456 Connection failed
  error details here
```

Format is auto-detected from file content.

## Usage

```bash
# Open a log file
logviewer /path/to/app.log

# Or launch empty and use Ctrl+O to open
logviewer
```

## Key Bindings

| Key | Action |
|-----|--------|
| `Ctrl+P` | Command palette (fuzzy search all commands) |
| `Ctrl+O` | Open file |
| `/` or `Ctrl+F` | Search logs (regex) |
| `Ctrl+D` | Filter by date range |
| `1-6` | Toggle level filters (ERR/WRN/INF/DBG/TRC/PRF) |
| `t` | Toggle tail mode (auto-scroll to new entries) |
| `w` | Toggle word wrap |
| `j/k` or `↑/↓` | Scroll up/down |
| `g/G` | Jump to top/bottom |
| `Ctrl+U/D` | Page up/down |
| `h/l` or `←/→` | Horizontal scroll |
| `n/N` | Next/previous search match |
| `q` or `Esc` | Quit |

Mouse scroll and click are also supported.

## Log Levels

| Level | Color | wd.log Token | wpc.log Token |
|-------|-------|--------------|---------------|
| Error | Red | `ERROR` | `ERR` |
| Warn | Yellow | `WARN` | `WRN` |
| Info | White | `INFO` | `INF` |
| Debug | Cyan | `=====` | `DBG` |
| Trace | Gray | `TRACE` | - |
| Profile | Magenta | `~~~~~` | - |

By default, only ERR/WRN/INF are shown. Toggle others with number keys.

## Building

```bash
cargo build --release
```

Binary will be at `target/release/logviewer` (~4MB).

## Requirements

- Rust 1.70+ (uses `LazyLock`)
- Terminal with 256-color support

## Design

- **Minimal chrome**: 95% of screen dedicated to logs
- **Command palette**: Single entry point for all actions (Ctrl+P)
- **Vim-style**: Familiar navigation for terminal users
- **Memory efficient**: Caps at 500k entries, drops oldest when exceeded
- **Fast startup**: Async file loading, immediate UI response
