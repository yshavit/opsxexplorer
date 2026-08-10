## Why

A delta spec.md section whose heading is neither `Purpose` nor one of the four recognised operation sections (`ADDED`/`MODIFIED`/`REMOVED`/`RENAMED Requirements`) currently makes the whole delta fail to parse, which blanks that capability's entire tab behind a full-pane red error — including all the requirements that parsed fine. The user has no way to tell, from the tool, that their spec.md has content the tool doesn't understand. ([opsxexplorer#11](https://github.com/yshavit/opsxexplorer/issues/11))

## What Changes

- `parse_delta` no longer fails the whole parse when it meets an unrecognised `##` section: it collects the section's title and keeps parsing the rest of the file.
- `Delta` carries those collected titles; `CapabilityDiff` carries them through unchanged.
- The right pane renders an "Unknown sections" heading below a tab's requirement groups whenever its capability has any, the same way a purpose or group heading occupies its own place in the layout: purple, followed by a row with an italic prompt to file an enhancement request and one bullet per unrecognised section title, in the order they appear in the file. That row is reachable by the cursor (so it can be scrolled to), but has no expand/collapse behavior of its own.
- The spec-of-record parser (`parse_spec`) is unaffected — it only ever serves as a diff base, is never rendered on its own, and keeps failing on unrecognised sections as it does today.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `spec-model`: `parse_delta` tolerates and collects unrecognised `##` sections instead of erroring on them; `Delta` gains a field carrying the collected titles.
- `spec-diff`: `CapabilityDiff` carries a capability's unrecognised section titles through from its `Delta`, unchanged.
- `tui-specdiff`: the right pane renders an "Unknown sections" heading and a selectable (but non-collapsible) row for a tab whose capability has unrecognised sections.

## Impact

- `src/specs/parse.rs`, `src/specs/model.rs` (parser and `Delta` model)
- `src/diff/model.rs`, `src/diff/mod.rs` (`CapabilityDiff`)
- `src/tui/diff_row.rs`, `src/tui/layout.rs`, `src/tui/mod.rs` (new `DiffRow` variant, styling, box rendering)
- No change to `parse_spec`, the spec-of-record parser, or any persisted file format.
