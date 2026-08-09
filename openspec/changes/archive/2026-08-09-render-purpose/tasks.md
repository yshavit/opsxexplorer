## 1. `spec-diff`: purpose comparison

- [x] 1.1 Add `purpose: Option<Piece>` to `CapabilityDiff` (`src/diff/model.rs`)
- [x] 1.2 In `diff()` (`src/diff/mod.rs`), compute `purpose` from `pair.delta.purpose` and `pair.base.and_then(|s| s.purpose)` via `compare::changed_or_unchanged`, mapping `Piece::Unchanged` to `None` and leaving `delta.purpose == None` as `None`
- [x] 1.3 Unit tests: delta with no purpose → `None`; delta purpose vs. absent base purpose (and absent base entirely) → `Some(Piece::Added)`; delta purpose equal to base purpose → `None`; delta purpose differing from base purpose → `Some(Piece::Changed)` or `Some(Piece::Replaced)` depending on similarity; determinism (comparing twice yields the same result)

## 2. `RowKey` becomes an enum

- [x] 2.1 Change `RowKey` (`src/tui/diff_row.rs`) from a struct to an enum with `Purpose { capability }`, `Requirement { capability, requirement }`, `Scenario { capability, requirement, scenario }` variants
- [x] 2.2 Update every existing construction site in `diff_row.rs` and `app.rs` (including tests) to the new shape
- [x] 2.3 Confirm existing `diff_row.rs` and `app.rs` tests still pass unchanged in behavior after the mechanical update

## 3. New `DiffRow` variants and flattening

- [x] 3.1 Add `DiffRow::PurposeHeading(&'a Piece)` (display-only, not selectable), `DiffRow::PurposeFull(&'a Piece)` (display-only, not selectable — full text fits in one line, nothing to collapse), and `DiffRow::Purpose { piece: &'a Piece, expanded: bool, key: RowKey }` (selectable — full text does not fit) to `src/tui/diff_row.rs`
- [x] 3.2 Update `is_selectable()` to cover both `DiffRow::Purpose` and `DiffRow::PurposeFull` (both are selectable); leave `key()` and `expanded()` covering only `DiffRow::Purpose`, so `PurposeFull` falls through their existing `_ => None` arm and the toggle keys are inert on it for free
- [x] 3.3 Give `flatten()` a `width: usize` parameter. When `diff.purpose.is_some()`, emit `PurposeHeading` immediately after the error `Notice` rows and before the first `GroupHeading`, then: if the piece is `Piece::Replaced`, always emit `Purpose`; otherwise (`Added`/`Changed`) measure the piece's current text (right-trimmed of trailing whitespace) against the row's available width at `width` and emit `PurposeFull` if it fits or `Purpose` if it doesn't
- [x] 3.4 Unit tests: purpose rows appear in the right position relative to notices and group headings; absent purpose emits neither row; fitting `Added`/`Changed` text yields `PurposeFull`, non-fitting yields `Purpose`; a `Replaced` piece always yields `Purpose` even when its current text alone would fit; both `PurposeFull` and `Purpose` are selectable; `PurposeFull`'s `key()`/`expanded()` are `None`; `Purpose`'s collapse state follows the `expanded` set like any other row; the fits/doesn't-fit boundary responds to `width`

## 4. Rendering: heading box

- [x] 4.1 Extract `group_heading_box`'s box-drawing and narrow-width degrade logic (`src/tui/mod.rs`) into a helper taking `(label: &str, style: Style, width: usize)`, with `group_heading_box` becoming a thin wrapper over it
- [x] 4.2 Add `purpose_heading_box(piece: &Piece, width: usize) -> Vec<Line>` deriving the label ("Added Purpose" / "Modified Purpose", no colon) and style (`added_style()` / `modified_style()`) from the `Piece` variant
- [x] 4.3 Wire `DiffRow::PurposeHeading` into `build_diff_lines`'s existing `if let DiffRow::GroupHeading(op) = row` special-casing
- [x] 4.4 Unit tests: label text and color per `Piece` variant; degrade to a plain line under the same width threshold `group_heading_box` uses

