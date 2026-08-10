## MODIFIED Requirements

### Requirement: Delta requirements are tagged with the operation that introduced them
The system SHALL recognise the four delta operation sections — added, modified, removed, and renamed requirements — and SHALL tag every parsed delta requirement with the operation section it appeared under, so that consumers can tell an addition from a modification without re-reading the source. A removed entry SHALL be parsed the same way an added or modified entry is: its heading supplies its name, and any body content following the heading — conventionally a **Reason** and **Migration** explanation, since OpenSpec's authoring convention requires both on a removal — SHALL be captured as its intro, exactly as an added or modified entry's body would be. A removed entry is not expected to carry scenarios of its own, but nothing in the document's structure forbids one, and a `#### Scenario:` heading under a removed entry SHALL be parsed like any other scenario rather than rejected. A renamed entry SHALL be parsed as a pairing of the name it is being renamed from with the name it is being renamed to.

#### Scenario: added and modified requirements are distinguished
- **WHEN** a delta spec contains both an added-requirements section and a modified-requirements section
- **THEN** each parsed requirement is tagged with the section it came from

#### Scenario: removed entry's body becomes its intro
- **WHEN** a delta spec's removed-requirements section names a requirement and follows the heading with body text, such as a Reason and Migration explanation
- **THEN** it parses as a removal carrying that name and that body text as its intro

#### Scenario: removed entry carries no body
- **WHEN** a delta spec's removed-requirements section names a requirement with no body text following the heading
- **THEN** it parses as a removal carrying that name and an empty intro

#### Scenario: renamed entry pairs the old and new names
- **WHEN** a delta spec's renamed-requirements section pairs a from-name with a to-name
- **THEN** it parses as a rename carrying both names

#### Scenario: modified requirement listing only some scenarios
- **WHEN** a modified requirement lists fewer scenarios than the corresponding requirement in the spec of record
- **THEN** it parses with exactly the scenarios it lists, and the omission is not treated as an error or as a deletion of the unlisted scenarios

#### Scenario: delta opening with a purpose section
- **WHEN** a delta spec begins with a `## Purpose` section before any operation section
- **THEN** the document parses successfully and the purpose section is not treated as a requirement section
