## Purpose

Defines opsxexplorer's overall terminal UI shell: the two-pane layout that every other screen renders into, and where keyboard focus starts.

## ADDED Requirements

### Requirement: Two-pane layout
The system SHALL render its terminal UI as two side-by-side panes: a left pane and a right pane.

#### Scenario: application launches
- **WHEN** the application starts
- **THEN** the screen is split into a left pane and a right pane

### Requirement: Right pane is a placeholder
The right pane SHALL render as an empty placeholder. It SHALL NOT display change contents, diffs, or any other content.

#### Scenario: right pane on launch
- **WHEN** the application is running
- **THEN** the right pane shows no change content, regardless of what is selected in the left pane

### Requirement: Left pane holds input focus
The left pane SHALL hold keyboard input focus for the duration of the application's runtime. There SHALL be no mechanism to move focus to the right pane.

#### Scenario: application launches
- **WHEN** the application starts
- **THEN** keyboard input is directed to the left pane

#### Scenario: user presses any key
- **WHEN** the user presses a key while the application is running
- **THEN** the key is handled by the left pane, since no other pane can hold focus

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
