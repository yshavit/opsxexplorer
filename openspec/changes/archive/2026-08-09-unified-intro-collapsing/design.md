## Context

See proposal.md - Why. The relevant existing pieces, all from the archived `render-purpose` change:

- `RowKey` (`src/tui/diff_row.rs`) is an enum keying a row's collapse state independent of its position in the flattened tree: `Purpose { capability }`, `Requirement { capability, requirement }`, `Scenario { capability, requirement, scenario }`.
- `DiffRow::PurposeHeading(&Piece)` (display-only), `DiffRow::PurposeFull(&Piece)` (fits in full, selectable, no toggle state), and `DiffRow::Purpose { piece, expanded, key }` (collapsible) are the three purpose-specific row kinds `flatten()` can emit. `push_purpose` (`diff_row.rs`) decides which of `PurposeFull`/`Purpose` to emit by comparing the piece's current text against `layout::purpose_available(width)`.
- `layout.rs`'s `purpose_available(width)` computes the collapsed row's character budget assuming indent 0 and the fixed `"▸ ¶ "` prefix; `collapsed_purpose_lines(piece, width)` renders the single collapsed line (a `truncate_chars` excerpt for `Added`/`Changed`, the italic `"Expand to view diff"` placeholder for `Replaced`), with no indent padding since indent is always 0 for purpose.
- `push_requirement` (`diff_row.rs`) unconditionally pushes `DiffRow::Intro { piece: &req.intro }` whenever a requirement is expanded — no width check, no `RowKey`, not selectable, always rendered through the ordinary `piece_spans` + `wrap_spans` path at indent 1.
- `App`'s cursor/toggle plumbing (`src/tui/app.rs`: `cursor_row_key_and_state`, `toggle_cursor_row`, `set_cursor_row_expanded`, `reset_cursor_to_first_selectable`) operates generically over `DiffRow::key()`, `expanded()`, and `is_selectable()` — it does not match on individual row variants, so it needs no changes for this proposal (the same reason `render-purpose` didn't need to touch it).
- `RequirementDiff.intro: Piece` (`src/diff/model.rs`) is already computed by the same `compare::changed_or_unchanged` machinery `CapabilityDiff.purpose` uses, but unlike `purpose` (filtered in `diff()` to only ever be `Added`, `Changed`, or `Replaced`), an intro's `Piece` can additionally be `Unchanged`, `Deleted`, or `Unmentioned` — every variant `Piece` defines.

## Goals / Non-Goals

**Goals:**
- Give a requirement's intro row the same collapse/expand/truncate/placeholder behavior the purpose row has, reusing rather than re-deriving the machinery `render-purpose` built.
- Collapse the duplicated row-kind shape between purpose and intro down to one code path, so a future change to this behavior (e.g. a different truncation rule) only needs to happen once.
- Correctly handle the `Piece` variants an intro can carry that a purpose comparison never does (`Unchanged`, `Deleted`, `Unmentioned`).

**Non-Goals:**
- Scenario bodies (`DiffRow::Body`) are not touched. They have the same "always full, never collapsible" shape today, but the issue's own resolution is explicit: a scenario's WHEN/THEN body is bounded and structured, and seeing it in full is the reason someone expands the scenario in the first place.
- No change to `spec-diff` or `src/diff/*` — an intro's `Piece` is already computed correctly; this is purely a `tui-specdiff` rendering change.
- No change to how a requirement itself is expanded/collapsed, or to scenario collapse behavior.

## Decisions

**Unify `PurposeFull`/`Purpose`/`Intro` into a shared `ParagraphFull`/`Paragraph` row family carrying an explicit `indent`, rather than keeping `Purpose` and `Intro` as separate `DiffRow` variants that call shared helper functions.**

```rust
DiffRow::ParagraphFull { piece: &'a Piece, indent: usize }   // replaces PurposeFull
DiffRow::Paragraph {                                          // replaces Purpose
    piece: &'a Piece,
    expanded: bool,
    key: RowKey,
    indent: usize,
}
```

