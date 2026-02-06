# Issue: Improve UI discoverability and in-app documentation

## Priority
Medium

## Description
The UI should be intuitive with clear labels so users don't need external documentation to use the TUI. Currently, many features are hidden or require prior knowledge to discover.

## Current State

### Strengths
- ✅ Command palette (Ctrl+P) shows all commands with shortcuts
- ✅ Inline help hints in dialogs (search, date filter, file open)
- ✅ Status bar shows current state (file, counts, filters, tail mode)
- ✅ Consistent visual feedback with color coding

### Missing or Unclear

#### 1. **No Help Screen** ❌
- No `?` or `F1` key to show help
- No first-time user guidance
- Users must either know commands or discover via Ctrl+P
- No explanation of features, modes, or format syntax

#### 2. **Level Filter Numbers Hidden** ⚠️
- Keys 1-6 toggle log levels (ERR/WRN/INF/DBG/TRC/PRF)
- Status bar shows active levels: "ERR WRN INF DBG"
- But nowhere shows which number maps to which level
- Default state (1-4 on, 5-6 off) not explained
- Users won't discover this feature without documentation

#### 3. **Expand/Collapse Feature Hidden** ⚠️
- Enter expands/collapses continuation lines
- Visual indicator shows `[+3]` or `[-3]`
- No hint that Enter toggles it
- Many users won't discover this

#### 4. **Search Mode Ambiguity** ⚠️
Current: `/ query | C-n/C-p: navigate | Enter: filter | Esc: cancel`
- Good inline help BUT:
  - Unclear distinction between navigating matches vs. applying filter
  - Not obvious that matches show in real-time
  - No hint on how to clear applied filter

#### 5. **Quit Behavior Undocumented** ⚠️
- Normal mode: `q` quits
- All dialogs: `Esc` closes dialog (not app)
- No in-app indication of this

#### 6. **Horizontal Scroll Invisible** ⚠️
- No indicator showing horizontal scroll position
- No hint that h/l or arrow keys scroll horizontally
- Users with long lines may not realize scrolling is possible

#### 7. **File Open Dialog Ambiguous** ⚠️
Shows: `Esc: cancel | Enter: open | Tab: fill | ^U: clear`
- Missing context:
  - What does "Tab: fill" do? (copies selected recent file)
  - Up/Down navigate recent files
  - ^W deletes path segment
  - Tilde expansion supported

#### 8. **Mouse Support Undocumented** ⚠️
- Code supports mouse scroll and click (`src/events.rs:566-590`)
- Never mentioned in UI
- Users may try to use mouse and be surprised when it works (or doesn't know it's available)

#### 9. **No Visual Feedback for Toggle Actions** ⚠️
- Toggling levels, wrap, tail updates status bar
- But easy to miss, no confirmation message
- Could benefit from brief status flash: "Word wrap enabled"

#### 10. **Status Bar Limited Context** ⚠️
Current: `filename | X/Y entries | ERR WRN INF DBG | [TAIL] | Ctrl+P`
- Missing:
  - Current mode indicator (NORMAL | SEARCH | etc.)
  - Wrap status more visible ("Wrap: ON/OFF")
  - Hint that `?` shows help (once implemented)

## Proposed Improvements

### High Priority

#### 1. Add Help Screen (`?` or `F1` key)
**Implementation:** `src/events.rs` - Add new `FocusState::Help` state

**Content:**
```
┌───────────── LogNav Help ─────────────┐
│                                        │
│  NAVIGATION                            │
│    j/↓       Next entry                │
│    k/↑       Previous entry            │
│    g/Home    Go to top                 │
│    G/End     Go to bottom              │
│    h/l/←/→   Scroll horizontally       │
│    Enter     Expand/collapse entry     │
│    Mouse     Scroll and select         │
│                                        │
│  SEARCH & FILTER                       │
│    /         Search (C-n/C-p to nav)   │
│    C-d       Date range filter         │
│    1-6       Toggle levels (1:ERR...6:PRF) │
│                                        │
│  VIEW                                  │
│    w         Toggle word wrap          │
│    t         Toggle tail mode          │
│    o/C-o     Open file                 │
│                                        │
│  OTHER                                 │
│    C-p       Command palette           │
│    q         Quit                      │
│    ?         This help                 │
│                                        │
│  ESC closes dialogs without quitting   │
└────────────────────────────────────────┘
```

**Files to modify:**
- `src/app.rs:22-34` - Add `Help` variant to `FocusState`
- `src/events.rs` - Add `?` key handler
- `src/ui.rs` - Add help dialog rendering

