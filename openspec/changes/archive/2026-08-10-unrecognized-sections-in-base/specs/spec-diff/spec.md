## RENAMED Requirements

- FROM: `### Requirement: A capability's diff carries the delta's unrecognised sections through`
- TO: `### Requirement: A capability's diff carries its unrecognised sections through, kept separate by origin`

## MODIFIED Requirements

### Requirement: A capability's diff carries its unrecognised sections through, kept separate by origin
The comparison the system produces for a capability SHALL carry the unrecognised sections its delta was parsed with and the unrecognised sections its spec of record was parsed with as two separate ordered lists — never merged into one — each preserving the order its own document carried them in, and each entry carrying both the section's title and its body. This makes a capability's diff a superset of everything the pane needs to render it, including content the tool did not understand well enough to diff, while still letting a consumer tell a section that came from the change itself apart from one that was already sitting in the spec of record. When a capability has no spec of record at all, its base-sourced list SHALL be empty. The system SHALL NOT interpret, validate, or otherwise act on an unrecognised section beyond carrying it through.

#### Scenario: delta with unrecognised sections
- **WHEN** a capability's delta was parsed with one or more unrecognised sections
- **THEN** the capability's diff carries those same sections, in the same order, in its delta-sourced list

#### Scenario: spec of record with unrecognised sections
- **WHEN** a capability's spec of record was parsed with one or more unrecognised sections
- **THEN** the capability's diff carries those same sections, in the same order, in its base-sourced list

#### Scenario: delta with no unrecognised sections
- **WHEN** a capability's delta was parsed with no unrecognised sections
- **THEN** its delta-sourced list is empty

#### Scenario: no spec of record at all
- **WHEN** a capability has no spec of record to compare against
- **THEN** its base-sourced list is empty
