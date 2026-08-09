## Context

The left pane's rendering lives entirely in `src/tui/mod.rs` (`render`,
`render_left_pane`, `row_spans`, `row_display_width`, `widest_row_width`)
plus `src/tui/app.rs` (`App::rows`, which flattens `changes.active` and
`changes.archived` via `row::flatten` according to `archived_expanded`) and
`src/tui/row.rs` (`Row` enum and `flatten`). See proposal.md for why these
three rendering details are changing.

Two facts about the current implementation matter for this design:

- The left/right split is a hardcoded `Layout::horizontal([Percentage(35),
  Percentage(65)])` in `render()`. `widest_row_width` today only feeds the
  horizontal-scroll/scrollbar math inside that fixed-width pane — it has
  no effect on the pane's actual size.
- `widest_row_width` is called on `app.rows()`, which is `row::flatten`
  applied with the *current* `archived_expanded` state. Collapsed archived
  rows are simply absent from that vector, so today's scroll/scrollbar
  math already varies with expand/collapse — which is the source of the
  width-instability bug in ask #2.

## Goals / Non-Goals

**Goals:**
- Make the left pane's rendered width a function of its content (all rows,
  archived included, regardless of expand state), capped at the existing
  35% default share.
- Make that same "all rows" width the basis for horizontal scroll
  clamping and scrollbar state, so behavior is consistent whether the
  archived section happens to be expanded or not.
- Remove the archived-row underline; relabel it `archived/`.
- Hide the left scrollbar when nothing is scrollable, matching the right
  pane's existing `render_right_scrollbar` early-return.

**Non-Goals:**
- Changing the 35%/65% default split ratio itself, or making it
  user-configurable.
- Changing right-pane layout or scrolling.
- Changing sort order, placeholder text, or navigation behavior.

## Decisions

**Compute the "all rows" set once, independent of `archived_expanded`.**
Add a helper — e.g. `App::all_rows(&self) -> Vec<Row<'_>>` — that calls
`row::flatten(&self.changes.active, &self.changes.archived, true)`
unconditionally (always expanded), used solely for width/scroll-max
computation. `App::rows()` keeps returning the currently-visible set for
actual list rendering and cursor navigation; nothing about navigation
changes. Alternative considered: thread an `include_collapsed` flag
through `flatten` itself — rejected, since `flatten`'s existing signature
already expresses "what's visible," and reusing it with the opposite
argument is simpler than adding a second mode to it.

**Compute the pane's width before building the layout, not inside
`render_left_pane`.** `render()` needs the content-driven width to decide
the `Constraint` for the left column, so `widest_row_width(&app.all_rows())`
must run in `render()` (or a small helper it calls) before
`Layout::horizontal(...)` executes, then pass a `Constraint::Length(width)`
for the left pane and let the right pane take `Constraint::Min(0)` (or the
remainder). The cap is `min(35% of frame.width, content_max + 1 +
borders)`; borders are 2 columns (one per side), matching the pane's
existing `Block::bordered()`. Computing "35% of frame" still requires
`frame.area().width`, so the percentage math itself is inlined as
`frame.width * 35 / 100` rather than relying on `Layout`'s own
percentage resolution, since that resolution is exactly what we're now
choosing between.

**`widest_row_width` and the scrollbar/scroll-clamp math in
`render_left_pane` switch from `app.rows()` to `app.all_rows()`.** This is
the same underlying fix as the width decision above — both the pane's
outer width and its inner scroll math need to stop varying with
`archived_expanded` — so both call sites move together. `row_display_width`
and `row_spans` themselves are untouched; only which row set feeds them
changes.

**Scrollbar visibility mirrors `render_right_scrollbar`'s existing
pattern.** Add the same `if max_scroll == 0 { return; }` guard (after
rendering the `List` widget, before rendering the `Scrollbar`) rather than
inventing a new mechanism — the right pane already established this
convention for exactly this situation.

**Archived label: change the string and drop the conditional style, no
new state.** `row_spans`'s `Row::ArchivedHeader` arm becomes
`format!("{marker} archived/")` with a single unconditional `Style::new()`
(the file's other rows already default to unstyled `Style::new()`, so
this doesn't need it own constant). No `Row` enum or `flatten` changes are
needed — `expanded` still selects the marker glyph, just not a style
anymore.

## Risks / Trade-offs

- [The content-driven width computation must run before the outer
  `Layout::horizontal` call, which is a structural reordering of
  `render()` — a bit more invasive than the other two asks] → Contained
  entirely within `render()`/`render_left_pane`; no change to `App`'s
  public surface beyond the new `all_rows()` accessor, and no change to
  `row.rs`'s `flatten` signature.
- [Because the width and scroll-max calculations now always account for
  collapsed archived rows, a pane that fit its active changes comfortably
  could still show a horizontal scrollbar (or be capped at 35% instead of
  shrinking) on account of an archived row's length the user can't
  currently see] → This is the intended, requested behavior (called out
  explicitly in proposal.md's Impact section) — noting it here so it
  isn't mistaken for a regression during review.
- [`row_display_width` assumes character count equals rendered column
  width (documented already in that function's doc comment, for plain
  ASCII/kebab-case content) — unchanged by this design, but now also
  underpins the pane's outer width, not just its scroll math, so any
  future wide-character content in change names would skew the pane's
  sizing as well as its scrolling] → No mitigation needed now; same
  assumption the codebase already made, just a wider blast radius if it's
  ever violated.
