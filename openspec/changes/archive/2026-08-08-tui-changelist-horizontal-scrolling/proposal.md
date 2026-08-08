## Why

The left changelist pane is a fixed 35% of terminal width, and row text (change names, and archived rows' date + name) can be wider than that. Today ratatui silently clips overflow with no indicator and no way to see the rest — the user has no way to read a truncated row.

## What Changes

- Add a single global horizontal scroll offset for the left pane's content. `h`/`l` and the left/right arrow keys scroll by one column; `^`/`$` and Home/End jump to the leftmost/rightmost extent.
- The entire pane scrolls as one unit: every row (active, the `▸ archived` header, and archived rows) shifts by the same offset, preserving vertical alignment across rows.
- Render a horizontal scrollbar (tied to the same offset/max) so the user can see when content extends beyond the visible width and how far they've scrolled.
- The offset is never explicitly reset. It's clamped live at render time against the current rows and current pane width, so it self-corrects on window resize, on selection moving to a shorter row, and on collapsing/expanding the `archived` section — without a dedicated reset rule for any of those.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `tui-changelist`: adds horizontal scrolling of the left pane's row content, including scroll keybindings, the scrollbar indicator, and the clamping/persistence behavior of the scroll offset across selection changes, section collapse/expand, and resize.

## Impact

- `src/tui/app.rs`: `App` gains horizontal scroll offset state and key handling for `h`/`l`/arrows/`^`/`$`/Home/End.
- `src/tui/mod.rs`: `render_left_pane` applies the clamped offset when building row text and renders a horizontal `Scrollbar`.
- `src/tui/row.rs`: no structural change expected; row text stays the source that gets offset at render time.
- Out of scope: making the 35/65 left/right pane split adjustable. That's a separate future change addressing the broader real-estate tension between the changelist and the (currently placeholder) right pane.
