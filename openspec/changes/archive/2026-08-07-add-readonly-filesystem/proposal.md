## Why

opsxexplorer needs to read files two different ways: the live working tree of a repo (including uncommitted/unstaged edits), and a repo's state as it was at an arbitrary historical commit (e.g. the spec of record immediately before a change was archived). Nothing in the codebase yet abstracts "read a file/list a directory rooted at some path" in a way that can be backed by either the plain filesystem or a git commit's tree, so the diff engine (a later change) has no read path to build on. This change adds that abstraction first.

## What Changes

- Add a small read-only view interface (read file, list directory, exists) with two backends: plain-disk reads, and git-tree/blob reads via `git2`.
- Given a starting path, discover whether it's inside a git repo (`git2::Repository::discover`). If so, root the view at the repository root (the requested path is only used to locate the repo, not as a view boundary) and expose both a working-tree view (current state, including uncommitted/unstaged changes) and ref-based views. If not, root a plain read-only disk view at the requested path.
- Add a `GitRef` struct wrapping a resolved `Oid` (resolved once via `revparse_single` + `peel_to_commit`, so a ref view stays pinned even if a branch name it was resolved from later moves).
- Ref-based reads walk the commit's tree/blob objects natively via `git2` — no worktrees, no working-directory mutation.
- Path traversal is guarded for the plain-disk backend (resolved paths cannot escape the configured root).
- Symlink entries encountered in a git tree return an error rather than being resolved.
- Untracked files are visible in the working-tree view; gitignored files are visible too for this change (not filtered) — documented as a known gap.

## Capabilities

### New Capabilities
- `filesystem`: read-only access to a root's contents, either the live filesystem/working tree or a specific git commit's tree, behind one interface.

### Modified Capabilities
(none — first capability in the project)

## Impact

- New Rust module(s) in `src/` implementing the view interface, disk backend, and git backend (uses the existing `git2` dependency; no new dependencies expected).
- Establishes the read path the future diff-rendering change will build on (not implemented in this change).
- Deferred, to be filed as follow-up GitHub issues rather than solved here: gitignore filtering of the working-tree view, tree/blob caching across repeated ref-views, and support for bare repositories (no working tree).
