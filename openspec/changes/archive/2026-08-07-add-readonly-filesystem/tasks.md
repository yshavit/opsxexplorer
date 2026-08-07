## 1. View interface

- [x] 1.1 Define the read-only view interface (read file, list directory, exists), with error types covering "not found," "escapes root," and "unsupported entry" (e.g. symlink)
- [x] 1.2 Wire the new module into the crate

## 2. Root discovery

- [x] 2.1 Implement discovery: given a starting path, use `git2::Repository::discover` to find an enclosing repository
- [x] 2.2 When a repository is found, resolve the root to the repository's working directory (not the starting path)
- [x] 2.3 When no repository is found, resolve the root to the canonicalized starting path
- [x] 2.4 Unit tests for both branches, including a starting path nested a few levels inside a repo

## 3. Disk-rooted backend

- [x] 3.1 Implement plain-disk read/list-directory/exists rooted at a given root
- [x] 3.2 Guard against path traversal: reject resolved paths that escape the configured root
- [x] 3.3 Unit tests: reading a file, listing a directory, traversal (`..`) rejected

## 4. Git-backed working-tree view

- [x] 4.1 Root the working-tree view at the repository root using the disk-rooted backend from section 3
- [x] 4.2 Unit tests: uncommitted edits visible, untracked files visible, gitignored files visible (documenting the known gap)

## 5. Git-backed ref view

- [x] 5.1 Implement `GitRef { oid: Oid }` with a constructor that resolves a revspec via `revparse_single` + `peel_to_commit` and stores only the resulting `Oid`
- [x] 5.2 Implement ref-based file reads via `Tree::get_path` + `Blob::content` (no working-directory access)
- [x] 5.3 Implement ref-based directory listing via tree entry iteration
- [x] 5.4 Return an error for symlink tree entries instead of resolving them
- [x] 5.5 Unit tests: ref view unaffected by later working-tree edits, ref view unaffected by a branch subsequently moving, directory listing at a ref, symlink entry returns an error

## 6. Follow-up tracking

- [x] 6.1 File a GitHub issue for gitignore-aware filtering of the working-tree view (documents the known gap that gitignored files currently appear in the view) — https://github.com/yshavit/opsxexplorer/issues/1
- [x] 6.2 File a GitHub issue for tree/blob caching across repeated ref-views — https://github.com/yshavit/opsxexplorer/issues/2
- [x] 6.3 File a GitHub issue for bare-repository handling (no working tree to back a current-state view) — https://github.com/yshavit/opsxexplorer/issues/3
