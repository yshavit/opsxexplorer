## MODIFIED Requirements

### Requirement: A renamed requirement shows both names, in one entry
A renamed requirement SHALL be shown once, displaying both its former name and its new name, rather than as two entries or as a modification whose name happens to differ. Where the same delta renames a requirement and also changes its content, that content comparison SHALL be shown under the single renamed entry.

The two names SHALL be rendered as whichever form their comparison was reported in, using the same rendering already applied to any other compared piece in the pane, rather than a rendering of its own: a changed comparison renders as one inline, reflowed passage with deleted and inserted runs interleaved in place (see "Changed content is shown as one inline word-level diff, not as before-and-after blocks"), styled with the same deletion and insertion colours used everywhere else in the pane; a wholesale replacement renders as the former name styled as a deletion followed by the new name styled as an insertion, each starting on its own line (see "A wholesale replacement is shown as stacked before-and-after text"). Neither form introduces a collapse or truncation behaviour of its own: the row wraps to the pane width like any other row's content (see "Content wraps to the pane width and is never scrolled horizontally"), and is never made collapsible on account of its length alone.

#### Scenario: rename alone
- **WHEN** a requirement is renamed and nothing else about it changed
- **THEN** one row is shown carrying both the former and the new name

#### Scenario: rename combined with a modification
- **WHEN** a requirement is renamed and its content also changed
- **THEN** one row is shown carrying both names, and expanding it reveals the content comparison

#### Scenario: similar names render as an inline diff
- **WHEN** a requirement's former and new names are similar enough to be reported as a changed comparison
- **THEN** the row renders `REQ` followed by one inline passage with the two names' word-level differences interleaved, styled the same way any other changed piece's runs are styled

#### Scenario: dissimilar names render as stacked before-and-after text
- **WHEN** a requirement's former and new names are too dissimilar and reported as a wholesale replacement
- **THEN** the row renders `REQ` followed by the former name styled as a deletion and the new name styled as an insertion, each on its own line, the same way any other wholesale replacement is rendered

Every line after the first — whether the new name's own line, or a continuation line from either name wrapping on its own — SHALL be indented to align beneath where the first line's own text began (past the marker, the disclosure triangle and `REQ `), not merely beneath the gutter, so the stacked names read as one aligned block.

#### Scenario: the new name aligns beneath the former name's text
- **WHEN** a requirement's former and new names are reported as a wholesale replacement
- **THEN** the new name's line is indented to start at the same column the former name's text started at on the line above, not at the gutter column

#### Scenario: a long rename wraps instead of collapsing
- **WHEN** a renamed requirement's rendered name comparison, in either form, is wider than the pane
- **THEN** it wraps like any other row's text, and does not become collapsible or truncate to an excerpt on account of its width
