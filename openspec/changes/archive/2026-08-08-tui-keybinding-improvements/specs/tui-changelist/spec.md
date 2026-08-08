## ADDED Requirements

### Requirement: Selecting a change moves focus to the right pane
When the cursor is on an active or archived change row, pressing Enter or Space SHALL move keyboard focus to the right pane, in addition to that row's existing effect of loading its diff into the right pane. When the cursor is on the `archived` header row, Enter and Space SHALL NOT move focus — they continue only to toggle the section, since the right pane has no content to show for that row.

#### Scenario: Enter moves focus from an active change row
- **WHEN** the cursor is on an active change row and the user presses Enter
- **THEN** focus moves to the right pane

#### Scenario: Space moves focus from an archived change row
- **WHEN** the `archived` section is expanded, the cursor is on an archived change row, and the user presses Space
- **THEN** focus moves to the right pane

#### Scenario: Enter and Space on the archived header do not move focus
- **WHEN** the cursor is on the `archived` header row and the user presses Enter or Space
- **THEN** focus remains on the left pane

## MODIFIED Requirements

### Requirement: Single cursor navigable over active, archived-header, and archived rows
The left pane SHALL maintain a single selection cursor over its currently visible rows (active changes, the `archived` header, and, when expanded, archived changes). The cursor SHALL be moved one row at a time with the up/down arrow keys and the vim-style `k`/`j` keys, and by a half-page of rows at a time with `Ctrl+d` (down) and `Ctrl+u` (up), where a half-page is derived from the pane's current visible row count. Cursor movement SHALL stop at the first or last selectable row rather than wrapping around or moving past it.

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

#### Scenario: half-page down with Ctrl+d
- **WHEN** the user presses `Ctrl+d`
- **THEN** the cursor moves down by roughly half the pane's visible row count, skipping over any placeholder rows as ordinary movement does

#### Scenario: half-page up with Ctrl+u
- **WHEN** the user presses `Ctrl+u`
- **THEN** the cursor moves up by roughly half the pane's visible row count

#### Scenario: half-page movement clamps at the ends
- **WHEN** fewer than half a page of rows remain in the direction of travel
- **THEN** the cursor stops at the first or last selectable row rather than overshooting
