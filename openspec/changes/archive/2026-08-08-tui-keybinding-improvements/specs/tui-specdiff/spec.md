## MODIFIED Requirements

### Requirement: Right-pane keys move the cursor and toggle rows
When the right pane holds focus, the system SHALL move the cursor to the previous row on `k` or the up arrow and to the next row on `j` or the down arrow, move the cursor by a half-page of rows at a time on `Ctrl+u` (up) and `Ctrl+d` (down) where a half-page is derived from the pane's current visible row count, toggle the row under the cursor between expanded and collapsed on `Enter` or `Space`, expand the row under the cursor on `l` or the right arrow, and collapse it on `h` or the left arrow. Cursor movement SHALL stop at the ends of the content rather than wrapping around. These keys SHALL have no effect on the right pane while the left pane holds focus.

#### Scenario: moving the cursor down
- **WHEN** the right pane holds focus and the user presses `j` or the down arrow
- **THEN** the cursor moves to the next selectable row

#### Scenario: moving the cursor up
- **WHEN** the right pane holds focus and the user presses `k` or the up arrow
- **THEN** the cursor moves to the previous selectable row

#### Scenario: toggling a row
- **WHEN** the right pane holds focus and the user presses `Enter` or `Space` on a collapsed row
- **THEN** that row expands, and pressing it again collapses the row

#### Scenario: expanding and collapsing directionally
- **WHEN** the right pane holds focus and the user presses `l` or the right arrow on a collapsed row, then `h` or the left arrow
- **THEN** the row expands and then collapses

#### Scenario: cursor stops at the ends
- **WHEN** the cursor is on the first row and the user presses `k`, or is on the last row and presses `j`
- **THEN** the cursor stays where it is

#### Scenario: keys are inert while the left pane holds focus
- **WHEN** the left pane holds focus and the user presses any of these keys
- **THEN** the right pane's cursor and collapse state are unchanged

#### Scenario: half-page down with Ctrl+d
- **WHEN** the right pane holds focus and the user presses `Ctrl+d`
- **THEN** the cursor moves down by roughly half the pane's visible row count, and the pane scrolls to keep it visible

#### Scenario: half-page up with Ctrl+u
- **WHEN** the right pane holds focus and the user presses `Ctrl+u`
- **THEN** the cursor moves up by roughly half the pane's visible row count, and the pane scrolls to keep it visible

#### Scenario: half-page movement clamps at the ends
- **WHEN** fewer than half a page of selectable rows remain in the direction of travel
- **THEN** the cursor stops at the first or last selectable row rather than overshooting
