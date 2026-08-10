## Context

See proposal.md — Why for the motivation and measurements.

The relevant current shape is in `src/changes/history.rs`. One private function does all history work:

```
first_commit(repo, path) -> Option<CommitInfo { oid, time }>
   ├── first_commit_time(repo, change)    -> keeps .time,  discards .oid
   └── resolve_archive_base(repo, change) -> keeps .oid,   discards .time
```

Each caller discards precisely what the other one wants, and each pays a full traversal to recompute it. `first_commit_time` is called from inside `Changes::discover`'s `sort_by` comparator; `resolve_archive_base` is called from `Changes::resolve`, which the TUI invokes on every selection change via `load_diff_state`.

Three properties of the existing code constrain the design:

- The traversal is `Sort::TIME | Sort::REVERSE` and the answer is "the first commit in that order whose tree contains the change's directory." Early `return` on a hit does not avoid the traversal's cost: `Sort::TIME` makes libgit2 enumerate and sort all reachable commits before yielding the first.
- Failures are swallowed per change. A revwalk error yields `None`, which the sort treats as "unresolvable" and orders ahead of any resolvable timestamp; the change still appears in the list.
- A change introduced by a root commit is a distinct case from a change with no introducing commit at all. `first_commit_time` returns `Some(time)` for it while `resolve_archive_base` errors, because the root commit has no first parent. `history.rs`'s `first_commit_time_resolves_for_root_commit_introduction` asserts both halves.

`Changes` holds a `Repository` used only by those two call sites, while `Workspace` independently opens its own handle from the same path.

## Goals / Non-Goals

**Goals:**

- One traversal of history per discovery, regardless of how many archived changes exist.
- No traversal at all when a diff base is obtained for an already-discovered change.
- Byte-identical results: same archived-list order, same diff base per change, same treatment of unresolvable and root-commit cases.
- Keep the existing test suite usable as a correctness oracle, unmodified.

**Non-Goals:**

