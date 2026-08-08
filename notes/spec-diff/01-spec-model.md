# Brief: `spec-model` capability — parse and load OpenSpec spec files

> Chain position: **1 of 3**. Read → compare → render.
> 1. **`spec-model`** (this file) — parse a spec.md into requirements/scenarios; load both sides for a change.
> 2. `spec-diff` (`02-spec-diff.md`) — compare a delta requirement against its base.
> 3. `tui-specdiff` (`03-tui-specdiff.md`) — render it in the right pane.
>
> This brief is self-contained. It is the output of an exploration session; the
> decisions in it were made deliberately and should be carried into the proposal.

## Project context

opsxexplorer is a terminal UI (Rust 2024, ratatui 0.30) for browsing OpenSpec
changes and specs in a local git repo. It reads OpenSpec's file layout:

```
openspec/specs/<capability>/spec.md              main spec ("spec of record")
openspec/changes/<change>/                       active change
openspec/changes/archive/<date>-<change>/        archived change
  └── specs/<capability>/spec.md                 delta spec
```

The tool's whole reason to exist: a change's delta spec has `## ADDED
Requirements` / `## MODIFIED Requirements` sections, but MODIFIED does not show
what actually changed. opsxexplorer computes and renders that delta. **The diff
unit is a single requirement, not the whole file.**

Existing code:

- `src/vfs/` — read-only `Fs` abstraction over either the live working tree
  (`Workspace::current()`) or a resolved git commit (`Workspace::at(&GitRef)`).
  `Fs::read`, `Fs::list_dir`, `Fs::exists`.
- `src/changes/` — `Changes::discover(path)` finds active + archived changes;
  `Changes::resolve(&Change) -> ChangeView` pairs a change with its `DiffBase`;
  `Changes::open(&ChangeView) -> Fs` opens the **diff base** view.
  Active change → `DiffBase::Current`. Archived change → `DiffBase::At(commit
  immediately before the commit that first introduced the change directory)`.
- `src/tui/` — left pane changelist. Not touched by this change.

Relevant deps already in `Cargo.toml`, currently unused: `pulldown-cmark`
0.13.4, `similar` 3.1.2, `tui-markdown` 0.3.9.

## Scope of this change

Introduce a `spec-model` capability that turns spec markdown into a structured
model, and that can load **both sides** of a change/capability pair.

### 1. The parsed model

Both main specs and delta specs parse into the same requirement/scenario shapes.

Main spec file structure:

```markdown
# <capability> Specification

## Purpose
<prose>

## Requirements

### Requirement: <name>
<intro block — one or more paragraphs, everything before the first scenario>

#### Scenario: <name>
- **WHEN** …
- **THEN** …
```

Delta spec file structure:

```markdown
## Purpose                        ← optional; only on a brand-new capability

## ADDED Requirements
### Requirement: <name>
<intro block>
#### Scenario: <name>
- …

## MODIFIED Requirements
### Requirement: <name>
<intro block may be ABSENT>
#### Scenario: <name>             ← may be a SUBSET of the base's scenarios
- …

## REMOVED Requirements
### Requirement: <name>           ← HEADER ONLY. No intro, no scenarios.

## RENAMED Requirements
- FROM: `### Requirement: Old Name`
- TO: `### Requirement: New Name`
```

Parser requirements that are easy to get wrong:

- **A delta may open with `## Purpose`.** Three of the four archived changes in
  this repo do; the newest does not. Must be tolerated, and is not a requirement
  section.
- **REMOVED entries carry no body.** To display a removed requirement's intro
  and scenarios (which the UI wants), that content must be read from the **base
  spec**, not the delta. This is a load-time concern, see §2.
- **MODIFIED entries may omit the intro block** and may list only a subset of
  scenarios. Preserve the distinction between "intro absent" and "intro present
  but empty" — change 2 depends on it.
- **`## Requirements` appears only in main specs**, never in deltas; delta
  operation headers never appear in main specs.
- Scenario bodies are bullet lists but should be kept as raw text lines. Do not
  normalise, reflow, or strip markdown — change 2 diffs this text and change 3
  renders it, and both need byte-faithful source.
