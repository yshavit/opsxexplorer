## RENAMED Requirements

- FROM: `### Requirement: A capability's unrecognised sections are listed below its requirement groups`
- TO: `### Requirement: A capability's unrecognised sections are listed below its requirement groups, grouped by origin`

## MODIFIED Requirements

### Requirement: Only collapsible rows are selectable
The right pane SHALL place its cursor only on rows that can be collapsed or expanded — requirement rows, scenario rows, and unrecognised-section rows — and SHALL skip group headings and notice rows when moving the cursor, so that every cursor position is one where the toggle keys do something, or is a deliberate exception documented below. A scenario's body remains skipped: it is expanded or collapsed only via its own scenario row, and carries no collapse state of its own. The purpose row and each of a removed requirement's removal-note line rows are each a deliberate exception: whenever a capability has a purpose comparison, or a removed requirement is expanded and its removal note has one or more lines, that row SHALL be selectable regardless of whether it is itself collapsible, so that moving the cursor row-by-row reaches every visible row without a gap — this is also what lets the pane's cursor-driven scrolling bring the row into view. A requirement's intro row is the same kind of exception. The purpose heading and each of the two unrecognised-sections headings — the delta-sourced heading and the base-sourced heading — SHALL both still be skipped, like any other group heading.

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
- **WHEN** the cursor moves across the delta-sourced or the base-sourced "Other sections" heading
- **THEN** it lands on the first unrecognised-section row in that heading's group rather than on the heading

#### Scenario: cursor stops on a purpose row even when it fits without truncation
- **WHEN** the capability's purpose text fits within the row's available width with no truncation needed
- **THEN** the cursor still stops on the purpose row when moving row-by-row

#### Scenario: cursor stops on an intro row even when it fits without truncation
- **WHEN** an expanded requirement's intro text fits within the row's available width with no truncation needed
- **THEN** the cursor still stops on the intro row when moving row-by-row

#### Scenario: cursor stops on a removal-note line row even when it fits without truncation
- **WHEN** an expanded removed requirement's removal-note line text fits within the row's available width with no truncation needed
- **THEN** the cursor still stops on that removal-note line row when moving row-by-row

#### Scenario: cursor stops on the unrecognised-sections row
- **WHEN** the selected tab's capability carries unrecognised sections from one or both origins
- **THEN** the cursor can stop on each section's own row when moving row-by-row, including via scrolling to it

#### Scenario: toggling a non-collapsible purpose row has no effect
- **WHEN** the cursor is on a purpose row that fits within its available width and the user presses Enter, Space, `l` or `h`
- **THEN** nothing changes: the row has no collapsed or expanded state to toggle

#### Scenario: toggling a non-collapsible intro row has no effect
- **WHEN** the cursor is on an intro row that fits within its available width and the user presses Enter, Space, `l` or `h`
- **THEN** nothing changes: the row has no collapsed or expanded state to toggle

#### Scenario: toggling a non-collapsible removal-note line row has no effect
- **WHEN** the cursor is on a removal-note line row that fits within its available width and the user presses Enter, Space, `l` or `h`
- **THEN** nothing changes: the row has no collapsed or expanded state to toggle

#### Scenario: toggling the unrecognised-sections row has no effect
- **WHEN** the cursor is on an unrecognised-section row and the user presses Enter, Space, `l` or `h`
- **THEN** the row toggles between collapsed and expanded, the same way a requirement row does — this row is no longer a no-op, since expanding it now reveals its body (see "An unrecognised section's row is collapsible, collapsed by default, and expands to its full body")

### Requirement: A capability's unrecognised sections are listed below its requirement groups, grouped by origin
When a tab's capability carries one or more delta-sourced unrecognised sections, one or more base-sourced unrecognised sections, or both, the right pane SHALL render them below that tab's requirement groups, after every group heading and requirement row, each origin as its own heading-and-rows group, the same way any other section of the pane sits in its own place in the layout. When both are present, the delta-sourced group SHALL render first, followed immediately by the base-sourced group.

