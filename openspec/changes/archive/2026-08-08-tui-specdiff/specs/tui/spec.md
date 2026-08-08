## RENAMED Requirements

- FROM: `### Requirement: Left pane holds input focus`
- TO: `### Requirement: Focus moves between the two panes`

## MODIFIED Requirements

### Requirement: Focus moves between the two panes
Exactly one pane SHALL hold keyboard input focus at any time. The left pane SHALL hold focus when the application starts. The system SHALL move focus to the other pane when the user presses Tab, and SHALL indicate which pane currently holds focus visually. A key pressed while a pane holds focus SHALL be handled by that pane, except for keys the application handles globally.

#### Scenario: application launches
- **WHEN** the application starts
- **THEN** the left pane holds keyboard input focus, and this is visually indicated

#### Scenario: user presses Tab
- **WHEN** the user presses Tab while the left pane holds focus
- **THEN** focus moves to the right pane and the focus indication follows it

#### Scenario: user presses Tab again
- **WHEN** the user presses Tab while the right pane holds focus
- **THEN** focus moves back to the left pane

#### Scenario: user presses any key
- **WHEN** the user presses a key that both panes bind, while one of them holds focus
- **THEN** the key is handled by the pane that holds focus, and the other pane's state is unchanged

#### Scenario: global keys work from either pane
- **WHEN** the user presses a key the application handles globally, whichever pane holds focus
- **THEN** it takes effect regardless of which pane is focused

## REMOVED Requirements

### Requirement: Right pane is a placeholder
**Reason**: The right pane now renders the selected change's spec diff — the capability the tool exists to provide. This requirement forbade exactly that ("It SHALL NOT display change contents, diffs, or any other content"); it was scaffolding for the initial two-pane shell, not intended behaviour.

**Migration**: None — nothing depends on the pane being empty. The right pane's behaviour is now specified by the `tui-specdiff` capability.
