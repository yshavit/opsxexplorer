## 1. `change-model`: expose both sides of a change's diff

- [x] 1.1 Add `ChangeViews<'a> { live: Fs<'a>, base: Fs<'a>, change: Change }` to `src/changes/` and `Changes::views(&ChangeView) -> Result<ChangeViews<'_>, FsError>`, where `live` is always `Workspace::current()`, `base` follows the view's already-resolved `DiffBase`, and `change` is cloned from the view so `capabilities`/`load` (section 5) can resolve `<change>/specs/` without a second parameter. (Amended during implementation — see design.md.)
- [x] 1.2 Extract the existing `DiffBase` → `Fs` resolution in `Changes::open` into one private helper, and have both `open` and `views` call it, so the two accessors cannot drift. Leave `open`'s signature and behavior unchanged.
- [x] 1.3 Test: for an archived change whose diff base predates its own directory, a delta spec file under `<change>/specs/` is readable through `views(...).live` and absent from `views(...).base`, while the spec of record read through `.base` is its state at the diff base (not its current state).
- [x] 1.4 Test: for an active change, both `.live` and `.base` reflect an uncommitted working-tree edit to the spec of record.
- [x] 1.5 Test: the existing `change_view_travels_without_relookup_of_status` guarantee still holds through `views` — a held `ChangeView` yields the pair with no re-lookup of active/archived status.

## 2. `spec-model` module scaffold and error type

- [x] 2.1 Create `src/specs/` (`mod.rs`, `model.rs`, `parse.rs`, `load.rs`, `error.rs`) and register it in `src/main.rs`. Add `mdq` 0.10 to `Cargo.toml`.
- [x] 2.2 Define `SpecError` in `src/specs/error.rs`: `Structure { path, at: Location, kind: StructureErrorKind }` (kinds: stray scenario before any requirement, unrecognised `## <OP> Requirements` section, rename missing its `TO:`), `Markdown { path, source: mdq::md_elem::InvalidMd }`, `MissingSpecDocument { capability }`, `MissingBaseSpec { capability, requirement }`, `Fs(FsError)`. Follow the hand-written `Display` / `Error::source` / `From` pattern in `src/vfs/error.rs` and `src/changes/error.rs` — no `thiserror`. `Location` is structural (operation section, and requirement name where known), **not** a line number: `md_elem` drops source positions, so line numbers are unavailable (see design.md).

## 3. The parsed model

- [x] 3.1 Define in `src/specs/model.rs`: `Scenario { name: String, body: String }`; `Requirement { name: String, intro: String, scenarios: Vec<Scenario> }` (a plain `String` — a requirement with no intro block and one with an empty intro block are indistinguishable in the source, so both parse to `""`; see design.md); `Spec { purpose: Option<String>, requirements: Vec<Requirement> }`.
- [x] 3.2 Define the delta shapes: `DeltaOp { Added, Modified, Removed }`, `DeltaEntry { op: DeltaOp, requirement: Requirement }`, `Rename { from: String, to: String }`, and `Delta { purpose: Option<String>, entries: Vec<DeltaEntry>, renames: Vec<Rename> }`. Renames are their own list, not a `DeltaOp` variant (see design.md).
- [x] 3.3 All body/name fields are owned `String`s — the model must not borrow from the file buffer or carry a lifetime.

## 4. The parser