Both headings read "Other sections", boxed and styled the same way a group heading or the purpose heading is, including degrading to a plain styled line under the same narrow-pane rule. The delta-sourced heading SHALL be styled in the same purple used before this requirement changed, distinct from every other colour already used in the pane — whether a delta-sourced section survives a future sync into the spec of record is unspecified, so it stays a deliberate call-out. The base-sourced heading SHALL be styled in the pane's ordinary (unstyled) text colour and SHALL qualify its label, within the same heading, with a parenthetical in that same unstyled colour and in italics, noting that these sections are in the spec of record but not this change — content the spec of record carries that the delta does not mention is preserved as-is by the project's own sync tooling, so it carries no comparable warning. The parenthetical is part of the heading it qualifies, not a line of its own beneath it, and so SHALL sit inside the heading's box and SHALL count toward the width the heading needs before the narrow-pane rule degrades it.

Each unrecognised section, of either origin, SHALL render as its own row showing that section's title, collapsed by default (see "An unrecognised section's row is collapsible, collapsed by default, and expands to its full body"). Neither heading's group SHALL render a bullet list, and neither SHALL carry a prompt encouraging the user to file an enhancement request. A tab whose capability carries no unrecognised sections of a given origin SHALL render neither that origin's heading nor any of its rows, and SHALL leave no gap in their place; a tab with neither origin renders none of this at all.

#### Scenario: a capability with unrecognised sections
- **WHEN** the selected tab's capability diff carries one or more delta-sourced unrecognised sections
- **THEN** the pane renders a purple "Other sections" heading below the tab's requirement groups, followed by one collapsed-by-default row per section, each showing that section's title

#### Scenario: a capability with base-sourced unrecognised sections
- **WHEN** the selected tab's capability diff carries one or more base-sourced unrecognised sections
- **THEN** the pane renders an unstyled "Other sections" heading whose label is followed, inside the same box, by an italic parenthetical noting they are in the spec of record but not this change, followed by one collapsed-by-default row per section, each showing that section's title

#### Scenario: both origins render in a fixed order
- **WHEN** the selected tab's capability diff carries unrecognised sections from both the delta and the spec of record
- **THEN** the delta-sourced heading and its rows render first, followed immediately by the base-sourced heading and its rows

#### Scenario: a capability with unrecognised sections from only one origin
- **WHEN** the selected tab's capability diff carries unrecognised sections from only one origin
- **THEN** only that origin's heading and rows are rendered, and no gap is left where the other origin's heading would go

#### Scenario: a capability with no unrecognised sections
- **WHEN** the selected tab's capability diff carries no unrecognised sections at all
- **THEN** neither heading is rendered, and no gap is left in their place

#### Scenario: several unrecognised sections are all listed
- **WHEN** the selected tab's capability diff carries more than one unrecognised section from the same origin
- **THEN** each is shown as its own row, in the order the diff carries them

#### Scenario: no enhancement-request prompt is shown
- **WHEN** either heading's group is rendered
- **THEN** no row prompts the user to file an enhancement request

#### Scenario: narrow pane degrades the heading
- **WHEN** the pane is too narrow for the boxed heading style
- **THEN** whichever "Other sections" heading is present renders as a single styled line, the same way a group heading or the purpose heading does

## ADDED Requirements

### Requirement: An unrecognised section's row is collapsible, collapsed by default, and expands to its full body
Each unrecognised-section row — whether under the delta-sourced heading or the base-sourced heading — SHALL open collapsed, matching the pane's default for a requirement row. Expanding it SHALL reveal its full body below the row, in the pane's ordinary unstyled text colour, wrapped to the pane width. Unlike a requirement's intro or a removal-note line, this body SHALL NOT itself be further collapsible or excerpted regardless of its length: expanding the row is a single step to its full content, not a first step toward a further collapse. Collapsing the row SHALL hide the body again, with the row's own collapsed/expanded indicator matching its state, the same way a requirement row's does.

#### Scenario: an unrecognised section starts collapsed
- **WHEN** a tab's capability diff carries an unrecognised section
- **THEN** its row is shown collapsed, with no body visible

#### Scenario: expanding an unrecognised section reveals its full body
- **WHEN** a collapsed unrecognised-section row is expanded
- **THEN** its full body becomes visible below it, in the pane's ordinary unstyled text colour, wrapped to the pane width

#### Scenario: collapsing an unrecognised section hides its body again
- **WHEN** an expanded unrecognised-section row is collapsed
- **THEN** its body is hidden again and the row's indicator reflects the collapsed state

#### Scenario: a long unrecognised section's body is never further collapsed
- **WHEN** an expanded unrecognised section's body is longer than the pane's width
- **THEN** it is shown in full, wrapped, with no truncation and no additional collapse layer of its own
