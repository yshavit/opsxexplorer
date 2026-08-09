## Why

A capability's `## Purpose` section is parsed and carried by both `Spec` and `Delta` (`src/specs/model.rs`), but `spec-diff` drops it and the right pane never renders it — a gap the `tui-specdiff` change explicitly deferred ("Rendering a delta's `## Purpose`... Tracked separately."). A reader currently has no way to see that a change adds or rewrites a capability's purpose without opening the spec file by hand.

## What Changes

- `spec-diff` compares a capability's purpose (delta vs. spec of record) using the same rules already applied to a requirement's intro: absent from the delta means nothing to report; present and there is no prior purpose means an insertion; present and equal to the prior purpose means nothing to report (a byte-identical restatement is not a change); present and different means a comparison, using the existing changed-vs-replaced legibility judgement.
- `tui-specdiff` renders that comparison, when there is one, above a tab's requirement groups (below any error notices):
  - A heading box reading "Added Purpose" or "Modified Purpose", styled like the existing Added/Modified Requirements group headings, degrading to a plain line under the same narrow-pane rule.
  - Beneath it, one row marked with the pillcrow (`¶`). If the comparison is an insertion or an ordinary edit (interleavable) and its text (right-trimmed of whitespace) fits within the row's available width, it renders in full, in one line, with no collapse affordance — there is nothing to hide. Otherwise the row is collapsible, with a disclosure triangle, collapsed by default. A comparison too dissimilar to interleave (a wholesale replacement) is always collapsible, regardless of length, since a short excerpt of just the new text would misrepresent a rewrite as an ordinary edit. Collapsed, an insertion or edit renders as a single line — a literal character slice of the current text sized to the available width (word boundaries ignored) followed by an ellipsis; a wholesale replacement instead renders a fixed italic placeholder, "Expand to view diff", in place of any excerpt. Expanded, either kind renders the full text through the pane's normal wrap-and-diff-styling path — interleaved word-level runs, or stacked before/after text for a replacement.
  - The row is always selectable, like a requirement or scenario row (useful for consistent up/down scrolling), but the toggle keys have no effect on it when it fits without truncation — there's nothing for them to do.
- No change to `spec-model`'s parsing of `## Purpose` — it already works.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `spec-diff`: adds purpose comparison to a capability's diff output, alongside the existing per-requirement comparison.
- `tui-specdiff`: adds rendering of the purpose comparison, including a new collapsible row kind and its placement relative to existing content.

## Impact

- `src/diff/model.rs`, `src/diff/mod.rs`, `src/diff/compare.rs`: `CapabilityDiff` gains a purpose field; `diff()` computes it from `SpecPair`.
- `src/tui/diff_row.rs`: new row kind(s) for the purpose heading and its collapsible content row; `RowKey` needs a variant that isn't tied to a requirement name.
- `src/tui/layout.rs`: new truncation primitive (character-slice-plus-ellipsis) distinct from the existing word-wrapping `wrap_spans`; heading box rendering reused for the purpose heading.
- `src/tui/mod.rs` / `src/tui/app.rs`: cursor movement, selectability and expand/collapse state need to cover the new row.
