## Context

See proposal.md — Why. `spec-model` (`src/specs/`) already delivers everything this layer consumes: `load(&ChangeViews, capability) -> Result<SpecPair, SpecError>`, where `SpecPair { delta: Delta, base: Option<Spec> }`, `Delta { entries: Vec<DeltaEntry>, renames: Vec<Rename> }`, `DeltaEntry { op: DeltaOp, requirement: Requirement }`, `DeltaOp` ∈ {`Added`, `Modified`, `Removed`}, and `Requirement { name, intro: String, scenarios: Vec<Scenario> }`. Two shapes of that model drive most of what follows:

- **`intro` is a `String`, not an `Option<String>`.** `spec-model` decided this deliberately: an omitted intro block and an emptied one differ only by a blank line in the source, which carries no authorial intent, so both parse to `""`. This layer therefore cannot distinguish them either, and must key its rule off emptiness.
- **`renames` is a separate list of `Rename { from, to }`, not a `DeltaOp` variant** — a RENAMED entry is `- FROM:`/`- TO:` bullets with no heading, intro or scenarios of its own.

`spec-model` also guarantees that both sides are normalised *identically* (its "Body content is preserved in full and normalised identically on both sides" requirement), while explicitly not promising byte-identity with the file: bodies are `mdq`'s re-serialisation of the parsed tree. Identical normalisation is exactly the property `Unchanged` needs, and byte-faithfulness was only ever an indirect way of getting it.

`similar` 3.1.2 is already in `Cargo.toml` and currently unused by any code in `src/`.

## Goals / Non-Goals

**Goals:**
- One pure function from `(&str capability, &SpecPair)` to a diff model. No `Fs`, no `git2`, no `ratatui` in the module's dependency set.
- Preserve the format's genuine ambiguity rather than resolving it in either direction.
- Emit run boundaries as byte offsets into the exact strings `spec-model` supplied, so change 3 can word-wrap them while keeping styling.

**Non-Goals:**
- Re-deriving anything `spec-model` already decided — the diff base, whether a spec file exists, whether markdown parsed.
- Choosing between run-accurate styling and markdown rendering inside a diffed region. Flagged below; change 3's call.
- A whole-file diff, or any notion of hunks, context lines or unified-diff output.

## Decisions

**`Unmentioned` is a third state, not a lean toward `Removed` or `Unchanged`.** A MODIFIED entry may restate a requirement in full or supply only the pieces that changed, and the two are syntactically indistinguishable; OpenSpec has no operation for removing one scenario, so a dropped scenario is unexpressible in the format. Given base scenarios `S_a S_b S_c S_d` and a delta listing `S_a S_b S_c`, the full-replacement reading says `S_d` was deleted and the patch reading says it was untouched, and nothing in the document decides between them. Note this contradicts `openspec/config.yaml`'s `context:` block, which asserts the full-replacement reading; OpenSpec's own tooling asserts the opposite (`.claude/skills/openspec-sync-specs/SKILL.md`: "adding new scenarios (don't need to copy existing ones)… Preserve scenarios/content not mentioned in the delta"), and its canonical example shows a MODIFIED requirement with no intro and a single new scenario. Both styles occur in the wild — this repo's one real MODIFIED restates in full. The config block is corrected as part of this change rather than left to mislead later work.

Alternatives considered: assume full replacement — shows false deletions for every skill-conformant delta, which is the style OpenSpec's own tooling teaches. Assume patch — can never surface a genuinely intended drop. Both silently lie, in opposite directions, in exactly the case where the format has no answer. `Unmentioned` is not a hedge: when an author restates in full there are no base-only pieces, so the model collapses into the ordinary added/changed/unchanged picture, and the third state appears only where the ambiguity is real.

**The rule is stated once and applied everywhere: absence in the delta means unmentioned, presence means authoritative for that piece.** Applying it to the intro settles the omitted-intro case — an empty delta intro yields `Unmentioned` carrying the base's intro as context, not an intro diffed against `""`, which would render the entire paragraph as a deletion. Applying it to a rename with no accompanying MODIFIED entry falls out for free (see below). The uniformity is the point: one rule, no per-position special cases to remember.

