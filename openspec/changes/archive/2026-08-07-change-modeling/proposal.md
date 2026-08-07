## Why

opsxexplorer can already read files at a disk-or-git-tree location (the `filesystem` capability), but it has no in-memory model of *which* OpenSpec changes exist, whether each is active or archived, or what the correct diff base is for each one. That model is the next building block before any TUI rendering can happen: the UI needs to list changes and, for each, resolve the right base to diff its delta specs against (live spec-of-record for active changes, the historical spec-of-record snapshot from just before archiving for archived ones).

## What Changes

- Introduce `Change`: a newtype wrapping a change's path relative to `openspec/changes/` — e.g. `Change("change-modeling")` for an active change, or `Change("archive/2026-08-07-add-readonly-filesystem")` for an archived one — `Change(String)`, no other fields. An archived value naturally includes the `archive/` segment, so the on-disk path is always `openspec/changes/{value}`, active or archived, with no branching. The display name and archive date, for archived changes, are derived on demand by stripping the `archive/` and `${date}-` segments from the value.
- Introduce `DiffBase`: an enum describing how to resolve a change's diff base — `Current` (the live disk view, for active changes) or `At(GitRef)` (a specific pinned commit, for archived changes — the commit immediately before the one that archived the change). Mirrors the existing `Fs::Disk` / `Fs::Git` split.
- Introduce `ChangeView`: pairs a `Change` with its resolved `DiffBase` so the two travel together once resolved, without callers needing to remember which list a `Change` came from. `ChangeView` holds no borrowed data (no lifetime), so it can be freely stored and passed around.
- Introduce `Changes`: the top-level model. Owns a `vfs::Workspace` and holds `active: Vec<Change>` and `archived: Vec<Change>`. Provides discovery (scan `openspec/changes/*` for active, `openspec/changes/archive/*` for archived) and resolution (produce a `ChangeView` for a given `Change`, including the git-history walk needed to find an archived change's pre-archive commit).
- No TUI wiring in this change — `main.rs` stays as-is. Rendering this model in ratatui is a follow-up change.

## Capabilities

### New Capabilities
- `change-model`: in-memory model of active/archived OpenSpec changes and diff-base resolution, built on top of the `filesystem` capability.

### Modified Capabilities
(none — `filesystem` is consumed as-is, no requirement changes)

## Impact

- New Rust module(s) under `src/` for `Change`, `DiffBase`, `ChangeView`, `Changes`.
- Depends on the existing `vfs` module (`Workspace`, `Fs`, `GitRef`) and `git2` for walking history to find the commit before an archiving commit.
- No changes to `vfs`'s public API.
- No changes to `main.rs` / no ratatui usage yet.
