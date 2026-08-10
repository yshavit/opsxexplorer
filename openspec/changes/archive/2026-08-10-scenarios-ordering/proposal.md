## Why

Today, scenarios within a requirement are reported in spec-of-record order, with delta-only additions appended at the end. That buries the additions and modifications — the parts of a diff a reviewer most wants to see first — behind whatever the base spec's original ordering happened to be. Issue #14 asks that scenarios instead be grouped and surfaced by how they changed, so the most important ones (additions, then modifications) come first and the least important (unchanged) come last.

## What Changes

- **BREAKING**: Change the reported order of a requirement's scenarios from "base order, then delta-only appended" to a fixed category order: added, then modified, then removed, then unmentioned, then unchanged.
- Within each category, break ties using a category-specific rule:
  - Added: the order the delta lists them
  - Modified (a matched pair reported as changed or replaced): the order the spec of record lists them
  - Removed (a deleted requirement's scenarios, recovered from the spec of record): the order the spec of record lists them
  - Unmentioned: the order the spec of record lists them
  - Unchanged: the order the spec of record lists them
- This ordering applies to a requirement's scenarios; it does not change the existing requirement-level operation grouping (added/modified/removed/renamed) already specified for `spec-diff`.
- No change to the classification rules themselves (what counts as added/changed/replaced/unmentioned/unchanged) — only to the order in which already-classified scenarios are reported.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `spec-diff`: replace the requirement "A modified requirement's scenarios are matched by name and ordered base-first" with a category-based ordering rule (added, modified, removed, unmentioned, unchanged), and clarify that a removed requirement's recovered scenarios follow the same category-then-base-order placement.

## Impact

- `src/diff/compare.rs`: `compare_requirement` (and the removal path that recovers a deleted requirement's scenarios) currently emit scenarios in "base order, then delta-only appended" order; this must change to sort by category first, using the tie-break appropriate to that category.
- `src/diff/mod.rs`, `src/tui/diff_row.rs`: no code changes expected, but existing tests asserting the old base-first ordering (e.g. `reordering_restated_scenarios_changes_nothing`, `subset_of_scenarios_restated_leaves_one_unmentioned`, the end-to-end fixture in `diff/mod.rs`) will need their expectations updated to the new order.
- No changes to parsing (`src/specs/parse.rs`) or to the data model (`ScenarioDiff`, `Piece`) — this is purely a reordering of already-classified scenarios before they're placed into `RequirementDiff.scenarios`.
