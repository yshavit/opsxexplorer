## Context

See proposal.md — Why. Two facts about the current code shape the approach:

- `runs()` in `src/diff/runs.rs` calls `TextDiff::from_words` and walks
  `iter_all_changes()`, merging adjacent same-tag changes. `similar-3.1.2`'s
  `tokenize_words` (`src/text/abstraction.rs:213`) emits maximal runs of
  whitespace as tokens in their own right, alongside word tokens. Whitespace
  tokens are therefore eligible to be matched as `Equal`, which is what splits
  one edit into two.
- `Piece` is matched exhaustively in five places (`compare.rs`, `diff/mod.rs`,
  `tui/layout.rs`, `tui/diff_row.rs`, `tui/app.rs`), so adding a variant is
  compiler-enforced rather than a silent hazard. Only `layout.rs` renders
  content; the rest match on specific variants.

Measurements quoted below come from every `Changed` piece in the repo's own
archive — twelve pairs, reconstructed by diffing each archived change's delta
spec against the spec of record at the commit before it was archived.

## Goals / Non-Goals

**Goals:**

- Whitespace never holds two edits apart.
- A piece that would read better as two passages is reported as two passages.
- The threshold is a single named constant with recorded calibration, so
  revisiting it is a one-line change with a table to argue against.

**Non-Goals:**

- Making the threshold configurable. One constant, chosen once. A user-facing
  knob for a legibility heuristic is more surface than the problem warrants.
- Changing which diff algorithm `similar` runs. Myers stays. Patience was
  considered and rejected (see Decisions).
- Any change to `wrap.rs`.

## Decisions

### Post-process the runs rather than re-tokenize

Git's word-diff avoids the whitespace-anchor problem by treating whitespace as
a separator rather than a token, and `similar` allows a custom tokenization via
`TextDiff::from_slices`. Attaching trailing whitespace to each word reproduces
git's behaviour and fixes the same cases.

Rejected, because it regresses a pure append. With word-plus-trailing-space
tokens, `"alpha beta gamma"` → `"alpha beta gamma delta"` diffs as
`alpha beta [-gamma-]{+gamma delta+}`, since `gamma` and `gamma ` are no longer
the same token. That is a visible regression and it breaks the existing
`trailing_append_yields_one_insert_and_no_delete` test, which is protecting
real behaviour.

Post-processing keeps the tokenization — and therefore every case that already
works — untouched, and only removes anchors that should not have been anchors.

### Coalesce whole change regions, not individual equal runs

The natural implementation is: find a whitespace-only `Equal` run with a change
on each side, split it into a `Delete` of its base span plus an `Insert` of its
delta span, then merge adjacent same-tag runs. **This does not work.** The
result alternates:

```
Delete("via") Insert("by") Delete(" ") Insert(" ") Delete("Ctrl+Q") Insert("pressing `q`")
```

Nothing is adjacent to a same-tag neighbour, so nothing merges, and the output
is worse than before. The pass has to work at region granularity: identify the
maximal spans of runs that contain no surviving anchor, then emit all of that
span's deleted text as one `Delete` followed by all of its inserted text as one
`Insert`.

This preserves reconstruction by construction — base order among deletes and
delta order among inserts are both unchanged — and it can only ever destroy
`Equal` runs, never fabricate one.

### A whitespace-only equal run containing a line break stays an anchor

Line structure is meaningful in this content: a scenario body is
`- **WHEN** …\n- **THEN** …`, and `wrap.rs` deliberately treats `\n` as a forced
break for exactly that reason. Dissolving a newline anchor would let a single
deletion span two bullets. Only whitespace runs with no line break are eligible.

### The legibility decision lives in `compare.rs`, not `runs.rs`

`runs.rs` computes runs; `compare.rs` classifies pieces. `changed_or_unchanged`
already decides between `Unchanged` and `Changed`, so deciding between `Changed`
and `Replaced` belongs in the same place. `runs()` keeps its signature and stays
unaware of the threshold, which is also what keeps the coalescing pass
reviewable on its own.

The measure is derived from the runs `runs()` already returned, so there is no
second diff pass.

### The measure, and the threshold

Similarity is `2 × equal_bytes ÷ (base_bytes + delta_bytes)`, computed over the
runs **after** coalescing. This mirrors `similar::TextDiff::ratio()` but is
measured on the runs actually reported, so the two can never disagree.

Measured across the whole archive, sorted:

| similarity | piece |
|---|---|
| **0.238** | `tui-specdiff` / *Focus moves between the two panes* intro |
| 0.483 | `changelist-archived-ordering` / intro |
| 0.503 | `tui-specdiff` / *user presses any key* |
| 0.585 | `tui-specdiff` / *application launches* |
| 0.628 | `tui-keybinding-improvements` / *Single cursor navigable* intro |
| 0.670 | `changelist-archived-ordering` / *alphabetical order* |
| 0.765 – 0.909 | the remaining six |

