## MODIFIED Requirements

### Requirement: A removed requirement's content is recovered from the spec of record
A delta's removal entry names a requirement without restating its content in full, but the user is shown what is being removed. The system SHALL take a removed requirement's intro and scenarios from the spec of record and SHALL report all of them as pure deletions, in the spec of record's order. A removal entry's own body — its removal note, if it has one (see "A removed requirement's own body is carried through as a removal note") — has no counterpart in the spec of record and SHALL NOT alter this comparison.

#### Scenario: removal shows the base's content

- **WHEN** a delta removes a requirement that the spec of record defines with an intro and several scenarios
- **THEN** that intro and those scenarios are reported as deleted content, taken from the spec of record

#### Scenario: removal reports no content of its own

- **WHEN** a removal entry is compared
- **THEN** nothing from the delta entry's own body contributes to the reported intro and scenario content — that comparison is still a pure deletion of the spec of record's content, even when the removal entry carries its own body text (see "A removed requirement's own body is carried through as a removal note")

#### Scenario: removed scenarios follow the spec of record's order

- **WHEN** a delta removes a requirement whose spec of record scenarios are all in the deleted category alike
- **THEN** those scenarios are reported in the spec of record's order

## ADDED Requirements

### Requirement: A removed requirement's own body is carried through as a removal note
A delta's removal entry may carry its own body text — conventionally a **Reason** and **Migration** explanation for why the requirement was removed and what replaces it. That text has no counterpart in the spec of record to compare it against, so it is not one of the five `Piece` states used for the requirement's intro and scenarios. The system SHALL instead carry it through as the removed requirement's own removal note, reported separately from those `Piece` comparisons. When a removal entry's own body is empty, the system SHALL report no removal note for that requirement, consistent with how an empty intro elsewhere is treated as absent rather than as emptied content.

#### Scenario: a removal note is reported

- **WHEN** a delta's removal entry carries its own body text
- **THEN** that text is reported as the removed requirement's removal note

#### Scenario: a bare removal carries no note

- **WHEN** a delta's removal entry carries no body text of its own
- **THEN** no removal note is reported for that requirement

#### Scenario: the removal note is not a diffed piece

- **WHEN** a removed requirement's removal note is reported
- **THEN** it is carried as its own value, distinguishable from the `Piece` states reported for that requirement's intro and scenarios
