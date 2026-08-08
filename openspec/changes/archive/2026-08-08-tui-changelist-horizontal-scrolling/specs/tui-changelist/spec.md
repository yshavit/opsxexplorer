## ADDED Requirements

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

## MODIFIED Requirements

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
