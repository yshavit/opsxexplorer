# change-model Specification

## Purpose

Models which OpenSpec changes exist (active and archived) and determines, for each one, the correct state of the spec of record to diff its delta specs against.

## Requirements

### Requirement: Change discovery
The system SHALL discover active changes from the changes directory and archived changes from the changes archive directory, and SHALL treat a change's active/archived status as determined by which of the two it was found in, not as separately recorded state.

#### Scenario: active change discovered
- **WHEN** a change directory exists directly under the changes directory (not under the archive subdirectory)
- **THEN** it is discovered as an active change

#### Scenario: archived change discovered
- **WHEN** a change directory exists under the changes archive directory
- **THEN** it is discovered as an archived change

### Requirement: Diff base for an active change is the live spec of record
For an active change, the system SHALL resolve the diff base to the current, live state of the spec of record, including any uncommitted edits.

#### Scenario: uncommitted spec edit reflected
- **WHEN** the spec of record has an uncommitted edit on disk
- **AND** the diff base is resolved for an active change touching that capability
- **THEN** the resolved diff base reflects the uncommitted edit, not the last-committed content

### Requirement: Diff base for an archived change is the commit before the directory first appears
For an archived change, the system SHALL resolve the diff base to the state of the spec of record as of the commit immediately preceding the earliest commit that introduced any file under that change's archived directory — not any later commit that also touched that directory, and not the current state of the spec of record.

"Earliest" is defined by ancestry, not committer timestamp: it is the introducing commit with no ancestor that also introduces the directory, found by a traversal ordered so that every commit is visited after all of its ancestors. Among commits that are not ancestrally ordered relative to each other (parallel branches), committer timestamp breaks ties. A commit's own committer timestamp SHALL NOT by itself determine which commit is "earliest" when doing so would contradict ancestry.

#### Scenario: diff base excludes the earliest commit's own changes
- **WHEN** the earliest commit that introduced the archived change's directory also modified the spec of record
- **THEN** the resolved diff base for that change does not include those modifications

#### Scenario: diff base ignores later commits that also touched the archived directory
- **WHEN** the archived change's directory is touched by more than one commit over its history (e.g. files added across a few commits, or edited after archiving)
- **THEN** the resolved diff base is determined by the earliest of those commits, not any later one

#### Scenario: diff base unaffected by later history
- **WHEN** the spec of record is modified in commits after a change was archived
- **THEN** the resolved diff base for that archived change is unchanged by those later commits

#### Scenario: ancestry wins over non-monotonic committer time
- **WHEN** a commit that introduces an archived change's directory has a descendant commit with an earlier committer timestamp than its own (e.g. clock skew or a rewritten `GIT_COMMITTER_DATE`)
- **THEN** the resolved diff base is still based on the introducing commit itself, not on the descendant with the earlier timestamp

### Requirement: Resolved diff base travels with its change
Once a change's diff base has been resolved, the system SHALL make the change and its resolved diff base available together, so that consumers do not need to re-determine whether the change is active or archived to use it.

#### Scenario: resolved pair used without re-checking status
- **WHEN** a change's diff base has already been resolved
- **THEN** reading the spec of record through that resolved diff base requires no separate lookup of the change's active/archived status

### Requirement: Both sides of a change's diff are reachable from the resolved change
Once a change's diff base has been resolved, the system SHALL make both views needed to diff that change reachable from it: the view holding the change's own delta specs, which is always the live working tree, and the view holding the spec of record, which is the resolved diff base. Obtaining the pair SHALL NOT require the consumer to know or re-check whether the change is active or archived, and SHALL NOT require re-deriving either view independently.

#### Scenario: archived change's own delta specs are readable
- **WHEN** both views are obtained for an archived change, whose resolved diff base is a commit predating the existence of its change directory
- **THEN** the change's delta specs are readable through the live view, even though they do not exist in the diff base view

#### Scenario: spec of record still read at the resolved diff base
- **WHEN** both views are obtained for an archived change
- **THEN** the spec of record read through the diff base view is its state at that resolved diff base, not its current state

#### Scenario: active change's two views are both live
- **WHEN** both views are obtained for an active change
- **THEN** the delta specs and the spec of record are both read from the live working tree, reflecting any uncommitted edits

#### Scenario: pair obtained without re-checking status
- **WHEN** a change's diff base has already been resolved
- **THEN** obtaining both views requires no separate lookup of the change's active/archived status

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