#### 2. Improve Status Bar
**Current:** `src/ui.rs:326-366`

**Proposed changes:**
- Add mode indicator: `[NORMAL]`, `[SEARCH]`, `[HELP]`
- Make wrap status clearer: "Wrap:ON" or "Wrap:OFF"
- Add help hint when in normal mode: "? for help"
- Show horizontal scroll position when scrolled: "Col:45"

**Example:**
```
[NORMAL] file.log | 1234/5000 | ERR WRN INF DBG | Wrap:ON | [TAIL] | ? help
```

#### 3. Add Level Filter Legend
**Option A:** Show in help screen (covered above)

**Option B:** Add tooltip to status bar (on hover or temporary display)
- When user first presses 1-6, show brief message: "Level filters: 1:ERR 2:WRN 3:INF 4:DBG 5:TRC 6:PRF"

**Files:** `src/events.rs` - Add status message when level toggled

### Medium Priority

#### 4. Enhance Command Palette
**Current:** `src/ui.rs:382-456`

**Proposed:**
- Show current state in command names: "Toggle Tail [ON]" or "Toggle Tail [OFF]"
- Group commands by category:
  - File: Open, ...
  - Search: Search, Date Filter, ...
  - View: Toggle Wrap, Toggle Tail, ...
  - Navigation: Go to Top, Go to Bottom, ...
- Add brief descriptions (optional, may clutter)

**Files:** `src/app.rs:60-116` - Modify command definitions

#### 5. Improve Search UX
**Current:** `src/ui.rs:314-324`

**Proposed:**
- Better label: `Search: query | C-n/C-p:next/prev | Enter:filter on | Esc:close`
- Or: `/ query (live search) | Enter:keep filter | Esc:cancel`
- Add hint about clearing filter (press `/` again and clear, then Esc)

**Files:** `src/ui.rs:314-324` - Update search bar help text

#### 6. Add Expand Indicator Clarity
**Current:** `[+3]` or `[-3]` shown after entry

**Proposed:**
- Keep as-is (concise) but add to help screen
- OR change to: `[+3 ↵]` (Enter symbol)
- OR add status bar hint when on expandable entry: "Press Enter to expand"

**Files:** `src/ui.rs` - Rendering logic for continuation indicators

#### 7. Status Messages for Toggle Actions
**Implementation:** Brief confirmation messages in status bar

**Examples:**
- "Word wrap enabled"
- "Tail mode OFF"
- "Level ERR disabled"
- Auto-clear after 2-3 seconds or next action

**Files:**
- `src/app.rs:146` - `status_message: Option<String>` (already exists!)
- `src/events.rs` - Set status message on toggle actions (lines 38-90)

### Low Priority

#### 8. File Dialog Enhancement
**Current:** `src/ui.rs:520-587`

**Proposed:**
- Add tooltips: "Tab: fill from recent | ^W: delete segment | ~/: expand home"
- Show count of recent files: "Recent (5)"

**Files:** `src/ui.rs:520-587` - Update help text

#### 9. Horizontal Scroll Indicator
**Proposed:**
- Show in status bar when scrolled: "Col: 45"
- Only visible when `horizontal_scroll > 0`

**Files:**
- `src/app.rs:156` - `horizontal_scroll: usize` state exists
- `src/ui.rs:326-366` - Add to status bar rendering

#### 10. Document Mouse Support
- Add to help screen (covered in #1)
- Mention in README

## Implementation Strategy

**Phase 1: Core Help**
1. Implement help screen (`?` key) - highest value for discoverability
2. Improve status bar with mode indicator and help hint

**Phase 2: Polish**
3. Add status messages for toggle actions (already have infrastructure)
4. Enhance search and dialog help text
5. Show level filter legend in help screen

**Phase 3: Nice-to-Have**
6. Command palette grouping and state indicators
7. Horizontal scroll indicator
8. Expand indicator improvements

## Files to Modify

### Primary
- `src/app.rs` - Add Help focus state, update status messages
- `src/ui.rs` - Help dialog rendering, status bar improvements
- `src/events.rs` - Help key handler (`?`), status messages on toggles

### Secondary
- `src/ui.rs` - Dialog help text improvements
- `README.md` - Ensure help screen matches documentation

## Benefits
- New users can learn the tool without external docs
- Reduces support burden (fewer "how do I...?" questions)
- Makes advanced features discoverable
- Improves overall user satisfaction and retention

## Related Issues
- #005 - Date range dialog improvements (mentions better help text)