- Requirement and scenario names are the text after `### Requirement: ` /
  `#### Scenario: `. Names are the **join key** used by change 2.

Whether to parse with `pulldown-cmark` or with line scanning is open. Line
scanning is likely sufficient and simpler given the rigidly fixed heading
grammar, and it makes byte-faithful body preservation trivial; `pulldown-cmark`
is already a dependency but earns its keep more in change 3. Decide in design.md
with the usual rejected-alternative note.

Malformed input (a scenario before any requirement, an unknown `## FOO
Requirements` header, a `- FROM:` with no `- TO:`) should surface as a
structured error the UI can display, not a panic and not a silent drop.

### 2. Loading both sides — and the `change-model` API gap

For a given `ChangeView` and capability, two files are needed:

```
        delta spec                              base spec
openspec/changes/<chg>/specs/<cap>/spec.md   openspec/specs/<cap>/spec.md
            │                                        │
      ALWAYS the live                     DiffBase::Current  (active change)
      working tree                        DiffBase::At(ref)  (archived change)
```

**This asymmetry is the gap.** `Changes` today exposes only
`open(&ChangeView) -> Fs`, which resolves to the *diff base*. For an archived
change that view is the commit *before* the change directory existed, so the
delta spec is **not readable through it**. `Changes.vfs` is private and
`Workspace::current()` is unreachable from outside.

So `change-model` needs an addition — as an **ADDED requirement**, e.g. "both
sides of a change's diff are reachable from the resolved change" — exposing the
current view alongside the base view. A combined accessor that hands back the
pair for a (change, capability) is probably better than a bare `current()`,
because `change-model`'s existing "Resolved diff base travels with its change"
requirement is specifically about consumers not having to re-derive this. Do not
weaken that requirement.

Also needed: **discovering which capabilities a change touches** — `list_dir` on
`<change>/specs/`, each subdirectory being a capability. Sort order should be
stable (alphabetical) since change 3 turns these into tabs.

### 3. Edge cases the loader must handle

- **Base spec file does not exist.** Real case: `2026-08-07-tui-initial`
  introduces both `tui` and `tui-changelist`, neither of which existed at the
  diff base. All-ADDED deltas must work with no base file at all. A MODIFIED or
  REMOVED entry with no base file is an error worth surfacing distinctly.
- **A change with no `specs/` directory** (proposal-only). Should render as
  "no spec changes", not an error.
- **A capability directory with no spec.md.**

## Non-goals

- Any diffing. Change 2 owns all comparison.
- Any rendering, styling, or TUI work. Change 3 owns that.
- Reading `proposal.md`, `design.md`, `tasks.md`.
- Applying/merging deltas into main specs (that is `openspec archive`'s job).

## Validation data in this repo

Parse-test against real files rather than only synthetic ones:

| Change | Capabilities | Sections present |
|---|---|---|
| `archive/2026-08-07-add-readonly-filesystem` | `filesystem` | `## Purpose`, ADDED |
| `archive/2026-08-07-change-modeling` | `change-model` | `## Purpose`, ADDED |
| `archive/2026-08-07-tui-initial` | `tui`, `tui-changelist` | `## Purpose`, ADDED (both) |
| `archive/2026-08-08-tui-changelist-horizontal-scrolling` | `tui-changelist` | ADDED (3 reqs), MODIFIED (1 req) — no Purpose |

**No REMOVED or RENAMED section exists anywhere in this repo yet.** Those paths
need synthetic fixtures. Note also that the longest single line across these
specs is 476 characters (`openspec/specs/tui-changelist/spec.md:115`) — relevant
to change 3, but a reminder here not to assume short lines.

`src/changes/mod.rs` and `src/vfs/mod.rs` both have a `test_support` module with
`TempDir` / `write_file` / `stage_and_commit` helpers for building throwaway git
repos; follow that pattern.

## Conventions

Design docs in this repo are dense and argumentative: every decision states the
alternative considered and why it was rejected, and `## Risks / Trade-offs`
entries say whether the risk is resolved-by-design or knowingly deferred. See
`openspec/changes/archive/2026-08-08-tui-changelist-horizontal-scrolling/design.md`
for the house style. Match it.
