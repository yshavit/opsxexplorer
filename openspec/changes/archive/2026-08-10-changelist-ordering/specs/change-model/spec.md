## MODIFIED Requirements

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
