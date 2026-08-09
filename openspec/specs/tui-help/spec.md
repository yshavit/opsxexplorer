# tui-help Specification

## Purpose

Defines the `?` help modal: an on-screen, always-available reference for every keybinding the application recognizes, organized so the keys relevant to whichever pane the user is working in are easy to find.

## Requirements

### Requirement: `?` toggles the help modal
The system SHALL open the help modal when the user presses `?` while it is closed, and SHALL close it when the user presses `?` again while it is open, regardless of which pane held focus.

#### Scenario: opening the modal
- **WHEN** the help modal is closed and the user presses `?`
- **THEN** the help modal opens, overlaying the two-pane layout

#### Scenario: closing the modal by pressing ? again
- **WHEN** the help modal is open and the user presses `?`
- **THEN** the help modal closes, returning to the two-pane layout exactly as it was before the modal opened

### Requirement: Esc also closes the modal
The system SHALL close the help modal when the user presses `Esc` while it is open.

#### Scenario: closing with Esc
- **WHEN** the help modal is open and the user presses `Esc`
- **THEN** the help modal closes

### Requirement: q quits even while the modal is open
Pressing `q` SHALL exit the application whether or not the help modal is open, taking priority over the modal's own key handling.

#### Scenario: quitting while the modal is open
- **WHEN** the help modal is open and the user presses `q` with no modifier keys held
- **THEN** the application exits