**The intro case is the same ambiguity as the scenario case, not a worse one.** `spec-model` cannot distinguish an omitted intro from an emptied one, so both reach this layer as `""`, and both resolve to `Unmentioned`. It is tempting to describe that as losing a deliberate deletion, but there is no such deletion to lose: OpenSpec has no operation for removing a requirement's intro, and the sync skill's canonical MODIFIED example goes straight from `### Requirement: Existing Feature` to `#### Scenario:` with no intro at all. An empty intro is the documented *normal* form of a MODIFIED entry. Reading it as a deletion gesture would be assuming an intent the format never assigns — the full-replacement mistake again, aimed at one slot instead of the scenario list. `Unmentioned` reports what is actually true, which is that the delta said nothing, and it is if anything the more confidently correct of the two applications of the rule: empty intros are common and almost always mean "untouched", while base-only scenarios are rare.

**A rename is a first-class operation, emitted once, with the base looked up under the former name.** The brief left this open — either a fourth operation or a MODIFIED whose name itself diffed. As a name diff, a rename reads as a large text change with no signal that it *is* a rename, and the requirement's real content diff gets crowded out. As a fourth operation it stays legible, and the ordering added → modified → removed → renamed keeps the first three exactly where the brief fixed them.

The interesting sub-case is a delta that both renames a requirement and modifies it under its new name — the only way to express "renamed and edited", since a rename entry has no body. Handled by resolving lookups through the rename map: a MODIFIED entry whose name is some rename's `to` is folded into that rename's entry, and its base counterpart is found under the rename's `from`. Without this, that entry's new name would be absent from the base and the change would report a spurious missing-base error for a perfectly well-formed delta, and the requirement would appear twice. When no such MODIFIED entry exists, the rename is compared against a synthetic empty delta requirement, and the uniform rule does the rest: intro `Unmentioned`, every base scenario `Unmentioned` — which is honest, because the delta genuinely says nothing about the body.

Alternative considered: emit the rename and the modification as two entries. Rejected — the requirement appears twice in the pane, and neither entry is the whole story.

**A missing base requirement is a per-entry error, not a per-capability failure.** `spec-model` established per-capability isolation (one malformed capability does not block the others); this extends the same principle one level down. A mistyped `### Requirement:` heading in a MODIFIED entry should not blank the pane for the eleven requirements that are fine. So the entry point returns a diff carrying both the requirements it could compute and a list of errors, rather than a `Result` that discards the good work on the first bad entry.

Alternative considered: fail the whole capability with a `Result::Err`. Rejected — strictly less information for the user, and change 3 would then have to render a capability tab that is either entirely content or entirely error.

**This layer does not re-check whether the base spec file exists — with one gap it closes as a side effect.** `spec-model` raises `MissingBaseSpec` at load time when the base file is absent and the delta contains a `Modified` or `Removed` entry, so those two never reach this layer with `base: None`. Its check does *not* consider `renames`, so a delta whose only base-requiring entry is a rename loads with `base: None`. Rather than reopen change 1's spec for this, `base: None` is treated here as an empty base for lookup purposes; every entry that needs a base and cannot find one produces the same per-entry error. In practice that path fires only for renames, and it does so with a message naming the requirement — which is the right message for the mistake anyway. The two errors stay distinct in kind, as the brief requires: a whole missing spec of record and a single mistyped requirement name are different authoring mistakes.

**Word-level runs, not `+`/`-` line pairs.** Spec prose lines are enormous — the longest in this repo is 684 characters (`openspec/specs/spec-model/spec.md:10`, comfortably past the 476 the brief cited). In change 3's right pane (~76 columns) a line-level pair of that paragraph costs roughly twelve wrapped rows to convey one appended sentence, with two near-identical versions stacked and the reader left to spot the difference. Word-diff renders one reflowed paragraph with deletions and insertions marked inline. Scenario bullet lines are short enough not to suffer the wrap explosion, but they use word-diff too: a mixed scheme is harder to explain than it is worth, and it would force change 3 to implement two renderers.

