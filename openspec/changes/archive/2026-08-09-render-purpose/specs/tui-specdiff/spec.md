## MODIFIED Requirements

### Requirement: Only collapsible rows are selectable
The right pane SHALL place its cursor only on rows that can be collapsed or expanded — requirement rows and scenario rows — and SHALL skip group headings, intro blocks, scenario bodies and notice rows when moving the cursor, so that every cursor position is one where the toggle keys do something. The purpose row is a deliberate exception: whenever a capability has a purpose comparison, its row SHALL be selectable regardless of whether it is collapsible, so that moving the cursor row-by-row reaches every visible row without a gap. The purpose heading itself SHALL still be skipped, like any other group heading.

#### Scenario: cursor skips a group heading
- **WHEN** the cursor moves across an operation group heading
- **THEN** it lands on the next requirement row rather than on the heading

#### Scenario: cursor skips content rows
- **WHEN** the cursor moves through an expanded requirement's intro block or an expanded scenario's body
- **THEN** it lands on the next requirement or scenario row rather than on the content

#### Scenario: cursor skips the purpose heading
- **WHEN** the cursor moves across the purpose heading
- **THEN** it lands on the purpose row rather than on the heading

#### Scenario: cursor stops on a purpose row even when it fits without truncation
- **WHEN** the capability's purpose text fits within the row's available width with no truncation needed
- **THEN** the cursor still stops on the purpose row when moving row-by-row

#### Scenario: toggling a non-collapsible purpose row has no effect
- **WHEN** the cursor is on a purpose row that fits within its available width and the user presses Enter, Space, `l` or `h`
- **THEN** nothing changes: the row has no collapsed or expanded state to toggle

## ADDED Requirements

### Requirement: A capability's purpose comparison is rendered above its requirement groups
When spec-diff reports a purpose comparison for the capability shown in the current tab, the right pane SHALL render it above the tab's group headings and requirement rows, but below any notices for that capability, as a heading naming the comparison's kind followed by a single row carrying the compared text. The heading SHALL read "Added Purpose" when the comparison is an insertion and "Modified Purpose" when it is a changed or replaced comparison, and SHALL be styled and boxed the same way an "Added Requirements" or "Modified Requirements" group heading is, including degrading to a plain styled line under the same narrow-pane rule.

#### Scenario: an added purpose
- **WHEN** the capability's purpose comparison is an insertion
- **THEN** the pane shows a heading reading "Added Purpose" above the requirement groups

#### Scenario: a modified purpose
- **WHEN** the capability's purpose comparison is changed or replaced
- **THEN** the pane shows a heading reading "Modified Purpose" above the requirement groups

#### Scenario: purpose sits below notices
- **WHEN** a capability has both an error notice and a purpose comparison
- **THEN** the notice is shown first, followed by the purpose heading and row, followed by the requirement groups

#### Scenario: narrow pane degrades the heading
- **WHEN** the pane is too narrow for the boxed heading style
- **THEN** the purpose heading renders as a single styled line, the same way a requirement-group heading does

### Requirement: A purpose row that fits in one line renders in full, with no collapse affordance
This requirement applies only to a comparison whose content can be shown as ordinary text — an insertion, or a changed comparison carrying runs. A comparison reported as a wholesale replacement never qualifies, regardless of how short its current text is (see "A purpose row is always collapsible when its comparison is a wholesale replacement").

The row beneath the purpose heading SHALL carry the pillcrow (¶) marker used for a requirement's intro. Whether it is collapsible at all is decided by measuring the current comparison's text, right-trimmed of trailing whitespace, against the row's available width: when the trimmed text's character count does not exceed the available width, the row SHALL render that full text on one line, with no disclosure triangle and no ellipsis. It remains a selectable row (see "Only collapsible rows are selectable"), but carries no expanded or collapsed state, so the toggle keys have no effect on it.

#### Scenario: short purpose renders in full
- **WHEN** the capability's purpose comparison is an insertion or a changed comparison, and its text, right-trimmed of trailing whitespace, fits within the row's available width
- **THEN** the row shows that full text on one line, with no disclosure triangle and no ellipsis

#### Scenario: trailing whitespace does not force truncation
- **WHEN** the purpose comparison's text is exactly as wide as the row's available width once trailing whitespace is trimmed, but wider than that width before trimming
- **THEN** the row renders the full (trimmed) text with no truncation

#### Scenario: growing the pane can remove the need to collapse
- **WHEN** the pane is widened such that a previously-truncated purpose text (from an insertion or a changed comparison) now fits within the row's available width
- **THEN** the row switches to rendering the full text on one line, with no disclosure triangle

### Requirement: A purpose row that does not fit in one line is collapsible, and starts collapsed
When the current comparison is an insertion or a changed comparison and its text, right-trimmed of trailing whitespace, is longer than the row's available width, the row beneath the purpose heading SHALL be collapsible, carrying a disclosure triangle indicating its own expanded or collapsed state, matching the convention used elsewhere in the pane. It SHALL start collapsed whenever a tab is first displayed.

