## Context

Scenario body rows are rendered by `content_spans` / `piece_spans` in `src/tui/layout.rs`, which turn a `Piece` (`Unchanged`, `Added`, `Deleted`, `Unmentioned`, or `Changed`) into `Vec<Span<'static>>`. For `Changed` pieces, `changed_spans` walks `Run`s whose `Range<usize>` byte offsets index directly into the stored `base`/`delta` strings (`src/diff/*`) to produce insertion/deletion styling. `row_lines` then passes whatever spans come out of `content_spans` into `wrap_spans` (`src/tui/wrap.rs`), which wraps by width while preserving each character's style and treats `\n` as a hard break — already relied on for scenario bodies' `- **WHEN** ...\n- **THEN** ...` bullets.

The prior `tui-specdiff` design ruled out general markdown rendering for diffed content: a markdown renderer would consume the `**` markup that `Run` offsets are computed against, so styled markdown and word-level diff highlighting can't coexist (see `openspec/changes/archive/2026-08-08-tui-specdiff/proposal.md`). That constraint still holds — this change must not parse markdown generally.

## Goals / Non-Goals

**Goals:**
- Render `**WHEN**` / `**THEN**` at the start of a scenario-body bullet as a styled keyword with no literal `**`.
- Keep the style itself defined in exactly one small function so it's a one-line change to try alternatives.
- Leave `Run` byte-offset computation (`src/diff`) and the stored `base`/`delta` markdown strings untouched.

**Non-Goals:**
- No general markdown rendering (no new markdown-parsing dependency, no styling of arbitrary `**bold**` elsewhere in requirement/scenario text).
- No change to how scenarios are authored or stored in `spec.md` files — the `**WHEN**`/`**THEN**` markdown stays the source-of-truth format; only its terminal rendering changes.

## Decisions

**Transform the rendered `Span` list, not the underlying strings.** The `WHEN`/`THEN` rewrite runs as a post-process over the `Vec<Span<'static>>` that `piece_spans` (and, for changed pieces, `changed_spans`) already produce for a `DiffRow::Body` row — after diff-run styling has been applied, before `wrap_spans` wraps the row. Because `Run` ranges are computed and consumed entirely within `changed_spans` against the original `base`/`delta` strings, a transform applied to its *output* spans can drop the `**` characters and add the keyword's style without touching a single byte offset. Alternative considered: strip `**` from `base`/`delta` before diffing — rejected, since that would shift every downstream `Run` range and re-introduce exactly the coupling the prior design avoided.

**One style function, one rewrite function.** `fn when_then_style() -> Style` returns the style (`Style::new().add_modifier(Modifier::BOLD)` — italic was tried first and read worse in the terminal, so this is the one line to change if that verdict ever flips, matching the existing `added_style`/`removed_style`/`modified_style` pattern in `layout.rs`) and is the single place to change the look. A second function does the mechanical work: scan the row's spans for a bullet opening `**WHEN**` or `**THEN**`, and rewrite that run of characters into three spans (or fewer, if a boundary is empty) — a stripped `WHEN`/`THEN` span carrying `when_then_style()`, with any text before/after in the matched span keeping its original style. Only a bullet's leading keyword is matched (start-of-line `- **WHEN**` / `- **THEN**`, consistent with every occurrence in this repo's specs), so incidental `**bold**` elsewhere is left alone.

**Scope the rewrite to `DiffRow::Body`.** Scenario body text is the only place this bullet convention appears in rendered rows (requirement names, intros, and headings don't contain WHEN/THEN bullets), so the call site is `content_spans`'s `DiffRow::Body` arm, applied after `piece_spans(piece)` returns.

## Risks / Trade-offs

- **Matching on literal text is a bit brittle** if a spec ever writes the bullet differently (extra spaces, a different case). → Scope is intentionally narrow (this repo's one existing convention); a near-miss just falls back to rendering the text unchanged, same as today, not a crash.
- **Multi-span bullets**: after `changed_spans` word-diff splitting, a bullet's `**WHEN**` text could in principle be split across more than one `Span` by a `Run` boundary. → The rewrite operates on the flattened character+style sequence for the row (the same representation `wrap_spans` already builds), so a keyword split across spans is still detected and restyled correctly; each output character keeps whichever original style (diff color) it had except where the keyword rewrite applies.
