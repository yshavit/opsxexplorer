## 1. History lookup

- [x] 1.1 In `src/changes/history.rs`, extract a private `first_commit(repo: &Repository, path: &Path) -> Option<CommitInfo>` helper (a small struct holding `oid: Oid` and `time: i64`) that runs the existing `Sort::TIME | Sort::REVERSE` revwalk from HEAD and returns the earliest commit (oid + timestamp) whose tree contains `path`, or `None` on any failure (path never found, revwalk/git error).
- [x] 1.2 Refactor `resolve_archive_base` to call `first_commit`, then resolve that commit's parent via `repo.find_commit(oid)?.parent_id(0)`, keeping its existing `Result<GitRef, ChangesError>` signature and error behavior unchanged (both "path never introduced" and "introducing commit has no parent" still return `Err(ChangesError::ArchiveHistoryNotFound)`).
- [x] 1.3 Add `pub fn first_commit_time(repo: &Repository, change: &Change) -> Option<i64>` to `src/changes/history.rs` as a thin wrapper over `first_commit(repo, &change.relative_path())`, mapping to its `time`.
- [x] 1.4 Add unit tests in `history.rs` covering `first_commit_time`: a change introduced by a non-root commit returns `Some` with the introducing commit's timestamp (not a later commit that also touched the directory); a change never committed (path absent from all history) returns `None`; a change introduced by the repository's root commit (no parent) still returns `Some` — the case where it now diverges from `resolve_archive_base`, which errors here.
- [x] 1.5 Run the existing `resolve_archive_base` tests in `src/changes/mod.rs` (`archived_change_diff_base_is_commit_before_it_was_introduced`, `archived_change_diff_base_anchored_to_earliest_commit_touching_it`, `archived_change_diff_base_unaffected_by_later_history`, and related `ChangeView`/`views` tests) and confirm they still pass unmodified after the refactor.

## 2. Sorting archived changes

- [x] 2.1 In `src/changes/mod.rs`, after `discovery::discover_archived` runs in `Changes::discover`, sort the resulting `Vec<Change>` in place using a comparator keyed on `(change.archive_date(), repo.as_ref().and_then(|r| history::first_commit_time(r, change)), &change.0)`: date descending, then timestamp descending (`None` treated as greater than any `Some(_)`), then dirname ascending as the final tiebreaker.
- [x] 2.2 Leave `discover_active`'s output and `active` field untouched (still ascending via the existing `list_dir` sort).

## 3. Tests

- [x] 3.1 Add a `Changes::discover` test with archived changes carrying different date prefixes, asserting `changes.archived` is ordered most-recent-date first.
- [x] 3.2 Add a `Changes::discover` test with two archived changes sharing the same date prefix but introduced by different commits, asserting the more recently introduced one sorts first.
- [x] 3.3 Add a `Changes::discover` test where one of two same-date archived changes has no resolvable introducing commit (e.g. present in the working tree but never committed), asserting it sorts before the one with a resolvable commit.
- [x] 3.4 Add a `Changes::discover` test with two archived changes tied on date and (absent or equal) introducing-commit timestamp, asserting the tiebreak falls back to directory name, ascending.
- [x] 3.5 Add a `Changes::discover` test with an archived change whose introducing commit cannot be resolved, asserting it still appears in `changes.archived` (not dropped) rather than only checking its relative position.
- [x] 3.6 Run `cargo test` for the `changes` module and confirm existing tests (e.g. `discovers_active_and_archived_changes`) still pass unmodified.

## 4. Spec sync check

- [x] 4.1 Re-read `openspec/changes/changelist-archived-ordering/specs/tui-changelist/spec.md` against the final implementation and confirm every scenario (descending date order, same-date tiebreak, unresolvable-commit-sorts-newest, unresolvable timestamp doesn't drop the row, dirname tiebreak) is covered by a test from section 3.
