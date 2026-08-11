## Why

A renamed requirement's title is currently rendered with its own bespoke formatting — dimmed former name, a literal `→` arrow, new name in modification-yellow — regardless of how similar the two names are. Every other diffed piece in the pane (an intro, a scenario body, the capability purpose) instead picks between an inline word-level diff or a stacked before/after block based on how similar the two texts are, and wraps like ordinary text when it's too long for the pane. The rename title never got wired into that shared path, so it reads inconsistently with the rest of the pane and a long rename title has no wrap behavior of its own to fall back on.

## What Changes

- The rename's former and new names are now compared as a single piece using the same changed-vs-wholesale-replacement judgement (the same similarity threshold) already used for every other compared text in the diff model.
- When the two names are similar enough, the row renders `REQ` followed by one inline, word-level diff of the two names — the same interleaved deletion/insertion run styling used for a changed intro or scenario body — replacing the old dimmed-former-name/arrow/yellow-new-name formatting.
- When the two names are too dissimilar, the row renders `REQ` followed by the former name styled as a deletion and the new name styled as an insertion, stacked on their own lines — the same stacked-replacement rendering already used for any other wholesale-replacement piece.
- Both forms wrap to the pane width like any other row content; neither introduces a new collapse/truncation behavior of its own beyond the wrapping every row already gets.
- No change to the row's gutter marker, its group heading, or how a rename is matched/identified — only how the two names are compared and rendered.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `spec-diff`: a rename's former and new names are now compared as a piece (changed-with-runs, or wholesale replacement) using the same judgement as any other compared text, rather than being carried as two bare strings.
- `tui-specdiff`: a renamed requirement's row renders that name comparison the same way any other compared piece is rendered — inline word-level diff, or stacked before/after text — instead of the previous dimmed-name/arrow formatting.

## Impact

- `src/diff/model.rs`: `Operation::Renamed` carries a name comparison (a `Piece`) instead of a bare former-name string.
- `src/diff/mod.rs`: rename construction computes that comparison using the existing changed-vs-replaced judgement.
- `src/tui/layout.rs`: the `Renamed` arm of the requirement row's content spans renders the name comparison through the same rendering used elsewhere for a changed or replaced piece, instead of its own dimmed/arrow spans.
