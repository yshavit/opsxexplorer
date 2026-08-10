## 1. Parser and model

- [x] 1.1 Add `unrecognized_sections: Vec<String>` to `Delta` in `src/specs/model.rs`
- [x] 1.2 In `parse_delta` (`src/specs/parse.rs`), change the title match's fallback arm from `other => return Err(unrecognised_section_error(path, other))` to pushing `other.to_string()` into a local `Vec<String>` and continuing the loop; populate `Delta.unrecognized_sections` from it. Leave `parse_spec`'s fallback arm untouched.
- [x] 1.3 Update the existing `unrecognised_operation_section_is_reported` test in `src/specs/parse.rs` (it currently asserts `parse_delta` on an unrecognised section returns `Err`; this is no longer true) and add tests: one unrecognised section among well-formed ones, several unrecognised sections preserve document order, no unrecognised sections yields an empty vec, `parse_spec` still errors on an unrecognised section

## 2. Diff

- [x] 2.1 Add `unrecognized_sections: Vec<String>` to `CapabilityDiff` in `src/diff/model.rs`
- [x] 2.2 Populate it verbatim from `pair.delta.unrecognized_sections` in `diff()` (`src/diff/mod.rs:135`)
- [x] 2.3 Add tests in `src/diff/mod.rs` (or its test module) covering a delta with unrecognised sections carried through unchanged, and a delta with none

## 3. Rendering

- [x] 3.1 Add `unrecognised_style()` to `src/tui/layout.rs`, alongside `added_style`/`modified_style`/`removed_marker_style`, using `Color::Rgb(147, 51, 234)`
- [x] 3.2 Add two new `DiffRow` variants in `src/tui/diff_row.rs`: an `UnrecognizedSectionsHeading` (returns `None` from `key()`/`expanded()`, like `GroupHeading`) and an `UnrecognizedSections(&[String])` content row carrying the titles (returns `Some` from `key()`, so it's cursor-selectable; has no meaningful `expanded()` state — toggling it is a no-op, matching how a non-collapsible purpose/intro row already behaves)
- [x] 3.3 In `flatten()` (`src/tui/diff_row.rs`), after the `for req in &diff.requirements` loop, push the heading row followed by the content row when `!diff.unrecognized_sections.is_empty()`
- [x] 3.4 Add an `unrecognized_sections_box` wrapper in `src/tui/mod.rs` alongside `group_heading_box`/`purpose_heading_box`, calling the shared `heading_box("Unknown sections", unrecognised_style(), width)` for the heading, then the italic prompt line (`unrecognised_style()` + `Modifier::ITALIC`) wrapped via `wrap_spans`, then one `"• <title>"` bullet per title in the pane's ordinary text style, for the content row
- [x] 3.5 Wire the new `DiffRow` variants into `build_diff_lines` (`src/tui/mod.rs:324-392`) alongside the existing `GroupHeading`/`PurposeHeading` special cases; wire the content row into the cursor-movement/selectability logic the same way the purpose/intro rows are wired in (selectable, no-op on toggle)
- [x] 3.6 Add tests covering: the heading and content row render below requirement groups when unrecognised sections are present, neither renders when absent, multiple titles each get their own bullet in order, narrow-pane degrade of the heading to a plain styled line, cursor movement skips the heading but stops on the content row (including via `j`/`k`/`Ctrl+d`/`Ctrl+u` scrolling it into view), and `Enter`/`Space`/`l`/`h` on the content row are no-ops

## 4. Verification

- [x] 4.1 `cargo test`
- [x] 4.2 `cargo clippy`
- [x] 4.3 Manually exercise the pane (see project's `run` skill) against a delta spec.md with an extra unrecognised `##` section, confirming the rest of the tab still renders, the "Unknown sections" heading and row appear at the bottom in purple, and the row can be reached and scrolled to with the cursor
