## MODIFIED Requirements

### Requirement: Only collapsible rows are selectable
The right pane SHALL place its cursor only on rows that can be collapsed or expanded — requirement rows and scenario rows — and SHALL skip group headings and notice rows when moving the cursor, so that every cursor position is one where the toggle keys do something, or is a deliberate exception documented below. A scenario's body remains skipped: it is expanded or collapsed only via its own scenario row, and carries no collapse state of its own. The purpose row and a requirement's intro row are each a deliberate exception: whenever a capability has a purpose comparison, or a requirement is expanded and therefore shows its intro row, that row SHALL be selectable regardless of whether it is itself collapsible, so that moving the cursor row-by-row reaches every visible row without a gap. The purpose heading itself SHALL still be skipped, like any other group heading.

#### Scenario: cursor skips a group heading
- **WHEN** the cursor moves across an operation group heading
- **THEN** it lands on the next requirement row rather than on the heading

#### Scenario: cursor skips content rows
- **WHEN** the cursor moves through an expanded scenario's body
- **THEN** it lands on the next requirement or scenario row rather than on the content

#### Scenario: cursor skips the purpose heading
- **WHEN** the cursor moves across the purpose heading
- **THEN** it lands on the purpose row rather than on the heading

#### Scenario: cursor stops on a purpose row even when it fits without truncation
- **WHEN** the capability's purpose text fits within the row's available width with no truncation needed
- **THEN** the cursor still stops on the purpose row when moving row-by-row

#### Scenario: cursor stops on an intro row even when it fits without truncation
- **WHEN** an expanded requirement's intro text fits within the row's available width with no truncation needed
- **THEN** the cursor still stops on the intro row when moving row-by-row

#### Scenario: toggling a non-collapsible purpose row has no effect
- **WHEN** the cursor is on a purpose row that fits within its available width and the user presses Enter, Space, `l` or `h`
- **THEN** nothing changes: the row has no collapsed or expanded state to toggle

#### Scenario: toggling a non-collapsible intro row has no effect
- **WHEN** the cursor is on an intro row that fits within its available width and the user presses Enter, Space, `l` or `h`
- **THEN** nothing changes: the row has no collapsed or expanded state to toggle

### Requirement: An unmentioned intro is visibly distinguished from an unchanged one
An intro block that the delta does not mention renders the spec of record's text — the same text an unchanged intro renders — so without a distinguishing treatment the delta's silence would be invisible. The pane SHALL therefore mark an unmentioned intro block with the unmentioned marker in the gutter **and** render its text in a de-emphasised style, so it cannot be mistaken for content the delta restated. This de-emphasis SHALL apply regardless of the intro row's own collapse state: both a collapsed excerpt and the fully expanded text SHALL be shown de-emphasised when the intro is unmentioned.

#### Scenario: intro the delta did not mention
- **WHEN** a modified requirement's intro is unmentioned by the delta
- **THEN** the intro block carries the unmentioned marker and its text is de-emphasised

#### Scenario: intro the delta restated unchanged
- **WHEN** a modified requirement's intro is restated by the delta and is unchanged
- **THEN** the intro block does not carry the unmentioned marker and its text is not de-emphasised

#### Scenario: an unmentioned intro stays de-emphasised whether collapsed or expanded
- **WHEN** a modified requirement's intro is unmentioned by the delta and is long enough to be collapsible
- **THEN** both its collapsed excerpt and its expanded text are shown in the de-emphasised style

## ADDED Requirements

### Requirement: A requirement's intro that fits in one line renders in full, with no collapse affordance
This requirement applies to any intro comparison whose content is a single passage of text — unchanged, an insertion, an ordinary edit, a deletion, or a passage unmentioned by the delta. An intro reported as a wholesale replacement never qualifies, regardless of how short its current text is (see "A requirement's intro is always collapsible when it is a wholesale replacement...").

The row beneath an expanded requirement SHALL carry the pillcrow (¶) marker, exactly as it does today. Whether it is collapsible at all is decided by measuring the intro's current text, right-trimmed of trailing whitespace, against the row's available width at its own nesting depth — one level deeper than a purpose row, and therefore narrower at the same pane width. When the trimmed text's character count does not exceed that available width, the row SHALL render the full text on one line, with no disclosure triangle and no ellipsis. It remains a selectable row (see "Only collapsible rows are selectable"), but carries no expanded or collapsed state, so the toggle keys have no effect on it.

#### Scenario: short intro renders in full
- **WHEN** an expanded requirement's intro comparison fits within the row's available width once right-trimmed of trailing whitespace
- **THEN** the row shows that full text on one line, with no disclosure triangle and no ellipsis

#### Scenario: trailing whitespace does not force truncation
- **WHEN** the intro's text is exactly as wide as the row's available width once trailing whitespace is trimmed, but wider than that width before trimming
- **THEN** the row renders the full (trimmed) text with no truncation

