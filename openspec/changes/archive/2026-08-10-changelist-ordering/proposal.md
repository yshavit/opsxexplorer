## Why

`history::resolve_all` walks commit history sorted by `Sort::TIME | Sort::REVERSE` (committer timestamp) to find, for each archived change, the "earliest commit that introduced" its directory. Committer time is only a proxy for ancestry: if a commit has an earlier committer time than its own parent (clock skew, `GIT_COMMITTER_DATE` overrides, imported/converted history), the walk can reach a descendant before the ancestor that actually introduced the directory. When that happens, the archived change's diff base resolves to a commit that already contains the change's own archived spec deltas, so the delta renders as empty or near-empty against a spec of record that has already absorbed it — silently wrong, with no error. The `change-model` spec's "earliest commit that introduced" wording is ambiguous between time order and ancestry order; this change also disambiguates it.

## What Changes

- Sort the revwalk in `history::resolve_all` topologically (`Sort::TOPOLOGICAL | Sort::TIME | Sort::REVERSE`) instead of by commit time alone, so "first sighting" reflects ancestry rather than committer timestamps. Time remains the tiebreaker among commits with no ancestry relationship to each other, keeping the walk order deterministic.
- Clarify the `change-model` spec's "earliest commit that introduced" requirement to define "earliest" as ancestor-first (topological) order, not committer-timestamp order, and add a scenario covering non-monotonic committer time across a parent-child edge.
- Add a regression test using `test_support::stage_and_commit_at` (commit `P` introducing a change directory, then a descendant `Q` with an earlier explicit committer timestamp) asserting the resolved diff base still anchors to `parent(P)`.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `change-model`: "Diff base for an archived change is the commit before the directory first appears" is clarified so "earliest" means ancestor-first (topological) order, not committer-timestamp order, with a new scenario for non-monotonic committer time.

## Impact

- `src/changes/history.rs`: `resolve_all`'s revwalk sort flags, plus a new regression test in its `#[cfg(test)] mod tests`.
- `openspec/specs/change-model/spec.md`: clarified requirement text and an added scenario.
- No API or on-disk format changes. No behavior change for repositories with monotonic committer time (the common case); only affects resolution when history has non-monotonic committer timestamps.