- Anything listed under proposal.md — Non-goals (topological ordering #17, vfs tree caching #2, cross-run persistence, date-collision shortcut).
- Changing the public shape of `Changes::resolve` / `Changes::views` or their error types. The TUI must not need edits.
- Making discovery itself incremental or lazy. Discovery stays eager; only the number of traversals changes.

## Decisions

### Resolve every archived change in one traversal, recording first sighting

Walk history once in the existing `Sort::TIME | Sort::REVERSE` order over a working set of unresolved changes. At each commit, probe the still-unresolved changes; on a hit, record the result and drop that change from the working set. Stop when the working set empties or history is exhausted.

**Why this is safe:** a single oldest-first traversal that records each change's *first sighting* returns, for every change, exactly what an independent oldest-first traversal for that change alone would have returned. The traversal order is identical and the per-change predicate (`tree.get_path(dir).is_ok()`) is identical, so the first commit satisfying it is the same commit. This equivalence is what lets the existing tests stand unchanged as the oracle — see Risks.

*Alternative considered — memoize `first_commit_time` per change inside the comparator* (#15 option 1). Cuts traversals from ~350 to ~34, roughly a 10× win, and is a much smaller diff. Rejected because it leaves the cost proportional to the number of archived changes, leaves `resolve_archive_base` walking history on every selection, and would have to be undone to get either of those. The batched form subsumes it.

### Record the diff-base commit during the traversal, not on demand

The map stores, per change, both the introducing commit's timestamp and its first parent — the diff base. The traversal has the commit in hand, so taking `parent_id(0)` there is free.

This is what makes `Changes::resolve` a pure lookup rather than a lookup plus a `find_commit`. With it, `Changes` no longer needs a `Repository` at all: the field goes away, and with it the second `Repository::discover` at startup (`Workspace` already opens one).

*Alternative considered — store only the introducing commit's oid and take the parent in `resolve`.* Marginally less state, but keeps `Changes::repo` alive for a single call, which is most of what the field costs.

### Distinguish "no introducing commit" from "introduced by a root commit"

Three outcomes must stay distinguishable, because the existing tests assert all three:

| Outcome | Sort behavior | `resolve` behavior |
| --- | --- | --- |
| Not found in history | unresolvable — orders ahead of resolvable timestamps | `ArchiveHistoryNotFound` |
| Found, commit has a first parent | orders by that commit's timestamp | diff base is that parent |
| Found, commit is a root commit | orders by that commit's timestamp | `ArchiveHistoryNotFound` |

So the stored value cannot be a bare `Option<GitRef>` keyed on presence in the map: absence from the map means the first row, while presence with no parent means the third. A per-change record carrying a timestamp and an optional base keeps the three apart.

### Keep `Sort::TIME | Sort::REVERSE` exactly as-is

Deliberately unchanged, even though [#17](https://github.com/yshavit/opsxexplorer/issues/17) argues it is wrong. Changing the ordering in the same commit as the batching would forfeit the property that every existing test passes untouched, turning any red test into "did I break the batching, or is this the intended ordering change?" #17 is a one-flag change with its own regression test and is cleanly separable afterwards.

### Whittle the working set; leave the per-commit descent alone for now

Dropping resolved changes from the working set is worth doing — it is nearly free. It is worth being clear that it is *not* where the win comes from, so nobody later mistakes it for the point of the change. For history where change *i* is introduced around commit `21i`:

```
per-change traversals, memoized:  Σ 21i,  i=1..34           ≈ 12,500 probes
one traversal, whittled:          Σ (34 − c/21), c=1..715   ≈ 12,100 probes
```

Essentially identical probe counts. The win is traversals: ~350 → 1.

A further refinement is available if probing ever shows up in a profile: all archived changes are siblings under one directory, so a single descent to that directory per commit, checking its entry names against a set, replaces one `tree.get_path` per unresolved change with one descent plus hash lookups. Deferred rather than adopted, because it trades the current path-generic helper for one coupled to the archive layout, and the measured cost is dominated by traversal count rather than by per-commit probing. Revisit only with a profile that says otherwise.

### On a traversal error, return what was resolved so far

A mid-traversal error abandons the remaining unresolved changes but keeps every change already resolved. Those abandoned changes land in the existing "unresolvable introducing commit" path, which the specs already define: they sort ahead of resolvable ones and still appear in the list.

This is a slight widening of today's blast radius — currently a failure affects one change, now it affects all changes not yet resolved when it happens. It is bounded by an already-specified behavior rather than introducing a new failure mode, and it preserves `first_commit_time`'s "infallible from the caller's perspective" contract.

*Alternative considered — propagate the error and fail discovery.* Rejected: it converts a currently-degraded case into a hard startup failure, which is strictly worse for a browsing tool.

### Resolution is a snapshot taken at discovery

Consequence of doing the work once, and specified in `specs/change-model/spec.md` rather than left implicit. Discovery already fixes which changes exist and whether each is active or archived; extending the same boundary to the history they resolve against makes one discovery result internally consistent instead of partly live.

The observable difference is narrow. A change already resolved cannot have its *earliest* introducing commit changed by later commits, so ordinary work during a session changes nothing. Two cases do differ: an archived directory that was uncommitted at discovery and is committed mid-session stays unresolvable, and a history rewrite mid-session is not picked up. Both were previously re-derived on selection.

## Risks / Trade-offs

**The equivalence argument is the whole safety case, and it is easy to break by accident** → The existing tests are the oracle, and they only work as one if they stay untouched. Land the batching with every existing test in `src/changes/` unmodified and green. If a test needs editing to pass, that is the signal that behavior moved — stop and work out why rather than adjusting the test. Add new tests alongside; do not rewrite old ones.

**Snapshot semantics silently stale a mid-session commit** → Accepted and specified. Narrow in practice (see the decision above), consistent with discovery already being a snapshot, and re-running discovery observes the newer history.

**One error now abandons every unresolved change instead of one** → Bounded by the existing "unresolvable introducing commit" behavior: those changes still appear and still sort deterministically. No new failure mode reaches the user.

**A later contributor folds #17 in and cannot tell which change caused a regression** → The decision above and #17 both record the sequencing reason. #17 carries its own regression test, which fails before the flag change and passes after, so it does not depend on this change's oracle.

**Measurements came from synthetic repos** → The synthetic repos pile noise files at the repository root, so their trees are fatter than a real repo's and the absolute timings are inflated. The 80× spread across archive placement, and the flatness of a single traversal, are the load-bearing findings and are structural rather than artifacts of tree width. Confirm against a real repo — the one from #15 with 34 changes — before calling the change done.

## Migration Plan

Internal refactor: no data migration, no persisted state, no API surface beyond the crate. Rollback is reverting the commit.

Sequencing matters more than deployment here. Land the batched traversal with existing tests unmodified first; only then add new tests for the snapshot semantics. Keeping those two steps separable is what preserves the oracle described above.
