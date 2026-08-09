## Why

Scenario bodies use `- **AND**` as a continuation bullet alongside `- **WHEN**` / `- **THEN**` — this repo's own `change-model` spec already has one (`openspec/specs/change-model/spec.md`) — but the right pane's bullet-keyword rendering only recognizes `WHEN`/`THEN`, so an `AND` bullet shows its literal `**` markers on screen instead of being styled. Fixing this by simply appending `"AND"` to the keyword list would also trip a latent bug: the rewrite advances past a matched bullet by a hardcoded 10 characters (`"- **WHEN**".len()`, which happens to equal `"- **THEN**".len()`), but `"- **AND**"` is only 9 characters, so an `AND` bullet would have its first following character silently eaten. Both need fixing together.

## What Changes

- Scenario body bullets that open with `- **AND**` are rendered without the surrounding `**`, with `AND` styled the same way `WHEN`/`THEN` already are.
- The bullet rewrite advances by the actual matched pattern length instead of a hardcoded constant, so keywords of different lengths (`AND` at 3 letters vs. `WHEN`/`THEN` at 4) are handled correctly and no longer coupled to that coincidence.
- No other Gherkin-style keywords (e.g. `GIVEN`, `BUT`) are added — a survey of every bold-caps marker actually used across this repo's `openspec/specs` and `openspec/changes` (593 `WHEN`, 588 `THEN`, 2 `AND`, plus unrelated `BREAKING` prose markers) and the OpenSpec skill templates found no other keyword in use.

## Capabilities

### Modified Capabilities
- `tui-specdiff`: the WHEN/THEN bullet-keyword rendering requirement is broadened to also cover `AND`.

## Impact

- **Code:** `src/tui/layout.rs` (`bullet_keyword`, `style_when_then`) — the keyword candidate list and the post-match advance calculation.
- **Tests:** existing WHEN/THEN unit tests in `src/tui/layout.rs` extend to cover `AND`, including a case that would catch the character-eating regression if the advance calculation were wrong.
- No change to `src/diff` (`Run`/byte-offset model) or `src/specs/parse.rs` — scenario body text is still stored and diffed untouched; only its rendering is affected.