- [x] 4.0 **Round-trip spike — do this first, before writing any of the model or parser.** Nothing in this change has been compiled against `mdq`; the whole design rests on reading its source. Write a throwaway test that, for `openspec/specs/tui-changelist/spec.md` (contains the repo's longest line, 476 chars) and `openspec/changes/archive/2026-08-08-tui-changelist-horizontal-scrolling/specs/tui-changelist/spec.md` (ADDED + MODIFIED, no `## Purpose`): reads the file, runs `MdDoc::parse(.., &ParseOptions::gfm())`, re-serialises the whole document with `MdWriter` (`text_width: None`, `include_thematic_breaks: false`), and diffs the result against the original. Check specifically that no content is lost, that the 476-character line comes back unbroken, that no `-----` separators are injected, that `- **WHEN** …` bullets and backticked identifiers survive recognisably, and that every requirement and scenario heading is still present and correctly nested. Then assert the property that actually matters: **the round trip is idempotent** — re-parsing and re-serialising the output reproduces it exactly. Idempotence, not fidelity to the file, is what makes "both sides normalised identically" true, so it is the real acceptance criterion.
- [x] 4.0a **If 4.0 does not pass, STOP and ask for human input. Do not proceed to 4.1.** Do not hand-normalise around the difference, do not add a post-processing fixup step, and do not silently relax the acceptance criteria. A failure here means the parser choice itself needs revisiting, which is a design decision and not an implementation detail — `design.md` records `markdown`/mdast as the documented fallback (it retains byte offsets, so bodies would be sliced from source instead of re-serialised), but switching to it is the user's call, not the implementer's. Report exactly what differed and wait.
- [x] 4.1 In `src/specs/parse.rs`, parse with `mdq::md_elem::MdDoc::parse(text, &ParseOptions::gfm())` and walk the resulting `MdElem::Section` tree: depth-2 sections are the `## …` sections, their depth-3 subsections are requirements, and each requirement's depth-4 subsections are its scenarios. Derive section names by flattening `Section::title` to plain text, then take the text after `Requirement: ` / `Scenario: `. Note the type mismatch: `title` is `Vec<Inline>`, but `PlainWriter::write` takes `IntoIterator<Item = &MdElem>` and mdq's inline-only flattener is private. Either wrap each inline as `MdElem::Inline(..)` before writing, or walk the `Inline` enum directly — pick one in implementation and keep it in a single helper, since requirement names are the join key and must flatten identically on both sides.
- [x] 4.2 Convert body elements back to markdown with `mdq::output::MdWriter::write(&doc.ctx, &nodes, &mut String)`. Set `text_width: None` (any value re-flows prose and wrecks diffs) and `include_thematic_breaks: false` (it injects `-----` separators absent from every source file). `MdWriterOptions` derives both `Default` and `Builder`, and those two happen to be the current defaults — set them explicitly anyway, with a comment, so a future default change cannot silently corrupt output. A requirement's intro is the elements of its section body preceding the first depth-4 subsection.
- [x] 4.2a Keep `mdq` confined to `parse.rs` — no `mdq` type may appear in `model.rs`, `load.rs`, or any signature outside the parser, so the library can be swapped without touching consumers.
- [x] 4.3 Parse a main spec: tolerate `# <capability> Specification`, capture `## Purpose`, and read requirements under `## Requirements`. A document with no requirement headings parses to an empty requirement list, not an error.
- [x] 4.4 Parse a delta spec: recognise the added/modified/removed/renamed sections and tag each entry with its `DeltaOp`. Tolerate a leading `## Purpose` and do not treat it as a requirement section.
- [x] 4.5 Parse removed entries as heading-only (empty `intro`, `scenarios: []`), and renamed entries as `- FROM:` / `- TO:` bullet pairs into `renames`, tolerating the backticked `` `### Requirement: <name>` `` form the OpenSpec convention uses inside those bullets.
- [x] 4.6 Emit `SpecError::Structure` naming the structural location for: a scenario heading before any requirement heading, an unrecognised `## <OP> Requirements` section, and a `- FROM:` with no matching `- TO:`. Map an `InvalidMd` from `MdDoc::parse` to `SpecError::Markdown`. Fail on the first malformed construct; never drop uninterpretable content silently.
- [x] 4.7 Parser tests against the repo's own real files read from disk: `openspec/specs/*/spec.md` (main specs), and the four archived deltas — `2026-08-07-add-readonly-filesystem` (`filesystem`), `2026-08-07-change-modeling` (`change-model`), `2026-08-07-tui-initial` (`tui` and `tui-changelist`), `2026-08-08-tui-changelist-horizontal-scrolling` (`tui-changelist`, ADDED + MODIFIED, no `## Purpose`). Assert requirement and scenario counts, names, and operation tags. (Deliberately excludes this change's own still-active `openspec/changes/spec-model/specs/**` deltas — those move to an `archive/<date>-spec-model/` path once archived, which would break a test asserting the pre-archive path.)
- [x] 4.8 Synthetic parser tests for the paths no real file covers: a REMOVED section, a RENAMED section, and each of the three malformed cases from 4.6.
- [x] 4.9 Content-preservation tests (replacing the byte-faithfulness test the earlier draft called for — byte-identity is no longer promised): a body with bullets, bold/inline markdown, a multi-paragraph intro, a code block, and a 470+ character paragraph parses with every construct still present and nothing dropped, and the 470+ character paragraph is not broken across lines. Assert content and structure, not byte equality with the source.
- [x] 4.9a Normalisation-consistency test: the same requirement source, parsed once as a delta entry and once as a spec-of-record requirement, yields equal body strings — the property change 2 actually depends on.
- [x] 4.9b Block-structure test: a fenced code block containing the lines `### Requirement: Not Real` and `#### Scenario: Not Real` produces no extra requirement and no extra scenario — the case a line-prefix scanner would have failed.
- [x] 4.10 Scenario-subset test: a MODIFIED requirement listing fewer scenarios than its base parses with exactly the scenarios it lists — no error, and no marker implying the unlisted ones were deleted. This is the identity-based absence the model does preserve, as distinct from the intro case it cannot (see design.md).

