## 1. spec-model: parse a removed entry's body

- [x] 1.1 In `src/specs/parse.rs`'s `parse_delta_entries_section`, drop the `DeltaOp::Removed` special case and call `parse_requirement(ctx, s)` for every operation uniformly
- [x] 1.2 Update `DeltaEntry`'s doc comment in `src/specs/model.rs` (currently claims a removed entry always has empty `intro` and `scenarios: []`)
- [x] 1.3 Update the `removed_entry_is_heading_only` test in `src/specs/parse.rs` (still valid for a bare removal) and add a synthetic parser test for a removed entry whose heading is followed by Reason/Migration body text, asserting it lands in `intro`

## 2. spec-diff: carry the removal note through

- [x] 2.1 In `src/diff/model.rs`, change `Operation::Removed` to `Operation::Removed { note: String }`
- [x] 2.2 In `src/diff/mod.rs`'s `diff()`, populate `note` from `entry.requirement.intro.clone()` when building a removed `RequirementDiff`
- [x] 2.3 Update every `Operation::Removed` match site across `src/diff/`, `src/tui/layout.rs`, and `src/tui/diff_row.rs` (marker/style lookup, group-heading lookup, tests) for the new field — `Operation::Removed { .. }` where the note isn't needed, destructured where it is
- [x] 2.4 Add diff tests: a removal entry with body text produces `Operation::Removed { note }` with that text; a bare removal produces an empty `note`; the requirement's `intro`/`scenarios` `Piece`s stay pure deletions from the base in both cases

## 3. tui-specdiff: render the removal note

- [x] 3.1 Split a removed requirement's non-empty `note` by line (e.g. `str::lines()`), skipping blank lines rather than emitting a row for them — NOT by markdown-paragraph/blank-line-block, since `**Reason**`/`**Migration**` share one markdown paragraph (soft line break, single `\n`) while a genuine paragraph gap comes through as `\n\n` (confirmed against `render_body` in `src/specs/parse.rs`) — and add one `DiffRow` (or equivalent) per line, positioned directly above the requirement's intro row, in document order
- [x] 3.2 Give each row the pillcrow marker and modification styling (the same marker glyph/colour used for `Operation::Modified` and `Piece::Changed`), reusing the intro row's fits-in-one-line / collapsible-with-ellipsis measurement and rendering logic (each row wraps only its own already-split line to the pane width; the line split itself must happen once, before wrapping, not be re-derived from the wrapped output)
- [x] 3.3 Add a `Reason`/`Migration` keyword-stripping helper alongside the existing `bullet_keyword`/`when_then_style` helpers in `src/tui/layout.rs`: detect a line beginning `**Reason**` or `**Migration**`, strip that keyword's `**` and style it, leaving the rest of the line and any non-matching line unaffected
- [x] 3.4 Start each removal-note line row collapsed the first time the requirement is expanded, matching the intro row's collapse-state initialization
- [x] 3.5 Add each removal-note line row to the cursor's selectable-row set and to the "stops even when it fits" / "toggle has no effect when not collapsible" handling alongside the purpose and intro rows
- [x] 3.6 Add rendering tests: Reason line and Migration line both render above the intro as separate rows, in document order, with their keyword de-asterisked and styled, despite sharing one markdown paragraph in the source; a genuine blank-line paragraph gap produces no empty row; an unrecognised line still renders without keyword stripping; no rows when the note is absent; modification styling distinct from both the added and deleted rows elsewhere in the pane; short-line-renders-in-full; long-line-collapsible-starts-collapsed; cursor reaches and can toggle each row

## 4. Verification

- [x] 4.1 Run the full test suite (`cargo test`)
- [x] 4.2 Manually exercise the TUI against a change with a REMOVED requirement carrying a Reason/Migration body — `openspec/changes/archive/2026-08-08-tui-keybinding-improvements/specs/tui/spec.md` already has a real one (`Ctrl+Q exits the application`, lines 25-27) usable as a live fixture — and confirm both lines render as separate rows, then discard any scratch edits
- [x] 4.3 Run `openspec validate --change spec-model-removals --strict` (or the project's equivalent) to confirm the delta specs are well-formed
