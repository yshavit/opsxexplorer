## Why

opsxexplorer exists to render what a change's `## MODIFIED Requirements` section actually changed, but nothing in the codebase yet turns a `spec.md` into anything other than bytes. Before any diffing (change 2) or rendering (change 3) can happen, both sides of the comparison have to be loadable as structured requirements: the delta spec from the change, and the base spec it modifies. Loading those two sides is not symmetric — the delta always lives in the live working tree while the base lives at the change's resolved diff base — and today `change-model` only exposes the base side, so the delta spec of an archived change is unreachable through the public API.

## What Changes

- Introduce a `spec-model` capability that parses OpenSpec spec markdown into a structured model of requirements and scenarios. Main specs (`## Requirements`) and delta specs (`## ADDED` / `## MODIFIED` / `## REMOVED` / `## RENAMED Requirements`) parse into the same requirement and scenario shapes, tagged by which operation section they came from.
- Preserve the distinctions the diff step depends on: a MODIFIED requirement may list only a subset of the base's scenarios, and the unlisted ones are unmentioned rather than deleted; a REMOVED entry is a bare header with no body at all, so its content must come from the base.
- Carry scenario and intro bodies through without losing authored content and without re-wrapping, normalising both sides identically so an unedited requirement compares equal. Byte-identity with the source file is explicitly not promised — change 2 diffs the two sides against each other and change 3 re-renders anyway.
- Determine requirement and scenario boundaries from markdown's real block structure, so a heading-shaped line inside a fenced code block is body content rather than a spurious requirement. opsxexplorer reads arbitrary OpenSpec repos, and specs elsewhere routinely fence example payloads.
- Tolerate a leading `## Purpose` section in a delta spec (three of the four archived changes in this repo have one) without treating it as a requirement section.
- Report malformed input (a scenario before any requirement, an unrecognised `## <OP> Requirements` header, a `- FROM:` with no matching `- TO:`) as a structured, displayable error rather than a panic or a silent drop.
- Add a loader that, for a given resolved change and capability, produces both the delta spec and the base spec together — reading the delta from the live working tree and the base through the change's resolved diff base.
- Handle the loader's real-world edge cases: a base spec file that does not exist (an all-ADDED delta introducing a brand-new capability), a change with no `specs/` directory at all, and a capability directory with no `spec.md`.
- Enumerate the capabilities a change touches, in stable alphabetical order, so change 3 can turn them into tabs.
- Extend `change-model` so both sides of a change's diff are reachable from an already-resolved change, closing the gap that makes an archived change's delta spec unreadable today.

## Capabilities

### New Capabilities

- `spec-model`: parsing OpenSpec spec markdown (main and delta) into requirements and scenarios, enumerating the capabilities a change touches, and loading the delta/base pair for a given change and capability — including the absent-base and no-specs cases.

### Modified Capabilities

- `change-model`: adds a requirement that both sides of a change's diff — the live view holding the change's own delta specs, and the resolved-diff-base view holding the spec of record — are reachable from a resolved change. Today only the diff base is exposed, which for an archived change is a commit predating the change directory, so the change's own delta specs cannot be read through it.

## Impact

- New `src/specs/` module: the parsed model (requirement, scenario, delta operation), the parser, the loader, and a parse/load error type.
- `src/changes/mod.rs`: `Changes` gains an accessor exposing the live view alongside the existing diff-base view (`open`). The existing `open(&ChangeView) -> Fs` and the "Resolved diff base travels with its change" requirement are preserved unchanged — this adds reach, it does not weaken the invariant that consumers never re-derive active/archived status.
- Adds `mdq` 0.10 as a dependency, used only inside `src/specs/parse.rs` — its `md_elem` module gives a hierarchical section tree that maps directly onto Requirements → Requirement → Scenario. No `mdq` type appears in the model, so the parser can be swapped later without touching consumers. `pulldown-cmark` stays reserved for change 3.
- No TUI changes. `src/tui/` is untouched by this change.
- Out of scope: all diffing (change 2, `spec-diff`), all rendering (change 3, `tui-specdiff`), reading `proposal.md`/`design.md`/`tasks.md`, and applying deltas onto main specs (that is `openspec archive`'s job).
