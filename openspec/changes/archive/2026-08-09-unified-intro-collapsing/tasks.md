## 1. `RowKey` and row model

- [x] 1.1 Add `RowKey::Intro { capability: String, requirement: String }` to `src/tui/diff_row.rs`.
- [x] 1.2 Replace `DiffRow::PurposeFull(&'a Piece)` with `DiffRow::ParagraphFull { piece: &'a Piece, indent: usize }`.
- [x] 1.3 Replace `DiffRow::Purpose { piece, expanded, key }` with `DiffRow::Paragraph { piece: &'a Piece, expanded: bool, key: RowKey, indent: usize }`.
- [x] 1.4 Remove `DiffRow::Intro { piece }`.

## 2. Shared push logic (`diff_row.rs`)

- [x] 2.1 Add `fn paragraph_text(piece: &Piece) -> Option<&str>` covering all six `Piece` variants (`Unchanged`/`Added`/`Deleted`/`Unmentioned`/`Changed` → `Some`, `Replaced` → `None`), per design.md's "Decisions".
- [x] 2.2 Add a shared `push_paragraph_row(rows, piece, key, indent, width, expanded)` that emits `ParagraphFull` when `paragraph_text` fits `layout::paragraph_available(width, indent)`, else `Paragraph { expanded: <from set>, .. }`.
- [x] 2.3 Update `push_purpose` to call `push_paragraph_row` with `key: RowKey::Purpose { capability }`, `indent: 0` (it still emits `PurposeHeading` first, unchanged).
- [x] 2.4 Update `push_requirement`'s intro line to call `push_paragraph_row` with `key: RowKey::Intro { capability, requirement }`, `indent: 1`, threading `width` and `expanded` through (`push_requirement` needs a new `width: usize` parameter; update its one call site in `flatten`).

## 3. Shared layout logic (`layout.rs`)

- [x] 3.1 Generalize `purpose_available(width)` to `paragraph_available(width: usize, indent: usize) -> usize`, additionally subtracting `indent * INDENT_UNIT`.
- [x] 3.2 Generalize `collapsed_purpose_lines(piece, width)` to `collapsed_paragraph_lines(piece, width, indent)`: prepend `indent * INDENT_UNIT` columns of blank padding; extend the content match to use `paragraph_text` for the excerpt (all variants except `Replaced`) instead of only `Added`/`Changed`; apply the existing de-emphasised style to the excerpt when `piece` is `Piece::Unmentioned`, matching what the expanded path already does for intro.
- [x] 3.3 Update `row_lines`'s collapsed special-case to match `DiffRow::Paragraph { expanded: false, indent, .. }` and pass `indent` through to `collapsed_paragraph_lines`.
- [x] 3.4 Update `indent_depth` to read the `indent` field from `ParagraphFull`/`Paragraph` instead of a hardcoded per-variant constant; remove the old `Intro` arm.
- [x] 3.5 Update `gutter_marker` and `content_spans` to use `ParagraphFull`/`Paragraph` in place of the old `PurposeFull`/`Purpose`/`Intro` arms, preserving the pillcrow marker and the `Unmentioned` dim-wrapping behavior for the expanded path.

## 4. Selectability

- [x] 4.1 Update `is_selectable()` to treat `ParagraphFull`/`Paragraph` as selectable regardless of collapsibility (widening the existing `PurposeFull` exception).
- [x] 4.2 Update `key()`/`expanded()` to read from `Paragraph` (in place of the old `Purpose` arm); confirm `ParagraphFull` still falls through to `None` for both.

## 5. Tests

- [x] 5.1 Update every existing `diff_row.rs` and `layout.rs` test that constructs or matches `DiffRow::PurposeFull`/`Purpose`/`Intro` to use the new `ParagraphFull`/`Paragraph` shapes (including the `indent` field).
- [x] 5.2 Add `diff_row.rs` tests: an intro at indent 1 with a long `Unchanged` text becomes collapsible and fits-checks against the narrower indent-1 budget (not the indent-0 purpose budget); an intro with `Deleted`/`Unmentioned` pieces also gets a truncatable excerpt rather than falling into the `Replaced`-only placeholder path; `RowKey::Intro` participates in the `expanded` `HashSet` the same way `RowKey::Purpose` does.
- [x] 5.3 Add `layout.rs` tests: `collapsed_paragraph_lines` at `indent: 1` renders the expected leading padding; a collapsed `Unmentioned` intro excerpt is de-emphasised; `paragraph_available` subtracts indent correctly at a couple of widths.
- [x] 5.4 Add an `app.rs`-level test exercising cursor movement: with a requirement expanded, the cursor stops on the intro row (fitting or not), and toggling a fitting intro row is a no-op — mirroring the existing purpose-row tests at `src/tui/app.rs`.

## 6. Verification

- [x] 6.1 `cargo test`
- [x] 6.2 `cargo fmt`
