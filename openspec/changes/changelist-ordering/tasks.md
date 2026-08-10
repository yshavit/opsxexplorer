## 1. Fix revwalk sorting

- [ ] 1.1 In `src/changes/history.rs`, change `resolve_all`'s `revwalk.set_sorting(...)` from `Sort::TIME | Sort::REVERSE` to `Sort::TOPOLOGICAL | Sort::TIME | Sort::REVERSE`.
- [ ] 1.2 Update the doc comment above `resolve_all` (and `first_commit`/`resolve_archive_base` if separately documented) to describe the traversal as ancestor-first with time as a tiebreaker, not purely time-ordered.

## 2. Regression test

- [ ] 2.1 Add a test in `src/changes/history.rs`'s `#[cfg(test)] mod tests` that, using `stage_and_commit_at`, creates commit `P` (introducing an archived change directory) at an explicit later timestamp, then a child commit `Q` at an explicit earlier timestamp (simulating clock skew), and asserts `resolve_all`'s resolved diff base for that change is `parent(P)`, not derived from `Q`.
- [ ] 2.2 Confirm the test fails against the old `Sort::TIME | Sort::REVERSE` sorting (e.g. by temporarily reverting the flag change) and passes after it, then leave the flag change in place.

## 3. Spec and validation

- [ ] 3.1 Verify `openspec/changes/changelist-ordering/specs/change-model/spec.md`'s MODIFIED requirement matches the final implementation behavior.
- [ ] 3.2 Run `openspec validate --change changelist-ordering --strict` (or the project's equivalent) to confirm the delta spec is well-formed.

## 4. Verification

- [ ] 4.1 Run the full test suite (`cargo test`) and confirm it passes, including the new regression test.
- [ ] 4.2 Run `cargo clippy` (or the project's standard lint command) and fix any new warnings introduced by the change.
