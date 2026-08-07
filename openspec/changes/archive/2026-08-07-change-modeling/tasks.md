## 1. Module scaffolding

- [x] 1.1 Create a new `changes` module under `src/` (e.g. `src/changes/mod.rs`) and wire it into `main.rs` alongside `vfs`
- [x] 1.2 Define `Change` as a newtype tuple struct wrapping the on-disk directory basename: `Change(String)` — no other fields, no active/archived discriminant
- [x] 1.3 Implement a helper to derive a `Change`'s on-disk path: `workspace_root/openspec/changes/{value}` — uniform for active and archived, no branching, since an archived value already includes its `archive/` segment
- [x] 1.4 Implement a helper to derive an archived change's display name and archive date from its value by stripping the `archive/` and `${date}-` segments; an active change's value is already its display name

## 2. Change discovery

- [x] 2.1 Implement discovery of active changes by listing the changes directory (excluding the `archive` subdirectory) via `vfs::Workspace::current()`
- [x] 2.2 Implement discovery of archived changes by listing the changes archive directory, constructing each `Change` value by prefixing the entry's name with `archive/`
- [x] 2.3 Implement `Changes::discover(start: &Path) -> Result<Changes, ...>` that opens a `vfs::Workspace` and populates `active: Vec<Change>` and `archived: Vec<Change>`

## 3. Diff base resolution

- [x] 3.1 Define `DiffBase` enum: `Current` and `At(vfs::GitRef)`
- [x] 3.2 Define `ChangeView` struct pairing a `Change` with its resolved `DiffBase`
- [x] 3.3 Implement resolution for an active change: `DiffBase::Current`
- [x] 3.4 Implement resolution for an archived change: using its derived on-disk path (1.3), walk git history for that path and take the earliest (oldest) commit that introduced any file there — not the most recent commit touching the path — then resolve `DiffBase::At` to that commit's parent
- [x] 3.5 Surface a distinct, explicit error when that earliest commit can't be identified (do not fall back silently to HEAD or current)

## 4. Top-level model

- [x] 4.1 Define `Changes` struct owning a `vfs::Workspace` plus `active`/`archived` vecs
- [x] 4.2 Implement a method to produce a `ChangeView` for a given `Change` (dispatching to 3.3 or 3.4 based on which list it came from)
- [x] 4.3 Implement a method to resolve a `ChangeView`'s `DiffBase` into an `vfs::Fs` for reading (`Current` -> `workspace.current()`, `At(r)` -> `workspace.at(&r)`)

## 5. Tests

- [x] 5.1 Test: active change discovered from the changes directory, archived change discovered from the archive subdirectory
- [x] 5.2 Test: active change's `ChangeView` resolves to a live view reflecting an uncommitted spec-of-record edit
- [x] 5.3 Test: archived change's `ChangeView` resolves to the commit before the earliest commit that introduced its directory, excluding changes made in that earliest commit itself
- [x] 5.4 Test: when an archived change's directory is touched by more than one commit, the resolved diff base is anchored to the earliest one, not a later one
- [x] 5.5 Test: archived change's resolved diff base is unaffected by commits made after archiving
- [x] 5.6 Test: resolving a `ChangeView` twice (or holding it across simulated "frames") requires no re-lookup of active/archived status
