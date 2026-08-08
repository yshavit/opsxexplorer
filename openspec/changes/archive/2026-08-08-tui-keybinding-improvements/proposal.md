## Why

Three small keyboard-navigation frictions have accumulated in the TUI: quitting requires an awkward Ctrl+Q instead of the conventional `q`, selecting a change in the left pane has no direct way to jump into its diff in the right pane, and there's no way to move by more than one row at a time through long lists. Each is minor on its own, but together they're cheap to fix in one pass.

## What Changes

- Pressing `q` (no modifier) quits the application. **BREAKING**: Ctrl+Q no longer quits.
- In the left pane, pressing `Enter` or `Space` on an Active or Archived change row shifts focus to the right pane (in addition to selecting/loading that change's diff, which already happens on cursor movement). On the archived-header row, `Enter`/`Space` keeps its current behavior (toggling the archived section) rather than shifting focus, since the right pane has nothing to show for that row.
- `Ctrl+u` / `Ctrl+d` move the cursor up/down by a half-page of selectable rows in whichever pane holds focus (left pane: change rows; right pane: diff rows), clamped at the ends the same way single-step `j`/`k` are. Half-page size is derived from the pane's current visible row count, reported by the renderer each frame (mirroring the existing `set_max_h_scroll`/`set_max_line_offset` pattern).

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `tui`: quit key changes from Ctrl+Q to `q`.
- `tui-changelist`: `Enter`/`Space` on a change row now also moves focus to the right pane; `Ctrl+u`/`Ctrl+d` half-page cursor movement is added.
- `tui-specdiff`: `Ctrl+u`/`Ctrl+d` half-page cursor movement is added.

## Impact

- `src/tui/mod.rs`: quit-key check in the event loop; new per-frame reporting of visible row counts for both panes.
- `src/tui/app.rs`: `handle_left_key`/`handle_right_key` gain `Ctrl+u`/`Ctrl+d` branches; `Enter`/`Space` handling in the left pane branches on row type; new `viewport_rows` state (or equivalent) and setters for both panes.
- No changes to on-disk formats, CLI flags, or non-interactive behavior.