## 5. Loading both sides

- [x] 5.1 Implement `capabilities(&ChangeViews) -> Result<Vec<String>, FsError>` in `src/specs/load.rs`: `list_dir` the change's `specs/` directory on the **live** view, keep directory entries only, sort alphabetically. Map `FsError::NotFound` on that directory to an empty `Vec` (a proposal-only change is "no spec changes"); propagate every other `FsError`.
- [x] 5.2 Implement `load(&ChangeViews, &str) -> Result<SpecPair, SpecError>` returning `SpecPair { delta: Delta, base: Option<Spec> }`: read and parse `<change>/specs/<cap>/spec.md` from `.live`, and `openspec/specs/<cap>/spec.md` from `.base`, treating a `NotFound` base as `None`.
- [x] 5.3 Report a missing delta document for an enumerated capability as `SpecError::MissingSpecDocument { capability }`.
- [x] 5.4 After parsing, if `base` is `None` and any entry's op is `Modified` or `Removed`, return `SpecError::MissingBaseSpec { capability, requirement }` naming the first such requirement — distinguishable from the tolerated all-ADDED absent-base case.
- [x] 5.5 Test: an all-ADDED delta for a capability with no spec of record at the diff base loads successfully with `base: None` (mirrors `2026-08-07-tui-initial`, which introduces both `tui` and `tui-changelist`).
- [x] 5.6 Test: a MODIFIED entry and, separately, a REMOVED entry with no base spec each produce `MissingBaseSpec`, not a success and not the same error as a missing delta.
- [x] 5.7 Test: a change with no `specs/` directory yields an empty capability list and no error; a change touching several capabilities yields them alphabetically, identically across repeated calls.
- [x] 5.8 Test: a capability directory containing no `spec.md` yields `MissingSpecDocument` for that capability while `load` still succeeds for the change's other capabilities — the per-capability isolation guarantee.
- [x] 5.9 Test: a removed requirement's intro and scenarios are recoverable from the loaded `base`, given the delta names it without a body.
- [x] 5.10 Test: for an archived change, `load` reads the delta from the working tree and the base at the resolved diff base, using a synthetic repo built with the existing `TempDir` / `write_file` / `stage_and_commit` helpers.

## 6. Verification

- [x] 6.1 Run `cargo test` and confirm all existing and new tests pass.
- [x] 6.2 Run `cargo clippy` and `cargo fmt --check` clean.
- [x] 6.3 Confirm no `src/tui/` file was modified — this change adds no rendering.
- [x] 6.4 Confirm `Changes::open`'s signature and behavior are unchanged and its existing tests still pass unmodified.

## 7. At archive time — deliberately NOT tasks, do not do these during apply

> These are intentionally written as plain bullets, not `- [ ]` checkboxes, so
> that `/opsx:apply` does not pick them up as pending work and `/opsx:archive`
> does not count them as incomplete tasks. They describe cleanup that only makes
> sense **after** this change has been archived. If you are running apply, skip
> this section entirely — doing any of it early would delete or rewrite briefs
> that later changes in the chain still read from.

- Reduce `notes/spec-diff/01-spec-model.md` to a pointer at
  `openspec/changes/archive/<date>-spec-model/`. The file currently opens with a
  SUPERSEDED banner and three struck-through reversed decisions; once the change
  is archived, the archived artifacts are the record and the banner's own
  instruction ("this file should be reduced to a pointer") comes due. Keep the
  filename — `02-spec-diff.md` and `03-tui-specdiff.md` both link to it from
  their chain headers, so deleting it outright leaves dangling references while
  those two briefs are still live.
- Do **not** delete `notes/spec-diff/` yet. That happens only once all three
  changes in the chain (`spec-model`, `spec-diff`, `tui-specdiff`) have landed.
- Nothing else in `notes/` needs touching: `02` and `03` were already updated
  for this change's downstream consequences (body text is no longer
  byte-faithful, the omitted-intro trigger, the split of the two missing-base
  cases, and change 1's error vocabulary).
