## REMOVED Requirements

### Requirement: Archived changes are grouped under a collapsible section
**Reason**: The archived row's underline, used to signal collapsed vs. expanded, is replaced by a trailing-slash `archived/` label that renders identically in both states (see the new "Archived section is a collapsible `archived/` row" requirement).
**Migration**: None. This requirement's collapse/expand behavior is preserved, unchanged, by the new requirement below; only its label and underline styling are gone.

### Requirement: Horizontal scroll offset persists across selection and section toggling, clamped to current content
**Reason**: The scroll clamp now always accounts for every row, including collapsed archived rows, so expanding or collapsing the archived section no longer changes the available scroll range — the opposite of this requirement's "collapsing the archived section reduces the available scroll range" scenario (see the new "Horizontal scroll offset persists across selection and section toggling, clamped to all rows" requirement).
**Migration**: None. The persistence behavior (no reset on cursor move or toggle) is preserved, unchanged, by the new requirement below; only the clamp's basis changes.

## ADDED Requirements

### Requirement: Archived section is a collapsible `archived/` row
The left pane SHALL show a single `archived/` row after the active changes. The archived row SHALL be collapsed on launch. When expanded, it SHALL reveal the archived changes as rows beneath it; when collapsed, those rows SHALL NOT appear in the list. The row's label and style SHALL be identical whether collapsed or expanded, except for the disclosure triangle (`▸` when collapsed, `▾` when expanded).

#### Scenario: archived row collapsed by default
- **WHEN** the application starts
- **THEN** the `archived/` row is present and collapsed, and no individual archived changes are shown

#### Scenario: expanding reveals archived changes
- **WHEN** the user expands the `archived/` row
- **THEN** the archived changes appear as rows immediately beneath it

#### Scenario: collapsing hides archived changes
- **WHEN** the user collapses an expanded `archived/` row
- **THEN** the archived changes beneath it no longer appear in the list

#### Scenario: row label carries a trailing slash
- **WHEN** the `archived/` row is rendered, collapsed or expanded
- **THEN** its label reads `archived/`, with a trailing slash

#### Scenario: only the disclosure triangle differs between states
- **WHEN** the `archived/` row's collapsed and expanded renderings are compared
- **THEN** the two differ only in the disclosure triangle (`▸` vs `▾`); the label text and style are identical

### Requirement: Horizontal scroll offset persists across selection and section toggling, clamped to all rows
The horizontal scroll offset SHALL NOT reset when the cursor moves to a different row or when the `archived` section is expanded or collapsed. The offset SHALL instead be clamped, at render time, to the maximum scroll needed across all rows — active changes, the `archived` header, and every archived change, whether or not the archived section is currently expanded — at the pane's current width. Because the clamp always accounts for archived rows, expanding or collapsing the archived section does not change the available scroll range.

#### Scenario: moving the cursor does not reset horizontal scroll
- **WHEN** the pane is scrolled right and the user moves the cursor to a different row
- **THEN** the pane remains scrolled to the same position

#### Scenario: scrolled past a shorter row's content
- **WHEN** the pane is scrolled right past the length of a given row's content
- **THEN** that row renders with no visible text, while rows with content reaching that far still show their content

#### Scenario: collapsing the archived section does not change the available scroll range
- **WHEN** the pane is scrolled right to reveal content only present in an expanded archived row, and the user collapses the `archived` section
- **THEN** the scroll offset is unchanged, because the maximum scroll range already accounted for the (now-hidden) archived row's content

#### Scenario: widening the pane reduces the available scroll range
- **WHEN** the pane is scrolled right and the terminal is resized wider such that the previously-scrolled content now fits
- **THEN** the scroll offset is reduced accordingly, down to zero if all content now fits

### Requirement: Left pane width is capped to its content
The left pane's width SHALL be the lesser of its default proportional share of the frame and the widest row's content width plus a one-column buffer plus the pane's borders, where the widest row is computed across all rows — active changes, the `archived` header, and every archived change — regardless of whether the archived section is currently expanded. When the content-driven width is narrower than the default proportional share, the freed columns SHALL be given to the right pane. When the content-driven width would exceed the default proportional share, the pane SHALL be capped at the proportional share instead, and horizontal scrolling applies as usual.

#### Scenario: narrow content shrinks the pane
- **WHEN** every row's content, including archived rows, is narrower than the pane's default proportional share of the frame
- **THEN** the left pane's width is the widest row's content width plus a one-column buffer plus borders, and the right pane receives the remaining columns

#### Scenario: wide content caps the pane at its default share
- **WHEN** the widest row's content is wider than the pane's default proportional share of the frame
- **THEN** the left pane's width does not exceed that default share, and the pane's content requires horizontal scrolling to view in full

#### Scenario: width does not change when the archived section is expanded or collapsed
- **WHEN** the archived section is expanded or collapsed
- **THEN** the left pane's width does not change, because its width is computed from all rows regardless of the archived section's current state

#### Scenario: comfortable space is left between content and border
- **WHEN** the left pane's width is determined by its content rather than capped by the default proportional share
- **THEN** there is a one-column buffer between the widest row's last character and the pane's border

## MODIFIED Requirements

### Requirement: Horizontal scroll position is indicated with a scrollbar
When at least one row — including collapsed archived rows — is wider than the pane, the left pane SHALL show a horizontal scrollbar reflecting the current scroll offset relative to the widest row's content. When no row is wider than the pane, the scrollbar SHALL NOT render at all.

#### Scenario: all rows fit within the pane
- **WHEN** every visible row's content fits within the left pane's width
- **THEN** the left pane renders no horizontal scrollbar

#### Scenario: a row is wider than the pane
- **WHEN** at least one visible row's content is wider than the left pane
- **THEN** the horizontal scrollbar reflects that there is additional content and shows the current scroll position within it
