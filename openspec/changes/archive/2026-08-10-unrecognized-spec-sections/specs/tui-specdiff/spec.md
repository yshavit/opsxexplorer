## MODIFIED Requirements

### Requirement: Only collapsible rows are selectable
The right pane SHALL place its cursor only on rows that can be collapsed or expanded — requirement rows and scenario rows — and SHALL skip group headings and notice rows when moving the cursor, so that every cursor position is one where the toggle keys does something, or is a deliberate exception documented below. A scenario's body remains skipped: it is expanded or collapsed only via its own scenario row, and carries no collapse state of its own. The purpose row, a requirement's intro row, and the row listing a capability's unrecognised sections are each a deliberate exception: whenever a capability has a purpose comparison, a requirement is expanded and therefore shows its intro row, or a capability carries unrecognised sections, that row SHALL be selectable regardless of whether it is itself collapsible, so that moving the cursor row-by-row reaches every visible row without a gap — this is also what lets the pane's cursor-driven scrolling bring the row into view. The purpose heading and the "Unknown sections" heading SHALL both still be skipped, like any other group heading.

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

#### Scenario: cursor stops on the unrecognised-sections row
- **WHEN** the selected tab's capability carries unrecognised sections
- **THEN** the cursor can stop on the row listing them when moving row-by-row, including via scrolling to it

#### Scenario: toggling a non-collapsible purpose row has no effect
- **WHEN** the cursor is on a purpose row that fits within its available width and the user presses Enter, Space, `l` or `h`
- **THEN** nothing changes: the row has no collapsed or expanded state to toggle

#### Scenario: toggling a non-collapsible intro row has no effect
- **WHEN** the cursor is on an intro row that fits within its available width and the user presses Enter, Space, `l` or `h`
- **THEN** nothing changes: the row has no collapsed or expanded state to toggle

#### Scenario: toggling the unrecognised-sections row has no effect
- **WHEN** the cursor is on the row listing a capability's unrecognised sections and the user presses Enter, Space, `l` or `h`
- **THEN** nothing changes: the row has no collapsed or expanded state to toggle

## ADDED Requirements

### Requirement: A capability's unrecognised sections are listed below its requirement groups
When a tab's capability carries one or more unrecognised section titles, the right pane SHALL render them below that tab's requirement groups, after every group heading and requirement row, the same way any other section of the pane sits in its own place in the layout: a heading reading "Unknown sections", styled in a purple distinct from every other colour already used in the pane, boxed and styled the same way a group heading or the purpose heading is, including degrading to a plain styled line under the same narrow-pane rule; followed by one row carrying a line, in the same purple and in italics, prompting the user to consider filing an enhancement request, wrapped to the pane width, followed by one bullet per unrecognised title in the order the diff carries them, in the pane's ordinary (unstyled) text colour. A tab whose capability carries no unrecognised sections SHALL render neither the heading nor the row, and SHALL leave no gap in their place.

#### Scenario: a capability with unrecognised sections
- **WHEN** the selected tab's capability diff carries unrecognised section titles
- **THEN** the pane renders the "Unknown sections" heading below the tab's requirement groups, followed by a row listing each title as its own bullet, in the order the diff carries them

#### Scenario: a capability with no unrecognised sections
- **WHEN** the selected tab's capability diff carries no unrecognised section titles
- **THEN** neither the heading nor the row is rendered, and no gap is left in their place

#### Scenario: several unrecognised sections are all listed
- **WHEN** the selected tab's capability diff carries more than one unrecognised section title
- **THEN** each title is shown as its own bullet within the row, and all of them are shown

#### Scenario: narrow pane degrades the heading
- **WHEN** the pane is too narrow for the boxed heading style
- **THEN** the "Unknown sections" heading renders as a single styled line, the same way a group heading or the purpose heading does
