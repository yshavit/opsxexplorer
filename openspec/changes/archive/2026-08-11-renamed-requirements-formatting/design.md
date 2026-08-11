## Context

See `proposal.md` - Why. The relevant existing machinery, all in this repo already:

- `compare::changed_or_unchanged(base, delta)` in `src/diff/compare.rs` picks between `Piece::Changed { runs, .. }` (inline-diffable) and `Piece::Replaced { base, delta }` (wholesale replacement) using `INLINE_DIFF_MIN_SIMILARITY` (0.35). This is the single similarity judgement already applied to every other compared text (intro, scenario body, purpose).
- `piece_spans(piece)` in `src/tui/layout.rs` renders any `Piece` uniformly: `Changed` as interleaved word-level runs (`changed_spans`, red delete / green insert), `Replaced` as the base text styled as a deletion, a newline, then the delta text styled as an insertion.
- Ordinary row wrapping (`wrap_spans`, used by `row_lines`) already wraps any row's content to the pane width, including a long requirement name (`tui-specdiff` spec: "a long requirement name" scenario) - no special handling needed.

Today, `Operation::Renamed { from: String }` bypasses all of this: it's rendered with its own dimmed-former-name / `→` / modification-yellow-new-name spans (`layout.rs`, the `Operation::Renamed` arm of `content_spans`), regardless of how similar the two names are, and is never collapsible or given any wrap treatment beyond whatever falls out of that bespoke span list.

## Goals / Non-Goals

**Goals:**
- Route a rename's two names through the same comparison and rendering path every other diffed piece already uses.

**Non-Goals:**
- No new collapse/truncation behavior for the rename row. A prior iteration of this design considered making an overlong inline name-diff collapsible (mirroring how an intro collapses); that was dropped - a long requirement name already just wraps today, and this change keeps that behavior for both the inline and replaced forms rather than introducing a new per-row collapse state that would also have to share the requirement row's existing expand/collapse toggle (used today only to show/hide the intro and scenarios).
- No change to the requirement row's gutter marker (`»`, modification-colored), to group headings, or to rename matching/identification. Only the two names' comparison and its rendering change.

## Decisions

**`Operation::Renamed` carries a `Piece`, not a bare `from: String`.** The former name is only ever used today for rendering (verified: no other code reads `Operation::Renamed.from`). Computing `compare::changed_or_unchanged(from, to)` once in `src/diff/mod.rs` at rename-construction time and storing the resulting `Piece` gives the renderer the same shape (`Changed` vs `Replaced`) every other piece already exposes, so `layout.rs` needs no rename-specific branching beyond picking this field. Alternative considered: keep `from: String` alongside a separately-computed `Piece`, computed at render time - rejected as redundant state that could drift, and there's no other consumer that wants the bare string.

**Rendering reuses `piece_spans` directly.** The `Renamed` arm of `content_spans` becomes `Span::styled("REQ", operation_style(op))` + `" "` + `piece_spans(&title_piece)`, deleting the bespoke dim/arrow/yellow span-building it replaces. This is the same call every other piece-bearing row already makes; no new function is introduced.

**No collapse state for the rename row's title, in either the inline or replaced form.** This was the main open question worked through with the user before proposing:
- The inline (`Changed`) case could in principle become collapsible when the diffed text overflows one line, mirroring how an intro collapses (`push_paragraph_row`). But a requirement row has exactly one expand/collapse toggle today, already spoken for by intro/scenario visibility - giving the title its own collapse state isn't possible without either overloading that one toggle (so expanding the title also reveals the body, and vice versa) or splitting the row in two. The user's call: skip this entirely and let it wrap, exactly like a long requirement name already does today.
- The replaced case could default to a collapsed placeholder (as a replaced intro does, showing "Expand to view diff"). The user's call: always show the full stacked before/after text, never collapse - this is simpler than the intro's placeholder-collapse behavior and avoids the same toggle-sharing problem.

Net effect: the rename row's title never has a collapse state of its own. It is either a single reflowed inline passage or two stacked lines, and both wrap to the pane width like ordinary content.

## Risks / Trade-offs

[A wholesale-replacement rename with two long names now always renders two full wrapped lines, with no way to collapse it out of view] → Accepted per the user's explicit choice; consistent with how a long requirement name already behaves today (no collapse), and avoids inventing a new toggle-sharing mechanism for a row that otherwise has none.