`DiffRow::Intro { piece }` is removed; a requirement's intro now flows through these two variants with `indent: 1` (vs. `0` for purpose). `DiffRow::PurposeHeading` is untouched — it draws the boxed "Added Purpose"/"Modified Purpose" heading, which has no intro equivalent.

Why unify the variants instead of the narrower option (add `RowKey::Intro` and a parallel `DiffRow::Intro { piece, expanded, key }`, with `push_purpose` and a new `push_intro` each calling shared sub-helpers in `layout.rs`): a purpose comparison and a requirement's intro are the same concept from the pane's own point of view — one paragraph-shaped `Piece` that may need to collapse — and the code should say that directly rather than modeling them as two lookalike-but-distinct kinds that happen to share internals. Keeping `Purpose` and `Intro` as separate variants would keep that as an implementation-only fact (visible only inside shared helper functions) instead of a fact about the pane's data model; anyone reading `DiffRow` later would have to rediscover that the two are interchangeable rather than see it in the type. Collapsing them into one variant family makes that unification the thing a reader of `diff_row.rs` sees first, not something they infer from matching helper-call patterns. This also happens to be what the issue is asking for — `push_purpose` and the intro path "call the same code" — and it eliminates the more mechanical risk of every existing per-variant match in `diff_row.rs` and `layout.rs` (`is_selectable()`, `key()`, `expanded()`, `indent_depth`, `gutter_marker`, `content_spans`, `row_lines`'s collapsed-row special case) permanently carrying two arms that do identical work. It mirrors the precedent `render-purpose` itself set when it turned `RowKey` from a struct into an enum for the same reason: making an impossible-to-conflate state impossible to represent, rather than merely inconvenient to conflate.

The one thing that made unification awkward before this change — `PurposeFull` carries no `RowKey`, so nothing told it its nesting depth — is resolved by adding an explicit `indent` field to both variants rather than deriving indent from the row's key or variant name. `indent_depth` (`layout.rs`) reads that field directly for `ParagraphFull`/`Paragraph` instead of returning a hardcoded constant.

`RowKey` gains `Intro { capability: String, requirement: String }`, kept as its own variant rather than folded into `RowKey::Purpose` with an `Option<String>` requirement field — the same reasoning `render-purpose`'s own design.md gave for keeping `Requirement`/`Scenario` as separate variants: a sentinel-shaped field lets an impossible state compile (a `Purpose` key with a requirement name means nothing).

**A single push function replaces `push_purpose` and the `push_requirement` intro line.** Something like:

```rust
fn push_paragraph_row<'a>(
    rows: &mut Vec<DiffRow<'a>>,
    piece: &'a Piece,
    key: RowKey,
    indent: usize,
    width: usize,
    expanded: &HashSet<RowKey>,
)
```

`push_purpose` calls it with `key: RowKey::Purpose { capability }`, `indent: 0`; the requirement-intro branch of `push_requirement` calls it with `key: RowKey::Intro { capability, requirement }`, `indent: 1`. Both still need their own thin wrapper for the parts that aren't shared — `push_purpose` also emits `PurposeHeading` first, which has no intro equivalent.

**Text extraction generalizes to cover every `Piece` variant, not just `Added`/`Changed`.** Today's fits-check (`push_purpose`) does:

```rust
match piece {
    Piece::Added { delta } | Piece::Changed { delta, .. } => Some(delta.as_str()),
    _ => None,  // Replaced only, in practice — purpose is never anything else
}
```

and treats `None` as "always collapsible." That catch-all is only correct because purpose is filtered upstream to `Added`/`Changed`/`Replaced`. An intro can also be `Unchanged`, `Deleted`, or `Unmentioned`, each of which is exactly as "a single passage of ordinary text" as `Added`/`Changed` — a long *unchanged* intro should truncate to an excerpt, not fall into the `Replaced`-only placeholder path. The shared helper's text extraction becomes:

