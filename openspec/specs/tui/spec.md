# tui Specification

## Purpose

Defines opsxexplorer's overall terminal UI shell: the two-pane layout that every other screen renders into, and where keyboard focus starts.

## Requirements

### Requirement: Two-pane layout
The system SHALL render its terminal UI as two side-by-side panes: a left pane and a right pane.

#### Scenario: application launches
- **WHEN** the application starts
- **THEN** the screen is split into a left pane and a right pane

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

### Requirement: Ctrl+Q exits the application
The system SHALL exit when the user presses Ctrl+Q.

#### Scenario: user presses Ctrl+Q
- **WHEN** the user presses Ctrl+Q while the application is running
- **THEN** the application exits

### Requirement: Terminal state is restored on exit
The system SHALL restore the terminal to its prior state (leaving raw mode and the alternate screen) whenever it exits, whether by normal quit or by an unexpected panic.

#### Scenario: normal exit restores terminal
- **WHEN** the user quits the application via Ctrl+Q
- **THEN** the terminal returns to its normal, non-raw, non-alternate-screen state

#### Scenario: panic restores terminal
- **WHEN** the application panics while running
- **THEN** the terminal is still restored to its normal state before the process exits
