## 1. Archived row label and style

- [x] 1.1 In `row_spans` (`src/tui/mod.rs`), change the `Row::ArchivedHeader` arm to render `format!("{marker} archived/")` with an unconditional, unstyled `Style::new()` — remove the `if !*expanded { ... UNDERLINED }` branch entirely.
- [x] 1.2 Update/replace the underline-focused unit tests in `src/tui/mod.rs` (`collapsed_archived_header_is_underlined_and_expanded_is_not`, `collapsed_archived_header_underline_persists_when_scrolled`) with tests asserting: the label is `archived/` in both states, and the only difference between collapsed/expanded spans is the marker glyph.

## 2. Row-set helper for width/scroll computation

- [x] 2.1 Add `App::all_rows(&self) -> Vec<Row<'_>>` in `src/tui/app.rs`, calling `row::flatten(&self.changes.active, &self.changes.archived, true)` (always expanded), for use by width/scroll-max computation independent of the current `archived_expanded` state.

## 3. Content-driven left pane width

- [x] 3.1 In `render()` (`src/tui/mod.rs`), compute `widest_row_width(&app.all_rows())` before building the outer `Layout::horizontal`, and derive the left pane's width as `min(frame.width * 35 / 100, content_max + 1 + 2)` (1-column buffer + 2 border columns).
- [x] 3.2 Replace the `Layout::horizontal([Constraint::Percentage(35), Constraint::Percentage(65)])` split with `Constraint::Length(<computed width>)` for the left pane and `Constraint::Min(0)` for the right pane.
- [x] 3.3 Add unit/integration coverage: narrow content shrinks the pane and grows the right pane; wide content caps the pane at the 35% share; pane width is identical whether `archived_expanded` is true or false, for the same underlying change set.

## 4. Scroll clamping and scrollbar over all rows

- [x] 4.1 In `render_left_pane`, switch the `widest_row_width(&rows)` call (currently over `app.rows()`) to use `app.all_rows()` instead, so `max_scroll` no longer depends on `archived_expanded`.
- [x] 4.2 Update the scroll-related tests/scenarios that assumed collapsing archived reduces the scroll range (per `openspec/specs/tui-changelist/spec.md`'s modified requirement) to assert the range is unchanged across expand/collapse instead.

## 5. Hide the left scrollbar when nothing is scrollable

- [x] 5.1 In `render_left_pane`, guard the `Scrollbar`/`ScrollbarState` rendering with `if max_scroll == 0 { return; }` (or equivalent early return after the `List` widget is rendered), mirroring `render_right_scrollbar`'s existing pattern.
- [x] 5.2 Add a test asserting no scrollbar is drawn when all rows (including collapsed archived ones) fit within the pane, and that it still renders when content is wider than the pane.

## 6. Verification

- [x] 6.1 Run `cargo test` and confirm all left-pane tests pass, including the new/updated ones from tasks 1–5.
- [x] 6.2 Run the TUI manually (`cargo run`) against a repo with both narrow and long change/spec names, in both archived-collapsed and archived-expanded states, to confirm: no underline, `archived/` label, stable pane width across toggling, and no scrollbar when content fits.
