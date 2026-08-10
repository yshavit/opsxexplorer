## ADDED Requirements

### Requirement: Diff bases for archived changes are resolved during discovery

The system SHALL determine the diff base for every archived change it discovers as part of discovery itself, rather than when a diff base is first asked for. Obtaining the diff base of an already-discovered change SHALL NOT consult repository history, and SHALL NOT become more expensive as the number of archived changes or the depth of history grows.

This requirement constrains when the work happens, not which commit is chosen. The commit selected for each change is unchanged: it remains the one immediately preceding the earliest commit that introduced any file under that change's archived directory.

#### Scenario: obtaining a diff base after discovery reads no history

- **WHEN** changes have been discovered
- **AND** the diff base of a discovered archived change is obtained
- **THEN** the diff base is produced without traversing repository history

#### Scenario: repeated resolution is consistent

- **WHEN** the diff base of the same discovered archived change is obtained more than once
- **THEN** every one of those results is the same commit

#### Scenario: resolution agrees with per-change derivation

- **WHEN** a repository contains several archived changes whose directories were introduced by different commits, interleaved with commits that touch neither
- **THEN** each change's resolved diff base is the same commit that deriving that change's diff base on its own would select

### Requirement: Archived change resolution reflects history as of discovery

The system SHALL resolve archived changes against the repository as it stood when those changes were discovered. Commits, rewrites, or other history changes made afterwards SHALL NOT alter the resolved diff base of an already-discovered change, for as long as that discovery result is in use. This holds equally for a change whose introducing commit could not be resolved at discovery: it SHALL remain unresolvable, and SHALL continue to sort and behave as an archived change with no resolvable introducing commit.

Discovery already fixes which changes exist and whether each is active or archived; this extends the same snapshot boundary to the history each archived change is resolved against, so a single discovery result is internally consistent rather than partly live.

#### Scenario: later commits do not move a resolved diff base

- **WHEN** an archived change has been discovered and its diff base resolved
- **AND** further commits are then made to the repository, including commits that modify the spec of record
- **THEN** that change's diff base is still the same commit as before those commits were made

#### Scenario: unresolvable at discovery stays unresolvable

- **WHEN** an archived change's directory is present but uncommitted at the time changes are discovered
- **AND** that directory is subsequently committed
- **THEN** the change discovered earlier still has no resolvable introducing commit
- **AND** it still appears in the archived list, ordered as a change whose introducing commit cannot be resolved

#### Scenario: a fresh discovery observes the newer history

- **WHEN** the repository's history has changed since an earlier discovery
- **AND** changes are discovered again
- **THEN** the new discovery result resolves archived changes against the newer history
