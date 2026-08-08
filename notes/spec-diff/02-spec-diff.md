# Brief: `spec-diff` capability — compare a delta requirement against its base

> Chain position: **2 of 3**. Read → compare → render.
> 1. `spec-model` (`01-spec-model.md`) — parse a spec.md; load both sides.
> 2. **`spec-diff`** (this file) — compute the requirement-level delta.
> 3. `tui-specdiff` (`03-tui-specdiff.md`) — render it in the right pane.
>
> Depends on change 1 being implemented. This brief is self-contained. It is the
> output of an exploration session; the decisions in it were made deliberately
> and should be carried into the proposal.

**Superseded.** This brief has been carried into the change at
`openspec/changes/spec-diff/` (proposal.md, specs/, design.md, tasks.md), which
is now the source of truth. Once that change is archived this file gets reduced
to a pointer at its archive path, as `01-spec-model.md` already has been.

Two things below were reconsidered while writing the change, and the change's
artifacts win:

- The claim under "Decision: render the ambiguity as a third state" that
  **deliberately deleting a requirement's intro is "not merely ambiguous — it is
  invisible"** is wrong, and its framing worse. OpenSpec defines no operation for
  removing an intro, and the sync skill's canonical MODIFIED example has no intro
  at all — an empty intro is the *normal* form, not a deletion gesture. So there
  is no deletion being lost. An emptied intro yields `Unmentioned` exactly as a
  base-only scenario does: same state, equally visible. See design.md.
- **RENAMED**, which this brief left open, is a first-class operation, emitted
  once, with the base looked up under the former name — and a delta that renames
  a requirement *and* modifies it under the new name folds into that single
  entry rather than erroring or appearing twice.

## Project context

opsxexplorer is a terminal UI (Rust 2024, ratatui) for browsing OpenSpec changes
and specs in a local git repo. A change's delta spec has `## ADDED Requirements`
/ `## MODIFIED Requirements` sections, but MODIFIED does not show what actually
changed — it restates the requirement. **This tool computes that delta.** The
diff unit is a single requirement, not the whole file.

Change 1 supplies, for a given change + capability: the parsed delta spec (its
ADDED / MODIFIED / REMOVED / RENAMED entries) and the parsed base spec (the spec
of record at the correct diff base — live working tree for an active change, the
commit before archiving for an archived one).

`similar` 3.1.2 is already in `Cargo.toml`, unused. Its `TextDiff` plus
`iter_inline_changes` gives word-level diffing inside changed regions, which is
what this change needs.

## The central problem: MODIFIED is a patch, not a replacement

This is the single most important thing in this brief. **It contradicts
`openspec/config.yaml`**, which says "the MODIFIED section prints each
requirement in full". The project's own tooling says otherwise:

