# spec-diff Specification

## Purpose

Compares a change's delta spec against the spec of record for the same capability and produces the per-requirement delta the delta spec's own text does not show: which requirements are added, modified, removed or renamed, and, within a modified requirement, which pieces changed and exactly where inside them.

## Requirements

### Requirement: A capability's delta is reported as per-requirement operations in a stable order
Given a change's delta entries for one capability and the corresponding spec of record, the system SHALL produce one entry per requirement the delta names, and SHALL NOT produce entries for requirements the delta does not name. The diff unit SHALL be a single requirement. Entries SHALL be reported grouped by operation in the order added, modified, removed, renamed, and within each group in the order the delta names them, so that consumers can rely on the sequence without re-sorting it. Comparing the same delta and spec of record more than once SHALL produce the same result each time.

#### Scenario: operations are grouped in a fixed order

- **WHEN** a delta contains additions, modifications, removals and renames interleaved in the source document
- **THEN** the reported entries are grouped added first, then modified, then removed, then renamed

#### Scenario: entries within a group follow the delta's order

- **WHEN** a delta adds several requirements
- **THEN** they are reported in the order the delta lists them

#### Scenario: untouched requirements are not reported

- **WHEN** the spec of record contains requirements the delta does not name under any operation
- **THEN** no entry is produced for them

#### Scenario: repeated comparison is stable

- **WHEN** the same delta and spec of record are compared twice
- **THEN** both comparisons produce the same entries, in the same order, with the same content

### Requirement: An added requirement is reported as an insertion in its entirety
The system SHALL report an added requirement's intro and every one of its scenarios as pure insertions, in the delta's order, without consulting the spec of record. An addition SHALL be reportable when no spec of record exists for the capability at all.

#### Scenario: added requirement's content is all insertion

- **WHEN** a delta adds a requirement with an intro and several scenarios
- **THEN** the intro and each scenario are reported as inserted content, in the order the delta lists them

#### Scenario: addition against an absent spec of record

- **WHEN** a delta consists only of additions and the capability has no spec of record at the diff base
- **THEN** the additions are reported normally and no error is raised

### Requirement: A removed requirement's content is recovered from the spec of record
A delta's removal entry names a requirement without restating its content, but the user is shown what is being removed. The system SHALL take a removed requirement's intro and scenarios from the spec of record and SHALL report all of them as pure deletions.

#### Scenario: removal shows the base's content

- **WHEN** a delta removes a requirement that the spec of record defines with an intro and several scenarios
- **THEN** that intro and those scenarios are reported as deleted content, taken from the spec of record

#### Scenario: removal reports no content of its own

- **WHEN** a removal entry is compared
- **THEN** nothing from the delta entry's own body contributes to the reported content, since a removal entry has none

### Requirement: A renamed requirement is reported as a rename, exactly once
The system SHALL report a rename as its own operation carrying both the former and the new requirement name, rather than as a modification whose name happens to differ, so that a rename is recognisable as a rename. A renamed requirement's content SHALL be compared against the spec of record's requirement under its former name, by the same rules that govern a modification. Where the same delta both renames a requirement and modifies it under its new name, the system SHALL report a single renamed entry carrying that content comparison, and SHALL NOT additionally report it as a modification nor treat the new name's absence from the spec of record as an error.

#### Scenario: rename alone

- **WHEN** a delta renames a requirement and says nothing else about it
- **THEN** a rename entry is reported carrying both names, and its content is compared against the spec of record's requirement under the former name

#### Scenario: rename combined with a modification

- **WHEN** a delta renames a requirement and also modifies it under its new name
- **THEN** a single rename entry is reported, carrying both names and the content comparison, with no separate modification entry and no missing-base error

#### Scenario: rename is not reported as a name diff

- **WHEN** a rename entry is reported
- **THEN** the former and new names are both available as names, not only as differing text within a modification

### Requirement: Content the delta does not mention is reported as unmentioned, not as a deletion
A delta's modified entry may restate a requirement in full or may supply only the pieces that changed, and the two are indistinguishable in the source; OpenSpec has no operation for removing a single scenario or a requirement's intro. The system SHALL therefore apply one rule uniformly: absence from the delta means the delta says nothing about that piece, and presence means the delta is authoritative for that piece. A piece of a requirement's content present in the spec of record and not mentioned by the delta SHALL be reported in a distinct unmentioned state — carrying the spec of record's content, so it can be shown as context — and SHALL NOT be reported as deleted, nor as unchanged. The unmentioned state SHALL be distinguishable by consumers from every other state.

#### Scenario: a delta that restates a requirement in full produces no unmentioned content

- **WHEN** a modified entry restates every scenario the spec of record's requirement has
- **THEN** every piece is reported as added, changed or unchanged, and nothing is reported as unmentioned

