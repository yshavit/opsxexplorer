## ADDED Requirements

### Requirement: q exits the application
The system SHALL exit when the user presses `q` with no modifier keys held.

#### Scenario: user presses q
- **WHEN** the user presses `q` while the application is running
- **THEN** the application exits

## MODIFIED Requirements

### Requirement: Terminal state is restored on exit
The system SHALL restore the terminal to its prior state (leaving raw mode and the alternate screen) whenever it exits, whether by normal quit or by an unexpected panic.

#### Scenario: normal exit restores terminal
- **WHEN** the user quits the application by pressing `q`
- **THEN** the terminal returns to its normal, non-raw, non-alternate-screen state

#### Scenario: panic restores terminal
- **WHEN** the application panics while running
- **THEN** the terminal is still restored to its normal state before the process exits

## REMOVED Requirements

### Requirement: Ctrl+Q exits the application
**Reason**: Replaced by `q`, which needs no modifier key and matches the vim-style single-key bindings used elsewhere in the app.
**Migration**: Press `q` instead of Ctrl+Q.
