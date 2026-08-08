# Brief: `spec-model` capability — parse and load OpenSpec spec files

> Chain position: **1 of 3**. Read → compare → render.
> 1. **`spec-model`** (this file) — parse a spec.md into requirements/scenarios; load both sides for a change.
> 2. `spec-diff` (`02-spec-diff.md`) — compare a delta requirement against its base.
> 3. `tui-specdiff` (`03-tui-specdiff.md`) — render it in the right pane.

This brief has been implemented and archived. The source of truth is now
`openspec/changes/archive/2026-08-08-spec-model/` (proposal.md, specs/,
design.md, tasks.md) and the resulting main specs at
`openspec/specs/spec-model/spec.md` and `openspec/specs/change-model/spec.md`.

`notes/spec-diff/` is not deleted yet — that happens only once all three
changes in the chain (`spec-model`, `spec-diff`, `tui-specdiff`) have landed.