Collapsed, the row SHALL render as exactly one line: a slice of the current comparison's text taken from its start, sized to fill the row's available width less one column, followed by a single ellipsis character. The slice SHALL be a literal count of characters and SHALL NOT seek a word boundary, so the truncation point is exact rather than word-aware. Expanded, the row SHALL render the comparison's full text, wrapped and styled the same way any other compared piece is — interleaved word-level runs for a changed comparison.

#### Scenario: pane opens with an overflowing purpose row collapsed
- **WHEN** a tab is first displayed and its purpose comparison (an insertion or a changed comparison) does not fit within the row's available width
- **THEN** the purpose row is collapsed

#### Scenario: collapsed row is truncated to one line
- **WHEN** the purpose row is collapsed and its comparison is an insertion or a changed comparison
- **THEN** it renders as one line ending in an ellipsis, showing as many characters from the start of the text as fit in the row's available width minus one

#### Scenario: truncation ignores word boundaries
- **WHEN** the purpose row is collapsed, its comparison is an insertion or a changed comparison, and the available width falls in the middle of a word
- **THEN** the rendered text is cut at that exact character position, not at the preceding word boundary

#### Scenario: truncation follows the pane width
- **WHEN** the pane is resized while the purpose row is collapsed and still does not fit
- **THEN** the number of characters shown before the ellipsis changes to match the new available width

#### Scenario: expanding reveals the full comparison
- **WHEN** the purpose row is expanded and its comparison is an insertion or a changed comparison
- **THEN** the full compared text is shown, wrapped to the pane width and carrying the same interleaved-run diff styling a requirement's intro or a scenario's body would carry for the same kind of comparison

#### Scenario: collapsing restores the single-line view
- **WHEN** an expanded purpose row is collapsed
- **THEN** it renders again as the single truncated line (or the placeholder, if its comparison is a wholesale replacement)

#### Scenario: shrinking the pane can introduce the need to collapse
- **WHEN** the pane is narrowed such that a previously-fitting purpose text (from an insertion or a changed comparison) no longer fits within the row's available width
- **THEN** the row switches to being collapsible, starting collapsed

### Requirement: A purpose row is always collapsible when its comparison is a wholesale replacement, and collapses to a placeholder rather than an excerpt
A purpose comparison too dissimilar to read as an inline diff is reported as a wholesale replacement (see `spec-diff`'s "A piece whose two texts are too dissimilar to read inline is reported as a wholesale replacement") and rendered, when expanded, as stacked before-and-after text like any other replaced piece. Truncating just the current text to a one-line excerpt would misrepresent that rewrite as an ordinary edit, indistinguishable from a changed comparison's truncated excerpt. The row beneath the purpose heading SHALL therefore be collapsible whenever its comparison is a wholesale replacement, regardless of whether the current text alone would otherwise fit within the row's available width, and SHALL start collapsed whenever a tab is first displayed.

Collapsed, such a row SHALL render, in place of any excerpt of either text, a fixed placeholder reading "Expand to view diff", styled in italics. If the row's available width is narrower than the placeholder text, the placeholder SHALL itself be truncated with a trailing ellipsis, the same way any other single-line text in this pane is. Expanded, the row SHALL render both texts stacked, styled and wrapped the same way any other wholesale replacement is rendered.

#### Scenario: a replaced purpose is collapsible even when short
- **WHEN** the capability's purpose comparison is a wholesale replacement and its current text alone would fit within the row's available width
- **THEN** the row is collapsible rather than rendered in full

#### Scenario: a replaced purpose collapses to a placeholder
- **WHEN** the purpose row is collapsed and its comparison is a wholesale replacement
- **THEN** the row shows the italicized placeholder "Expand to view diff" instead of an excerpt of either text

#### Scenario: the placeholder truncates at extreme widths
- **WHEN** the purpose row is collapsed, its comparison is a wholesale replacement, and the row's available width is narrower than the placeholder text
- **THEN** the placeholder itself is truncated with a trailing ellipsis

#### Scenario: expanding a replaced purpose reveals the stacked diff
- **WHEN** a purpose row whose comparison is a wholesale replacement is expanded
- **THEN** the spec of record's text and the delta's text are both shown in full, stacked, styled the same way any other wholesale replacement is

### Requirement: A capability with no purpose comparison renders nothing for it
When a capability's purpose comparison is absent — because the delta carries no purpose section, or because the compared texts are equal — the right pane SHALL render no heading and no row for it, and the tab SHALL open directly on its notices and requirement groups exactly as it did before purpose comparisons existed.

#### Scenario: delta has no purpose section
- **WHEN** the selected capability's delta carries no purpose section
- **THEN** no purpose heading or row is shown, and no gap is left in its place

#### Scenario: purpose restated unchanged
- **WHEN** the selected capability's delta's purpose section is byte-identical to the spec of record's purpose
- **THEN** no purpose heading or row is shown
