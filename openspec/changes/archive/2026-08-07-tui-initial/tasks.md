## 1. Application shell

- [x] 1.1 Add a `src/tui` module wired into `main.rs`, replacing the current `println!`
- [x] 1.2 Set up the crossterm terminal lifecycle: enter raw mode + alternate screen on start, restore on exit (including panics)
- [x] 1.3 Implement the main event loop: read `crossterm::event::Event`, dispatch key events, redraw via `terminal.draw(...)`
- [x] 1.4 Wire Ctrl+Q to exit the loop cleanly

## 2. Two-pane layout

- [x] 2.1 Split the frame into left/right panes (`ratatui::layout::Layout`)
- [x] 2.2 Render the right pane as an empty bordered placeholder block
- [x] 2.3 Construct `App` at startup from `Changes::discover`, holding `active`/`archived` changes, `archived_expanded: bool` (initially `false`), and `ListState`

## 3. Row model and rendering

- [x] 3.1 Define the `Row` enum (`Active`, `ArchivedHeader`, `Archived`, `Placeholder`) per design.md
- [x] 3.2 Implement row flattening: active changes, then the `archived` header, then (if expanded) archived changes — substituting `Placeholder` rows for empty sections
- [x] 3.3 Convert `Row` values to `ratatui::widgets::ListItem`, including indentation and the `▸`/`▾` marker on the archived header only; render archived changes as a dimmed `archive_date()` span followed by a normal-style `display_name()` span, falling back to `display_name()` alone when `archive_date()` is `None`
- [x] 3.4 Render the left pane with `List` + `ListState`, bordered, with a highlight style for the selected row

## 4. Selection and navigation

- [x] 4.1 Initialize `ListState` selection to row 0 on startup
- [x] 4.2 Handle Up/`k` and Down/`j`: move selection by one row, skipping `Placeholder` rows
- [x] 4.3 Handle Enter/Space: toggle `archived_expanded` when the selected row is the `ArchivedHeader`; no-op otherwise
- [x] 4.4 On collapsing the archived section, reset selection to the (rebuilt) index of the `ArchivedHeader` row

## 5. Verification

- [x] 5.1 Unit tests for row flattening: active-only, archived-only, both empty, both populated, expanded vs collapsed
- [x] 5.2 Unit tests for navigation: skipping placeholders in both directions, collapsing while a child is selected snaps to the header
- [x] 5.3 Manually run the TUI against this repo's own `openspec/changes/` to confirm layout, sorting, and toggle behavior match specs/tui and specs/tui-changelist
