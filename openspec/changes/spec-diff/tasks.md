## 1. Module scaffold and the diff model

- [ ] 1.0 **Repair the red baseline first — `cargo test` does not currently pass on this branch.** `specs::parse::tests::parses_change_model_main_spec` (`src/specs/parse.rs:357`) asserts the four requirement names `openspec/specs/change-model/spec.md` had before change 1 was archived; archiving it (commit `9c950bd`) synced its own delta into that main spec and added a fifth, `Both sides of a change's diff are reachable from the resolved change`. Add the fifth name and correct the scenario total. This is change 1's loose end, not change 2's work — fix it in its own commit so it does not muddy this change's diff. While there, note the general fragility for later: change 1's task 4.7 deliberately asserted against the repo's own live spec files, so **any** change that edits a main spec breaks these tests at archive time. Task 6.3 below already avoids the trap by inlining its fixture text; archiving *this* change only adds a new `openspec/specs/spec-diff/spec.md`, so it will not break the existing assertions.
- [ ] 1.1 Create `src/diff/` (`mod.rs`, `model.rs`, `compare.rs`, `runs.rs`, `error.rs`) and register `mod diff;` in `src/main.rs`. No `Cargo.toml` change: `similar` 3.1.2 is already declared and unused, and `TextDiff::from_words` is available under its default `text` feature — do **not** enable the non-default `inline` feature (see design.md).
- [ ] 1.2 Define in `src/diff/model.rs`: `Run { Equal { base: Range<usize>, delta: Range<usize> }, Delete { base: Range<usize> }, Insert { delta: Range<usize> } }` — byte offsets into the two body strings, never into the source file (see design.md).
- [ ] 1.3 Define `Piece { Unchanged { text }, Changed { base, delta, runs: Vec<Run> }, Added { delta }, Deleted { base }, Unmentioned { base } }`. One enum for every position — a requirement's intro and a scenario's body, under every operation — so change 3 has a single set of match arms to style.
- [ ] 1.4 Define `ScenarioDiff { name, body: Piece }`, `Operation { Added, Modified, Removed, Renamed { from: String } }`, `RequirementDiff { name, op, intro: Piece, scenarios: Vec<ScenarioDiff> }`, and `CapabilityDiff { capability, requirements: Vec<RequirementDiff>, errors: Vec<DiffError> }`. `RequirementDiff::name` is always the display name; for a rename that is the new name, with the former name on `Operation::Renamed`.
- [ ] 1.5 All fields are owned `String`s — the model must not borrow from the `SpecPair` or carry a lifetime (see design.md).
- [ ] 1.6 Define `DiffError::MissingBaseRequirement { capability, requirement }` in `src/diff/error.rs`, with hand-written `Display` / `Error` impls following `src/vfs/error.rs` and `src/specs/error.rs` — no `thiserror`. Its message must be recognisably different from `SpecError::MissingBaseSpec`'s: this one is a mistyped requirement name, that one is an absent spec of record.

## 2. Word-level runs

