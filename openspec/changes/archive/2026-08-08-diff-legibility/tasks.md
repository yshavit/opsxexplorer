## 1. Close the test gap that would hide an alignment bug

- [x] 1.1 In `src/diff/runs.rs` tests, extend `assert_reconstructs` (or add a
      sibling assertion used everywhere it is) to check that every `Run::Equal`
      addresses identical text on both sides: `base[r.base] == delta[r.delta]`.
      Reconstruction alone cannot catch a mis-aligned equal run — see
      design.md, Risks.
- [x] 1.2 Confirm the new assertion passes against today's behaviour before any
      production change, so it is a guard and not a moving target.

## 2. Whitespace stops anchoring a diff

- [x] 2.1 Add a private post-processing pass in `src/diff/runs.rs` that takes
      the merged runs and returns coalesced runs. An `Equal` run is a
      *non-anchor* when its text is entirely whitespace, contains no line break,
      and it has a non-`Equal` run on both sides.
- [x] 2.2 Implement the pass at region granularity: find maximal spans of runs
      containing no surviving anchor, and emit each span as one `Delete` of all
      its base text followed by one `Insert` of all its delta text. Do **not**
      split individual equal runs and re-merge — that produces alternating
      `D I D I` and merges nothing (design.md, Decisions).
- [x] 2.3 Call the pass at the end of `runs()`. No signature change.
- [x] 2.4 Test: two consecutive words replaced by two different words yields one
      `Delete` and one `Insert`, not two of each. Use the real case —
      `- **WHEN** the user quits the application via Ctrl+Q` against
      `…by pressing \`q\`` — and assert the exact run sequence.
- [x] 2.5 Test: an equal run that is not purely whitespace still anchors.
      `"a b c d e"` → `"a X c Y e"` keeps `c` as an equal run and reports two
      distinct changes.
- [x] 2.6 Test: a whitespace-only equal run containing a newline still anchors,
      so a scenario body whose `WHEN` and `THEN` lines both change does not
      collapse into one deletion spanning both bullets.
- [x] 2.7 Confirm the existing `trailing_append_yields_one_insert_and_no_delete`
      and `edits_are_word_level_not_line_level` tests still pass unchanged.

## 3. Report a too-dissimilar piece as a wholesale replacement

- [x] 3.1 Add `Piece::Replaced { base: String, delta: String }` to
      `src/diff/model.rs`. No `runs` field.
- [x] 3.2 Add a similarity helper computing
      `2 × equal_bytes ÷ (base_bytes + delta_bytes)` over the coalesced runs.
      Derive it from the runs, not from a second `TextDiff` pass, so the measure
      and the reported runs cannot disagree.
- [x] 3.3 Add the threshold as a single named constant, `0.35`, documented so
      the number stands on its own without opening another file. Inline three
      calibration rows — the piece that fires, the nearest piece that must not,
      and an ordinary edit — and state the gap that positions the threshold
      between the first two. Name the change (`diff-legibility`) rather than
      writing a bare "see design.md", since the design doc moves into
      `openspec/changes/archive/` once the change is archived and the codebase
      has ten of them. Sketch:

      ```rust
      /// Below this similarity, a piece is reported as a wholesale replacement
      /// rather than as runs. Measured as
      /// `2 × equal_bytes ÷ (base_bytes + delta_bytes)` over the coalesced runs.
      ///
      /// Calibrated against every changed piece in this repo's own archive:
      ///
      /// | similarity | piece | verdict |
      /// |---|---|---|
      /// | 0.238 | `tui-specdiff` rename, *Focus moves between the two panes* intro | replace |
      /// | 0.483 | `changelist-archived-ordering` intro | keep inline |
      /// | 0.909 | `tui-keybinding-improvements`, *normal exit restores terminal* | keep inline |
      ///
      /// 0.238 → 0.483 is the largest gap in that distribution (0.245; the next
      /// largest is 0.095), so the threshold sits in the widest empty band the
      /// data offers. Full table and the rejected alternatives: the
      /// `diff-legibility` change's design.md.
      const INLINE_DIFF_MIN_SIMILARITY: f32 = 0.35;
      ```
- [x] 3.4 In `compare.rs::changed_or_unchanged`, choose `Replaced` over
      `Changed` when similarity is below the threshold and neither text is
      empty. Leave `runs.rs` unaware of the threshold.
- [x] 3.5 Test: the real worst case — the `tui-specdiff` rename's
      *Focus moves between the two panes* intro — is reported as `Replaced`.
      Inline the two texts as fixtures so the test survives archiving, following
      the pattern of `horizontal_scrolling_change_diffs_as_expected_against_its_base`.
- [x] 3.6 Test: the two `tui-specdiff` scenario bodies and the
      `changelist-archived-ordering` intro — the next-nearest pieces, at 0.483
      to 0.585 — are still reported as `Changed`.
- [x] 3.7 Test: `Changed { base: "", delta: "…" }` stays `Changed`, never
      `Replaced`.
- [x] 3.8 Test: a `Replaced` piece carries both texts byte-identical to the
      inputs.

## 4. Render a replacement as stacked before-and-after text

- [x] 4.1 Add the `Piece::Replaced` arm to `piece_marker` in
      `src/tui/layout.rs`, returning the existing modified marker and style.
- [x] 4.2 Add the `Piece::Replaced` arm to `piece_spans`: the base text styled
      with `removed_style()`, a raw `"\n"`, then the delta text styled with
      `added_style()`. No change to `wrap.rs` — it already treats `\n` as a
      forced break.
- [x] 4.3 Build and fix any remaining non-exhaustive `Piece` matches the
      compiler reports in `diff/mod.rs`, `tui/diff_row.rs` and `tui/app.rs`.
      (None were needed: only `layout.rs` matches `Piece` exhaustively.)
- [x] 4.4 Test: a `Replaced` piece renders both texts in full, the delta text
      begins on a new line, and the two carry the deletion and insertion styles.
- [x] 4.5 Test: a `Replaced` piece whose text exceeds the pane width wraps and
      keeps its styling across the break.

## 5. Verify against the real archive (manually done by a human)

- [x] 5.1 Run the TUI against this repo and inspect the `tui-specdiff` change's
      `tui` capability: the renamed requirement's intro should render stacked,
      and its two scenario bodies should still render inline.
- [x] 5.2 Inspect the `tui-keybinding-improvements` change: the
      *Terminal state is restored on exit* scenario should read
      `WHEN the user quits the application [-via Ctrl+Q-]{+by pressing \`q\`+}`
      with one deletion and one insertion.
- [x] 5.3 Spot-check the remaining archived changes for pieces that changed
      shape unexpectedly.
- [x] 5.4 Run `cargo test` and `cargo clippy`; run `openspec validate
      diff-legibility --strict`. (178 tests pass, clippy clean, validate passes.)
