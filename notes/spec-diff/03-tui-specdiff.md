# Brief: `tui-specdiff` capability — render the spec diff in the right pane

> Chain position: **3 of 3**. Read → compare → render.
> 1. `spec-model` (`01-spec-model.md`) — parse a spec.md; load both sides.
> 2. `spec-diff` (`02-spec-diff.md`) — compute the requirement-level delta.
> 3. **`tui-specdiff`** (this file) — the right pane.
>
> Depends on changes 1 and 2 being implemented. This brief is self-contained. It
> is the output of an exploration session; the decisions in it were made
> deliberately and should be carried into the proposal.

## Project context

opsxexplorer is a terminal UI (Rust 2024, ratatui 0.30, crossterm 0.29) for
browsing OpenSpec changes and specs in a local git repo. This change delivers
the thing the whole tool exists for: **select a change in the left pane, see its
spec diff in the right pane.**

Existing TUI:

- `src/tui/mod.rs:44` — `render()` splits the frame 35% / 65% into left and
  right.
- `src/tui/mod.rs:52` — `render_left_pane`: a ratatui `List` of `Row`s, with a
  single horizontal scroll offset applied by skipping characters across each
  row's styled spans, plus a horizontal `Scrollbar`.
- `src/tui/mod.rs:86` — `render_right_pane`: currently `Block::bordered()` and
  nothing else.
- `src/tui/app.rs` — `App` holds `changes`, `archived_expanded`, `ListState`,
  `h_scroll`, `max_h_scroll`. `handle_key` (`app.rs:51`) binds `Up`/`k`,
  `Down`/`j`, `Enter`/`Space`, `Left`/`h`, `Right`/`l`, `Home`/`^`, `End`/`$`.
  `Ctrl+Q` quits, handled in the event loop.
- `src/tui/row.rs` — `Row` enum + `flatten()` + `is_selectable()`, which already
  encodes the "some rows are display-only and skipped by cursor navigation"
  idea. **Reuse that concept**: in the right pane only requirement and scenario
  *headers* are selectable, since only they collapse.

Change 2 supplies, per change + capability: requirement entries ordered ADDED →
MODIFIED → REMOVED, each with an intro block and scenarios, where changed blocks
carry word-level `Equal`/`Delete`/`Insert` runs over faithful source text.

## Decisions already made

These came out of an exploration session and are settled. Carry them in.

### Capabilities become tabs

A change can touch several capabilities — `archive/2026-08-07-tui-initial`
touches both `tui` and `tui-changelist`, so this is real today, not
hypothetical. **One tab per capability, within the right pane.** Tabs are the
capabilities the selected change touches, in stable (alphabetical) order.

Open: whether a single-capability change still shows a one-tab bar (recommend
yes, for layout stability — consistent with the existing scrollbar decision to
render in its "nothing to scroll" state rather than disappear). Also open: which
keys switch tabs. `Tab` is spoken for (see focus, below); `[`/`]` or `H`/`L` are
the candidates.

### Word-diff, wrapped — not `+`/`-` line pairs, not horizontal scroll

The longest line in this repo's specs is **476 characters**
(`openspec/specs/tui-changelist/spec.md:115`). At 65% of a 120-column terminal
the right pane's inner width is ~76. Line-level `+`/`-` on such a paragraph
costs twelve wrapped rows to convey one appended sentence, with two
near-identical blocks stacked. **Word-diff (`git diff --word-diff` style):** one
reflowed paragraph, deletions red, insertions green, no duplication.

And the pane **wraps**; it does not scroll horizontally like the left pane.
Spec prose is unreadable otherwise.

Wrapping is the hard part of this change:

- Needs something like `wrap_spans(Vec<Span>, width) -> Vec<Vec<Span>>` that
  preserves per-span styling across the break — because a word-diff run can
  straddle a wrap point.
- The gutter marker (`+` / `~` / `-` / `?`) must repeat or blank on continuation
  rows, and continuation rows must stay aligned under their first row's indent.
- **`ratatui::List` scrolls by item, not by line.** A tree node that wraps to
  seven lines makes vertical scrolling lurch. The alternative — render the pane
  as a `Paragraph` with a computed line offset and manage the cursor by hand —
  is a real fork and deserves an argued decision in design.md, because the left
  pane's `List` approach does not prepare the codebase for it either way.
- **Upside of wrapping:** with no horizontal scroll, `h`/`l` are free in the
  right pane for vim-tree collapse/expand, which is the expected idiom.

### Word-diff and markdown rendering are mutually exclusive

`tui-markdown` (already a dependency, unused) strips the `**` from `**WHEN**`,
so character offsets from change 2's diff runs no longer map onto rendered
spans. **You cannot have both styled markdown and word-level highlighting in a
diffed region.** Pick one and say so. Recommend: render source text throughout,
so the same content looks the same whether or not it happens to be inside a
changed block. A scheme where unchanged blocks render as pretty markdown and
changed blocks render as raw source is visually incoherent.

### Tree shape: group headers *and* gutter markers

Both. Group header rows for scanning, plus a per-requirement gutter marker so a
requirement scrolled away from its header is still self-identifying.

Four gutter states — `+` added, `~` modified, `-` removed, and **`?`
unmentioned** (dimmed). The `?` state comes from change 2 and means "present in
the base spec, not mentioned in the delta; cannot tell whether dropped or
untouched" — see `02-spec-diff.md` for why this exists. It must be visually
distinct from `-`, not a shade of red.

