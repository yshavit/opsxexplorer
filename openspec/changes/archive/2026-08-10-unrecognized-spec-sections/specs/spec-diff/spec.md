## ADDED Requirements

### Requirement: A capability's diff carries the delta's unrecognised sections through
The comparison the system produces for a capability SHALL carry the same unrecognised section titles its delta was parsed with, unchanged and in the same order, so that a capability's diff is a superset of everything the pane needs to render it — including content the tool did not understand well enough to diff. The system SHALL NOT interpret, validate, or otherwise act on an unrecognised section title beyond carrying it through.

#### Scenario: delta with unrecognised sections
- **WHEN** a capability's delta was parsed with one or more unrecognised section titles
- **THEN** the capability's diff carries those same titles, in the same order

#### Scenario: delta with no unrecognised sections
- **WHEN** a capability's delta was parsed with no unrecognised section titles
- **THEN** the capability's diff carries none either