> **MODIFIED Requirements:** Apply the changes — this can be: adding new
> scenarios (*don't need to copy existing ones*)… **Preserve scenarios/content
> not mentioned in the delta**
> — `.claude/skills/openspec-sync-specs/SKILL.md:97`

> To add a scenario, just include that scenario under MODIFIED — don't copy
> existing scenarios
> — `.claude/commands/opsx/sync.md:186`

The canonical example in that skill shows a MODIFIED requirement with **no intro
paragraph at all**, just one new scenario. Meanwhile this repo's one real
MODIFIED (`archive/2026-08-08-tui-changelist-horizontal-scrolling`) restates the
requirement in full. Both styles occur, and **they are syntactically
indistinguishable**:

```
  base has:  intro, S_a, S_b, S_c, S_d
  delta has: intro', S_a, S_b, S_c

  full-replacement reading  →  S_d was DELETED
  patch reading             →  S_d is UNTOUCHED
```

Compounding it: **OpenSpec has no delta operation for "remove one scenario."**
REMOVED operates on whole requirements only. So under a strict reading, a
dropped scenario is unexpressible.

### Decision: render the ambiguity as a third state

Do **not** guess. A base-only scenario is reported as a distinct
**`Unmentioned`** state — "present in base, not mentioned in delta; cannot
determine whether dropped or untouched" — rather than being forced into either
`Removed` or `Unchanged`.

The general rule, applied uniformly:

> **Absence in the delta means "unmentioned". Presence means "authoritative for
> that piece."**

This also settles the omitted-intro case: a MODIFIED entry with **an empty intro
block** yields an `Unmentioned` intro (show the base's intro as context), *not*
an intro diffed against empty (which would render the whole paragraph as
deleted).

> **Trigger reworded after change 1 was specified.** This originally said "with
> no intro block." Change 1 cannot distinguish an omitted intro from an empty
> one — markdown has no way to express the difference — so both arrive as an
> empty string, and the rule keys off emptiness. The consequence is worth
> stating outright rather than leaving to be discovered: **deliberately deleting
> a requirement's intro is inexpressible.** An author who empties an intro in a
> MODIFIED entry gets `Unmentioned`, i.e. the base's intro shown as unchanged
> context. That is the correct behaviour under this brief's own "absence means
> unmentioned" rule, and it is consistent with OpenSpec having no delta
> operation for removing a sub-part of a requirement (the same reason a dropped
> scenario is unexpressible), but it does mean the deletion is not merely
> ambiguous — it is invisible.

Why this is the right call rather than a hedge: when an author does restate the
requirement in full, there are no base-only scenarios, so the model collapses
exactly into the straightforward added/modified/unchanged picture. The third
state only ever appears in precisely the situation where the format genuinely
cannot tell you the answer. Rejected alternatives — assume full replacement
(shows false deletions for skill-conformant deltas), assume patch (can never
surface a genuine intended drop) — both silently lie in one direction.

**`openspec/config.yaml`'s `context:` block should be corrected** as part of this
change, since it currently asserts the full-replacement reading and will keep
misleading future work.

## Scope of this change

A pure comparison layer — no I/O, no rendering. Given (delta entries, base
spec), produce a per-requirement diff model.

### Requirement-level operations

Emit, in this order: **ADDED, then MODIFIED, then REMOVED.** (Change 3 relies on
this ordering.) RENAMED needs a decision — either its own fourth operation, or
resolved into a MODIFIED whose name itself diffed. Recommend treating it as a
first-class operation so the rename is visible as a rename.

- **ADDED** — entire requirement is new. Intro and every scenario are pure
  insertions. No base lookup needed (and the base spec file may not exist at
  all: `2026-08-07-tui-initial` introduces two brand-new capabilities).
- **REMOVED** — the delta carries only the header. **The body must be pulled
  from the base spec**, since the UI displays a removed requirement's intro and
  scenarios. Everything is a pure deletion.
- **MODIFIED** — the interesting case, below.

A MODIFIED or REMOVED entry naming a requirement that does not exist in the base
spec is a real failure mode (typo in the header, or a rename done by hand). It
must surface as a displayable error, not a panic and not a silent skip.

> **Split of responsibility, settled in change 1.** There are two distinct
> versions of "no base to modify," and they have different owners:
>
> - **The base spec file does not exist at all.** Change 1 owns this and raises
>   `MissingBaseSpec { capability, requirement }` at load time, before change 2
>   ever runs. An all-ADDED delta against an absent base is *not* an error (real
>   case: `2026-08-07-tui-initial`); only a MODIFIED or REMOVED entry makes it
>   one.
> - **The base spec exists but does not contain that requirement.** This one is
>   change 2's, and is the case described above.
>
> Change 2 should not re-implement the first check. Keeping them separate is
> deliberate: a whole missing spec of record and a single mistyped requirement
> name are different authoring mistakes and deserve different messages.

### Inside a MODIFIED requirement

Requirements and scenarios are matched **by name** — the text after
`### Requirement: ` / `#### Scenario: `.

Intro block:
- absent in delta → `Unmentioned` (render base intro as context)
- present and equal to base → `Unchanged`
- present and different → `Changed`, with word-level diff runs

Scenarios, in base order first, then delta-only additions appended:
- in both, bodies equal → `Unchanged`
- in both, bodies differ → `Changed`, with word-level diff runs
- delta only → `Added`
- base only → `Unmentioned`

### Word-level diff, not line-level

**Decided: word-diff (`git diff --word-diff` style), not `+`/`-` line pairs.**

Spec prose lines are enormous — the longest in this repo is 476 characters
(`openspec/specs/tui-changelist/spec.md:115`). In change 3's right pane (~76
columns) a line-level `+`/`-` of that paragraph costs **twelve wrapped rows to
convey one appended sentence**, with the two versions rendered nearly identically
one above the other. Word-diff renders one reflowed paragraph with deletions and
insertions marked inline.

So this layer's output for a changed block is **a sequence of runs** — `Equal`,
`Delete`, `Insert` — over the source text, not a pair of before/after line sets.
`similar::TextDiff` with `iter_inline_changes` (word-level tokenisation) is the
intended tool.

Two consequences to record in design.md:

- **Run boundaries must be expressible as offsets into the body text change 1
  supplies**, because change 3 has to word-wrap the runs across pane width while
  preserving their styling. Change 2 must not normalise whitespace or strip
  markdown *further* — it diffs the strings it is given, unmodified.

  > **Updated after change 1 was specified.** This originally read "offsets into
  > faithful source text." Change 1 no longer supplies byte-faithful source: its
  > bodies are `mdq`'s re-serialisation of the parsed markdown tree — no content
  > lost and no re-wrapping, but bullet markers, emphasis characters and
  > escaping may differ from the file. Offsets are therefore into *that* string,
  > and neither change 2 nor change 3 can map them back onto the file's bytes.
  > What change 1 does now guarantee, as a spec requirement, is that **both
  > sides receive identical normalisation** — which is precisely the property
  > the `Unchanged` determinations below rely on, and which byte-faithfulness
  > was only ever an indirect way of getting.
- **Word-diff and markdown rendering are mutually exclusive in diffed regions.**
  `tui-markdown` strips the `**` from `**WHEN**`, so offsets computed here no
  longer map onto rendered spans. Change 3 will have to choose; this layer
  should emit runs over *source* text and let change 3 decide. Flag it, don't
  solve it here.

Scenario bullet lines are short (~100 chars) and don't suffer the wrap
explosion, but use word-diff there too for consistency — a mixed scheme is
harder to explain than it is worth.

## Non-goals

- Reading files or walking git history — change 1 and the existing
  `change-model` / `filesystem` capabilities own that.
- Any styling, colour, wrapping, or TUI concern — change 3.
- Diffing whole spec files, or requirements the change doesn't touch. Only
  ADDED / MODIFIED / REMOVED (/ RENAMED) entries are ever shown.

## Validation data in this repo

The one real MODIFIED, and what a correct implementation must produce for it:

**`archive/2026-08-08-tui-changelist-horizontal-scrolling`**, capability
`tui-changelist`, base = commit `3d5e380` (parent of `27b90c8`):

- 3 ADDED requirements — `Left pane scrolls horizontally as a single unit`,
  `Horizontal scroll position is indicated with a scrollbar`, `Horizontal scroll
  offset persists across selection and section toggling, clamped to current
  content`.
- 1 MODIFIED — `Archived changes are grouped under a collapsible section`:
  - intro: `Changed`. Base ends `…SHALL NOT appear in the list.`; delta appends
    ` While collapsed, the \`archived\` row SHALL render with an underline
    style; while expanded, it SHALL NOT.` A correct word-diff yields **one
    trailing insert run and no delete runs**.
  - scenarios `archived row collapsed by default`, `expanding reveals archived
    changes`, `collapsing hides archived changes` → `Unchanged`.
  - scenarios `collapsed row is underlined`, `expanded row is not underlined`,
    `underline persists under horizontal scroll` → `Added`.
  - **zero `Unmentioned`, zero removed.**

Note this example does *not* discriminate between full-replacement and patch
semantics — it restates in full, so both readings agree. The `Unmentioned` path
has no real-world case in this repo and needs synthetic fixtures, as do REMOVED
and RENAMED.

Other changes (`2026-08-07-tui-initial`, `-add-readonly-filesystem`,
`-change-modeling`) are all-ADDED, and the first of those has **no base spec
file at all** at its diff base.

`src/changes/mod.rs` and `src/vfs/mod.rs` have `test_support` modules with
`TempDir` / `write_file` / `stage_and_commit` helpers for building throwaway git
repos; follow that pattern.

## Conventions

Design docs here are dense and argumentative: every decision states the
alternative considered and why it was rejected, and `## Risks / Trade-offs`
entries say whether the risk is resolved-by-design or knowingly deferred. See
`openspec/changes/archive/2026-08-08-tui-changelist-horizontal-scrolling/design.md`.
Match it.