### Collapse defaults

- Requirement headers: **collapsed by default.** With everything collapsed,
  `2026-08-08-tui-changelist-horizontal-scrolling` shows four requirement rows —
  a useful summary view.
- Expanding a requirement reveals its intro block plus its scenario headers.
- Scenario headers: **collapsed by default**, expand to show the bullet body.
- Requirement names wrap like body text (they run long — `Horizontal scroll
  offset persists across selection and section toggling, clamped to current
  content` is 103 characters). No truncation with `…`.

Sketch, mid-expansion:

```
┌ Changes ──────────┐┌ tui-changelist │ tui ────────────────────────────────┐
│ ▸ archived        ││ ADDED                                                │
│                   ││ + ▸ Left pane scrolls horizontally as a single unit  │
│                   ││ + ▸ Horizontal scroll position is indicated with a   │
│                   ││     scrollbar                                        │
│                   ││ MODIFIED                                             │
│                   ││ ~ ▾ Archived changes are grouped under a collapsible │
│                   ││     section                                          │
│                   ││     The left pane SHALL show a single `archived` row │
│                   ││     after the active changes. … in the list. While   │
│                   ││     collapsed, the `archived` row SHALL render with  │
│                   ││     an underline style; while expanded, it SHALL NOT.│
│                   ││                        └──── inserted run, green ────┘
│                   ││     ▸ archived row collapsed by default              │
│                   ││   + ▸ collapsed row is underlined                    │
│                   ││   ? ▸ a scenario only present in the base spec       │
└───────────────────┘└──────────────────────────────────────────────────────┘
```

## This change must modify the `tui` capability

Two existing requirements in `openspec/specs/tui/spec.md` directly contradict
this work:

- **`Right pane is a placeholder`** (`:16`) — "It SHALL NOT display change
  contents, diffs, or any other content." → REMOVED, or rewritten wholesale.
- **`Left pane holds input focus`** (`:23`) — "There SHALL be no mechanism to
  move focus to the right pane." → MODIFIED. A collapsible right pane needs
  focus.

A focus model is therefore in scope. Recommended: `Tab` toggles pane focus;
focus is visually indicated (border style or title emphasis). Right-pane keys
mirror the left where sensible — `j`/`k`/`Up`/`Down` move the cursor,
`Enter`/`Space` toggle, `l`/`Right` expand, `h`/`Left` collapse. `Ctrl+Q` stays
global.

Pleasing side effect worth noting in the proposal: this change will itself
produce a delta containing ADDED, MODIFIED **and** REMOVED requirements across
two capabilities — the first change in the repo's history to exercise every path
the feature renders. It is its own end-to-end test fixture.

## Other behaviour to pin down

- **Selection in the left pane drives the right pane.** Today the left pane's
  `archived` header is selectable but is not a change; decide what the right
  pane shows when it (or a placeholder row) is selected.
- **Vertical scrolling / scrollbar** in the right pane, mirroring the left
  pane's horizontal one.
- **Error and empty states**, rendered in-pane rather than crashing: a change
  with no `specs/` directory; a MODIFIED requirement naming something absent
  from the base spec; a malformed delta file.
- **Recomputation cost.** Diffs are recomputed on selection change. At this
  repo's scale that is trivially fine; say so rather than caching pre-emptively
  (consistent with the existing decision to recompute `max_scroll` every render).

## Non-goals

- Making the 35/65 pane split adjustable — already deferred explicitly in
  `archive/2026-08-08-tui-changelist-horizontal-scrolling/proposal.md`.
- Editing specs. The tool is read-only throughout (`filesystem` capability is
  read-only by requirement).
- Rendering `proposal.md`, `design.md`, or `tasks.md`.
- Any parsing or diffing logic — changes 1 and 2 own those.

## Validation data in this repo

| Change | Capabilities | What it exercises |
|---|---|---|
| `archive/2026-08-08-tui-changelist-horizontal-scrolling` | `tui-changelist` | 3 ADDED + 1 MODIFIED with a changed intro and 3 added scenarios — the best single end-to-end case |
| `archive/2026-08-07-tui-initial` | `tui`, `tui-changelist` | multi-capability tabs; base spec files absent entirely |
| `archive/2026-08-07-add-readonly-filesystem` | `filesystem` | all-ADDED, single tab |
| `archive/2026-08-07-change-modeling` | `change-model` | all-ADDED, single tab |

No REMOVED, RENAMED, or `Unmentioned` case exists in the repo yet — synthetic
fixtures needed, until this change itself lands and supplies a REMOVED.

`src/changes/mod.rs` and `src/vfs/mod.rs` have `test_support` modules with
`TempDir` / `write_file` / `stage_and_commit` helpers; `src/tui/mod.rs` and
`src/tui/app.rs` show the established pattern of unit-testing span construction
and key handling directly rather than through a rendered terminal.

## Conventions

Design docs here are dense and argumentative: every decision states the
alternative considered and why it was rejected, and `## Risks / Trade-offs`
entries say whether the risk is resolved-by-design or knowingly deferred. See
`openspec/changes/archive/2026-08-08-tui-changelist-horizontal-scrolling/design.md`.
Match it.
