## ADDED Requirements

### Requirement: A capability's purpose is compared against the spec of record, using the same changed-vs-replaced judgement as any other piece
Given a change's delta for one capability and the corresponding spec of record, the system SHALL compare the delta's purpose section (if any) against the spec of record's purpose section (if any), applying the same rule already applied to a requirement's intro: presence in the delta is authoritative. Comparison against the spec of record's purpose SHALL treat an absent purpose section — whether because the capability has no spec of record at all or because its spec of record carries no purpose section — as empty text.

The system SHALL report a purpose comparison only for two outcomes: an insertion, when the spec of record's purpose is empty and the delta's purpose is not; or a comparison — changed or replaced, by the same legibility judgement applied to any other piece — when both are non-empty and differ. The system SHALL report no purpose comparison at all — not an unchanged result, not an absent-but-marked result — when the delta carries no purpose section, or when the delta's purpose section is equal to the spec of record's purpose.

#### Scenario: delta adds a purpose to a capability with none
- **WHEN** a delta's purpose section is non-empty and the spec of record has no purpose (or no spec of record at all)
- **THEN** the purpose comparison is reported as an insertion, carrying the delta's text

#### Scenario: delta changes an existing purpose
- **WHEN** a delta's purpose section differs from the spec of record's purpose
- **THEN** the purpose comparison is reported as changed or replaced, by the same measure used for any other piece

#### Scenario: delta has no purpose section
- **WHEN** a delta carries no purpose section
- **THEN** no purpose comparison is reported for that capability

#### Scenario: delta restates the purpose unchanged
- **WHEN** a delta's purpose section is equal to the spec of record's purpose
- **THEN** no purpose comparison is reported for that capability

#### Scenario: repeated comparison is stable
- **WHEN** the same delta and spec of record are compared twice
- **THEN** the purpose comparison, or its absence, is the same both times
