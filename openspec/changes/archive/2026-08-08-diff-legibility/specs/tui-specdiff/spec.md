## MODIFIED Requirements

### Requirement: Changed content is shown as one inline word-level diff, not as before-and-after blocks
For a piece whose content changed and whose difference is reported as word-level runs, the pane SHALL render a single reflowed passage in which deleted and inserted runs are interleaved in place, each styled distinctly from unchanged text and from each other, rather than rendering the spec of record's text and the delta's text as two separate blocks. Deleted and inserted text SHALL both be visible in that one passage.

This applies to every piece reported with runs. A piece reported instead as a wholesale replacement — because the two texts are too dissimilar for an inline reading to help — is rendered as stacked before-and-after text (see "A wholesale replacement is shown as stacked before-and-after text"). The pane SHALL NOT make that judgement itself: it renders whichever form the piece was reported in.

#### Scenario: a sentence appended to a long paragraph
- **WHEN** a changed piece is the spec of record's text with one further sentence appended
- **THEN** the passage is shown once, with only the appended sentence styled as an insertion

#### Scenario: an edit in the middle of a line
- **WHEN** a changed piece differs from the spec of record's only in a few words
- **THEN** those words are shown as a deletion and an insertion in place, with the surrounding text shown unchanged and shown only once

#### Scenario: both sides are visible
- **WHEN** a piece's content is reported with runs
- **THEN** both the removed text and the added text are visible in the rendered passage

## ADDED Requirements

### Requirement: A wholesale replacement is shown as stacked before-and-after text
A piece reported as a wholesale replacement has no runs to interleave, and the two texts are by construction too dissimilar for interleaving to have helped. The pane SHALL render such a piece as the spec of record's text styled as a deletion, followed by the delta's text styled as an insertion, each beginning on its own line so the two read as consecutive passages rather than as one run-on sentence. Both texts SHALL be shown in full.

The two texts SHALL carry the same deletion and insertion styling that a deleted run and an inserted run carry inside an inline diff, so that the colours mean the same thing everywhere in the pane. Each text SHALL wrap to the pane width under the same rules as any other content, and the piece SHALL carry the same gutter marker as any other changed piece, since a replacement is a modification and not a removal.

#### Scenario: a replaced piece renders both texts
- **WHEN** a piece is reported as a wholesale replacement
- **THEN** the spec of record's text is shown in full styled as a deletion, and the delta's text is shown in full styled as an insertion

#### Scenario: the two texts do not run together
- **WHEN** a replaced piece is rendered
- **THEN** the delta's text begins on a line of its own rather than continuing the line the spec of record's text ended on

#### Scenario: a replaced piece is marked as modified
- **WHEN** a replaced piece is rendered
- **THEN** its gutter marker is the one a changed piece carries, not the one a removed piece carries

#### Scenario: a long replaced piece wraps
- **WHEN** either text of a replaced piece is wider than the pane
- **THEN** it wraps to the pane width, keeping its styling across the break, with no horizontal scrolling required
