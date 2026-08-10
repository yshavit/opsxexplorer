## Context

`compare_requirement` in `src/diff/compare.rs` (lines 94-139) currently builds `scenarios: Vec<ScenarioDiff>` in two passes: it walks `base.scenarios` first (pushing matched or `Unmentioned` pieces, deduplicating by name via a `seen: HashSet<&str>`), then walks `delta.scenarios` for anything not already `seen` (pushing `Added` pieces). The result is "base order, then delta-only appended." See proposal.md - Why for the motivation to change this to a category-first order.

`Piece` (src/diff/model.rs) already carries enough information to classify each scenario after the fact: `Added`, `Changed`/`Replaced` (the "modified" category), `Unmentioned`, `Unchanged`. `Piece::Deleted` also exists but never occurs inside `compare_requirement`'s output — it's produced separately, in the removal path that recovers a whole requirement's scenarios from the spec of record, where every scenario is `Deleted` and the existing (already base-ordered) iteration needs no change.

## Goals / Non-Goals

**Goals:**
- Reorder the `Vec<ScenarioDiff>` `compare_requirement` returns so scenarios group by category (added, modified, unmentioned, unchanged) before being handed to the TUI's row-flattening code.
- Keep the tie-break rule per category exactly as specced: delta order for added, base order for every other category.
- Keep the change local to the ordering step — no change to how a scenario is classified.

**Non-Goals:**
- No change to the removal path (`Piece::Deleted` scenarios recovered from the spec of record) — it is already single-category and already base-ordered, so it needs no sorting step.
- No change to requirement-level ordering (added/modified/removed/renamed grouping in `src/diff/mod.rs`) — untouched by this change.
- No change to `Piece`, `ScenarioDiff`, or any other data model type.

## Decisions

**Two-pass classify-then-sort, not a merged single pass.** Keep `compare_requirement`'s existing two-pass construction (base pass produces matched/`Unmentioned` scenarios, delta pass appends `Added` ones) exactly as is to preserve the base/delta index each scenario came from, then sort the resulting `Vec<ScenarioDiff>` once by a computed key before returning it. This is simpler than interleaving classification and ordering into one pass, and keeps the existing dedup-by-name (`seen: HashSet`) logic untouched.

**Sort key: `(category_rank: u8, tie_break_index: usize)`.** `category_rank` comes from a small match on `&Piece` (`Added` → 0, `Changed`/`Replaced` → 1, `Unmentioned` → 2, `Unchanged` → 3; `Deleted` is unreachable here and can `unreachable!()` or be given an arbitrary rank since this function never produces it). `tie_break_index` is the position in `base.scenarios` for every category except `Added`, where it's the position in `delta.scenarios`. Both indices are already available as the loop counter in the existing two loops — capture them into each `ScenarioDiff` construction as a local tuple `(ScenarioDiff, key)` before the final sort, then discard the key. Using `Vec::sort_by_key` (stable) rather than hand-rolled bucketing avoids a subtle bug class (forgetting a bucket, mis-ordering the four buckets) for a handful of elements per requirement — performance is a non-concern at this scale.

**Alternative considered — sort by `Piece` variant order directly (`#[derive(PartialOrd)]` on `Piece` or a wrapper enum) and rely on original `Vec` order as the intra-category tie-break.** Rejected: `Piece`'s variant declaration order doesn't match the required category order and would need to stay hand-maintained in sync with it, and a plain "stable sort by category, relying on insertion order for the rest" wouldn't correctly tie-break `Added` by delta order and everything else by base order, since insertion order today is "base scenarios then delta-only scenarios," which is not the same as "base order" for every non-added category once mixed with re-sorting — an explicit tie-break index avoids relying on incidental insertion order surviving the sort.

## Risks / Trade-offs

- [Existing tests assert the old base-first order] → Update `reordering_restated_scenarios_changes_nothing`, `subset_of_scenarios_restated_leaves_one_unmentioned`, and the end-to-end fixture in `diff/mod.rs` (`horizontal_scrolling_change_diffs_as_expected_against_its_base`) to the new category order; add new tests for a requirement that mixes all four categories at once, per specs/spec-diff/spec.md.
- [`Piece::Deleted` reaching the new sort key function unexpectedly] → It can't: `compare_requirement` never constructs a `Deleted` piece, only the separate removal path does, and that path doesn't call this sort function. Documented via a comment at the match site rather than a runtime check, since it's a compile-time-checkable invariant of this module's own construction.
