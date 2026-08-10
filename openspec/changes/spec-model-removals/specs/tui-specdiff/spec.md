## MODIFIED Requirements

### Requirement: Only collapsible rows are selectable
The right pane SHALL place its cursor only on rows that can be collapsed or expanded — requirement rows and scenario rows — and SHALL skip group headings and notice rows when moving the cursor, so that every cursor position is one where the toggle keys do something, or is a deliberate exception documented below. A scenario's body remains skipped: it is expanded or collapsed only via its own scenario row, and carries no collapse state of its own. The purpose row, a requirement's intro row, a removed requirement's removal-note row, and the row listing a capability's unrecognised sections are each a deliberate exception: whenever a capability has a purpose comparison, a requirement is expanded and therefore shows its intro row, a removed requirement is expanded and carries a removal note, or a capability carries unrecognised sections, that row SHALL be selectable regardless of whether it is itself collapsible, so that moving the cursor row-by-row reaches every visible row without a gap — this is also what lets the pane's cursor-driven scrolling bring the row into view. The purpose heading and the "Unknown sections" heading SHALL both still be skipped, like any other group heading.

#### Scenario: cursor skips a group heading
- **WHEN** the cursor moves across an operation group heading
- **THEN** it lands on the next requirement row rather than on the heading

#### Scenario: cursor skips content rows
- **WHEN** the cursor moves through an expanded scenario's body
- **THEN** it lands on the next requirement or scenario row rather than on the content

#### Scenario: cursor skips the purpose heading
- **WHEN** the cursor moves across the purpose heading
- **THEN** it lands on the purpose row rather than on the heading

#### Scenario: cursor skips the "Unknown sections" heading
- **WHEN** the cursor moves across the "Unknown sections" heading
- **THEN** it lands on the row listing the unrecognised section titles rather than on the heading

#### Scenario: cursor stops on a purpose row even when it fits without truncation
- **WHEN** the capability's purpose text fits within the row's available width with no truncation needed
- **THEN** the cursor still stops on the purpose row when moving row-by-row

#### Scenario: cursor stops on an intro row even when it fits without truncation
- **WHEN** an expanded requirement's intro text fits within the row's available width with no truncation needed
- **THEN** the cursor still stops on the intro row when moving row-by-row

#### Scenario: cursor stops on a removal-note row even when it fits without truncation
- **WHEN** an expanded removed requirement's removal-note text fits within the row's available width with no truncation needed
- **THEN** the cursor still stops on the removal-note row when moving row-by-row

#### Scenario: cursor stops on the unrecognised-sections row
- **WHEN** the selected tab's capability carries unrecognised sections
- **THEN** the cursor can stop on the row listing them when moving row-by-row, including via scrolling to it

#### Scenario: toggling a non-collapsible purpose row has no effect
- **WHEN** the cursor is on a purpose row that fits within its available width and the user presses Enter, Space, `l` or `h`
- **THEN** nothing changes: the row has no collapsed or expanded state to toggle

#### Scenario: toggling a non-collapsible intro row has no effect
- **WHEN** the cursor is on an intro row that fits within its available width and the user presses Enter, Space, `l` or `h`
- **THEN** nothing changes: the row has no collapsed or expanded state to toggle

#### Scenario: toggling a non-collapsible removal-note row has no effect
- **WHEN** the cursor is on a removal-note row that fits within its available width and the user presses Enter, Space, `l` or `h`
- **THEN** nothing changes: the row has no collapsed or expanded state to toggle

#### Scenario: toggling the unrecognised-sections row has no effect
- **WHEN** the cursor is on the row listing a capability's unrecognised sections and the user presses Enter, Space, `l` or `h`
- **THEN** nothing changes: the row has no collapsed or expanded state to toggle

## ADDED Requirements

### Requirement: A removed requirement's removal note is shown above its deleted content
A removed requirement's own body — its removal note, typically a Reason and Migration explanation — has no counterpart in the spec of record to compare it against, so `spec-diff` reports it separately from the requirement's intro and scenarios rather than as one of their diffed pieces (see `spec-diff`'s "A removed requirement's own body is carried through as a removal note"). The right pane SHALL render a non-empty removal note as its own row directly above the requirement's intro row, carrying the pillcrow (¶) marker, a blank gutter marker, and plain text styling — neither the insertion styling nor the deletion styling used elsewhere in the pane — so it reads as explanatory text attached to the requirement rather than as a diffed piece of it. The row SHALL follow the same fits-in-one-line-else-collapsible convention a requirement's intro row follows: it renders in full with no disclosure triangle when the note, right-trimmed of trailing whitespace, fits the row's available width, and is otherwise collapsible, starting collapsed the first time the requirement is expanded, showing a character-exact truncated excerpt ending in an ellipsis while collapsed and the full note, wrapped, when expanded. A removed requirement whose delta entry carries no body SHALL render no such row, leaving the requirement's intro row as the first row shown, exactly as before this requirement existed.

#### Scenario: a removal note renders above the intro
- **WHEN** an expanded removed requirement's diff carries a removal note
- **THEN** the pane shows a row for it directly above the requirement's intro row, carrying the pillcrow marker

#### Scenario: the removal note uses plain styling, not a diff colour
- **WHEN** a removal-note row is rendered
- **THEN** it carries a blank gutter marker and plain text styling, with neither the insertion styling used for added content nor the deletion styling of the intro and scenario rows below it

#### Scenario: a bare removal shows no note row
- **WHEN** an expanded removed requirement's diff carries no removal note
- **THEN** no removal-note row is rendered and the requirement's intro row is the first row shown

#### Scenario: a short removal note renders in full
- **WHEN** a removal note, right-trimmed of trailing whitespace, fits within the row's available width
- **THEN** the row shows the full note on one line, with no disclosure triangle and no ellipsis

#### Scenario: a long removal note is collapsible and starts collapsed
- **WHEN** a removal note's trimmed text is longer than the row's available width
- **THEN** the row is collapsible, starts collapsed the first time the requirement is expanded, and renders a character-exact truncated excerpt ending in an ellipsis while collapsed

#### Scenario: expanding reveals the full removal note
- **WHEN** a collapsed removal-note row is expanded
- **THEN** the full note is shown, wrapped to the pane width