#### Scenario: growing the pane can remove the need to collapse
- **WHEN** the pane is widened such that a previously-truncated intro text now fits within the row's available width
- **THEN** the row switches to rendering the full text on one line, with no disclosure triangle

#### Scenario: an intro's available width accounts for its own nesting depth
- **WHEN** an expanded requirement's intro and the capability's purpose comparison hold the same text at the same pane width
- **THEN** the intro's fits-check uses its own, narrower available width — one level of indent deeper than the purpose row's — so the two can disagree about whether that text fits

### Requirement: A requirement's intro that does not fit in one line is collapsible, and starts collapsed
When the current intro comparison's text, right-trimmed of trailing whitespace, is longer than the row's available width, the row SHALL be collapsible, carrying a disclosure triangle indicating its own expanded or collapsed state, matching the convention used elsewhere in the pane. It SHALL start collapsed the first time a requirement is expanded, absent any earlier toggle of its own.

Collapsed, the row SHALL render as exactly one line: a slice of the current text taken from its start, sized to fill the row's available width less one column, followed by a single ellipsis character. The slice SHALL be a literal count of characters and SHALL NOT seek a word boundary, so the truncation point is exact rather than word-aware. Expanded, the row SHALL render the full text, wrapped and styled the same way any other compared piece is — interleaved word-level runs for a changed comparison.

#### Scenario: expanding a requirement with an overflowing intro shows it collapsed
- **WHEN** a requirement is expanded for the first time and its intro comparison does not fit within the row's available width
- **THEN** the intro row is collapsed

#### Scenario: collapsed row is truncated to one line
- **WHEN** the intro row is collapsed
- **THEN** it renders as one line ending in an ellipsis, showing as many characters from the start of the text as fit in the row's available width minus one

#### Scenario: truncation ignores word boundaries
- **WHEN** the intro row is collapsed and the available width falls in the middle of a word
- **THEN** the rendered text is cut at that exact character position, not at the preceding word boundary

#### Scenario: truncation follows the pane width
- **WHEN** the pane is resized while the intro row is collapsed and still does not fit
- **THEN** the number of characters shown before the ellipsis changes to match the new available width

#### Scenario: expanding reveals the full comparison
- **WHEN** the intro row is expanded
- **THEN** the full compared text is shown, wrapped to the pane width and carrying the same interleaved-run diff styling a requirement's intro already carries for a changed comparison

#### Scenario: collapsing restores the single-line view
- **WHEN** an expanded intro row is collapsed
- **THEN** it renders again as the single truncated line (or the placeholder, if its comparison is a wholesale replacement)

#### Scenario: shrinking the pane can introduce the need to collapse
- **WHEN** the pane is narrowed such that a previously-fitting intro text no longer fits within the row's available width
- **THEN** the row switches to being collapsible, starting collapsed

### Requirement: A requirement's intro is always collapsible when it is a wholesale replacement, and collapses to a placeholder rather than an excerpt
An intro comparison too dissimilar to read as an inline diff is reported as a wholesale replacement and rendered, when expanded, as stacked before-and-after text like any other replaced piece. Truncating just the current text to a one-line excerpt would misrepresent that rewrite as an ordinary edit, indistinguishable from a changed comparison's truncated excerpt. The intro row SHALL therefore be collapsible whenever its comparison is a wholesale replacement, regardless of whether the current text alone would otherwise fit within the row's available width, and SHALL start collapsed the first time the requirement is expanded.

Collapsed, such a row SHALL render, in place of any excerpt of either text, the fixed placeholder reading "Expand to view diff", styled in italics — the same placeholder a replaced purpose row shows. If the row's available width is narrower than the placeholder text, the placeholder SHALL itself be truncated with a trailing ellipsis, the same way any other single-line text in this pane is. Expanded, the row SHALL render both texts stacked, styled and wrapped the same way any other wholesale replacement is rendered.

#### Scenario: a replaced intro is collapsible even when short
- **WHEN** a requirement's intro comparison is a wholesale replacement and its current text alone would fit within the row's available width
- **THEN** the row is collapsible rather than rendered in full

#### Scenario: a replaced intro collapses to a placeholder
- **WHEN** the intro row is collapsed and its comparison is a wholesale replacement
- **THEN** the row shows the italicized placeholder "Expand to view diff" instead of an excerpt of either text

#### Scenario: the placeholder truncates at extreme widths
- **WHEN** the intro row is collapsed, its comparison is a wholesale replacement, and the row's available width is narrower than the placeholder text
- **THEN** the placeholder itself is truncated with a trailing ellipsis

#### Scenario: expanding a replaced intro reveals the stacked diff
- **WHEN** an intro row whose comparison is a wholesale replacement is expanded
- **THEN** the spec of record's text and the delta's text are both shown in full, stacked, styled the same way any other wholesale replacement is