#### Scenario: a scenario present only in the spec of record

- **WHEN** a modified entry lists a subset of the spec of record's scenarios
- **THEN** each scenario present only in the spec of record is reported as unmentioned, carrying the spec of record's content, and none of them is reported as deleted or as unchanged

#### Scenario: unmentioned is not silently resolved

- **WHEN** a piece is unmentioned
- **THEN** the reported state records that the delta did not mention it, rather than resolving the ambiguity to either outcome

### Requirement: A modified requirement's intro is compared as a single piece
The system SHALL compare a modified requirement's intro against the intro of the spec of record's requirement of the same name and report exactly one of: unmentioned, when the delta entry's intro is empty; unchanged, when the two intros are equal; or changed, carrying the comparison of the two texts. Because an omitted intro and an emptied intro are indistinguishable in the source, an emptied intro SHALL be reported as unmentioned rather than as content deleted in full.

#### Scenario: intro omitted from the delta

- **WHEN** a modified entry supplies no intro text
- **THEN** the intro is reported as unmentioned, carrying the spec of record's intro

#### Scenario: intro restated unchanged

- **WHEN** a modified entry's intro is equal to the spec of record's intro
- **THEN** the intro is reported as unchanged

#### Scenario: intro edited

- **WHEN** a modified entry's intro differs from the spec of record's intro
- **THEN** the intro is reported as changed, carrying the comparison of the two texts

### Requirement: A modified requirement's scenarios are matched by name and ordered base-first
The system SHALL match a modified entry's scenarios to the spec of record's scenarios by scenario name, independently of their position in either document. Each matched pair SHALL be reported as unchanged when the two bodies are equal and as changed otherwise, carrying the comparison of the two bodies. A scenario present only in the delta entry SHALL be reported as added. Scenarios SHALL be reported in the spec of record's order first, with scenarios present only in the delta entry appended in the delta entry's order, so that the reported sequence is stable regardless of how the delta orders what it restates.

#### Scenario: scenario restated with an edit

- **WHEN** a modified entry restates a scenario under a name the spec of record also uses, with a different body
- **THEN** that scenario is reported as changed, carrying the comparison of the two bodies

#### Scenario: scenario restated unchanged

- **WHEN** a modified entry restates a scenario with a body equal to the spec of record's
- **THEN** that scenario is reported as unchanged

#### Scenario: scenario new in the delta

- **WHEN** a modified entry contains a scenario whose name the spec of record's requirement does not use
- **THEN** that scenario is reported as added

#### Scenario: reported order is base order then delta-only

- **WHEN** a modified entry restates the spec of record's scenarios in a different order and adds new ones
- **THEN** the scenarios shared with the spec of record are reported in the spec of record's order, followed by the delta-only scenarios in the delta entry's order

### Requirement: Changed content is reported as word-level runs over the compared texts
For a piece reported as changed, the system SHALL report the difference as an ordered sequence of runs, each marking a span of text as equal, deleted or inserted, rather than as a pair of whole before-and-after line sets. Runs SHALL be at word granularity, so an edit confined to part of a line leaves the rest of that line reported as equal. Whitespace alone SHALL NOT hold two edits apart: where the only equal text separating one change from the next is whitespace, the system SHALL report the whole span as a single deletion followed by a single insertion rather than as two changes bridged by an equal run. The runs SHALL address positions in the two body texts exactly as supplied: the system SHALL NOT trim, re-wrap, re-flow or otherwise normalise the texts it is given, so that a consumer can reconstruct each side from the runs and lay it out itself. Concatenating the equal and deleted runs in order SHALL reproduce the spec of record's text, and concatenating the equal and inserted runs in order SHALL reproduce the delta's text. Every equal run SHALL address the same text on both sides.

A piece too dissimilar for an inline reading SHALL instead be reported as a wholesale replacement and SHALL carry no runs (see "A piece whose two texts are too dissimilar to read inline is reported as a wholesale replacement").

#### Scenario: sentence appended to a long paragraph

- **WHEN** a modified entry's intro is the spec of record's intro with one further sentence appended
- **THEN** the runs are the unchanged text followed by a single inserted run, with no deleted run

#### Scenario: edit within a line leaves its surroundings equal

- **WHEN** a scenario body differs from the spec of record's only in a few words in the middle of a long line
- **THEN** the text before and after those words is reported as equal runs, and only the differing words are reported as deleted and inserted

#### Scenario: two adjacent word edits separated only by a space

- **WHEN** a piece replaces two consecutive words with two different words, so that the only equal text between the two edits is the space that separates them
- **THEN** the whole span is reported as one deleted run and one inserted run, not as two deletions and two insertions bridged by an equal run

#### Scenario: equal text elsewhere still anchors the diff

- **WHEN** two edits within a piece are separated by equal text that is not purely whitespace
- **THEN** that separating text is still reported as an equal run and the two edits remain distinct

