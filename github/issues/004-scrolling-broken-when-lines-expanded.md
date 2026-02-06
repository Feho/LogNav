# Issue: Scrolling broken when lines are expanded

## Priority
High

## Description
When a user expands a log entry with continuation lines (by pressing Enter), the scrolling behavior breaks. The highlighted/selected line can scroll off-screen and become invisible, especially when navigating to the bottom of the viewport.

## Root Cause
The scrolling system uses entry-based indexing (`scroll_offset` and `selected_index` track entry indices) but the viewport is measured in visual lines. When entries are expanded, they consume multiple visual lines, but the scroll calculations assume 1 entry = 1 visual line.

### Affected Code Locations

**Entry-based scroll state:**
- `src/app.rs:154` - `scroll_offset: usize` (entry index, not visual line index)
- `src/app.rs:155` - `selected_index: usize` (selected entry in filtered list)

**Broken calculations:**
- `src/app.rs:507-508` - `ensure_selected_visible_with_height()` assumes 1:1 mapping
  ```rust
  } else if self.selected_index >= self.scroll_offset + viewport_height {
      self.scroll_offset = self.selected_index - viewport_height + 1;
  }
  ```
  This fails when entries above the viewport are expanded.

- `src/events.rs:576-586` - Mouse click selection also assumes 1 entry = 1 visual line
  ```rust
  let clicked_row = mouse.row as usize;
  let target_index = app.scroll_offset + clicked_row;
  ```

**Expansion state:**
- `src/app.rs:160` - `expanded_entries: HashSet<usize>` tracks which entries are expanded
- `src/app.rs:591-608` - Toggle and query expansion state

## Steps to Reproduce
1. Open a log file with entries that have continuation lines
2. Expand one or more entries near the top (press Enter)
3. Navigate down (press `j` or Down arrow) toward the bottom of the viewport
4. Observe: The highlighted line scrolls off the bottom of the screen and becomes invisible

## Expected Behavior
When navigating up/down, the selected line should always remain visible in the viewport, even when entries are expanded.

## Proposed Solution
The scrolling system needs to be refactored to track visual lines rather than entry indices. This requires:

1. **Calculate visual line positions** - Build a mapping of entry indices to visual line ranges, accounting for expanded entries
2. **Update scroll calculations** - Modify `ensure_selected_visible_with_height()` to work with visual lines
3. **Fix mouse click handling** - Convert clicked visual row to correct entry index by accounting for expanded entries above

### Alternative Approach
Keep entry-based indexing but calculate the actual visual height consumed by entries in the viewport when determining scroll position. This would require iterating through entries to count visual lines.

## Impact
- Users cannot reliably navigate expanded entries
- Makes the expand feature nearly unusable for longer continuation lines
- Breaks user expectation of selection always being visible

## Related Files
- `src/app.rs` - Scroll state and logic
- `src/ui.rs` - Rendering (lines 44-269)
- `src/events.rs` - Navigation and mouse handling
