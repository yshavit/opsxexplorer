## Why

`parse_spec` (the base "spec of record" parser) hard-fails on any `##` section other than `Purpose` or `Requirements`, unlike `parse_delta`, which was made lenient in [opsxexplorer#11](https://github.com/yshavit/opsxexplorer/issues/11). A base spec authored outside this tool's own conventions — for example OpenSpec's own `cli-init` spec, which has a `## Why` section — blanks the entire capability's tab behind a full-pane error, exactly the failure mode #11 fixed for delta specs, just on the other file. ([opsxexplorer#24](https://github.com/yshavit/opsxexplorer/issues/24))

Separately, #11's delta-side "Unknown sections" box only ever showed titles, never content, and framed every unrecognized section as something to report upstream. That framing doesn't fit a base-spec section the user didn't author and can't fix. Fixing #24 by simply mirroring #11 onto `parse_spec` would leave two different unrecognized-section concepts with two different meanings sharing one visual treatment; this change unifies the mechanism and gives each origin its own honest framing instead.

## What Changes

- `parse_spec` no longer fails the whole parse when it meets an unrecognised `##` section: it collects the section's title and rendered body and keeps parsing the rest of the file, mirroring `parse_delta`.
- `parse_delta`'s existing unrecognised-section handling is upgraded to also capture each section's rendered body, not just its title (`Delta.unrecognized_sections` and the new `Spec.unrecognized_sections` share one `UnrecognizedSection { title, body }` shape).
- `CapabilityDiff` carries both lists through separately (delta-sourced, base-sourced), rather than merging them into one list.
- The right pane renders two separate group headings at the bottom of a tab, in this order:
  - Delta-sourced unrecognised sections, under a purple heading (unchanged color from #11) — purple specifically because a section's survival through a future `openspec archive`/sync is unspecified behavior (verified against the sync skill's own instructions), not a guarantee, so it stays a call-out.
  - Base-sourced unrecognised sections, under a plain/unstyled heading with a subtitle noting they're in the base spec but not this change — unstyled because base-spec content the delta doesn't touch is guaranteed preserved by sync, so there's nothing urgent to flag.
- Each unrecognised section (either origin) renders as its own collapsible row, collapsed by default, expanding to show its full rendered body (no further nested collapsing). This replaces the old flat bullet-list-of-titles row.
- The old "Please consider filing an enhancement request..." prompt is removed; a section's content is now visible directly, so the prompt's job (telling the user something is being hidden) no longer applies the same way.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `spec-model`: `parse_spec` tolerates and collects unrecognised `##` sections (title + rendered body) instead of erroring; `Spec` gains the same collected-sections field `Delta` has. `Delta`'s existing field gains body text, not just titles.
- `spec-diff`: `CapabilityDiff` carries a capability's delta-sourced and base-sourced unrecognised sections through as two separate lists.
- `tui-specdiff`: the right pane renders two distinct "unrecognised sections" headings (delta-sourced purple, base-sourced plain), each unrecognised section its own collapsed-by-default row showing full content on expand; the old title-only bullet list and enhancement-request prompt are removed.

## Impact

- `src/specs/parse.rs`, `src/specs/model.rs`, `src/specs/error.rs` (parser, `Spec`/`Delta` models, dropped `UnrecognisedOperationSection` structure error for `parse_spec`)
- `src/diff/model.rs`, `src/diff/mod.rs` (`CapabilityDiff`)
- `src/tui/diff_row.rs`, `src/tui/layout.rs`, `src/tui/mod.rs` (row shapes, styling, box rendering)
- No change to persisted file formats; base and delta spec.md files are read the same as today, just parsed more leniently.
