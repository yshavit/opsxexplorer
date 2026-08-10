## 1. spec-model: parse a removed entry's body

- [ ] 1.1 In `src/specs/parse.rs`'s `parse_delta_entries_section`, drop the `DeltaOp::Removed` special case and call `parse_requirement(ctx, s)` for every operation uniformly
- [ ] 1.2 Update `DeltaEntry`'s doc comment in `src/specs/model.rs` (currently claims a removed entry always has empty `intro` and `scenarios: []`)
- [ ] 1.3 Update the `removed_entry_is_heading_only` test in `src/specs/parse.rs` (still valid for a bare removal) and add a synthetic parser test for a removed entry whose heading is followed by Reason/Migration body text, asserting it lands in `intro`

## 2. spec-diff: carry the removal note through

- [ ] 2.1 In `src/diff/model.rs`, change `Operation::Removed` to `Operation::Removed { note: String }`
- [ ] 2.2 In `src/diff/mod.rs`'s `diff()`, populate `note` from `entry.requirement.intro.clone()` when building a removed `RequirementDiff`
- [ ] 2.3 Update every `Operation::Removed` match site across `src/diff/`, `src/tui/layout.rs`, and `src/tui/diff_row.rs` (marker/style lookup, group-heading lookup, tests) for the new field — `Operation::Removed { .. }` where the note isn't needed, destructured where it is
- [ ] 2.4 Add diff tests: a removal entry with body text produces `Operation::Removed { note }` with that text; a bare removal produces an empty `note`; the requirement's `intro`/`scenarios` `Piece`s stay pure deletions from the base in both cases

## 3. tui-specdiff: render the removal note

- [ ] 3.1 Split a removed requirement's non-empty `note` into paragraphs (reusing the same paragraph-boundary rule `spec-model` already applies to a multi-paragraph intro) and add one `DiffRow` (or equivalent) per paragraph, positioned directly above the requirement's intro row, in document order
- [ ] 3.2 Give each row the pillcrow marker and modification styling (the same marker glyph/colour used for `Operation::Modified` and `Piece::Changed`), reusing the intro row's fits-in-one-line / collapsible-with-ellipsis measurement and rendering logic
- [ ] 3.3 Add a `Reason`/`Migration` keyword-stripping helper alongside the existing `bullet_keyword`/`when_then_style` helpers in `src/tui/layout.rs`: detect a paragraph beginning `**Reason**` or `**Migration**`, strip that keyword's `**` and style it, leaving the rest of the paragraph and any non-matching paragraph unaffected
- [ ] 3.4 Start each removal-note paragraph row collapsed the first time the requirement is expanded, matching the intro row's collapse-state initialization
- [ ] 3.5 Add each removal-note paragraph row to the cursor's selectable-row set and to the "stops even when it fits" / "toggle has no effect when not collapsible" handling alongside the purpose and intro rows
- [ ] 3.6 Add rendering tests: Reason paragraph and Migration paragraph both render above the intro in document order with their keyword de-asterisked and styled, an unrecognised paragraph still renders without keyword stripping, no rows when the note is absent, modification styling distinct from both the added and deleted rows elsewhere in the pane, short-paragraph-renders-in-full, long-paragraph-collapsible-starts-collapsed, cursor reaches and can toggle each row

## 4. Verification

- [ ] 4.1 Run the full test suite (`cargo test`)
- [ ] 4.2 Manually exercise the TUI against a change with a REMOVED requirement carrying a Reason/Migration body (e.g. construct one under `openspec/changes/` locally, or temporarily edit an existing removal) and confirm the note renders correctly, then discard the scratch edit
- [ ] 4.3 Run `openspec validate --change spec-model-removals --strict` (or the project's equivalent) to confirm the delta specs are well-formed
