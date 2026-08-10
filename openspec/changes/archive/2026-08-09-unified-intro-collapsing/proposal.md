## Why

`render-purpose` (archived at `openspec/changes/archive/2026-08-09-render-purpose/`) built width-aware collapse/expand machinery for a capability's `## Purpose` row — a fits-check, literal-character truncation with an ellipsis, an "Expand to view diff" placeholder for a wholesale replacement, and a disclosure-triangle expand state — but left a requirement's intro rendered the old way: always expanded, never selectable, with no collapse state of its own. A long intro on a modified requirement runs on for as many wrapped lines as it needs, with no way to collapse it, even though it's the exact same shape of content the Purpose row already handles well.

## What Changes

- Generalize the Purpose row's `DiffRow` variants, `RowKey`, and `layout.rs` helpers into a shared "paragraph" row family used by both the capability-level purpose row and a requirement's intro row, rather than reimplementing the same collapse/truncate/placeholder behavior a second time.
- Extend the underlying text-extraction to cover every `Piece` variant an intro can carry (`Unchanged`, `Deleted`, `Unmentioned`), not just the `Added`/`Changed`/`Replaced` set purpose is limited to today.
- Thread the row's indent level through the width budget so the collapse fits-check and truncation stay correct at indent 1 (a requirement's intro), not just indent 0 (purpose).
- Make a requirement's intro row selectable, following the same "selectable even when not collapsible" exception the purpose row already has, and update the "Only collapsible rows are selectable" requirement's scenario coverage to match.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `tui-specdiff`: a requirement's intro row gains the same collapse/expand/truncate/placeholder behavior the purpose row has, including its own collapse state, selectability, and width-aware fits-check at its own nesting depth.

## Impact

- `src/tui/diff_row.rs`: `RowKey` gains an `Intro` variant; `DiffRow::PurposeFull`/`Purpose`/`Intro` are replaced by a shared `ParagraphFull`/`Paragraph` row family carrying an explicit indent; `push_purpose` and `push_requirement`'s intro handling both go through one shared push function.
- `src/tui/layout.rs`: `purpose_available`/`collapsed_purpose_lines` generalize to take an indent parameter and cover every `Piece` variant's own "current text" field, not just `Added`/`Changed`.
- No changes expected to `src/tui/app.rs`, `src/tui/mod.rs`, or `src/diff/*` — the intro's underlying `Piece` comparison is already computed the same way purpose's is, and `App`'s cursor/toggle plumbing already operates generically over `DiffRow::key()`/`expanded()`/`is_selectable()`.