**`TextDiff::from_words` + `iter_all_changes`, not `iter_inline_changes`.** The brief suggested the latter. Two reasons against it: it is gated behind `similar`'s non-default `inline` feature, and it is structured around line-diff groups — it yields per-line `InlineChange`s, which would have to be stitched back into a single run sequence over the body. `from_words` (available under the default `text` feature, no `Cargo.toml` change) tokenises the whole body directly into whitespace-runs and non-whitespace-runs, whose concatenation is exactly the input string. Walking `iter_all_changes()` while accumulating `value().len()` into two cursors — one per side — yields byte offsets for free, and adjacent tokens sharing a tag are merged into one run. `from_words` also handles multi-line bodies without special-casing, since newlines are just whitespace tokens.

Tokenisation trade-off: without the `unicode` feature, `tokenize_words` splits only on `char::is_whitespace` boundaries, so punctuation stays attached to its word — `list.` → `list,` is reported as one whole-token delete plus insert rather than a one-character edit. Coarser, never wrong (the reconstruction invariant holds either way). `from_unicode_words` would split finer at the cost of a `unicode-segmentation` dependency; not worth it until someone complains.

**Runs address offsets into the strings `spec-model` supplied — which are not the file's bytes, and neither this layer nor change 3 can map them back.** `spec-model`'s bodies are `mdq` re-serialisations: no content lost, no re-wrapping, but bullet markers, emphasis characters and escaping may differ from the source file. That is fine because the guarantee that matters is identical normalisation of both sides, which is precisely what `Unchanged` rests on. The hard constraint for this layer is that it must not normalise *further* — no trimming, no whitespace collapsing, no markdown stripping. It diffs the strings it is given, unmodified, and its offsets are into those strings.

**The diff model owns its text rather than borrowing from the `SpecPair`.** Borrowing would thread a lifetime through every type in the model and force change 3 to keep the `SpecPair` alive alongside the diff. Specs are a few kilobytes; a handful of `String` clones per capability is not a cost worth a lifetime parameter for.

