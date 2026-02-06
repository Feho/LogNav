# Issue: Date range dialog UX improvements

## Priority
Medium

## Description
The current date range filter dialog (Ctrl+D) requires manual typing of dates in specific formats, which is tedious and error-prone. The UX could be significantly improved with pre-defined periods and more flexible partial date input.

## Current Implementation

**Dialog UI** (`src/ui.rs:472-518`):
- Two text input fields: "From" and "To"
- Format hint: "MM-dd HH:mm"
- Tab to switch fields, Enter to apply

**Date Parsing** (`src/events.rs:592-616`):
Supports 4 formats:
- `%Y-%m-%d %H:%M:%S` (e.g., "2024-12-25 14:30:00")
- `%Y-%m-%d %H:%M` (e.g., "2024-12-25 14:30")
- `%m-%d %H:%M:%S` (assumes current year, e.g., "12-25 14:30:00")
- `%m-%d %H:%M` (assumes current year, e.g., "12-25 14:30")

**Filtering Logic** (`src/app.rs:276-289`):
- Filters entries where timestamp is outside `[date_from, date_to]` range
- Both bounds are optional (can filter just from or just to)

## Pain Points

1. **No pre-defined periods** - Common use cases like "Last hour", "Last 24 hours", "Today", "Yesterday" require manual calculation and typing
2. **Requires full date+time entry** - Even when user just wants to filter by date (not time), they must type the full format
3. **No date picker or shortcuts** - Users must type everything manually
4. **No smart defaults** - Common patterns like "from start of today" require full timestamp
5. **Error feedback unclear** - Invalid dates fail silently (no error message shown)
6. **No clear indication of active filter** - Status bar shows "From: X To: Y" but could be more prominent

## Proposed Improvements

### 1. Pre-defined Quick Filters
Add a list of common time ranges that can be selected with a single key:
- `1` - Last hour
- `2` - Last 24 hours
- `3` - Today (00:00:00 to now)
- `4` - Yesterday (full day)
- `5` - Last 7 days
- `6` - Custom (opens current manual input dialog)

**Implementation approach:**
- Add new `FocusState::DateFilterQuick` state
- Show quick filter menu first, custom input on demand
- Calculate absolute timestamps based on current time

### 2. Smarter Partial Date Parsing
Allow more flexible input formats:
- **Date only**: "12-25" → assumes 00:00:00 for From, 23:59:59 for To
- **Time only**: "14:30" → assumes today
- **Relative times**: "-2h" (2 hours ago), "-30m" (30 minutes ago)
- **Keywords**: "today", "yesterday", "now"

**Example usage:**
- From: "12-25", To: "12-26" → Full day on Dec 25
- From: "-1h", To: "" → Last hour
- From: "14:00", To: "" → From 2pm today onwards

### 3. Improved UI

**Option A: Two-stage dialog**
1. First screen: Quick filter options (numbered list)
2. If "Custom" selected: Current manual input dialog

**Option B: Combined dialog**
```
┌─────────── Date Range Filter ───────────┐
│                                          │
│  Quick Filters:                          │
│    1. Last hour                          │
│    2. Last 24 hours                      │
│    3. Today                              │
│    4. Yesterday                          │
│    5. Last 7 days                        │
│                                          │
│  Or enter custom range:                  │
│    From: [________________]              │
│    To:   [________________]              │
│                                          │
│  Formats: MM-dd HH:mm, -2h, today        │
│  Tab: switch | Enter: apply | Esc: close │
└──────────────────────────────────────────┘
```

### 4. Better Error Feedback
- Show parse error inline: "Invalid date format" in red below field
- Highlight invalid field in red
- Keep dialog open on error (don't close and lose input)

### 5. Clear Active Filter Indicator
- Status bar could show: `📅 12-25 00:00 → 12-26 00:00` or `⏱ Last hour`
- Add command to clear date filter (currently no way except re-opening dialog and clearing both fields)

## Implementation Files
- `src/app.rs` - Add quick filter logic, expand date parsing
- `src/ui.rs:472-518` - Redesign dialog UI
- `src/events.rs:285-347` - Handle quick filter keys, improve parsing
- `src/events.rs:592-616` - Expand `parse_date_input()` function

## Benefits
- Faster filtering for common use cases (1-2 keystrokes instead of 10+)
- Less error-prone (pre-defined ranges are always valid)
- More intuitive for new users
- Maintains full flexibility with custom input option

## Related Issues
None

## Dependencies
- chrono crate (already in use for date parsing)
