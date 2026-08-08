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

#### Scenario: diff base excludes the earliest commit's own changes
- **WHEN** the earliest commit that introduced the archived change's directory also modified the spec of record
- **THEN** the resolved diff base for that change does not include those modifications

#### Scenario: diff base ignores later commits that also touched the archived directory
- **WHEN** the archived change's directory is touched by more than one commit over its history (e.g. files added across a few commits, or edited after archiving)
- **THEN** the resolved diff base is determined by the earliest of those commits, not any later one

#### Scenario: diff base unaffected by later history
- **WHEN** the spec of record is modified in commits after a change was archived
- **THEN** the resolved diff base for that archived change is unchanged by those later commits

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
