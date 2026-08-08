## ADDED Requirements

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
