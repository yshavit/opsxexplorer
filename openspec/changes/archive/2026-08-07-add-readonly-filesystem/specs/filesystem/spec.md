## Purpose

Gives opsxexplorer a single, read-only way to read files and list directories rooted at a given location, whether that location is a plain directory or a git repository — and, for a git repository, whether the caller wants the live working tree or the state as of a specific commit.

## ADDED Requirements

### Requirement: Root discovery
When given a starting path, the system SHALL determine whether the path is inside a git repository. If it is, the resulting view SHALL be rooted at the repository's top-level directory. If it is not, the resulting view SHALL be rooted exactly at the given path.

#### Scenario: starting path inside a git repo
- **WHEN** a starting path inside a git repository is provided
- **THEN** the resulting view is rooted at the repository's top-level directory, not the starting path

#### Scenario: starting path outside any git repo
- **WHEN** a starting path with no enclosing git repository is provided
- **THEN** the resulting view is rooted exactly at the given path

### Requirement: Current-state view reflects the live working tree
The system SHALL provide a view of the current on-disk state of the root, including uncommitted and unstaged changes and untracked files.

#### Scenario: uncommitted edit visible
- **WHEN** a tracked file has been modified on disk but not committed
- **THEN** reading that file through the current-state view returns the on-disk content, not the last-committed content

#### Scenario: untracked file visible
- **WHEN** a file exists on disk under the root but is not tracked by git and not ignored
- **THEN** it appears in directory listings and can be read through the current-state view

#### Scenario: gitignored file visible
- **WHEN** a file exists on disk under the root and matches a gitignore pattern
- **THEN** it still appears in the current-state view, since gitignore-aware filtering is not part of this capability

### Requirement: Ref-based view reflects a specific commit
The system SHALL provide a view of the root's contents as recorded in a specific, previously-resolved git commit, independent of the current working tree state.

#### Scenario: ref view unaffected by working tree edits
- **WHEN** a file is modified on disk after a ref-based view has been opened for a commit that predates the edit
- **THEN** reading that file through the ref-based view returns the content as recorded in that commit, not the on-disk content

#### Scenario: ref view unaffected by branch movement
- **WHEN** a ref-based view has been resolved to a specific commit and a branch reference is later moved to point elsewhere
- **THEN** the already-resolved view continues to reflect the original commit

#### Scenario: listing a directory at a ref
- **WHEN** a directory path that exists in the resolved commit's tree is listed through a ref-based view
- **THEN** the entries recorded in that commit's tree are returned

### Requirement: Path access is confined to the configured root
For a view rooted at a plain filesystem path with no enclosing git repository, the system SHALL prevent access to paths outside that root.

#### Scenario: traversal outside a disk-rooted view is rejected
- **WHEN** a path that resolves outside the configured root (for example, via `..` segments) is requested from a disk-rooted view
- **THEN** the system returns an error rather than the file's content

### Requirement: Unsupported tree entries produce an error
When a ref-based view encounters a tree entry it does not support (a symbolic link), the system SHALL return an error for that entry rather than resolving or misinterpreting it.

#### Scenario: reading a symlink entry at a ref
- **WHEN** a path in a resolved commit's tree corresponds to a symbolic link entry
- **THEN** reading that path through the ref-based view returns an error
