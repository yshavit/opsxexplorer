## MODIFIED Requirements

### Requirement: The right pane scrolls vertically and indicates its scroll position
Because a capability's diff can be taller than the pane, the right pane SHALL scroll vertically to keep the cursor visible as it moves, and SHALL indicate the current scroll position with a vertical scrollbar shown only while the content overflows the pane; the scrollbar SHALL be hidden entirely when the content fits. Scrolling SHALL advance by rendered line, so that a row wrapping onto several lines does not make scrolling skip content.

#### Scenario: cursor moved below the visible area
- **WHEN** the cursor moves to a row below the visible area
- **THEN** the pane scrolls so that the row is visible

#### Scenario: content shorter than the pane
- **WHEN** the diff fits entirely within the pane
- **THEN** no scrollbar is rendered

#### Scenario: content taller than the pane
- **WHEN** the diff is taller than the pane
- **THEN** the scrollbar is rendered, indicating the current scroll position

#### Scenario: scrolling past a wrapped row
- **WHEN** the content is scrolled through a row that wraps onto several lines
- **THEN** each of that row's lines can be brought into view in turn
