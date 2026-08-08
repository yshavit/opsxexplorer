## Context

`App::handle_key` (`src/tui/app.rs:163`) dispatches global keys (currently just Tab) before routing to `handle_left_key`/`handle_right_key` based on `Focus`. The quit key is checked even earlier, in the render loop itself (`src/tui/mod.rs:42`), outside `App` entirely. Cursor movement in both panes already goes through delta-based helpers — `move_selection(delta: isize)` for the left pane and `move_cursor(delta: isize)` for the right pane — that clamp/skip non-selectable rows. Neither pane currently knows its own visible row count; the renderer computes layout geometry locally each frame and only feeds two pieces of it back into `App`: `max_h_scroll` (left pane) and `max_line_offset` (right pane), via setters called every frame before `handle_key` runs. See proposal.md for motivation.

## Goals / Non-Goals

**Goals:**
- Reuse the existing delta-based movement functions for half-page jumps rather than introducing a parallel movement path.
- Keep "half-page" defined in terms of selectable rows (consistent with how single-step movement already works), not rendered terminal lines.

**Non-Goals:**
- Pixel/line-exact vim parity for half-page scrolling in the right pane, where rows can wrap to multiple lines. Row-count-based paging is an approximation and that's acceptable here.
- Any change to how the right pane's scroll offset is computed (`recompute_diff`, keep-cursor-visible logic) — half-page movement only changes *how far* the cursor jumps, not how scrolling follows it.

## Decisions

**Quit key**: move the check in `mod.rs`'s event loop from `Char('q') + CONTROL` to `Char('q')` with no modifier. No interaction with `App`/`Focus` needed since it's handled before `app.handle_key`.

**Enter/Space focus-shift**: change `handle_left_key`'s `Enter | Char(' ')` arm to branch on the row under the cursor. On `Row::ArchivedHeader`, keep calling `toggle_archived_at_cursor` (unchanged). On `Row::Active`/`Row::Archived`, additionally set `self.focus = Focus::Right`. On `Row::Placeholder`, no-op (unreachable via cursor movement today, but handled for completeness). This stays entirely inside `handle_left_key` — no change to the global `Tab` handling in `handle_key`.

**Half-page size**: add `viewport_rows: usize` to both the left-pane and right-pane state in `App` (or a single field per pane, mirroring the existing split between `max_h_scroll` and `max_line_offset`), set by new setters (`set_left_viewport_rows` / `set_right_viewport_rows`, naming TBD at implementation time) that the renderer calls each frame from the same place it already computes `inner_width`/pane height for `render_left_pane`/`render_right_pane`. `Ctrl+d`/`Ctrl+u` then call `move_selection(viewport_rows / 2 as isize)` / `move_cursor(viewport_rows / 2 as isize)` (sign flipped for up) — no new movement logic, just a bigger delta through the existing clamped path.

Alternative considered: hardcode a fixed row count (e.g. 10) for "half page" instead of deriving it from the render. Rejected — it wouldn't scale with terminal size, and the codebase already has a two-instance precedent (`max_h_scroll`, `max_line_offset`) for exactly this render-computes/App-stores pattern, so following it is cheaper than inventing a new constant to tune.

## Risks / Trade-offs

- [Half-page is row-count-based, not line-count-based, in the right pane where rows wrap] → Acceptable per Non-Goals; document the approximation in the spec scenario wording ("roughly half") rather than promising exact vim parity.
- [`viewport_rows` becomes stale for one frame after a resize, same as the existing `max_h_scroll`/`max_line_offset` fields] → Consistent with existing behavior for those fields; not a new class of bug.
