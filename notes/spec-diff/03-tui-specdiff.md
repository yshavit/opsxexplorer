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
MODIFIED → REMOVED → RENAMED, each with an intro block and scenarios, where
changed blocks carry word-level `Equal`/`Delete`/`Insert` runs over the body text
change 1 produced — plus a list of per-entry errors alongside the entries that
did compute.

> **Updated after change 1 was specified.** This originally said "over faithful
> source text." Change 1's bodies are `mdq`'s re-serialisation of the parsed
> markdown tree, not the file's bytes: no content is lost and nothing is
> re-wrapped, but bullet markers, emphasis characters and escaping may differ
> from what is on disk. The right pane therefore cannot claim to be showing the
> file verbatim, and diff-run offsets do not map back onto it. Change 1 also
> guarantees that both sides are normalised identically, and that capability
> enumeration is alphabetically stable — the property the tab bar below relies
> on.

> **Updated after change 2 was specified.** Four things it settled that this
> pane has to absorb. See
> `openspec/changes/spec-diff/{specs/spec-diff/spec.md,design.md}`.
>
> - **Four operation groups, not three.** RENAMED became a first-class operation
>   rather than a name-diffed MODIFIED, so it needs a gutter marker of its own
>   (see below). A delta that renames a requirement *and* modifies it under the
>   new name yields **one** entry carrying both names and the content diff — the
>   pane must not render it twice.
> - **Every intro and scenario body arrives as one of five states**, not as raw
>   text: `Unchanged`, `Changed` (with runs), `Added`, `Deleted`, `Unmentioned`.
>   All five need a visual treatment, and they sit *below* the requirement level.
> - **A `Changed` block holds two strings, not one.** Runs are byte ranges into
>   the base body and the delta body separately — `Equal` carries a range into
>   each, `Delete` only into the base, `Insert` only into the delta. Rendering
>   one reflowed paragraph means interleaving slices of two strings, which is
>   what `wrap_spans` is actually being handed. Offsets are into change 1's
>   normalised bodies; there is no mapping back to the file.
> - **Errors are per entry, and collected rather than fatal.** A capability's
>   diff carries both the requirements it computed and the entries that failed,
>   so a tab renders good requirements *and* an error notice together. This is a
>   level below the per-capability isolation noted further down — that keeps one
>   bad capability from blanking the other tabs; this keeps one bad requirement
>   from blanking the rest of its own tab.

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

The longest line in this repo's specs is **684 characters**
(`openspec/specs/spec-model/spec.md:10`, written since this brief was drafted;
the 476-character `openspec/specs/tui-changelist/spec.md:115` it originally
cited is still there and still the worst case in that file). At 65% of a
120-column terminal
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
diffed region.** Pick one and say so. Recommend: render change 1's body text
throughout, so the same content looks the same whether or not it happens to be
inside a changed block. A scheme where unchanged blocks render as pretty
markdown and changed blocks render as raw text is visually incoherent.

> **One supporting argument weakened after change 1 was specified.** The
> recommendation stands, but "render source text" is no longer literally
> available — change 1's bodies are re-serialised, so the pane is not
> byte-identical to the file under *either* option. The choice is now between
> two normalised renderings rather than between the file's own bytes and a
> prettified version. The reason to pick one uniformly (visual coherence) is
> untouched; the reason that used to make "raw source" feel like the honest
> default no longer applies.

### Tree shape: group headers *and* gutter markers

Both. Group header rows for scanning, plus a per-requirement gutter marker so a
requirement scrolled away from its header is still self-identifying.

Four gutter states — `+` added, `~` modified, `-` removed, and **`?`
unmentioned** (dimmed). The `?` state comes from change 2 and means "present in
the base spec, not mentioned in the delta; cannot tell whether dropped or
untouched" — see `02-spec-diff.md` for why this exists. It must be visually
distinct from `-`, not a shade of red.

> **Updated after change 2 was specified.** The four states above are right but
> mislevelled, and one is missing.
>
> - **A fifth marker is needed for RENAMED**, now that it is a first-class
>   operation. It marks a requirement whose *name* changed, so the row has to
>   show both names — old and new — while its intro and scenarios carry their own
>   states underneath. `»` is the obvious candidate; the brief takes no position.
> - **`+`, `~`, `-` and the rename marker are requirement-level; `?` is not.**
>   Change 2's `Operation` has no unmentioned variant — a base requirement the
>   delta never names produces no entry at all, so it is not on screen to mark.
>   `Unmentioned` is a *piece* state, reached only inside a modified or renamed
>   requirement. So `?` can never appear on a requirement row: it belongs on
>   scenario rows and on the intro block, which means those rows need markers of
>   their own rather than inheriting their requirement's. The sketch below
>   already gets this right — the `?` sits on a scenario row inside a `~`
>   requirement — but the prose above does not.
> - **The intro block is the sharp case.** An `Unmentioned` intro renders the
>   base's intro, which is the same text an `Unchanged` intro renders. Change 2
>   keeps the two states apart, but unless this pane gives the intro block a
>   marker or dims it, they are indistinguishable on screen — and an intro is
>   the one piece with no header row to hang a gutter marker off. Solving this is
>   this change's job, not change 2's; it is the one place where the delta's
>   silence could genuinely go unnoticed by a reader.

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
  from the base spec; a malformed delta file. Change 1's error vocabulary, now
  settled, is `MissingSpecDocument` (an enumerated capability with no spec.md),
  `MissingBaseSpec` (a MODIFIED/REMOVED entry with no spec of record at all),
  `Markdown` (the file is not parseable markdown) and `Structure` (a stray
  scenario, an unrecognised `## <OP> Requirements` section, a `FROM:` with no
  `TO:`). Change 2 adds one more, and it is the one this bullet's own example
  actually names: `MissingBaseRequirement` — the base spec exists but has no
  requirement under that name, i.e. a mistyped heading or a rename done by hand.
  Do not conflate it with `MissingBaseSpec`; they are different authoring
  mistakes and change 2 kept them separate deliberately. Three properties of the
  vocabulary shape this pane:
  - **Failures are isolated per capability.** A malformed delta in one
    capability must render as an error *inside that tab* while the change's
    other tabs still display normally — change 1 loads one capability per call
    specifically so this is true. Do not let one bad capability blank the pane.
  - **Failures are isolated per requirement too, not only per capability.**
    `MissingBaseRequirement` is reported against the entry it concerns and
    collected alongside the requirements that did compute, so a tab shows an
    error notice *and* its good content in the same view. Rendering the error
    instead of the content would throw away work change 2 went out of its way to
    preserve.
  - **Errors carry a structural location, not a line number.** `mdq` drops
    source positions, so an error identifies itself as e.g. *under
    `## MODIFIED Requirements`, requirement "Change discovery"* — there is no
    "line 42" to show, and the pane should not imply one.
- A change with no `specs/` directory is **not** an error: change 1 returns an
  empty capability list, so this renders as "no spec changes" with no tab bar.
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
