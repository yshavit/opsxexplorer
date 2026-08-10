## 1. Establish the baseline

- [x] 1.1 Record the current `cargo test` result for `src/changes/` — the full list of passing tests is the oracle for sections 2 and 3, so capture it before touching anything
- [x] 1.2 Record a startup timing baseline on a repo with a meaningful number of archived changes (this repo, and ideally the 34-change repo from issue #15) so section 5 has something to compare against

## 2. Batch the traversal in `src/changes/history.rs`

- [x] 2.1 Add a per-change record holding the introducing commit's timestamp and its optional diff-base commit, keeping the three outcomes in design.md — Decisions distinguishable (absent from the map, present with a base, present with no base for a root commit)
- [x] 2.2 Add a batched entry point that takes the archived changes and returns a map of that record, resolving all of them in one `Sort::TIME | Sort::REVERSE` traversal by recording each change's first sighting
- [x] 2.3 Drop each change from the working set as it resolves, and end the traversal early once the set is empty
- [x] 2.4 On a traversal error, return the entries resolved so far rather than discarding them or propagating the error
- [x] 2.5 Leave `Sort::TIME | Sort::REVERSE` unchanged — the ordering fix is issue #17 and must not be folded in here
- [x] 2.6 Remove `first_commit`, `first_commit_time`, and `resolve_archive_base` once nothing calls them, so exactly one function traverses history

## 3. Wire it into `Changes` in `src/changes/mod.rs`

- [x] 3.1 Build the map once in `Changes::discover`, before the sort, and store it on `Changes`
- [x] 3.2 Drive `archived.sort_by` from the map, keeping the existing comparison chain (archive date descending, then `cmp_introduced_at`, then directory name ascending) exactly as it is
- [x] 3.3 Reduce `Changes::resolve` to a map lookup, keeping its signature and both existing error cases — `FsError::NotAGitRepo` when there is no repository, `ChangesError::ArchiveHistoryNotFound` when there is no resolvable diff base
- [x] 3.4 Remove the `Changes::repo` field and its `Repository::discover` call, now that nothing in `Changes` needs a repository handle
- [x] 3.5 Confirm `src/tui/`, `src/specs/`, `src/diff/`, and `src/vfs/` needed no edits; if any did, work out why before continuing — the public shape was supposed to be unchanged

## 4. Verify against the oracle, then extend it

- [x] 4.1 Run `cargo test` and confirm every test recorded in 1.1 passes with **no edits to any existing test** — an existing test that needs changing means behavior moved, so stop and diagnose rather than adjusting the test
- [x] 4.2 Add a test that several archived changes introduced by different commits, interleaved with unrelated commits, each resolve to the same diff base that resolving them individually would produce
- [x] 4.3 Add a test that a resolved diff base does not move when further commits — including commits modifying the spec of record — are made after discovery
- [x] 4.4 Add a test that an archived change uncommitted at discovery stays unresolvable after its directory is committed, still appears in the archived list, and still sorts as a change with no resolvable introducing commit
- [x] 4.5 Add a test that a fresh `Changes::discover` after history changes resolves against the newer history
- [x] 4.6 Confirm the root-commit case still behaves as before: the change sorts by the root commit's timestamp, and resolving its diff base still errors

## 5. Confirm the win and close out

- [x] 5.1 Re-measure startup against the 1.2 baseline and confirm the improvement is on the order the proposal claims, not marginal
- [x] 5.2 Verify on a real repo — the 34-change one from issue #15 — not only on synthetic repos, per design.md — Risks
- [x] 5.3 Check that selecting archived changes in the TUI no longer costs a traversal per selection, by navigating the archived list in a large repo and confirming it feels responsive
- [x] 5.4 Run `cargo fmt --check` and `cargo clippy` clean
- [x] 5.5 Comment on issue #15 with the measured before/after, noting that options 2 and 3 are done and that option 4 remains deliberately not done
