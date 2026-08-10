## Context

See proposal.md - Why. Two things worth restating from the exploration that shaped this design, because they aren't obvious from the specs alone:

- Today, `parse_delta`'s title `match` (`src/specs/parse.rs:47-80`) has one arm per recognised section and `other => return Err(...)` as the fallback. That `return` is a hard stop mid-loop: nothing else in the file gets parsed, and the caller sees only `SpecError::Structure { kind: UnrecognisedOperationSection, .. }`, which `App::pane_view()` turns into a full-pane red error banner (`src/tui/mod.rs:157-183`), replacing the tab's content entirely rather than adding to it.
- A structurally identical soft-failure path already exists for a different error class: `DiffError::MissingBaseRequirement` is collected into `CapabilityDiff.errors` and rendered as inline `Notice` rows at the top of an otherwise-normal tree (`src/tui/diff_row.rs:125-127`, `flatten()`). This change follows that precedent rather than inventing a new one, just at the bottom of the tab instead of the top.

## Goals / Non-Goals

**Goals:**
- Stop one unrecognised `##` section from discarding an entire capability's diff.
- Surface enough information (the section title) for the user to know something wasn't shown, without attempting to render markdown generically.

**Non-Goals:**
- Rendering the unrecognised section's *content* in any form. Only its heading text is captured.
- Changing `parse_spec` (the spec-of-record parser). It keeps failing on unrecognised sections; per the proposal, that path is never rendered on its own, so there's nothing to degrade gracefully into.
- A generic `Section` AST or pluggable section-handler mechanism. The parser stays a flat title `match`; the only change is what the fallback arm does.
- Precise wrap/truncate behavior for the "Please consider filing..." line. It reuses `wrap_spans` (`src/tui/wrap.rs`) exactly as-is, including that function's existing behavior of hard-breaking an over-wide unbreakable word (which is what happens to the bare URL if it can't fit alone on a line). No new wrap or truncate mode is introduced for this one line.

## Decisions

**Collect into a `Vec<String>` on `Delta`, not a richer type.** The fallback arm in `parse_delta`'s loop changes from `other => return Err(...)` to pushing `other.to_string()` into a new `unrecognized_sections: Vec<String>` and `continue`-ing the loop. Because the loop already just falls through to the next section on every recognised arm, this is the whole change at the parser level — order is preserved for free. No position/line info is captured (none is available; see the existing "Errors are located structurally, never by line number" requirement in tui-specdiff, which this design doesn't disturb).

**`CapabilityDiff` gets the same field name and shape**, populated verbatim from `pair.delta.unrecognized_sections` in `diff()` (`src/diff/mod.rs:135`). `diff()` doesn't interpret the titles at all — no dedup, no sorting, no validation. If the same malformed title appears twice in a file (unlikely, but not excluded), it's listed twice; that's a truthful reflection of the source, not a bug to guard against.

**One heading + one row per tab, rendered at the bottom, like any other section in the layout.** `flatten()` (`src/tui/diff_row.rs`) appends two new rows after the existing `for req in &diff.requirements` loop, only when `!diff.unrecognized_sections.is_empty()`: a heading row (styled/boxed like a group heading) and a content row carrying the prompt and all the bullets. This matches the mockup in the issue (one box, bulleted) and the user's explicit call for placement at the bottom rather than the top — this is optional context, not an urgent warning like a parse failure elsewhere in the same tab. It is not a lesser or separate "footnote" concept: it's rendered the same way the pane already renders its other sections (group headings, the purpose heading), just positioned after the requirement groups instead of before them.

**New color: `Color::Rgb(147, 51, 234)`** (a vibrant, deep purple — the same value as Tailwind CSS's `purple-600`), defined as a new `unrecognised_style()` function in `src/tui/layout.rs` alongside `added_style()`/`modified_style()`/`removed_marker_style()`. This is the first hand-picked RGB color in the codebase (everything else uses `ratatui::style::Color`'s named variants); the user explicitly preferred this over `Color::Magenta` and considered the RGB cost trivial. The italic "Please consider filing..." line reuses the same style with `Modifier::ITALIC` added; the bullets use the pane's ordinary (unstyled) text color, per the issue.

**Reuse `heading_box` as a third thin wrapper**, the same way `group_heading_box` and `purpose_heading_box` already do (`src/tui/mod.rs:442-456`), so the narrow-pane degrade-to-plain-line behavior comes for free and stays visually consistent with the other two headings.

**The heading is skipped, like every other heading; the content row is selectable but never collapsible.** The right pane's scrolling is cursor-driven — it "scrolls vertically to keep the cursor visible as it moves" — so a row with no `RowKey` can never be scrolled into view if it falls outside the initially-visible area; a tall requirement tree would make the unrecognised-sections content permanently unreachable. The content row therefore needs a `RowKey`, following the precedent already set by the purpose row and a requirement's intro row (`src/tui/diff_row.rs`, `layout.rs`'s "Only collapsible rows are selectable" handling): both are selectable even when they have nothing to collapse, and `Enter`/`Space`/`l`/`h` on them are no-ops. The unrecognised-sections content row follows the same pattern — carries a key, has no expanded/collapsed state (or reuses a row shape that has one but for which toggling is a no-op, whichever the existing `Paragraph`-row machinery makes cheaper). The heading itself carries no `RowKey` and is skipped exactly like a group heading or the purpose heading.

## Risks / Trade-offs

- **Silent behavior change for anyone relying on the old hard failure.** Before this change, an unrecognised delta section was loud (the whole tab errored). After, it's a purple heading at the bottom of the tab, easy to miss if you're not scrolling that far in a long capability. Making the row cursor-reachable mitigates this somewhat (`j`/`Ctrl+d` will eventually land on it), but there's no other affordance (e.g. a summary count in the tab bar) proposed here to make it more visible. This is the intended trade-off per the issue (loud failure was itself the bug being fixed); out of scope unless it proves to be a problem in practice.
- **`Color::Rgb` may not render identically across all terminals/themes** the way a named `Color` does (some terminals remap named colors to a theme palette but pass RGB through literally, which is usually what's wanted here but is a behavior change for this codebase's color handling). Accepted per the user's explicit preference.