#### Scenario: runs reconstruct both sides

- **WHEN** a changed piece's runs are reported
- **THEN** the equal and deleted runs concatenate to the spec of record's text and the equal and inserted runs concatenate to the delta's text

#### Scenario: equal runs address the same text on both sides

- **WHEN** a changed piece's runs are reported
- **THEN** the text each equal run addresses in the spec of record's body is identical to the text it addresses in the delta's body

#### Scenario: content is compared as supplied

- **WHEN** the texts being compared contain long lines, list markers and inline markup
- **THEN** they are compared unmodified, with no re-wrapping or normalisation applied before or after the comparison

### Requirement: A piece whose two texts are too dissimilar to read inline is reported as a wholesale replacement
When a piece is rewritten rather than edited, the words the two texts still share are largely incidental — articles, modal verbs and other filler matching across unrelated sentences — and interleaving them produces a reading strictly harder than reading each text in turn. The system SHALL measure how much of the two texts the difference reports as equal, and SHALL report the piece as a wholesale replacement, carrying both full texts and no runs, when that measure falls below a fixed threshold. Otherwise the piece SHALL be reported as changed, with runs.

The measure and the threshold SHALL be fixed rather than derived from the surrounding requirement or capability, so that the same pair of texts always yields the same verdict regardless of what else the change touches. A piece SHALL NOT be reported as a replacement when either text is empty, since there is then nothing to compare and no interleaving to avoid.

Reporting a replacement is a legibility judgement, not a claim about correctness: the two texts SHALL be carried through unmodified, so a consumer that would rather render them as an inline diff still can.

#### Scenario: a substantially rewritten intro

- **WHEN** a modified requirement's intro is rewritten rather than edited, so that the two texts share only scattered filler words
- **THEN** the piece is reported as a wholesale replacement carrying both texts, and no runs are reported for it

#### Scenario: an ordinary edit is unaffected

- **WHEN** a piece differs from the spec of record's by a phrase, a sentence, or an appended paragraph, leaving most of the two texts in common
- **THEN** the piece is reported as changed with runs, not as a replacement

#### Scenario: a replacement carries both texts unmodified

- **WHEN** a piece is reported as a wholesale replacement
- **THEN** the spec of record's text and the delta's text are both carried in full, exactly as supplied, with no trimming or normalisation

#### Scenario: an empty side is never a replacement

- **WHEN** a changed piece's spec-of-record text or delta text is empty
- **THEN** the piece is reported as changed with runs rather than as a replacement

#### Scenario: the verdict is deterministic

- **WHEN** the same pair of texts is compared twice, in different requirements or different capabilities
- **THEN** both comparisons reach the same verdict

### Requirement: A delta entry with no matching requirement in the spec of record is reported as a displayable error
A modification or removal naming a requirement the spec of record does not contain is an authoring mistake — a mistyped heading, or a rename made by hand. The system SHALL report it as a structured error that identifies the capability and the requirement name and can be displayed to the user. It SHALL NOT crash and SHALL NOT skip the entry silently. The error SHALL be attributed to the entry it concerns, so that the capability's other requirements are still reported. This case SHALL remain distinct from the case of a capability having no spec of record at all, which is detected when both sides are loaded.

#### Scenario: modification naming an unknown requirement

- **WHEN** a delta modifies a requirement whose name does not appear in the spec of record
- **THEN** a displayable error identifying that capability and requirement name is reported for that entry

#### Scenario: removal naming an unknown requirement

- **WHEN** a delta removes a requirement whose name does not appear in the spec of record
- **THEN** a displayable error identifying that capability and requirement name is reported for that entry

#### Scenario: rename naming an unknown requirement

- **WHEN** a delta renames a requirement whose former name does not appear in the spec of record
- **THEN** a displayable error identifying that capability and requirement name is reported for that entry

#### Scenario: one bad entry does not suppress the others

- **WHEN** one entry names a requirement absent from the spec of record and the delta's other entries are sound
- **THEN** the sound entries are still reported alongside the error

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

### Requirement: A capability's diff carries the delta's unrecognised sections through
The comparison the system produces for a capability SHALL carry the same unrecognised section titles its delta was parsed with, unchanged and in the same order, so that a capability's diff is a superset of everything the pane needs to render it — including content the tool did not understand well enough to diff. The system SHALL NOT interpret, validate, or otherwise act on an unrecognised section title beyond carrying it through.

#### Scenario: delta with unrecognised sections
- **WHEN** a capability's delta was parsed with one or more unrecognised section titles
- **THEN** the capability's diff carries those same titles, in the same order

#### Scenario: delta with no unrecognised sections
- **WHEN** a capability's delta was parsed with no unrecognised section titles
- **THEN** the capability's diff carries none either
