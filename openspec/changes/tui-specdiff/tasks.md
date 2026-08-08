## 1. Wrapping primitive

- [ ] 1.1 Add `src/tui/wrap.rs` with `wrap_spans(Vec<Span<'static>>, width) -> Vec<Vec<Span<'static>>>`: word-boundary breaking, per-span style preserved across a break, characters not bytes, over-long single words hard-broken
- [ ] 1.2 Unit-test `wrap_spans`: text shorter than the width returns one line; a run straddling a break keeps its style on both sides; a word longer than the width is broken rather than overflowing; multi-byte content does not panic; `width == 0` and empty input are handled

## 2. Right-pane row model

- [ ] 2.1 Add `src/tui/diff_row.rs` with `DiffRow` (`GroupHeading`, `Requirement`, `Intro`, `Scenario`, `Body`, `Notice`), a name-based `RowKey` for collapse state, and `is_selectable()` true only for `Requirement` and `Scenario`
- [ ] 2.2 Implement `flatten(&CapabilityDiff, &HashSet<RowKey>) -> Vec<DiffRow>`: group headings only for operations with entries, entries in the order `diff::diff` reports them, children emitted only when their parent is expanded, `Notice` rows for `CapabilityDiff::errors` above the tree
- [ ] 2.3 Unit-test `flatten`: everything collapsed yields one row per requirement plus group headings; expanding a requirement reveals its intro and scenario headers with scenarios collapsed; expanding a scenario reveals its body; an operation with no entries emits no heading; errors and requirements appear together

## 3. Styling and layout

- [ ] 3.1 Implement requirement markers (`+`/`~`/`-`/`»` from `Operation`) and piece markers (`+`/`~`/`-`/`?`/blank from `Piece`), with `?` dimmed
- [ ] 3.2 Implement word-diff span construction from `Piece::Changed`: walk `runs` in order, `Equal` slices the delta range, `Delete` slices base styled red, `Insert` slices delta styled green, all via `str::get` so a bad range yields an empty span instead of panicking
- [ ] 3.3 Implement `row_lines(&DiffRow, width) -> Vec<Line<'static>>`: build the row's spans, wrap to `width - gutter - indent`, prefix the first line with the marker and continuation lines with blanks, left-pad by indent
- [ ] 3.4 Unit-test styling: each operation and each `Piece` variant gets its expected marker; an unmentioned intro is both marked `?` and dimmed while an unchanged intro is neither; a changed piece renders deleted and inserted text once each in one passage; continuation lines carry no marker and align under the first line's text

## 4. App state and key routing

- [ ] 4.1 Add right-pane state to `App`: `focus`, per-change `Vec<CapabilityDiff>` plus load errors, selected tab index, expanded `HashSet<RowKey>`, right-pane cursor row and line offset, cached max line offset
- [ ] 4.2 Recompute the diff when the left-pane selection changes: `Changes::resolve` → `views` → `capabilities` → per-capability `load` + `diff`, storing per-capability `Result`s so one failure does not affect its siblings; reset tab to the first and clear the expanded set
- [ ] 4.3 Intercept `Tab` in `handle_key` to flip focus, then dispatch the event to the focused pane's handler; leave the existing left-pane handler and the event loop's `Ctrl+Q` untouched
- [ ] 4.4 Implement right-pane key handling: `j`/`k`/arrows move over selectable rows only and clamp at the ends, `Enter`/`Space` toggle, `l`/`Right` expand, `h`/`Left` collapse, `]`/`[` move the tab selection without wrapping
- [ ] 4.5 Unit-test key handling: right-pane keys are inert while the left pane holds focus and vice versa; `Tab` round-trips focus; the cursor skips group headings, intro blocks, bodies and notices; tab selection stops at both ends and is a no-op for a single capability

## 5. Rendering

- [ ] 5.1 Replace `render_right_pane` with the real pane: focused/unfocused border style, the tab bar built as styled spans passed to `Block::title` (selected tab reversed, separator between tabs — not the `Tabs` widget, which cannot render into a border), `Paragraph::scroll((line_offset, 0))` over the flattened lines with no `Wrap`
- [ ] 5.2 Apply `Modifier::REVERSED` to every line belonging to the selected row, and clamp the stored line offset at render time (`min(max_line_offset)`, then adjust so the selected row's first line is visible), caching `max_line_offset` back onto `App`
- [ ] 5.3 Render a vertical `Scrollbar` (`VerticalRight`) from the same offset and max, `content_length = max_offset + 1`, shown in its nothing-to-scroll state when the content fits
- [ ] 5.4 Add the same focused/unfocused border treatment to the left pane

## 6. Empty and error states

- [ ] 6.1 Render the placeholder when the left-pane cursor is on a non-change row (archived header, placeholder), and the "no spec changes" message with no tab bar when `capabilities()` returns empty
- [ ] 6.2 Render pane-level notices for a failing `Changes::resolve`/`views`/`capabilities`, and tab-level notices for a failing `load`, using the existing `Display` impls verbatim so no line number can appear
- [ ] 6.3 Unit-test the states: no capabilities yields the message and no tabs; one capability failing to load leaves its siblings rendering normally; `CapabilityDiff::errors` renders notices alongside the computed requirements

## 7. Cleanup

- [ ] 7.1 Remove `tui-markdown` from `Cargo.toml` and update `Cargo.lock`

## 8. Verification

- [ ] 8.1 `cargo fmt`, `cargo clippy` and `cargo test` clean
- [ ] 8.2 Run the TUI against this repo and confirm each archived change renders: `2026-08-08-tui-changelist-horizontal-scrolling` (ADDED + MODIFIED with a changed intro), `2026-08-07-tui-initial` and `2026-08-08-spec-model` (two tabs each), `2026-08-07-add-readonly-filesystem` (single tab, all ADDED)
- [ ] 8.3 Confirm this change's own `tui` delta renders its RENAMED and REMOVED entries correctly once the change is active — the repo's only fixture for either
