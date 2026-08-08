## Why

opsxexplorer exists to answer one question — *what does this change actually do to the specs?* — and it currently cannot answer it. The left pane lists changes; the right pane is a bordered rectangle, by requirement (`tui`: "Right pane is a placeholder"). The two capabilities that compute the answer are now implemented: `spec-model` loads both sides of a change-and-capability pair and `spec-diff` reduces them to per-requirement operations with word-level runs. Nothing renders their output. This change connects the two and makes the tool do its job.

## What Changes

- **New `tui-specdiff` capability** owning the right pane: for the change selected in the left pane, one tab per capability it touches, and within a tab a collapsible tree of requirement diffs.
- **Capabilities become tabs.** Two archived changes already touch more than one capability (`2026-08-07-tui-initial` → `tui` + `tui-changelist`; `2026-08-08-spec-model` → `change-model` + `spec-model`), so this is load-bearing today. Tabs follow `spec-model`'s alphabetical enumeration; `[` / `]` switch between them; a single-capability change still renders a one-tab bar.
- **Word-level diff, wrapped, not `+`/`-` line pairs.** Changed prose renders as one reflowed paragraph with deletions and insertions styled inline. The right pane wraps rather than scrolling horizontally like the left pane — the longest line in this repo's specs is 782 characters (`openspec/specs/spec-diff/spec.md:77`) against a ~76-column pane.
- **Gutter markers at two levels.** Requirement rows carry `+` / `~` / `-` / `»` for added, modified, removed and renamed; piece rows (the intro block and each scenario) carry their own marker, including `?` for `spec-diff`'s unmentioned state, which can only ever occur below the requirement level.
- **Collapsible tree with vim keys.** Requirement and scenario headers are collapsed by default and are the only selectable rows, reusing the left pane's display-only-row concept. `j`/`k`, `h`/`l`, `Enter`/`Space` operate the tree; a vertical scrollbar mirrors the left pane's horizontal one.
- **Errors and empty states render in the pane, never crash.** `spec-model`'s and `spec-diff`'s error vocabulary is surfaced structurally (no line numbers — `mdq` drops source positions), isolated per capability *and* per requirement, so a bad entry costs its own row and nothing else.
- **BREAKING (spec-level, not user-facing API):** two `tui` requirements are contradicted by this work and must go — `Right pane is a placeholder` is removed outright, and `Left pane holds input focus` is replaced by a two-pane focus model with `Tab` toggling.
- **Markdown rendering is ruled out**, explicitly and permanently for diffed content: `tui-markdown` strips the markup that `spec-diff`'s byte offsets address, so styled markdown and word-level highlighting cannot coexist. Bodies render as `spec-model` normalised them, uniformly.

## Capabilities

### New Capabilities
- `tui-specdiff`: the right pane — capability tabs, the collapsible requirement-diff tree, word-diff and gutter styling, wrapping, vertical scrolling, and the pane's error and empty states.

### Modified Capabilities
- `tui`: `Right pane is a placeholder` is REMOVED (the right pane now displays diffs); `Left pane holds input focus` is MODIFIED into a focus model where `Tab` moves focus between the two panes and the focused pane is visually indicated.

## Impact

- **Code:** `src/tui/mod.rs` (`render_right_pane`, currently a bare `Block::bordered()`), `src/tui/app.rs` (`App` gains right-pane state: focus, selected tab, collapse set, vertical offset), and new modules for the right pane's row model, wrapping and styling. `src/tui/row.rs` is untouched — the right pane gets its own row type rather than widening the left pane's.
- **Consumed APIs, all already implemented:** `Changes::views`, `specs::capabilities`, `specs::load`, `diff::diff`, and the `CapabilityDiff` / `RequirementDiff` / `Piece` / `Run` model. No changes to any of them.
- **Dependencies:** none added. `tui-markdown` (a declared but unused dependency) stays unused and should be dropped from `Cargo.toml` by this change rather than left as a standing suggestion that the pane might one day render markdown.
- **Wrapping is the real cost.** `ratatui::List` scrolls by item, not by line, so a requirement that wraps to seven lines makes vertical scrolling lurch. Whether the pane stays a `List` or becomes a line-addressed `Paragraph` is a genuine fork and is argued in design.md; either way this change owns a `wrap_spans`-style primitive the codebase does not have.
- **No caching.** Diffs are recomputed on selection change, consistent with the existing decision to recompute `max_scroll` every render. At this repo's scale (six changes, ten spec files) that is free.
- **Self-testing:** this change's own delta contains ADDED, MODIFIED and REMOVED requirements across two capabilities — the first in the repo's history to exercise more than ADDED and MODIFIED. It is its own end-to-end fixture. REMOVED, RENAMED and unmentioned cases have no other instance in the repo and need synthetic fixtures.
