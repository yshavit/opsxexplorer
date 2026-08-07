## Purpose

Defines the left pane's content and behavior: which changes it lists, how they're grouped and sorted, how the archived section expands and collapses, and how the user navigates and selects among them.

## ADDED Requirements

### Requirement: Active changes listed first, alphabetically
The left pane SHALL list all active changes before any archived content, sorted alphabetically by change name. Each active change SHALL be shown by its name only.

#### Scenario: multiple active changes
- **WHEN** the repo has active changes `zebra-support`, `change-modeling`, and `dark-mode`
- **THEN** the left pane lists them in the order `change-modeling`, `dark-mode`, `zebra-support`

### Requirement: Archived changes are grouped under a collapsible section
The left pane SHALL show a single `archived` row after the active changes. The archived row SHALL be collapsed on launch. When expanded, it SHALL reveal the archived changes as rows beneath it; when collapsed, those rows SHALL NOT appear in the list.

#### Scenario: archived row collapsed by default
- **WHEN** the application starts
- **THEN** the `archived` row is present and collapsed, and no individual archived changes are shown

#### Scenario: expanding reveals archived changes
- **WHEN** the user expands the `archived` row
- **THEN** the archived changes appear as rows immediately beneath it

#### Scenario: collapsing hides archived changes
- **WHEN** the user collapses an expanded `archived` row
- **THEN** the archived changes beneath it no longer appear in the list

### Requirement: Archived changes sorted alphabetically, displayed with date
When expanded, the archived section SHALL list archived changes sorted alphabetically by their full directory name (date prefix included). Each SHALL be displayed as its date followed by its change name (date prefix removed from the name portion), with the date rendered in a visually de-emphasized (dimmed) style relative to the change name.

#### Scenario: alphabetical order matches chronological order
- **WHEN** the repo has archived changes `2026-01-03-foo` and `2026-06-19-bar`
- **THEN** the left pane lists them in that same order (earliest date first), displayed as `2026-01-03 foo` and `2026-06-19 bar`

#### Scenario: date is visually de-emphasized
- **WHEN** an archived change row is rendered
- **THEN** the date portion is styled distinctly (dimmed) from the change name portion

#### Scenario: malformed archive name has no date to show
- **WHEN** an archived change's directory name does not have a well-formed date prefix
- **THEN** the left pane displays it using its change name only, with no date shown

### Requirement: Single cursor navigable over active, archived-header, and archived rows
The left pane SHALL maintain a single selection cursor over its currently visible rows (active changes, the `archived` header, and, when expanded, archived changes). The cursor SHALL be moved with the up/down arrow keys and with the vim-style `k`/`j` keys.

#### Scenario: launch selects the first row
- **WHEN** the application starts
- **THEN** the cursor is on the first row in the list

#### Scenario: navigating with arrow keys
- **WHEN** the user presses the down arrow key
- **THEN** the cursor moves to the next row in the list

#### Scenario: navigating with vim keys
- **WHEN** the user presses `j`
- **THEN** the cursor moves to the next row in the list, identically to pressing the down arrow key

#### Scenario: cursor reaches archived children after expansion
- **WHEN** the `archived` row is expanded and the cursor moves down past it
- **THEN** the cursor lands on the first archived change row

### Requirement: Archived section toggles when its header is selected
When the cursor is on the `archived` row, pressing Enter or Space SHALL toggle the section between expanded and collapsed.

#### Scenario: expanding via Enter
- **WHEN** the cursor is on the collapsed `archived` row and the user presses Enter
- **THEN** the section expands and the cursor remains on the `archived` row

#### Scenario: expanding via Space
- **WHEN** the cursor is on the collapsed `archived` row and the user presses Space
- **THEN** the section expands and the cursor remains on the `archived` row

#### Scenario: pressing Enter or Space elsewhere has no effect on the section
- **WHEN** the cursor is on an active change or an archived change and the user presses Enter or Space
- **THEN** the `archived` section's expanded/collapsed state does not change

### Requirement: Collapsing returns the cursor to the archived header
Collapsing the `archived` section SHALL move the cursor to the `archived` row itself, regardless of which row was selected beforehand.

#### Scenario: collapsing while a child is selected
- **WHEN** the `archived` section is expanded, the cursor is on one of its archived changes, and the section is collapsed
- **THEN** the cursor moves to the `archived` row

### Requirement: Empty sections show placeholder text
When there are no active changes, the left pane SHALL show a placeholder row reading `(no active changes)` where the active section would be. When the `archived` section is expanded and there are no archived changes, it SHALL show a placeholder row reading `(no archived changes)` beneath the header. Placeholder rows SHALL NOT be selectable and SHALL be skipped by cursor navigation.

#### Scenario: no active changes
- **WHEN** the repo has no active changes
- **THEN** the left pane shows a `(no active changes)` placeholder row instead of any active change rows

#### Scenario: no archived changes, section expanded
- **WHEN** the repo has no archived changes and the user expands the `archived` row
- **THEN** the left pane shows a `(no archived changes)` placeholder row beneath the header

#### Scenario: navigation skips placeholder rows
- **WHEN** the cursor is adjacent to a placeholder row and the user moves the cursor toward it
- **THEN** the cursor skips over the placeholder row and lands on the next selectable row
