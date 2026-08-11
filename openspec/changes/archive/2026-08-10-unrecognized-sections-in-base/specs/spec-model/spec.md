## RENAMED Requirements

- FROM: `### Requirement: Unrecognised delta sections are collected without failing the parse`
- TO: `### Requirement: Unrecognised sections are collected without failing the parse`

## MODIFIED Requirements

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
