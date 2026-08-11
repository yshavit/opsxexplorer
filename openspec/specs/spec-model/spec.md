# spec-model Specification

## Purpose

Turns OpenSpec spec markdown — both a capability's spec of record and a change's delta spec — into a structured model of requirements and scenarios, and loads both sides of a given change-and-capability pair so they can be compared.

## Requirements

### Requirement: Spec markdown parses into requirements and scenarios
The system SHALL parse a spec document into an ordered list of requirements, each carrying its name, its intro block (the content between the requirement's heading and its first scenario), and its ordered list of scenarios. Each scenario SHALL carry its name and its body. A requirement's name SHALL be the text following its `### Requirement: ` heading and a scenario's name the text following its `#### Scenario: ` heading, and requirement names SHALL serve as the identity by which a delta requirement is matched to a base requirement. A spec of record and a delta spec SHALL parse into the same requirement and scenario shapes, so that consumers can treat the two sides uniformly.

#### Scenario: requirement with intro and scenarios
- **WHEN** a spec contains a requirement whose heading is followed by intro prose and then one or more scenario headings
- **THEN** the parsed requirement carries that name, that intro block, and those scenarios in document order

#### Scenario: requirement names identify requirements across sides
- **WHEN** a delta requirement and a base requirement carry the same requirement name
- **THEN** they are identified as the same requirement, without reference to their position in either document

#### Scenario: a capability's spec of record parses
- **WHEN** a spec of record with a `## Requirements` section is parsed
- **THEN** its requirements and scenarios are produced in the same shapes a delta spec produces

#### Scenario: spec with no requirements
- **WHEN** a spec document contains no requirement headings
- **THEN** it parses successfully into an empty list of requirements, not an error

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

### Requirement: Body content is preserved in full and normalised identically on both sides
The system SHALL carry intro and scenario body content through parsing without losing authored content: markdown constructs such as bullet lists, emphasis, inline code, code blocks, and tables SHALL survive, and no body element SHALL be dropped. Body content MAY be normalised in its incidental formatting, but the same normalisation SHALL be applied to every spec the system parses, so that two requirements with equivalent source content always yield equal body content. Bodies SHALL NOT be re-wrapped or re-flowed to any line width.

The system is NOT required to reproduce body content byte-for-byte from the source. Consumers compare the two sides against each other rather than against the file, so identical treatment of both sides — not byte identity — is what the contract guarantees.

#### Scenario: markdown constructs survive
- **WHEN** a scenario body contains bullet lines with bold markers and inline code
- **THEN** the parsed body still contains those bullets and their inline markup, with no content dropped

#### Scenario: equivalent content on both sides compares equal
- **WHEN** a requirement's source content is identical in a delta spec and in the spec of record
- **THEN** the two parsed bodies are equal, with no incidental difference introduced by parsing

#### Scenario: long lines are not re-wrapped
- **WHEN** a spec contains a paragraph hundreds of characters long
- **THEN** the parsed body does not break it across multiple lines

#### Scenario: multi-paragraph intro is preserved
- **WHEN** a requirement's intro block spans more than one paragraph
- **THEN** all of its paragraphs are present in the parsed intro, still separated as distinct paragraphs

### Requirement: Markdown block structure is respected when locating requirements
The system SHALL determine requirement and scenario boundaries according to markdown's block structure, not by matching line prefixes. A heading-shaped line that markdown does not treat as a heading — because it sits inside a fenced or indented code block, an HTML block, or another container — SHALL NOT be treated as a requirement or scenario heading.

#### Scenario: heading-shaped line inside a code block
- **WHEN** a scenario body contains a fenced code block whose content includes a line reading `### Requirement: Example`
- **THEN** that line is part of the scenario's body and does not begin a new requirement

#### Scenario: code block content is not split into scenarios
- **WHEN** a requirement's intro contains a fenced code block that includes a line reading `#### Scenario: Example`
- **THEN** no scenario is created from that line

### Requirement: Malformed spec content is reported as a structured error
The system SHALL report malformed spec content as a structured error that identifies what was malformed and where, in a form that can be displayed to the user. It SHALL NOT crash, and SHALL NOT silently discard content it cannot interpret. An unrecognised `##` section — in either a delta spec or a spec of record — is not malformed content under this requirement; see "Unrecognised sections are collected without failing the parse". Every other case below still applies to both a spec of record and a delta spec.

#### Scenario: scenario appearing before any requirement
- **WHEN** a spec contains a scenario heading before any requirement heading
- **THEN** a structured error identifying the stray scenario is reported

#### Scenario: rename missing its target
- **WHEN** a renamed-requirements section supplies a from-name with no matching to-name
- **THEN** a structured error identifying the incomplete rename is reported

#### Scenario: unparseable content is never dropped silently
- **WHEN** content in a spec document cannot be interpreted
- **THEN** the outcome is a reported error, never a successful parse that omits that content

#### Scenario: unrecognised operation section
- **WHEN** a spec of record contains a `##` section whose title is neither `Purpose` nor `Requirements`
- **THEN** no structured error is reported for that section alone — see "Unrecognised sections are collected without failing the parse"

### Requirement: Unrecognised sections are collected without failing the parse
A `##` section whose title is not one of the ones the system already recognises — for a delta spec, `Purpose` or one of the four operation sections `ADDED`/`MODIFIED`/`REMOVED`/`RENAMED Requirements`; for a spec of record, `Purpose` or `Requirements` — is not itself malformed: neither document format has a closed list of section titles it is allowed to carry, and content the tool does not recognise is not evidence the file is broken. The system SHALL therefore continue parsing the rest of the document after encountering such a section, and SHALL record both that section's title and its rendered body — the same rendered-body content the system produces for a requirement's intro — so the caller can show what the tool did not understand rather than only that something was hidden. Titles and bodies SHALL be collected in the order their sections appear in the document. This requirement applies identically to a spec of record and to a delta spec; the two are never merged together, since each is read from a different document and a consumer needs to be able to tell them apart.

#### Scenario: one unrecognised section among well-formed ones
- **WHEN** a delta spec contains a `## ADDED Requirements` section and one `##` section whose title is not recognised
- **THEN** the delta parses successfully, with the added requirement present and the unrecognised section's title and body recorded

#### Scenario: an unrecognised section in a spec of record
- **WHEN** a spec of record contains a `##` section whose title is neither `Purpose` nor `Requirements`
- **THEN** the spec of record parses successfully, with its requirements present and the unrecognised section's title and body recorded

#### Scenario: several unrecognised sections
- **WHEN** a spec document contains more than one `##` section whose title is not recognised
- **THEN** all of their titles and bodies are recorded, in the order they appear in the document

#### Scenario: no unrecognised sections
- **WHEN** every `##` section in a spec document is one the system already recognises
- **THEN** no unrecognised sections are recorded

### Requirement: The capabilities a change touches are enumerated in stable order
The system SHALL determine which capabilities a change carries delta specs for, and SHALL return them in a stable alphabetical order that does not vary between runs over unchanged inputs.

#### Scenario: change touching several capabilities
- **WHEN** a change carries delta specs for more than one capability
- **THEN** all of those capabilities are enumerated, in alphabetical order

#### Scenario: order is stable across runs
- **WHEN** the same change is enumerated more than once with no intervening file changes
- **THEN** the capabilities are returned in the same order each time

#### Scenario: change with no spec deltas
- **WHEN** a change carries no spec deltas at all
- **THEN** the enumeration is empty and no error is reported, so the change can be presented as having no spec changes

### Requirement: Both sides of a change-and-capability pair load together
For a given resolved change and one of its capabilities, the system SHALL load both the change's delta spec and the corresponding spec of record, and SHALL make them available together. The delta spec SHALL be read from the live working tree, and the spec of record SHALL be read at the change's resolved diff base — including for an archived change, whose diff base precedes the existence of its own change directory. The loaded spec of record SHALL be the complete parsed base, so that content a delta names but does not restate — such as the intro and scenarios of a removed requirement — remains recoverable from it.

#### Scenario: archived change's delta and base both load
- **WHEN** both sides are loaded for an archived change and one of its capabilities
- **THEN** the delta spec is read from the live working tree and the spec of record is read as of that change's resolved diff base

#### Scenario: active change's delta and base both load
- **WHEN** both sides are loaded for an active change and one of its capabilities
- **THEN** the delta spec and the spec of record are both read from the live working tree, reflecting any uncommitted edits

#### Scenario: removed requirement's body recoverable from the base
- **WHEN** a delta removes a requirement, naming it without restating its intro or scenarios
- **THEN** that requirement's intro and scenarios are available from the loaded spec of record

### Requirement: A missing spec of record is tolerated only where the delta needs no base
The system SHALL load successfully when a change's delta introduces a capability that has no spec of record yet, treating the base side as absent rather than reporting a failure. Where a delta entry does require a base — a modification or a removal — and no spec of record exists, the system SHALL report a structured error that is distinguishable from the tolerated absent-base case.

#### Scenario: all-added delta for a brand-new capability
- **WHEN** a change's delta consists only of added requirements and no spec of record exists for that capability at the diff base
- **THEN** both sides load successfully with the base side reported as absent

#### Scenario: modification with no spec of record
- **WHEN** a change's delta modifies a requirement and no spec of record exists for that capability at the diff base
- **THEN** a structured error is reported, distinguishable from the tolerated absent-base case

#### Scenario: removal with no spec of record
- **WHEN** a change's delta removes a requirement and no spec of record exists for that capability at the diff base
- **THEN** a structured error is reported, distinguishable from the tolerated absent-base case

### Requirement: A failure affecting one capability does not prevent loading the others
Where a change touches several capabilities, the system SHALL report a load or parse failure against the specific capability it affects, and SHALL still make the change's other capabilities loadable. An enumerated capability that turns out to carry no spec document SHALL be reported as such a per-capability failure.

#### Scenario: one capability fails to parse
- **WHEN** a change touches several capabilities and one of their delta specs is malformed
- **THEN** the failure is reported against that capability and the change's other capabilities still load

#### Scenario: enumerated capability with no spec document
- **WHEN** a capability is enumerated for a change but carries no spec document
- **THEN** a structured error is reported against that capability, and the change's other capabilities still load
