## Why

Startup takes about 4 seconds in a repo with 34 archived changes ([#15](https://github.com/yshavit/opsxexplorer/issues/15)). `Changes::discover` sorts archived changes with a comparator that resolves each change's introducing commit inside the comparison, so an `O(m log m)` sort triggers roughly `2 · m · log m` independent history traversals — about 350 for 34 changes. Every one of them pays a full traversal, because `Sort::TIME` makes libgit2 enumerate and sort all reachable commits before yielding the first one; finding the answer early saves the per-commit probing but not the traversal itself.

Measured on synthetic 715-commit / 34-change repos, varying only where in history the changes were archived:

| archives introduced… | startup | one full `git log` walk |
| --- | --- | --- |
| in the first 34 commits | 0.44 s | — |
| evenly interleaved | 2.10 s | 0.14 s |
| in the last 34 commits | 34.4 s | 0.14 s |

Two things follow. The 80× spread is driven by a variable nobody controls, and it drifts the wrong way: changes are archived at HEAD, so as a repo grows its archives sit ever deeper behind the tip and the workload trends toward the worst column. And a single traversal is flat across all three layouts, at a fraction of even the best one — the traversal is not the expensive part, doing it hundreds of times is.

The same defect is paid again during use. `Changes::resolve` runs its own full traversal, and the TUI calls it on every selection change, so each `j`/`k` onto an archived change re-walks all of history to recompute a value that discovery already computed and discarded.

## What Changes

- Replace per-change history lookup with a single batched traversal: one pass over history resolves the introducing commit for every archived change at once, recording each change's first sighting.
- Keep the resolved commits on `Changes`, so the archived-list sort and `Changes::resolve` both read from one precomputed map instead of each re-deriving it.
- `Changes::resolve` becomes a map lookup and no longer walks history. Selecting an archived change stops costing a traversal.
- Store each change's diff-base commit (the introducing commit's first parent) at traversal time, alongside the timestamp the sort needs. `first_commit`'s existing `CommitInfo` already computes both and each of its two callers discards the half the other one wants; this change stops discarding.
- Drop the `Changes::repo` field, which exists only to serve those two now-precomputed lookups. That also removes one of the two `Repository::discover` calls at startup — `Workspace` already opens its own handle.
- Ordering of the archived list, and the diff base chosen for every change, are unchanged. The existing test suite is the oracle for that and must pass untouched.

### Non-goals

- **Switching the revwalk from commit-time to topological ordering.** Tracked separately as [#17](https://github.com/yshavit/opsxexplorer/issues/17). It is a latent correctness bug with a one-flag fix, but folding it in here would destroy this change's safety property: that every existing test passes unchanged, so a red test means the batching is wrong rather than the ordering is different.
- **Caching resolved trees and blobs across ref-views** ([#2](https://github.com/yshavit/opsxexplorer/issues/2)). Different cache, different layer: #2 memoizes tree resolution for a given commit inside the vfs, this change memoizes history-walk results. Neither subsumes the other and #2 stays open.
- **Persisting resolved commits across runs.** Ruled out in #15 (option 4) — branch divergence and storage-location problems outweigh the benefit.
- **Skipping history resolution when archive dates do not collide.** The introducing-commit timestamp is only a tiebreaker within one archive date, so in principle it could be computed lazily for tied groups only. This repo's 18 archived changes fall into 4 date groups of sizes 8, 5, 3 and 2 — every change is in a tie group, so the shortcut saves nothing under the workflow that motivated the issue.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `change-model`: adds two requirements about *when* and *how* diff bases are resolved. Neither changes which commit is chosen for any change — they pin down the resolution's timing and its cost profile, both of which are currently unspecified and both of which this change fixes in place. Existing requirements about which commit is chosen, and about unresolvable introducing commits, are unaffected.

## Impact

- `src/changes/history.rs`: `first_commit` (single-path traversal) is replaced by a batched entry point resolving many changes in one pass. `first_commit_time` and `resolve_archive_base` stop being traversal entry points.
- `src/changes/mod.rs`: `Changes` gains the resolved map and loses the `repo` field; `discover` sorts from the map; `resolve` reads from it.
- No dependency changes. No changes to `src/vfs/`, `src/specs/`, `src/diff/`, or `src/tui/` — `Changes::resolve` and `Changes::views` keep their signatures and their error types, so `load_diff_state` is untouched.
- Behavior visible to a user: startup and archived-change navigation get faster; nothing about the list's contents, its order, or any rendered diff changes.