### Requirement: Keybindings are grouped into Global, Left pane, and Right pane sections
The help modal SHALL present its content as three sections, in this order: **Global** (keys bound identically regardless of which pane holds focus, differing only in which pane's state they act on), **Left pane** (keys the left pane alone binds), and **Right pane** (keys the right pane alone binds). Each entry SHALL show the key or key combination and a succinct description of its effect. A blank line SHALL separate each section from the next.

#### Scenario: modal content
- **WHEN** the help modal is open
- **THEN** it shows a Global section listing `q`, `?`, `Tab`, `j`/`k`/`↓`/`↑`, `Ctrl+d`/`Ctrl+u`, and `Enter`/`Space`, a Left pane section listing `h`/`l`/`←`/`→` (scroll) and `^`/`Home`/`$`/`End` (jump to start/end), and a Right pane section listing `h`/`l`/`←`/`→` (expand/collapse) and `[`/`]` (previous/next tab)

#### Scenario: sections are separated by a blank line
- **WHEN** the help modal is rendered
- **THEN** a blank line appears between the Global and Left pane sections, and between the Left pane and Right pane sections

### Requirement: Section headings render as underlined, bold text
Each section heading's text SHALL render underlined and bold.

#### Scenario: heading text is underlined and bold
- **WHEN** the help modal renders a section heading
- **THEN** the heading's text is underlined and bold

### Requirement: The modal's content is always shown in full
The help modal SHALL always show every section's entries; there is nothing to expand or collapse.

#### Scenario: all entries are always visible
- **WHEN** the help modal is open
- **THEN** every entry in the Global, Left pane, and Right pane sections is present in its content, subject only to scrolling into and out of the visible area

### Requirement: The modal scrolls line by line
While the help modal is open, `j`/`k`/`↓`/`↑` SHALL scroll the modal's content by one line, and `Ctrl+d`/`Ctrl+u` SHALL scroll it by half a page. Scrolling SHALL clamp at the very first line and the very last line of content, both of which SHALL be reachable.

#### Scenario: j/k and arrows scroll by one line
- **WHEN** the help modal is open
- **THEN** pressing `j`, `k`, the down arrow, or the up arrow moves the visible content by exactly one line in the corresponding direction

#### Scenario: Ctrl+d/Ctrl+u scroll by half a page
- **WHEN** the help modal is open
- **THEN** pressing `Ctrl+d` or `Ctrl+u` moves the visible content by half the modal's visible height, in lines

#### Scenario: the first line is always reachable
- **WHEN** the help modal is open and its content is scrolled anywhere below the top
- **THEN** scrolling up enough reaches the very first line of content

#### Scenario: scrolling clamps at the last line
- **WHEN** the help modal's content is already scrolled to its last line
- **THEN** further downward scrolling has no effect

### Requirement: Only modal-recognized keys have any effect while the modal is open
While the help modal is open, keys other than `?`, `Esc`, `q`, `j`/`k`/`↓`/`↑`, and `Ctrl+d`/`Ctrl+u` SHALL have no effect — in particular `h`/`l`/`←`/`→`, `[`, `]`, `Tab`, `^`/`Home`, `$`/`End`, `Enter`, and `Space` SHALL NOT reach whichever pane held focus underneath the modal, and SHALL NOT change focus, scroll position, tab selection, or collapse state in either pane.

#### Scenario: pane-scroll and expand/collapse keys are inert
- **WHEN** the help modal is open and the user presses `h`, `l`, `[`, or `]`
- **THEN** neither pane's state changes, and the key has no visible effect beyond whatever the modal itself does with it

#### Scenario: Tab does not switch pane focus while the modal is open
- **WHEN** the help modal is open and the user presses `Tab`
- **THEN** focus does not move between the left and right panes

#### Scenario: closing the modal returns to the unaffected pane state
- **WHEN** the help modal is closed after being open
- **THEN** the left and right panes show the same focus, selection, scroll, tab, and collapse state they had the moment the modal opened

### Requirement: The modal is sized to its content, capped to the available frame, at a fixed width
The help modal's popup SHALL be no taller than its own content requires, up to the height available in the current terminal frame. When its content is shorter than the available frame height, the popup SHALL render at exactly its content's height, not the frame's full height. When its content is taller than the available frame height, the popup SHALL render at the available height and scroll vertically, indicating scroll position with a vertical scrollbar shown only while the content overflows. The popup's width SHALL be constant and SHALL NOT vary with scroll position.

#### Scenario: content fits within the frame
- **WHEN** the help modal's full content fits within the terminal's height
- **THEN** the popup renders at exactly the height its content needs, not larger

#### Scenario: content exceeds the frame
- **WHEN** the help modal's full content is taller than the space available in the terminal frame
- **THEN** the popup renders at the available height and its content scrolls vertically, with a scrollbar indicating the current position

#### Scenario: scrollbar hidden when content fits
- **WHEN** the help modal's content fits entirely within the popup
- **THEN** no vertical scrollbar is rendered

#### Scenario: width does not change while scrolling
- **WHEN** the help modal is open and its content is scrolled
- **THEN** the popup's width remains exactly what it was before scrolling

### Requirement: The modal's content is padded on its left and right edges
The help modal SHALL render its content with equal padding between its border and its content on both the left and right edges, wide enough that the widest line's text never touches the border (see design.md for the specific amount).

#### Scenario: padding separates content from the border
- **WHEN** the help modal is open
- **THEN** there is at least one blank column between the modal's left border and its content, and at least one blank column between its content and its right border

#### Scenario: the widest line is never clipped by the padding
- **WHEN** the help modal renders its widest content line
- **THEN** the line's full text is visible, not truncated by the border or the padding around it

### Requirement: Both panes render without color while the modal is open
While the help modal is open, the left and right panes SHALL render with all color removed from their borders and content alike — including the focus-indicating border color and any color used to convey diff state or emphasis — leaving a neutral, uniform treatment, so it is visually unambiguous that the panes are inert background and the modal has input. Closing the modal SHALL restore each pane's normal coloring.

#### Scenario: pane content renders without color while the modal is open
- **WHEN** the help modal is open
- **THEN** neither pane's content uses color to convey state or emphasis

#### Scenario: pane borders render without color while the modal is open
- **WHEN** the help modal is open
- **THEN** neither pane's border renders with its usual focus-indicating color, whether or not that pane holds focus

#### Scenario: normal coloring restored on close
- **WHEN** the help modal closes
- **THEN** both panes' borders and content render with their normal coloring again

### Requirement: The modal renders with a border visually distinct from the panes
The help modal SHALL render with a border style that makes it unmistakably a modal overlay when open, visually distinct from either pane's border, so it is immediately obvious that the modal — not a pane — has appeared.

#### Scenario: modal is visually distinct as a modal
- **WHEN** the help modal is open
- **THEN** its border is visually distinct from the two panes' borders, making it immediately obvious that a modal is showing
