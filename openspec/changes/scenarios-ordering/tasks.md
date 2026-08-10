## 1. Implement category-based scenario ordering

- [x] 1.1 In `src/diff/compare.rs`, capture each scenario's tie-break index while building the `Vec<ScenarioDiff>` in `compare_requirement`: the position in `base.scenarios` for matched/`Unmentioned` scenarios, the position in `delta.scenarios` for `Added` scenarios.
- [x] 1.2 Add a small helper that maps a `&Piece` to its category rank (`Added` → 0, `Changed`/`Replaced` → 1, `Unmentioned` → 2, `Unchanged` → 3), with a comment noting `Deleted` is unreachable from this function.
- [x] 1.3 Sort the built `Vec<ScenarioDiff>` by `(category_rank, tie_break_index)` using a stable sort before returning it from `compare_requirement`.
- [x] 1.4 Update the doc comment above `compare_requirement` (currently describing "base scenarios first, then delta-only appended") to describe the new category-based order.

## 2. Update existing tests to the new order

- [x] 2.1 Update `reordering_restated_scenarios_changes_nothing` in `src/diff/compare.rs` to assert the new category order instead of base order. (Both scenarios in this fixture are `Unchanged`, so they land in the same category and the existing base-order assertion is unchanged — confirmed no edit needed.)
- [x] 2.2 Update `subset_of_scenarios_restated_leaves_one_unmentioned` in `src/diff/compare.rs` to assert the new category order.
- [x] 2.3 Update the end-to-end fixture assertion in `src/diff/mod.rs` (`horizontal_scrolling_change_diffs_as_expected_against_its_base`) to the new category order.
- [x] 2.4 Search `src/tui/diff_row.rs` and `src/tui/mod.rs` for any test that depends on the previous scenario ordering and update it if found. (All `DiffRow`/`RequirementDiff` fixtures there are built directly, not via `compare_requirement`, so none depend on this ordering — no changes needed.)

## 3. Add coverage for the new ordering rule

- [x] 3.1 Add a test in `src/diff/compare.rs` for a requirement whose scenarios mix all four categories (added, changed/replaced, unmentioned, unchanged), asserting they're grouped in that order.
- [x] 3.2 Add a test asserting added scenarios within their category follow delta order (independent of base order).
- [x] 3.3 Add a test asserting modified (changed/replaced) scenarios within their category follow base order, not delta order.
- [x] 3.4 Add a test asserting unchanged scenarios within their category follow base order, not delta order.
- [x] 3.5 Add a determinism test: comparing the same modified entry against the same spec of record twice produces the same order both times.
- [x] 3.6 Add or confirm a test that a removed requirement's recovered scenarios (all `Deleted`) are still reported in spec-of-record order, unaffected by this change.

## 4. Verify

- [x] 4.1 Run `cargo test` and confirm all tests pass.
- [x] 4.2 Run `cargo clippy` and address any new warnings introduced by this change.
