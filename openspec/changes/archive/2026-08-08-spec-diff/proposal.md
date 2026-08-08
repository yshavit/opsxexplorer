## Why

`spec-model` can now load both sides of a change-and-capability pair, but nothing compares them. The whole point of opsxexplorer is that a `## MODIFIED Requirements` entry restates a requirement without saying what moved inside it; turning those two loaded sides into a per-requirement delta is the missing middle layer, and change 3 cannot render anything until it exists.

The comparison is not mechanical, because OpenSpec's delta format is genuinely ambiguous. A MODIFIED entry may restate a requirement in full, or supply only the pieces that changed — the two are syntactically indistinguishable, and OpenSpec has no operation for removing a single scenario. So a scenario present in the base and absent from the delta may have been dropped or may simply be untouched, and there is no way to tell. This change decides not to guess.

## What Changes

- Introduce a `spec-diff` capability: a pure comparison layer that takes the delta entries and base spec `spec-model` supplies and produces a per-requirement diff model. No file reads, no git, no styling, no wrapping.
- Emit requirement-level operations in a fixed order — added, then modified, then removed, then renamed — so change 3 can rely on it. Renames stay a first-class operation rather than being folded into a modification, so a rename reads as a rename.
- **Report base-only content as a third state, `Unmentioned`, rather than forcing it into removed or unchanged.** The uniform rule: absence in the delta means unmentioned; presence means authoritative for that piece. When an author does restate a requirement in full, no base-only pieces exist and the model collapses into the ordinary added/changed/unchanged picture, so the third state surfaces only where the format genuinely cannot answer the question.
- Apply the same rule to a modified requirement's intro: an empty intro yields `Unmentioned` (the base's intro shown as context), not an intro diffed against nothing, which would render the entire paragraph as deleted.
- Recover a removed requirement's intro and scenarios from the base, since the delta carries only its header and the UI has to show what is being removed.
- Report a modified or removed entry naming a requirement absent from the base as a displayable error against that entry — a real authoring mistake (mistyped header, hand-done rename), distinct from `spec-model`'s existing "no base spec file at all" error and not re-implementing it.
- **Emit word-level diff runs (`Equal` / `Delete` / `Insert`) over the body text, not `+`/`-` line pairs.** Spec prose lines run to hundreds of characters — the longest in this repo is 684 — so a line-level pair of one edited paragraph costs a dozen wrapped rows in change 3's ~76-column pane to convey a single appended sentence, printed twice, nearly identically. Runs carry offsets into the body strings `spec-model` supplies, so change 3 can word-wrap them while preserving their styling.
- Correct `openspec/config.yaml`'s `context:` block, which currently asserts that the MODIFIED section prints each requirement in full and that modified requirements are shown as a `+++`/`---` diff. Both are contradicted by this change and by OpenSpec's own sync tooling, and left uncorrected they will keep misleading later work.

## Capabilities

### New Capabilities

- `spec-diff`: comparing a change's delta entries against the spec of record to produce a per-requirement diff — requirement-level operations in a stable order, per-piece states for a modified requirement's intro and scenarios (including the ambiguity-preserving unmentioned state), word-level runs within changed pieces, and a displayable error for a delta entry with no matching base requirement.

### Modified Capabilities

<!-- None. spec-model's contract is used as specified; no requirement of it changes. -->

## Impact

- New `src/diff/` module: the diff model (requirement-level operations, per-piece states, runs) and the comparison itself. Consumes `crate::specs` types only.
- Activates the already-declared, currently unused `similar` 3.1.2 dependency; its `TextDiff` with `iter_inline_changes` provides the word-level tokenisation. No new dependencies.
- `src/specs/mod.rs`: the spec parsers become `pub(crate)` so the diff tests can build both sides from markdown fixtures rather than hand-constructed model values. Visibility only — no behaviour changes, and no requirement of `spec-model` changes.
- `openspec/config.yaml`: the `context:` block's description of MODIFIED semantics and of how modified requirements are displayed is corrected.
- No TUI changes; `src/tui/` is untouched.
- Flagged for change 3, not solved here: word-diff offsets are into source text, and `tui-markdown` strips markup such as `**` when rendering, so within a diffed region markdown rendering and run-accurate styling cannot both hold. This layer emits runs over source text and leaves the choice to the renderer.
- Out of scope: reading files or walking git history (`spec-model`, `change-model`, `filesystem`), all rendering and layout (change 3, `tui-specdiff`), and diffing whole spec files or requirements a change does not touch.
