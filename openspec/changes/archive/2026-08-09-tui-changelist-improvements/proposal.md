## Why

Three small usability rough edges in the left pane's rendering: the collapsed
`archived` row's underline reads as a link/affordance hint rather than a
label, the pane's fixed 35%-of-frame width wastes columns when actual
content is narrower (and inconsistently changes as the archived section is
expanded/collapsed), and the horizontal scrollbar always renders even when
there's nothing to scroll. All three are cheap, independent fixes to the
same rendering path, worth bundling into one change.

## What Changes

- The `archived` row drops its underline styling entirely and is always
  labeled `archived/` (trailing slash), rendering identically whether
  collapsed or expanded except for the disclosure triangle (`▸`/`▾`).
- **BREAKING** (behavior, not API): the left pane's width is no longer a
  fixed 35% of the frame. It is capped at `min(35% of frame, widest row's
  content width + 1-column buffer + borders)`, computed over *all* rows —
  active, the archived header, and every archived change — regardless of
  whether the archived section is currently expanded. This removes dead
  space when content is narrow, and keeps the pane's width constant across
  expand/collapse of the archived section. When content is wider than the
  35% cap, horizontal scrolling still applies exactly as before.
- The left pane's horizontal scrollbar no longer renders when the pane's
  content already fits (matching the right pane's existing
  render-nothing-when-nothing-to-scroll behavior), instead of rendering in
  an always-visible empty/full state.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `tui-changelist`: the archived-row underline requirement and the
  "clamped to currently visible rows" scroll requirement are each replaced
  (removed and re-added under new requirement names, since their scenario
  sets change rather than just their wording) by a plain `archived/`-label
  requirement and a "clamped to all rows, including collapsed archived
  ones" requirement, respectively; the scrollbar requirement is modified
  in place, from "shows an empty/full state" to "does not render at all"
  when nothing is scrollable; and a new requirement is added for the
  pane's content-driven width cap.

## Impact

- `src/tui/mod.rs`: `render()` (left/right layout split), `render_left_pane`
  (scrollbar visibility), `row_spans` (archived label/underline),
  `widest_row_width` (must be computed over the full row set, not just the
  currently visible one).
- `src/tui/app.rs`: likely needs a way to get the full row set (active +
  archived, as if expanded) independent of `archived_expanded`, for width
  computation.
- `openspec/specs/tui-changelist/spec.md`: requirement text and scenarios
  for the archived-row underline, horizontal scroll clamping, and
  scrollbar visibility all need updating; a new requirement for pane width
  is added.
- No changes to diffing, spec parsing, or the right pane.
