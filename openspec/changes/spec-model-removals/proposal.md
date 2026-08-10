## Why

A REMOVED requirement's own body is discarded during parsing (`src/specs/parse.rs:162`), so the **Reason** and **Migration** text OpenSpec's own conventions require on every removal never reaches a consumer. `spec-diff`'s spec currently justifies this on a false premise ("a removal entry has none"), and `tui-specdiff` renders a removed requirement's old content with no explanation attached, even though the source file the user is looking at a diff of usually has one. Fixes [#4](https://github.com/yshavit/opsxexplorer/issues/4).

## What Changes

- `spec-model` stops blanking a REMOVED entry's body: it parses the same way an ADDED or MODIFIED entry's body does, so any body text (conventionally a Reason and Migration explanation) lands in the parsed requirement's `intro`, exactly as authored.
- `spec-diff` carries that text through as a distinct **removal note** on a removed `RequirementDiff`, separate from the `Piece` comparisons of the requirement's intro and scenarios (which remain pure deletions recovered from the spec of record — there is no base counterpart for a removal note, so it is not itself a diffed piece). `spec-diff`'s spec is corrected: the "removal reports no content of its own" scenario is factually wrong (a removal entry can and conventionally does carry content) and is replaced.
- `tui-specdiff` renders a removed requirement's removal note as its own row, directly above the requirement's intro row, in plain text styling with a blank gutter marker — neither the insertion nor the deletion styling used elsewhere in the pane — following the same fits-in-one-line-or-collapsible convention already used for a requirement's intro row.

## Capabilities

### Modified Capabilities
- `spec-model`: a REMOVED entry's body is parsed instead of discarded.
- `spec-diff`: a removed requirement's own body is carried through as a removal note, and the spec's incorrect "a removal entry has none" claim is corrected.
- `tui-specdiff`: a removed requirement's removal note, when present, is displayed above its deleted intro and scenarios.

## Impact

- `src/specs/parse.rs`: drop the `DeltaOp::Removed` special case in `parse_delta_entries_section`, parsing removed entries via `parse_requirement` like every other entry.
- `src/specs/model.rs`: `DeltaEntry`'s doc comment describing removed entries as always-empty is stale and needs updating.
- `src/diff/model.rs`: `Operation::Removed` gains a `note: String` field (empty when the delta carries no body), mirroring how `Operation::Renamed` already carries `from`.
- `src/diff/mod.rs`: populate `Operation::Removed { note }` from the delta entry's `intro` when building a removed `RequirementDiff`.
- `src/tui/layout.rs`, `src/tui/diff_row.rs`: every existing match on `Operation::Removed` as a unit variant needs updating for the new field; add rendering and cursor/selectability handling for the new removal-note row.
- No changes to `src/changes/*` or diff-base resolution — this is confined to the parse → diff → render pipeline for a single requirement's content.