```rust
fn paragraph_text(piece: &Piece) -> Option<&str> {
    match piece {
        Piece::Unchanged { text } => Some(text),
        Piece::Added { delta } => Some(delta),
        Piece::Deleted { base } => Some(base),
        Piece::Unmentioned { base } => Some(base),
        Piece::Changed { delta, .. } => Some(delta),
        Piece::Replaced { .. } => None,  // no single "current text"; always collapsible
    }
}
```

This is a strict generalization: for `Added`/`Changed`/`Replaced` it produces the same `Some`/`None` split as today, so purpose's own behavior is unchanged; it only adds coverage for the three variants purpose never reaches. The same generalization applies on the `layout.rs` side, in the collapsed-row renderer: only `Piece::Replaced` gets the italic placeholder; every other variant gets a `truncate_chars` excerpt of its own text field.

**Indent threads through the width budget.** `purpose_available(width)` generalizes to `paragraph_available(width, indent) -> usize`, additionally subtracting `indent * INDENT_UNIT` from the budget. `collapsed_purpose_lines(piece, width)` generalizes to `collapsed_paragraph_lines(piece, width, indent) -> Vec<Line>`, prepending `indent * INDENT_UNIT` columns of blank padding before the row's content — today's version emits none, which only worked because purpose is always indent 0. `row_lines`'s special case (`if let DiffRow::Purpose { expanded: false, .. } = row`) widens to match `DiffRow::Paragraph { expanded: false, .. }` and passes its `indent` through.

**Selectability.** `is_selectable()`'s existing exception for `PurposeFull` widens to `ParagraphFull`/`Paragraph` generally — both purpose and intro rows are selectable whether or not they're collapsible, per the same rationale `render-purpose` gave (consistent row-by-row cursor traversal). See the `tui-specdiff` spec delta's revision to "Only collapsible rows are selectable."

**Unmentioned dimming applies uniformly across collapse states.** Today's `DiffRow::Intro` arm in `content_spans` wraps its spans in `dim()` when the piece is `Piece::Unmentioned`, but that only ever ran on the always-expanded path. With intro now collapsible, the collapsed excerpt needs the same treatment — `collapsed_paragraph_lines` applies the de-emphasised style to the excerpt span whenever `piece` is `Unmentioned`, mirroring what the expanded path already does. This is spelled out explicitly as its own spec scenario (see the spec delta) rather than left as an incidental consequence of sharing code — it would be an easy case to silently drop when merging the two rendering paths.

## Risks / Trade-offs

- **Removing `DiffRow::Intro` and `DiffRow::Purpose`/`PurposeFull` touches every existing call site and test that constructs or matches one.** Mitigation: mechanical — the compiler catches every site; `render-purpose`'s own `RowKey` struct-to-enum change went through the same kind of mechanical, compiler-verified update.
- **The generalized `paragraph_text` extraction changes behavior for `Piece::Unchanged`/`Deleted`/`Unmentioned`, which purpose's narrower version never exercised.** Mitigation: for the three variants purpose already handles (`Added`/`Changed`/`Replaced`), the extraction produces identical `Some`/`None` results to today, so purpose's existing tests continue to hold unchanged; the three new variants get dedicated new tests exercising the fits-check and collapsed-excerpt paths for each.
- **A requirement's intro row becoming selectable changes cursor-movement behavior a user may already be relying on (intro rows used to be silently skipped).** Mitigation: this is exactly what the issue asks for, and it mirrors the identical exception already shipped for the purpose row — not a new pattern, just a wider application of one the pane already has.
- **Threading `indent` as an explicit field (rather than continuing to hardcode it per-variant) is a small but real widening of `DiffRow`'s surface.** Mitigation: `indent_depth`'s other arms are untouched constants; only the two unified variants carry the field, and it's populated at exactly two call sites (`push_purpose`, the intro branch of `push_requirement`), so there's no risk of an inconsistent value drifting in from elsewhere.

## Migration Plan

N/A — internal rendering change to a TUI, no persisted state or external interface changes. Ships as a normal code change.
