## MODIFIED Requirements

### Requirement: Malformed spec content is reported as a structured error
The system SHALL report malformed spec content as a structured error that identifies what was malformed and where, in a form that can be displayed to the user. It SHALL NOT crash, and SHALL NOT silently discard content it cannot interpret. An unrecognised `##` section in a delta spec is not malformed content under this requirement — see "Unrecognised delta sections are collected without failing the parse" — but every other case below still applies to both a spec of record and a delta spec.

#### Scenario: scenario appearing before any requirement
- **WHEN** a spec contains a scenario heading before any requirement heading
- **THEN** a structured error identifying the stray scenario is reported

#### Scenario: unrecognised operation section
- **WHEN** a spec of record contains a `##` section whose title is neither `Purpose` nor `Requirements`
- **THEN** a structured error identifying the unrecognised section is reported

#### Scenario: rename missing its target
- **WHEN** a renamed-requirements section supplies a from-name with no matching to-name
- **THEN** a structured error identifying the incomplete rename is reported

#### Scenario: unparseable content is never dropped silently
- **WHEN** content in a spec document cannot be interpreted
- **THEN** the outcome is a reported error, never a successful parse that omits that content

## ADDED Requirements

### Requirement: Unrecognised delta sections are collected without failing the parse
A delta spec's `##` section whose title is not one of the ones the system already recognises — `Purpose`, or one of the four operation sections `ADDED`/`MODIFIED`/`REMOVED`/`RENAMED Requirements` — is not itself malformed: OpenSpec has no closed list of section titles a delta spec.md is allowed to carry, and content the tool does not recognise is not evidence the file is broken. The system SHALL therefore continue parsing the rest of the delta spec after encountering such a section, and SHALL record that section's title so the caller can indicate it did not fully understand the file, rather than discarding it or aborting the parse. Titles SHALL be collected in the order they appear in the document. This requirement applies only to a delta spec; a spec of record's unrecognised sections remain governed by "Malformed spec content is reported as a structured error", since the spec of record is never itself rendered — it is read only as a diff base for a delta, one named section at a time.

#### Scenario: one unrecognised section among well-formed ones
- **WHEN** a delta spec contains a `## ADDED Requirements` section and one `##` section whose title is not recognised
- **THEN** the delta parses successfully, with the added requirement present and the unrecognised section's title recorded

#### Scenario: several unrecognised sections
- **WHEN** a delta spec contains more than one `##` section whose title is not recognised
- **THEN** all of their titles are recorded, in the order they appear in the document

#### Scenario: no unrecognised sections
- **WHEN** every `##` section in a delta spec is one the system already recognises
- **THEN** no unrecognised section titles are recorded
