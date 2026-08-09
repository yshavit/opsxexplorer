# Change: diff-legibility

## Why

Word-level runs are computed straight over the whole piece with no notion of
whether the result is worth reading. Two distinct failure modes follow, both
observed on this repo's own archived changes (issue #5):

1. **A lone space anchors a diff.** `similar`'s word tokenizer emits runs of
   whitespace as tokens in their own right, so the diff engine can match a
   single space between two otherwise-unrelated edits and split one edit into
   two. In `tui-keybinding-improvements`, ` via Ctrl+Q` → `` by pressing `q` ``
   reports as `` [-via-]{+by+} [-Ctrl+Q-]{+pressing `q`+}`` instead of one
   deletion and one insertion. Git's `--word-diff` does not have this problem
   because it treats whitespace as a separator, not a token.

2. **A substantial rewrite degenerates into confetti.** When a piece is
   rewritten rather than edited, filler words match by coincidence across
   unrelated sentences. In `tui-specdiff`, the renamed *Focus moves between the
   two panes* intro reports as 55 runs across 20 change regions, only ~24%
   similar. Reading the two paragraphs one after the other is strictly easier
   than reading the interleaving.

These are independent causes but one user-facing symptom, and the first
materially shrinks the second: coalescing whitespace anchors alone takes the
number of pieces in the archive that read badly from three down to one.

## What Changes

- **Whitespace stops anchoring a diff.** An `Equal` run that is entirely
  whitespace and has a change on both sides is no longer treated as an anchor.
  Each resulting change region collapses to exactly one `Delete` followed by one
  `Insert`. No threshold; word granularity and the reconstruction invariant are
  both preserved.

- **A piece too dissimilar to diff inline is reported as a wholesale
  replacement.** `spec-diff` gains a `Piece::Replaced { base, delta }` variant,
  produced instead of `Piece::Changed` when the two texts' similarity falls
  below a calibrated threshold. Runs are not reported for such a piece.

- **The right pane renders a replacement as stacked before-and-after text** —
  the spec of record's text styled as a deletion, then the delta's text styled
  as an insertion, on separate lines.

- **BREAKING** (internal only): `Piece` gains a variant. Every consumer matches
  `Piece` exhaustively, so this is compiler-enforced; there is no external API.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- **`spec-diff`** — the requirement *"Changed content is reported as word-level
  runs over the compared texts"* currently opens "For every piece reported as
  changed", which a wholesale replacement contradicts. It needs qualifying, plus
  a scenario pinning the whitespace-anchor behaviour. A new requirement
  specifies when a piece is reported as a replacement instead.

- **`tui-specdiff`** — the requirement *"Changed content is shown as one inline
  word-level diff, not as before-and-after blocks"* forbids exactly the
  rendering a replacement needs. It needs qualifying to the inline case, plus a
  requirement for how a replacement is displayed.

## Impact

- `src/diff/runs.rs` — coalescing pass over the computed runs. No signature
  change; no consumer change.
- `src/diff/model.rs` — the `Piece::Replaced` variant.
- `src/diff/compare.rs` — the legibility decision, choosing between `Changed`
  and `Replaced`.
- `src/tui/layout.rs` — two match arms (`piece_marker`, `piece_spans`).
- `src/tui/wrap.rs` — none. It already treats `\n` as a forced break, which is
  what stacks the two halves of a replacement.
- No dependency changes; `similar` stays at 3.1.2 and the algorithm stays Myers.

## Non-goals

- **Changing the tokenizer.** Re-tokenizing as word-plus-trailing-whitespace
  (git's model) fixes the same cases but regresses a pure append —
  `"alpha beta gamma"` → `"alpha beta gamma delta"` becomes
  `alpha beta [-gamma-]{+gamma delta+}` — because `gamma` and `gamma ` stop
  being the same token. The runs are post-processed instead.

- **A line-anchored two-pass diff** (git's model of line diff first, word
  refinement within a hunk). It would make the diff more robust to a scenario
  body gaining or losing a bullet, but on the whole archive it changes no
  output today. Left as a separate follow-up.

- **Deciding per line rather than per piece.** Measured on the archive, per-line
  similarity is systematically lower — a changed bullet loses the credit its
  unchanged sibling lends it — and the gap between the piece that reads badly
  and the nearest piece that reads fine narrows from 0.245 to 0.142.
  Requirement intros, where every low-scoring piece in the archive is, are
  single lines anyway.
