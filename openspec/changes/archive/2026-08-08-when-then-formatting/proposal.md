## Why

Scenario bodies in the right pane render straight from the spec's markdown source, so a bullet written as `- **WHEN** the user presses ...` shows its literal `**` characters on screen instead of being styled. That's a rendering gap, not a content problem: the WHEN/THEN convention is used across every spec in this repo, so the raw asterisks show up constantly. Replacing them with an actual style (bold) makes the pane read as intended, and doing it through one small styling function keeps the exact look tweakable without touching the diff logic.

## What Changes

- Scenario body bullets that open with `**WHEN**` or `**THEN**` are rendered without the surrounding `**` markers, with the keyword styled bold instead.
- The keyword styling lives in one small function (`when_then_style()`) that the rest of the rendering path calls, so the look can be changed in one place.
- The transform is a display-only post-process over the spans already produced for a scenario body row (`piece_spans` / `changed_spans` output) — it does not touch the underlying `base`/`delta` strings the diff engine's `Run` byte offsets index into, so word-level diff highlighting for changed scenario text is unaffected.
- Only the leading `WHEN` / `THEN` bullet keyword is affected. Other markdown emphasis inside requirement or scenario text (e.g. inline `**bold**` elsewhere in a passage) is untouched — the codebase's existing decision to not render general markdown in diffed content stays in force.

## Capabilities

### Modified Capabilities
- `tui-specdiff`: scenario body rows gain a requirement describing how `WHEN`/`THEN` bullet keywords are styled.

## Impact

- **Code:** `src/tui/layout.rs` (`piece_spans`, `content_spans` for `DiffRow::Body`), where raw scenario-body text currently becomes spans verbatim. No change to `src/diff` (the `Run`/byte-offset model) or to `src/specs/parse.rs` (the stored markdown stays untouched — only its rendering changes).
- **No dependency changes.** This stays within the existing hand-rolled span/wrap pipeline (`wrap_spans`); it does not pull in the previously-rejected `tui-markdown` general markdown renderer.
