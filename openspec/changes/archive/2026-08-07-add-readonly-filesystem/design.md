## Context

See proposal.md - Why. opsxexplorer is a fresh scaffold: `git2` is already a dependency, but there is no filesystem-reading code yet. This capability is the foundation a later change (rendering per-requirement diffs between an active/archived change's spec and the spec of record) will read through — it needs "read file / list directory at HEAD," "read file / list directory at some other commit," and "read the live working tree," all through one interface.

## Goals / Non-Goals

**Goals:**
- One read-only interface backing both a plain-disk view and a git-tree view.
- Ref-based reads that are immutable/pinned once resolved, regardless of later branch movement or working-tree edits.
- A git backend simple enough that a virtual path and a git tree path are the same string.

**Non-Goals:**
- Gitignore-aware filtering of the working-tree view.
- Caching of resolved trees/blobs across repeated ref-views.
- Support for bare repositories (no working tree to back a current-state view).
- Any write access — this capability is read-only.
- Resolving or following symlinks encountered in a git tree.

## Decisions

### Interface shape: `Workspace` opens a root, `Fs` is the view you read from

```
Fs                        — read-only view: read(path), list_dir(path), exists(path)
                             one shape, two backends behind it:
                               - disk-backed (plain reads, escape-guarded)
                               - git-tree-backed (Tree/Blob reads, pinned to a GitRef)

Workspace                 — the opened root: holds the discovery result
  ::open(start: &Path)      (repo root + Repository handle, or plain disk root)
  .current() -> Fs           disk-backed Fs, rooted per root-discovery below
  .at(&GitRef) -> Result<Fs> git-tree-backed Fs; errors if not a git repo
```

`.current()` and `.at(ref)` both hand back the same `Fs` type, so callers elsewhere in the TUI never need to know which backend they got — they just call `.read()`/`.list_dir()`. That mirrors the requirement that current-state and ref-based views share a root and behave interchangeably to callers.

Both live in a `crate::vfs` module rather than `crate::fs`. This isn't about Rust correctness — `std::fs` is a module and `Fs` a type, so there's no compiler-level collision even with both in scope. It's about avoiding a token-level trap for whoever (human or LLM) implements this: the disk backend's own implementation needs to `use std::fs` internally to do real reads, so a `crate::fs::Fs` sitting next to `std::fs::read_to_string(...)` in the same file is exactly the setup where the wrong `fs` gets written or imported by reflex. `vfs` (virtual filesystem) sidesteps that while also signaling "this is a view over the filesystem, not the filesystem itself."

### Root discovery snaps to the repository root, not the requested path

When `git2::Repository::discover` finds an enclosing repository, the view roots itself at that repository's top-level directory rather than at the path the caller asked for. The requested path is only a discovery seed.

This removes the need for any prefix translation between "virtual path" and "git tree path" — they become identical strings. It also removes path-escape bookkeeping for the git backend entirely, since every valid git tree path is inside the repository by construction. It has one consequence worth naming: the current-state (working) view and the ref view end up sharing the same root when git is present, so a given virtual path means the same thing in both — which is also desirable on its own, since a caller comparing "this path now" to "this path at ref X" shouldn't have to reason about two different roots.

Alternative considered: keep a per-session path prefix so the exposed root matches exactly the subdirectory the caller asked for. Rejected — it adds join/escape arithmetic that's easy to get subtly wrong, in exchange for a capability (viewing a strict subtree of a repo) that opsxexplorer's actual use case doesn't need; it always wants full-repo context to resolve refs and history.

### Ref-based reads use git2's native Tree/Blob API, not worktrees

A resolved commit's `Tree` is walked in memory (`tree.get_path`, `tree.iter`/`tree.walk`) and file content comes from `Blob::content()`. Nothing is written to disk and the working directory is never touched.

Alternative considered: `git worktree add` a real checkout per ref, then treat it as a plain disk root. Rejected — it mutates repository state (registers a worktree, writes real files), needs explicit cleanup/pruning, and raises concurrency questions when multiple ref-views are open in the same session. It solves a problem (needing real filesystem semantics for something outside the process, e.g. handing paths to an external tool) that this capability doesn't have.

### `GitRef` is an eagerly-resolved struct, not a lazy revspec string

```
struct GitRef {
    oid: Oid,
}
```

Constructing a `GitRef` from a revspec (branch name, tag, short/long hash) resolves it immediately via `revparse_single` + `peel_to_commit` and stores only the resulting `Oid`. A `GitRef` therefore stays pinned to the exact commit it was built from even if, say, a branch it was resolved from later moves. This matters because the primary caller (the future diff-base-selection logic) resolves refs like "the commit immediately before this archive commit" once and needs that pin to hold for the life of the view.

It's a plain struct rather than a tuple newtype so the field has a name. Alternative considered: also carry the original revspec string for display. Rejected for now — per the project's actual usage, refs are always resolved by hash already (the caller says "this change was archived at commit abde, the one before it is 0123, look at 0123"), so the view layer has no independent need to remember how it was named. Nothing about this design prevents adding a label field later if that changes.

### Working-tree view does no git-aware filtering

The disk backend does plain reads with no consultation of git status. Untracked files show up because they're real content under the root; gitignored files also show up, because filtering them would require calling into the repository (`is_path_ignored`) for every entry, and that was deferred by choice rather than discovered as a gap later.

## Risks / Trade-offs

- [Gitignored files appear in the working-tree view] → Acceptable for this change; documented behavior. Follow-up: file a GitHub issue for `is_path_ignored`-based filtering.
- [No caching of resolved trees/blobs] → Repeated navigation to the same ref/path re-walks the tree each time. Acceptable at interactive TUI scale. Follow-up: file a GitHub issue to revisit if profiling shows it matters.
- [Bare repositories have no working tree] → Out of scope for this change; behavior when discovery lands on a bare repo is undefined. Follow-up: file a GitHub issue to decide (e.g. ref-views only, working-view returns an error) if/when it comes up.
- [Symlink tree entries are rejected outright] → Acceptable since they're unlikely in OpenSpec content (proposals/specs/design/tasks are plain markdown). Can be upgraded later (e.g. return the link target as text) without breaking the interface, since it still returns a result for the path, just different content.

## Migration Plan

N/A — this is a new, additive module with no existing behavior to migrate or roll back.
