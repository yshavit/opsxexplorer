## MODIFIED Requirements

### Requirement: A renamed requirement is reported as a rename, exactly once
The system SHALL report a rename as its own operation carrying both the former and the new requirement name, rather than as a modification whose name happens to differ, so that a rename is recognisable as a rename. A renamed requirement's content SHALL be compared against the spec of record's requirement under its former name, by the same rules that govern a modification. Where the same delta both renames a requirement and modifies it under its new name, the system SHALL report a single renamed entry carrying that content comparison, and SHALL NOT additionally report it as a modification nor treat the new name's absence from the spec of record as an error.

The former and new names SHALL themselves be compared as a single piece, using the same changed-vs-wholesale-replacement judgement applied to any other compared text (see "A piece whose two texts are too dissimilar to read inline is reported as a wholesale replacement"): reported as changed, carrying word-level runs, when the two names are similar enough for an inline reading to help, or as a wholesale replacement, carrying both full names and no runs, when they are not.

#### Scenario: rename alone

- **WHEN** a delta renames a requirement and says nothing else about it
- **THEN** a rename entry is reported carrying both names, and its content is compared against the spec of record's requirement under the former name

#### Scenario: rename combined with a modification

- **WHEN** a delta renames a requirement and also modifies it under its new name
- **THEN** a single rename entry is reported, carrying both names and the content comparison, with no separate modification entry and no missing-base error

#### Scenario: rename is not reported as a name diff

- **WHEN** a rename entry is reported
- **THEN** the former and new names are both available as names, not only as differing text within a modification

#### Scenario: similar names are reported as a changed piece

- **WHEN** a requirement is renamed and its former and new names are similar enough for an inline reading to help
- **THEN** the name comparison is reported as changed, carrying word-level runs over the two names

#### Scenario: dissimilar names are reported as a wholesale replacement

- **WHEN** a requirement is renamed and its former and new names are too dissimilar for an inline reading to help
- **THEN** the name comparison is reported as a wholesale replacement, carrying both full names and no runs