- [ ] 2.1 In `src/diff/runs.rs`, implement `runs(base: &str, delta: &str) -> Vec<Run>` over `similar::TextDiff::from_words(base, delta)`, walking `iter_all_changes()` and accumulating `value().len()` into one byte cursor per side. Merge adjacent changes sharing a tag into a single run. Do not trim, collapse whitespace, re-wrap or otherwise normalise either input — this layer diffs the strings `spec-model` gave it, unmodified.
- [ ] 2.2 Test the reconstruction invariant, which is the contract change 3 depends on: for a set of input pairs, slicing `base` by the `Equal`/`Delete` runs in order reproduces `base` exactly, and slicing `delta` by the `Equal`/`Insert` runs reproduces `delta` exactly. Include multi-line inputs, inputs differing only at the start, only at the end, and only in the middle, an empty base, an empty delta, and two identical strings.
- [ ] 2.3 Test word granularity, not line granularity: two long single-line bodies differing by a few words in the middle yield `Equal` runs covering the untouched text on either side, with `Delete`/`Insert` runs bounded to the differing words — not a whole-line delete plus a whole-line insert.
- [ ] 2.4 Test that runs are byte-offset-safe on multi-byte UTF-8 content (this repo's specs already contain `→`, `—` and similar): every run boundary lands on a `char` boundary, so slicing never panics.
- [ ] 2.5 Test the trailing-append case that the real validation data exercises: `delta` equal to `base` plus one appended sentence yields runs ending in exactly one `Insert` and containing **no** `Delete`.

## 3. Comparing one requirement

- [ ] 3.1 In `src/diff/compare.rs`, implement intro comparison by the uniform rule: delta intro empty → `Unmentioned { base }`; equal to base → `Unchanged`; otherwise → `Changed` with runs. Because `spec-model`'s `intro` is a `String`, an omitted intro and an emptied one both arrive as `""` and both yield `Unmentioned`. Do not add a special case trying to read an emptied intro as a deletion — the format defines no such gesture, and an absent intro is the sync skill's canonical MODIFIED form (see design.md).
- [ ] 3.2 Implement scenario matching **by name**, never by position: base scenarios first, in base order, each `Unchanged` when the bodies are equal and `Changed` with runs otherwise; then delta-only scenarios appended in delta order as `Added`; then base-only scenarios as `Unmentioned`, in their base positions.
- [ ] 3.3 Test that reordering a restated scenario changes nothing — the same set of scenarios listed in a different order in the delta yields all `Unchanged`, in base order.
- [ ] 3.4 Test the subset case end to end: a delta restating three of a base's four scenarios yields three resolved states and one `Unmentioned` carrying the base's body, with nothing reported as `Deleted`.

## 4. Requirement-level operations

- [ ] 4.1 Implement `pub fn diff(capability: &str, pair: &SpecPair) -> CapabilityDiff` in `src/diff/mod.rs`, returning the model directly rather than a `Result` — errors are per-entry and collected into `CapabilityDiff::errors`, so one bad entry never suppresses the requirements that are fine (see design.md).
- [ ] 4.2 Build the base index by requirement name (with `pair.base == None` treated as an empty index, not as a special case) and the rename index keyed by `Rename::to`.
- [ ] 4.3 `Added` entries: emit with intro and every scenario as `Added`, with no base lookup. Must work when `pair.base` is `None`.
- [ ] 4.4 `Modified` entries: if the name is some rename's `to`, set the entry aside for 4.6 and emit nothing here; otherwise look up the base by name and compare with the section 3 routine, or record a `MissingBaseRequirement` error.
- [ ] 4.5 `Removed` entries: look up the base by name, then emit the **base's** intro and scenarios as `Deleted` — the delta entry carries only a header and contributes no body. A miss records a `MissingBaseRequirement` error.
- [ ] 4.6 `Renamed` entries: look up the base under `Rename::from`, and compare it against the MODIFIED entry set aside in 4.4 when there is one, or against an empty requirement when there is not — in which case the uniform rule yields `Unmentioned` for the intro and for every base scenario, with no special-casing. A miss records a `MissingBaseRequirement` error naming the `from` name.
- [ ] 4.7 Emit the four groups in order — added, then modified, then removed, then renamed — preserving the delta's document order within each group. Change 3 relies on this ordering.
- [ ] 4.8 Test the ordering: a delta interleaving all four operations in its source document produces the four groups in the fixed order, in delta order within each.
- [ ] 4.8a Test that duplicate names resolve first-wins and do not panic: a base with two requirements sharing a name, and a requirement with two scenarios sharing a name, each match against the first occurrence (see design.md — this is an upstream contract violation, not an error this layer diagnoses).
- [ ] 4.9 Test determinism: comparing the same pair twice produces identical output (guards against any incidental `HashMap` iteration leaking into the result order).
- [ ] 4.10 Test that requirements the delta does not name produce no entry at all.

## 5. Missing-base errors

- [ ] 5.1 Test that a MODIFIED entry, a REMOVED entry, and a RENAMED entry each naming a requirement absent from the base produce a `MissingBaseRequirement` carrying the capability and that requirement name — no panic, no silent skip.
- [ ] 5.2 Test partial success: a delta with one bad entry and several sound ones reports the error **and** every sound entry.
- [ ] 5.3 Test the `base: None` path this layer closes: a delta whose only base-requiring entry is a rename loads with `base: None` (`spec-model`'s pre-check inspects only `Modified`/`Removed` entries), and must produce a `MissingBaseRequirement` rather than a panic (see design.md).

## 6. Test fixtures

- [ ] 6.1 Widen `src/specs/mod.rs` to expose the parsers as `pub(crate)` so `src/diff/` tests can build `Delta`/`Spec` values from markdown fixtures. Visibility only — no behaviour change, no public API change, no `spec-model` requirement affected.
- [ ] 6.2 Add a test helper that builds a `SpecPair` from a delta markdown string and an optional base markdown string, so every test in sections 3–5 reads as spec source rather than as hand-constructed nested structs.
- [ ] 6.3 Reproduce `archive/2026-08-08-tui-changelist-horizontal-scrolling` (capability `tui-changelist`, requirement `Archived changes are grouped under a collapsible section`) as an inline fixture — its delta text and its base text as of commit `3d5e380` — and assert the expected result exactly: intro `Changed` with one trailing `Insert` run and **no** `Delete` runs; scenarios `archived row collapsed by default`, `expanding reveals archived changes`, `collapsing hides archived changes` → `Unchanged`; scenarios `collapsed row is underlined`, `expanded row is not underlined`, `underline persists under horizontal scroll` → `Added`; zero `Unmentioned`; nothing `Deleted`. Copy the text into the test rather than reading the repo's own files, so the test does not break when this change is itself archived.
- [ ] 6.4 Confirm the fixture above also produces the change's three ADDED requirements as additions, in delta order, ahead of the modified one.

## 7. Correct `openspec/config.yaml`

- [ ] 7.1 Rewrite the `context:` block's "Core problem" paragraph: the MODIFIED section does **not** reliably print each requirement in full — it may restate the requirement or supply only the pieces that changed, and the two are indistinguishable. State that content present in the spec of record and unmentioned by the delta is ambiguous by construction and is surfaced as such.
- [ ] 7.2 Rewrite the per-requirement display bullets: modified requirements are shown as a word-level inline diff against the spec of record, not as a `+++`/`---` line diff. Keep the rest of the block (file layout, diff base selection, UI flow) as it is — it is still accurate.
- [ ] 7.3 Confirm nothing else in the repo repeats the corrected claim; fix it in the same commit if it does.

## 8. Verification

- [ ] 8.1 `cargo test` passes. Every existing test is unmodified except `parses_change_model_main_spec`, repaired in 1.0 for a reason unrelated to this change.
- [ ] 8.2 `cargo clippy` and `cargo fmt --check` clean.
- [ ] 8.3 Confirm `src/tui/` is untouched — this change adds no rendering.
- [ ] 8.4 Confirm `src/diff/` imports nothing from `crate::vfs`, `crate::changes` or `ratatui`, and no `similar` type appears in `src/diff/model.rs` or in any signature outside `runs.rs`.
- [ ] 8.5 Confirm the only edit under `src/specs/` is the `pub(crate)` visibility change from 6.1, and that `spec-model`'s tests still pass unmodified.

## 9. At archive time — deliberately NOT tasks, do not do these during apply

> Written as plain bullets, not `- [ ]` checkboxes, so `/opsx:apply` does not pick
> them up as pending work and `/opsx:archive` does not count them as incomplete.
> They describe cleanup that only makes sense **after** this change is archived.
> If you are running apply, skip this section entirely.

- Reduce `notes/spec-diff/02-spec-diff.md` to a pointer at
  `openspec/changes/archive/<date>-spec-diff/`, which becomes the record. Keep
  the filename: `03-tui-specdiff.md` links to it from its chain header, and
  deleting it outright leaves a dangling reference while that brief is still live.
- Do **not** delete `notes/spec-diff/` yet — that happens only once all three
  changes in the chain (`spec-model`, `spec-diff`, `tui-specdiff`) have landed.
- Check `notes/spec-diff/03-tui-specdiff.md` against what this change actually
  settled, in particular the rename operation being first-class and folded (so
  the right pane renders four operation groups, not three), the `Piece` state set
  it must style (including `Unmentioned`), and the source-text-versus-markdown
  constraint this change flagged and left to it.
- Reconcile that brief's four gutter states with the levels this change actually
  emits. `+`/`~`/`-` are requirement-level (`Operation`); `?` is piece-level
  (`Piece::Unmentioned`) and can never mark a requirement row, so scenario rows
  and the intro block need markers of their own. The intro is the sharp case: an
  `Unmentioned` intro renders the same text as an `Unchanged` one, so without a
  marker or dimming of its own the two are indistinguishable on screen even
  though this layer keeps them apart.
