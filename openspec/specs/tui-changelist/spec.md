# tui-changelist Specification

## Purpose

Defines the left pane's content and behavior: which changes it lists, how they're grouped and sorted, how the archived section expands and collapses, and how the user navigates and selects among them.

## Requirements

### Requirement: Active changes listed first, alphabetically
The left pane SHALL list all active changes before any archived content, sorted alphabetically by change name. Each active change SHALL be shown by its name only.

#### Scenario: multiple active changes
- **WHEN** the repo has active changes `zebra-support`, `change-modeling`, and `dark-mode`
- **THEN** the left pane lists them in the order `change-modeling`, `dark-mode`, `zebra-support`

### Requirement: Archived changes are grouped under a collapsible section
The left pane SHALL show a single `archived` row after the active changes. The archived row SHALL be collapsed on launch. When expanded, it SHALL reveal the archived changes as rows beneath it; when collapsed, those rows SHALL NOT appear in the list. While collapsed, the `archived` row SHALL render with an underline style; while expanded, it SHALL NOT.

#### Scenario: archived row collapsed by default
- **WHEN** the application starts
- **THEN** the `archived` row is present and collapsed, and no individual archived changes are shown

#### Scenario: expanding reveals archived changes
- **WHEN** the user expands the `archived` row
- **THEN** the archived changes appear as rows immediately beneath it

#### Scenario: collapsing hides archived changes
- **WHEN** the user collapses an expanded `archived` row
- **THEN** the archived changes beneath it no longer appear in the list

#### Scenario: collapsed row is underlined
- **WHEN** the `archived` row is collapsed
- **THEN** it renders with an underline style

#### Scenario: expanded row is not underlined
- **WHEN** the `archived` row is expanded
- **THEN** it renders without an underline style

#### Scenario: underline persists under horizontal scroll
- **WHEN** the `archived` row is collapsed and the pane is scrolled horizontally such that only part of the row's text is visible
- **THEN** the visible portion of the row's text still renders underlined

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

### Requirement: Left pane scrolls horizontally as a single unit
When any row's content is wider than the left pane, the user SHALL be able to scroll the pane's content horizontally. Scrolling SHALL apply a single offset to every row — active changes, the `archived` header, and archived changes alike — so all rows shift together and stay vertically aligned. The offset SHALL be moved one column at a time with `h`/`l` and the left/right arrow keys, and jumped to the leftmost or rightmost extent with `^`/Home and `$`/End respectively.

#### Scenario: scrolling right with l
- **WHEN** the cursor is anywhere in the left pane and the user presses `l`
- **THEN** the pane's content shifts one column to the left (revealing content further right), identically across all rows

#### Scenario: scrolling right with the right arrow key
- **WHEN** the user presses the right arrow key
- **THEN** the pane's content scrolls right by one column, identically to pressing `l`

#### Scenario: scrolling left with h
- **WHEN** the pane is scrolled right and the user presses `h`
- **THEN** the pane's content shifts one column back toward the left, identically across all rows

#### Scenario: scrolling left with the left arrow key
- **WHEN** the pane is scrolled right and the user presses the left arrow key
- **THEN** the pane's content scrolls left by one column, identically to pressing `h`

#### Scenario: jumping to the leftmost extent
- **WHEN** the pane is scrolled right and the user presses `^` (or Home)
- **THEN** the pane's content returns to its unscrolled, leftmost position

#### Scenario: jumping to the rightmost extent
- **WHEN** the user presses `$` (or End)
- **THEN** the pane scrolls to the furthest position needed to reveal the end of its widest row

#### Scenario: scrolling past content has no further effect
- **WHEN** the pane is already scrolled to its leftmost or rightmost extent and the user scrolls further in that direction
- **THEN** the scroll position does not move further

### Requirement: Horizontal scroll position is indicated with a scrollbar
The left pane SHALL show a horizontal scrollbar reflecting the current scroll offset relative to the widest row's content. The scrollbar SHALL only indicate scrollable content when at least one row is wider than the pane.

#### Scenario: all rows fit within the pane
- **WHEN** every visible row's content fits within the left pane's width
- **THEN** the horizontal scrollbar shows no scrollable range

#### Scenario: a row is wider than the pane
- **WHEN** at least one visible row's content is wider than the left pane
- **THEN** the horizontal scrollbar reflects that there is additional content and shows the current scroll position within it

### Requirement: Horizontal scroll offset persists across selection and section toggling, clamped to current content
The horizontal scroll offset SHALL NOT reset when the cursor moves to a different row or when the `archived` section is expanded or collapsed. The offset SHALL instead be clamped, at render time, to the maximum scroll needed for the currently visible rows at the pane's current width — so it self-corrects whenever the visible content or the pane's width changes, without an explicit reset.

#### Scenario: moving the cursor does not reset horizontal scroll
- **WHEN** the pane is scrolled right and the user moves the cursor to a different row
- **THEN** the pane remains scrolled to the same position

#### Scenario: scrolled past a shorter row's content
- **WHEN** the pane is scrolled right past the length of a given row's content
- **THEN** that row renders with no visible text, while rows with content reaching that far still show their content

#### Scenario: collapsing the archived section reduces the available scroll range
- **WHEN** the pane is scrolled right to reveal content only present in an expanded archived row, and the user collapses the `archived` section
- **THEN** the scroll offset is reduced to the maximum needed for the now-visible rows, if it exceeded that maximum

#### Scenario: widening the pane reduces the available scroll range
- **WHEN** the pane is scrolled right and the terminal is resized wider such that the previously-scrolled content now fits
- **THEN** the scroll offset is reduced accordingly, down to zero if all content now fits
