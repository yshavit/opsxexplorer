## 1. Horizontal scroll state and key handling

- [x] 1.1 Add horizontal scroll state to `App` (`src/tui/app.rs`): the scroll offset (initialized to 0), and a cached `max_scroll` from the most recent render (initialized to 0), used by `$`/`End`.
- [x] 1.2 Extend `App::handle_key` to handle `h`/`Left` (scroll left 1, saturating at 0), `l`/`Right` (scroll right 1), `^`/`Home` (jump to 0), `$`/`End` (jump to the cached `max_scroll` value from the most recent render — not a sentinel value; see design.md).
- [x] 1.3 Add unit tests in `app.rs` for offset changes on each key, including saturating at 0 on the low end.

## 2. Clamped max-scroll computation

- [x] 2.1 Add a function that computes the widest row's rendered width for a given `Vec<Row>` (matching the same text `row_to_list_item` produces, so width matches what's rendered).
- [x] 2.2 Add a function that computes `max_scroll` from that widest width and the pane's inner width (accounting for the block's border).
- [x] 2.3 Add unit tests covering: all rows shorter than pane width (max_scroll = 0), one row wider, widest row is an archived row with date+indent, empty active/archived (placeholder rows).
- [x] 2.4 After computing `max_scroll` in `render_left_pane`, write it into `App`'s cached field so `$`/`End` reflect the current value on the next keypress.

## 3. Rendering: apply offset and slice row text

- [x] 3.1 Update row rendering in `src/tui/mod.rs` so the effective offset (stored offset clamped to current `max_scroll`) is applied by consuming characters cumulatively across each row's existing styled spans (indent, dimmed date, marker, name) in order — not by flattening to plain text first — so each remaining fragment keeps its own span's original style (in particular, so a partially-scrolled archived row's date remains dimmed rather than losing its styling). Skip by character count (`.chars()`), not raw byte index: row text already contains multi-byte UTF-8 (the `▸`/`▾` markers), and byte-index slicing would panic when `offset` lands mid-character.
- [x] 3.2 Verify all row kinds (Active, ArchivedHeader, Archived, Placeholder) render correctly when scrolled past their own content (render as empty/blank rather than panicking on out-of-bounds slicing).
- [x] 3.3 Apply `Modifier::UNDERLINED` to the `ArchivedHeader` row's spans when collapsed (not when expanded).
- [x] 3.4 Add a regression test that renders the `archived` header row (both collapsed and expanded) at every horizontal offset from 0 through past its full length and asserts it never panics — covers skipping through the multi-byte `▸`/`▾` marker.
- [x] 3.5 Add a test that scrolls an archived row to an offset partway through its date, and asserts the remaining visible date text still carries the dimmed style (regression guard for the existing "date is visually de-emphasized" requirement).

## 4. Scrollbar indicator

- [x] 4.1 Render a horizontal `Scrollbar` (`ScrollbarOrientation::HorizontalBottom`) along the bottom of the left pane's `Block::bordered()`, driven by `ScrollbarState` built from `(max_scroll, effective_offset)`.
- [x] 4.2 Confirm the scrollbar shows a "nothing to scroll" state (not hidden, not erroring) when `max_scroll == 0`.

## 5. Verification

- [x] 5.1 Run `cargo test` and confirm all existing and new tests pass.
- [x] 5.2 Manually run the TUI in a narrowed terminal with a long active change name and a long archived name (date + name) and confirm: `h`/`l`/arrows scroll all rows together, `^`/`$`/Home/End jump to extremes, the scrollbar reflects position, scroll position survives moving the cursor and toggling `archived`, and scroll clamps down correctly when collapsing archived or widening the terminal. Also confirm the collapsed `archived` row is underlined, the expanded one isn't, and the underline stays visible on whatever text remains on screen when scrolled.