**Threshold: 0.35.** The 0.238 → 0.483 step is the largest gap anywhere in the
distribution — 0.245, where the next largest is 0.095 — so the threshold sits in
the widest empty band the real data offers, 0.112 clear of the piece it fires on
and 0.133 clear of the nearest piece it must not fire on.

Alternatives considered:

- *Change-region density* (borders per 100 words), the first hypothesis. It
  does not discriminate: after coalescing, the intro that reads badly scores
  8.6 and *user presses any key*, which reads fine, scores 8.3.
- *Per-line rather than per-piece.* Per-line similarity is systematically lower,
  because a changed bullet loses the credit its unchanged sibling lends it.
  Scored per line, the same worst piece sits at 0.238 and the nearest piece that
  must not fire moves down to 0.380 — a 0.142 band instead of 0.245, so the
  threshold gets 42% less room for no benefit. Every low-scoring piece in the
  archive is a requirement intro, which is a single line regardless.
- *Patience diff.* `similar` offers it, and it sounds right — unique-token
  anchoring means `the` and `SHALL` can never anchor. But `similar`'s
  implementation recursively falls back to Myers inside the gaps between unique
  anchors, so the confetti returns; swapping the algorithm changes anchor
  selection and nothing else. A hand-rolled variant with wholesale replacement
  in the gaps does do better (3 change regions instead of 5 on the worst piece),
  but see Risks.

### `Piece::Replaced` over the alternatives

- *Keep `Piece::Changed` with `runs = [Delete(all base), Insert(all delta)]`.*
  No model change and no consumer change, but the pane would flow the deleted
  text straight into the inserted text with no break — `…to the right
  pane.Exactly one pane SHALL…` — because nothing in the run vocabulary says
  "these are two passages".
- *Let the renderer break between a large `Delete` and a large `Insert`.*
  Cheapest, but it puts a diff-quality judgement in the layout layer and a
  second threshold in a second place.
- *`Piece::Replaced { base, delta }`.* Chosen. The judgement is made once, where
  the other piece classifications are made, and the renderer stays a renderer.

### Rendering needs no change to `wrap.rs`

`piece_spans` emits the deleted text, a raw `"\n"`, then the inserted text.
`wrap_spans` already treats `\n` as a forced break that width-based wrapping
cannot swallow, so the stacked layout falls out of existing behaviour.

`piece_marker` returns the existing modified marker for `Replaced`. A
replacement is a modification; the stacked layout is what distinguishes it. This
also means the existing `tui-specdiff` requirement *"A requirement's intro and
each of its scenarios carry a marker for their own state"* needs no amendment.

### An empty side is never a replacement

`intro_piece` guards an empty *delta* (reporting `Unmentioned`) but not an empty
*base*, so `Changed { base: "", delta: "…" }` is reachable — a base requirement
with no intro paired against a delta that adds one. Its similarity is 0, so
without a guard it would render as an empty deletion stacked above the
insertion. No spec of record in the repo currently has an intro-less
requirement, so this is latent rather than live, but the rule is cheap and the
alternative is an empty red line.

## Risks / Trade-offs

**The threshold is calibrated on twelve pairs from one repository.** → It sits in
the largest gap in that data with margins wider than any other gap in the
distribution, and it is one named constant. The twelve pairs become fixtures, so
a future recalibration has a regression corpus rather than an argument. Prose in
other repos may distribute differently; the failure mode is a piece rendering
stacked when it could have read inline, which is a degradation in polish, not in
correctness.

**Coalescing loses genuine anchors in principle.** Dissolving an equal run
always costs information. → Only whitespace-only, line-break-free runs with a
change on both sides are eligible, which is the case where the "anchor" carries
no information at all. Verified against the archive: no piece that reads well
today changes shape, and six of twelve improve.

**A smarter anchor strategy could silently mis-align runs, and the existing
tests would not catch it.** Base reconstruction reads `Equal.base` + `Delete.base`;
delta reconstruction reads `Equal.delta` + `Insert.delta`. Neither ever compares
the two halves of an `Equal` run, so a run claiming `base: "included). Each "`
and `delta: "ascending. Each "` passes both invariants. This was hit for real
while prototyping the patience variant. → The coalescing pass is structurally
immune, since it only removes `Equal` runs. The spec now requires equal runs to
address identical text on both sides, and the tests assert it, so the gap is
closed before anyone tries a smarter strategy.

**The similarity measure and the reported runs could drift apart** if one is
computed from `TextDiff::ratio()` and the other from the coalesced runs. →
Derive the measure from the coalesced runs. One source of truth.

## Migration Plan

None. No persisted state, no external API, no data format. `Piece` gains a
variant; every consumer is in-tree and compiler-checked.

Rollback is reverting the change; the two spec amendments revert with it.