## 5. Rendering: the purpose row

- [x] 5.1 Add `truncate_chars(text: &str, width: usize) -> String` to `src/tui/layout.rs`: takes `width.saturating_sub(1)` characters from the start of `text` and appends `'…'`
- [x] 5.2 In `layout.rs`, handle `DiffRow::PurposeFull`: gutter marker from `piece_marker(piece)`, indent `0`, fixed prefix `¶ `, full trimmed text through `piece_spans` + `wrap_spans` — no arrow, no truncation
- [x] 5.3 In `layout.rs`, handle `DiffRow::Purpose { expanded: false, .. }` for `Piece::Added`/`Piece::Changed`: gutter marker from `piece_marker(piece)`, indent `0` (sibling of `Requirement`, not nested); collapsed content is the fixed prefix (`▸ ¶ `) plus `truncate_chars` applied to the piece's trimmed current text, budgeted against `available` minus the prefix's display width
- [x] 5.4 In `layout.rs`, handle `DiffRow::Purpose { expanded: false, .. }` for `Piece::Replaced`: collapsed content is the fixed prefix (`▸ ¶ `) plus the placeholder `"Expand to view diff"` in italics (`modified_style().add_modifier(Modifier::ITALIC)`), rendered as-is if it fits in `available` or via `truncate_chars` if it doesn't
- [x] 5.5 Handle `DiffRow::Purpose { expanded: true, .. }` (any piece) falling through to the existing `piece_spans` + `wrap_spans` path used by `Body`/`Intro`, prefixed with `▾ ¶ `
- [x] 5.6 Unit tests: `PurposeFull` renders the full text with no ellipsis and no arrow; collapsed `Added`/`Changed` `Purpose` is exactly one line ending in `…`, sized to available width, truncation indifferent to word boundaries; collapsed `Replaced` `Purpose` shows the italicized placeholder instead of any excerpt, truncating the placeholder itself only at extreme widths; expanded row (any piece) shows full text with the same interleaved-run or stacked-replacement styling a changed/replaced intro would get; resizing changes the collapsed truncation point for `Added`/`Changed`

## 6. App wiring

- [x] 6.1 Add a `right_pane_width: usize` field to `App`, mirroring `right_viewport_rows`, with a `set_right_pane_width` setter called from `render_diff_tabs` (`src/tui/mod.rs`) alongside the existing `set_right_viewport_rows` call
- [x] 6.2 Update `App::diff_rows()` to call `flatten(diff, &self.expanded, self.right_pane_width)`
- [x] 6.3 Confirm `App`'s cursor movement, `toggle_cursor_row`, `set_cursor_row_expanded` and `reset_cursor_to_first_selectable` work for both `DiffRow::Purpose` and `DiffRow::PurposeFull` with no further code changes (they operate generically via `DiffRow::key()`/`expanded()`/`is_selectable()`) — add a test if any gap turns up
- [x] 6.4 Test: pressing Enter, Space, `l` and `h` with the cursor on a `PurposeFull` row leaves `App`'s expanded-set and cursor position unchanged
- [x] 6.5 Integration-style test: a capability with a purpose comparison, an error notice, and requirements — confirm render order (notice, purpose heading, purpose row, group headings/requirements) and that cursor navigation reaches the purpose row (fitting or not) and skips its heading
- [x] 6.6 Test: resizing the right pane across the fits/doesn't-fit boundary changes whether the purpose row is `Purpose` or `PurposeFull`, while it remains reachable by the cursor either way

## 7. Verification

- [x] 7.1 `cargo test`
- [x] 7.2 `cargo fmt` / `cargo clippy`
- [x] 7.3 Have a human manually run the TUI (`/run` or equivalent) against a change whose delta spec has an added or modified `## Purpose` section and confirm the rendering matches the spec: heading box, collapsed truncation, expand/collapse toggling, and placement relative to notices and requirement groups
