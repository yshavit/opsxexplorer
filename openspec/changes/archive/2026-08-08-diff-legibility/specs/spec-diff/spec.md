## MODIFIED Requirements

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

## ADDED Requirements

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