**Shape of the model** (`src/diff/`, mirroring `src/specs/`'s split of `model` / logic / `error`):

```rust
pub enum Run {                                   // offsets into the strings below
    Equal  { base: Range<usize>, delta: Range<usize> },
    Delete { base: Range<usize> },
    Insert { delta: Range<usize> },
}

pub enum Piece {                                 // a requirement's intro, or one scenario's body
    Unchanged   { text: String },
    Changed     { base: String, delta: String, runs: Vec<Run> },
    Added       { delta: String },               // delta-only
    Deleted     { base: String },                // under a removed requirement
    Unmentioned { base: String },                // present in base, delta silent
}

pub struct ScenarioDiff   { pub name: String, pub body: Piece }
pub enum   Operation      { Added, Modified, Removed, Renamed { from: String } }
pub struct RequirementDiff{ pub name: String, pub op: Operation,
                            pub intro: Piece, pub scenarios: Vec<ScenarioDiff> }
pub struct CapabilityDiff { pub capability: String,
                            pub requirements: Vec<RequirementDiff>,
                            pub errors: Vec<DiffError> }

pub fn diff(capability: &str, pair: &SpecPair) -> CapabilityDiff
```

One `Piece` enum covers every position rather than a separate type per operation: an added requirement's intro is `Added`, a removed one's is `Deleted`, and change 3 gets a single match arm set to style. `Operation::Renamed { from }` carries the former name while `RequirementDiff::name` stays the display name, so a rename needs no second name field on the common path.

Note the level at which each state lives, because change 3's brief blurs it: `Operation` is requirement-level and has no unmentioned variant, since a base requirement the delta never names produces no entry at all. `Unmentioned` is piece-level only — an intro, or a scenario inside a modified or renamed requirement. So a `?` gutter marker can never sit on a requirement row; it belongs on scenario rows and on the intro block, which means those rows need markers of their own rather than inheriting their requirement's.

**Algorithm**, in the order that produces the required output ordering:

1. Index the base's requirements by name (`base: None` → empty index). Index `delta.renames` by `to`.
2. Partition `delta.entries` by `op`, preserving document order within each group.
3. `Added` → emit directly, no base lookup: intro and every scenario `Added`.
4. `Modified` → if the name is some rename's `to`, set it aside for step 6; otherwise look up the base by name (miss → error) and compare.
5. `Removed` → look up the base by name (miss → error); intro and every base scenario `Deleted`. Nothing from the entry's own body contributes — it has none.
6. `Renamed` → look up the base by the rename's `from` (miss → error); compare against the MODIFIED entry set aside in step 4, or against an empty requirement when there is none.

**Duplicate names resolve first-wins, without new error surface.** Nothing in `spec-model` rejects a spec with two requirements sharing a name, or a requirement with two scenarios sharing one, so this layer has to be deterministic about it. It does not need to *diagnose* it: `spec-model`'s spec already declares that "requirement names SHALL serve as the identity by which a delta requirement is matched to a base requirement", so a duplicate name is an upstream contract violation, not an ambiguity this layer discovered. Indexing keeps the first occurrence and ignores later ones, which is stable across runs and matches how a reader scanning the document top-down would resolve it.

Alternative considered: report duplicates as a third `DiffError` variant. Rejected as scope this change would be inventing — it is a malformed-input check, which is `spec-model`'s job by its own error requirement, and adding it here would put the diagnosis in the layer least able to point at where in the file the collision is. Worth raising against `spec-model` separately if it ever bites.

Comparing a delta requirement against a base requirement is the same routine for steps 4 and 6: intro by the empty/equal/differ rule; scenarios matched by name, emitted in base order first with delta-only names appended in delta order. Matching by name — never by position — is what makes reordering a restated scenario a no-op rather than a spurious pair of changes.

**Tests build `SpecPair`s from markdown fixtures, not by hand.** `specs::parse` is currently private to the `specs` module, so `src/diff/` cannot reach it. Widening it to `pub(crate)` (a visibility change only — no behaviour, no spec impact) lets the diff tests write delta and base specs as markdown, which is both far more readable than hand-constructing nested model values and a check that the two layers agree on shapes. The `archive/2026-08-08-tui-changelist-horizontal-scrolling` case is reproduced as such a fixture with its expected output asserted exactly: intro `Changed` with one trailing insert run and *no* delete runs, three `Unchanged` scenarios, three `Added`, zero `Unmentioned`. `Unmentioned`, `Removed` and `Renamed` have no real case anywhere in this repo and need synthetic fixtures.

## Risks / Trade-offs

- [Consumers must handle a third state they would not expect from a normal diff] → Resolved by design, and deliberately so: `Unmentioned` is the whole point of this layer, and making it a distinct variant forces change 3 to decide how to show it rather than letting it collapse into an existing colour by accident.
- [Neither a dropped scenario nor an emptied intro can be distinguished from an untouched one] → Knowingly accepted, and surfaced rather than swallowed: both resolve to `Unmentioned`, which says exactly what is known. The limitation is the format's — OpenSpec has no operation for removing a sub-part of a requirement — so closing it needs a format change, not a code change. Neither case is worse than the other; in particular an empty intro is the sync skill's own canonical MODIFIED form, so it is far more likely to mean "untouched" than to be a deletion someone intended.
- [Someone reads `Unmentioned` on an intro as this layer having lost a deletion] → Resolved by design, but only in prose: there is no deletion to lose, because emptying an intro is not a gesture the format defines. Recorded here and in the spec text because the reading is an easy one to arrive at independently.
- [Word-diff offsets and markdown rendering are mutually exclusive inside a diffed region] → Knowingly deferred to change 3. `tui-markdown` strips the `**` from `**WHEN**`, so offsets computed over source text do not map onto rendered spans. This layer emits runs over source text and states the constraint; change 3 chooses (most likely: render source text verbatim inside diffed regions, markdown elsewhere).
- [`from_words` without the `unicode` feature reports punctuation-only edits as whole-token replacements] → Knowingly accepted. Coarser runs, never incorrect ones; the reconstruction invariant is unaffected. `from_unicode_words` behind the `unicode` feature is the escape hatch if it ever reads badly.
- [A delta that renames `A` → `B` *and* carries a MODIFIED entry for `A` would emit the requirement twice] → Knowingly deferred. It is self-contradictory input (modifying a name the same delta says no longer exists), no such case exists, and both entries would still be individually truthful. Not worth a validation pass this layer would otherwise not need.
- [`base: None` reaching this layer at all depends on `spec-model`'s pre-check staying as specified] → Resolved by design: the empty-index treatment means this layer is correct whether or not that pre-check fires, so the two cannot drift into a gap. The pre-check only improves the error message for the cases it does catch.
