## MODIFIED Requirements

### Requirement: Only collapsible rows are selectable
The right pane SHALL place its cursor only on rows that can be collapsed or expanded — requirement rows and scenario rows — and SHALL skip group headings and notice rows when moving the cursor, so that every cursor position is one where the toggle keys do something, or is a deliberate exception documented below. A scenario's body remains skipped: it is expanded or collapsed only via its own scenario row, and carries no collapse state of its own. The purpose row, a requirement's intro row, each of a removed requirement's removal-note line rows, and the row listing a capability's unrecognised sections are each a deliberate exception: whenever a capability has a purpose comparison, a requirement is expanded and therefore shows its intro row, a removed requirement is expanded and its removal note has one or more lines, or a capability carries unrecognised sections, that row SHALL be selectable regardless of whether it is itself collapsible, so that moving the cursor row-by-row reaches every visible row without a gap — this is also what lets the pane's cursor-driven scrolling bring the row into view. The purpose heading and the "Unknown sections" heading SHALL both still be skipped, like any other group heading.

#### Scenario: cursor skips a group heading
- **WHEN** the cursor moves across an operation group heading
- **THEN** it lands on the next requirement row rather than on the heading

#### Scenario: cursor skips content rows
- **WHEN** the cursor moves through an expanded scenario's body
- **THEN** it lands on the next requirement or scenario row rather than on the content

#### Scenario: cursor skips the purpose heading
- **WHEN** the cursor moves across the purpose heading
- **THEN** it lands on the purpose row rather than on the heading

#### Scenario: cursor skips the "Unknown sections" heading
- **WHEN** the cursor moves across the "Unknown sections" heading
- **THEN** it lands on the row listing the unrecognised section titles rather than on the heading

#### Scenario: cursor stops on a purpose row even when it fits without truncation
- **WHEN** the capability's purpose text fits within the row's available width with no truncation needed
- **THEN** the cursor still stops on the purpose row when moving row-by-row

#### Scenario: cursor stops on an intro row even when it fits without truncation
- **WHEN** an expanded requirement's intro text fits within the row's available width with no truncation needed
- **THEN** the cursor still stops on the intro row when moving row-by-row

#### Scenario: cursor stops on a removal-note line row even when it fits without truncation
- **WHEN** an expanded removed requirement's removal-note line text fits within the row's available width with no truncation needed
- **THEN** the cursor still stops on that removal-note line row when moving row-by-row

#### Scenario: cursor stops on the unrecognised-sections row
- **WHEN** the selected tab's capability carries unrecognised sections
- **THEN** the cursor can stop on the row listing them when moving row-by-row, including via scrolling to it

#### Scenario: toggling a non-collapsible purpose row has no effect
- **WHEN** the cursor is on a purpose row that fits within its available width and the user presses Enter, Space, `l` or `h`
- **THEN** nothing changes: the row has no collapsed or expanded state to toggle

#### Scenario: toggling a non-collapsible intro row has no effect
- **WHEN** the cursor is on an intro row that fits within its available width and the user presses Enter, Space, `l` or `h`
- **THEN** nothing changes: the row has no collapsed or expanded state to toggle

#### Scenario: toggling a non-collapsible removal-note line row has no effect
- **WHEN** the cursor is on a removal-note line row that fits within its available width and the user presses Enter, Space, `l` or `h`
- **THEN** nothing changes: the row has no collapsed or expanded state to toggle

#### Scenario: toggling the unrecognised-sections row has no effect
- **WHEN** the cursor is on the row listing a capability's unrecognised sections and the user presses Enter, Space, `l` or `h`
- **THEN** nothing changes: the row has no collapsed or expanded state to toggle

### Requirement: Content is rendered as text, uniformly, with no markdown formatting applied
Rendering markdown would strip the very markup that the word-level comparison addresses, so styled markdown and word-level highlighting cannot both be applied to a diffed passage. The pane SHALL render every requirement's intro and scenario bodies as plain text with only diff and state styling applied, whether or not that content sits inside a changed passage, so that the same content looks the same in every position. Two deliberate exceptions apply: a scenario body's leading `WHEN`/`THEN`/`AND` bullet keyword (see "WHEN/THEN/AND bullet keywords render without markdown asterisks"), and a removed requirement's removal-note line's leading `Reason`/`Migration` keyword (see "A removed requirement's removal note is shown as modification-styled lines above its deleted content"), both of which are rewritten rather than shown as literal `**` characters; no other markdown markup is exempted.

#### Scenario: markup inside a changed passage
- **WHEN** a changed passage contains markdown markup such as emphasis or list markers, other than a scenario body's leading `WHEN`/`THEN`/`AND` bullet keyword or a removal note's leading `Reason`/`Migration` keyword
- **THEN** the markup characters are shown as text and only diff styling is applied

#### Scenario: markup inside an unchanged passage
- **WHEN** an unchanged passage contains the same markdown markup, other than a scenario body's leading `WHEN`/`THEN`/`AND` bullet keyword or a removal note's leading `Reason`/`Migration` keyword
- **THEN** it is rendered the same way as it would be inside a changed passage

## ADDED Requirements

### Requirement: A removed requirement's removal note is shown as modification-styled lines above its deleted content
`spec-diff` reports a removed requirement's own body — its removal note — as a single block of text separate from the requirement's intro and scenario `Piece` comparisons (see `spec-diff`'s "A removed requirement's own body is carried through as a removal note"), since that text has no base counterpart to compare it against. OpenSpec's own authoring convention writes a removal's `**Reason**` and `**Migration**` as two lines with no blank line between them; CommonMark treats that as a single paragraph with a soft line break rather than as two paragraphs, so the parsed removal note carries them as one string with an internal line break, not as two separately-parsed paragraphs — a genuine paragraph break (a blank line in the source) is distinguishable from that soft break because it leaves a blank line in the parsed text, where a soft break does not. The right pane SHALL split a non-empty removal note by line and render each non-blank line as its own row directly above the requirement's intro row, in the order the lines appear, each carrying the pillcrow (¶) marker and modification styling — the same marker glyph and colour used for a modified operation and a changed piece — distinct from both the added and the deletion styling used elsewhere in the requirement. A blank line SHALL NOT produce a row of its own.

Following that convention, a line beginning with the markdown-bold form `**Reason**` or `**Migration**` SHALL be rendered with that keyword styled and shown without its surrounding `**` characters, the same way a scenario body's leading `WHEN`/`THEN`/`AND` bullet keyword is (see "WHEN/THEN/AND bullet keywords render without markdown asterisks"), with the rest of the line shown as-is. A line beginning with neither SHALL still be rendered as-is, with the same modification styling and paragraph-row formatting and no keyword stripped, so that a removal note that does not follow the Reason/Migration convention is not silently dropped.

Each removal-note line row SHALL follow the same fits-in-one-line-else-collapsible convention a requirement's intro row follows: it renders in full with no disclosure triangle when its text, right-trimmed of trailing whitespace, fits the row's available width, and is otherwise collapsible, starting collapsed the first time the requirement is expanded, showing a character-exact truncated excerpt ending in an ellipsis while collapsed and the full line, wrapped, when expanded. A removed requirement whose delta entry carries no body SHALL render no removal-note rows, leaving the requirement's intro row as the first row shown, exactly as before this requirement existed.

#### Scenario: a Reason line renders above the intro
- **WHEN** an expanded removed requirement's removal note has a line beginning `**Reason**`
- **THEN** the pane shows a row for that line directly above the requirement's intro row, with `Reason` styled and shown without its surrounding `**` characters

#### Scenario: a Migration line renders above the intro
- **WHEN** an expanded removed requirement's removal note has a line beginning `**Migration**`
- **THEN** the pane shows a row for that line directly above the requirement's intro row, with `Migration` styled and shown without its surrounding `**` characters

#### Scenario: Reason and Migration render as separate rows despite sharing one markdown paragraph
- **WHEN** a removal note's `**Reason**` and `**Migration**` lines have no blank line between them in the source, as OpenSpec's own convention writes them
- **THEN** the pane still shows them as two separate rows, in the order they appear, rather than as one row containing both

#### Scenario: a genuine paragraph break does not create an empty row
- **WHEN** a removal note contains a blank line separating two genuine paragraphs
- **THEN** no row is rendered for the blank line itself, and the lines before and after it each still render as their own rows

#### Scenario: removal-note rows use modification styling, not removal or insertion styling
- **WHEN** a removal-note line row is rendered
- **THEN** it carries the same marker glyph and colour used for a modified operation and a changed piece, distinct from the added styling and from the deletion styling of the intro and scenario rows below it

#### Scenario: an unrecognised line still renders
- **WHEN** a removal note contains a line beginning with neither `**Reason**` nor `**Migration**`
- **THEN** that line is still rendered as its own row, with the same modification styling and paragraph-row formatting, and no keyword is stripped from it

#### Scenario: a bare removal shows no removal-note rows
- **WHEN** an expanded removed requirement's diff carries no removal note
- **THEN** no removal-note rows are rendered and the requirement's intro row is the first row shown

#### Scenario: a short removal-note line renders in full
- **WHEN** a removal-note line, right-trimmed of trailing whitespace, fits within the row's available width
- **THEN** the row shows the full line on one line, with no disclosure triangle and no ellipsis

#### Scenario: a long removal-note line is collapsible and starts collapsed
- **WHEN** a removal-note line's trimmed text is longer than the row's available width
- **THEN** the row is collapsible, starts collapsed the first time the requirement is expanded, and renders a character-exact truncated excerpt ending in an ellipsis while collapsed

#### Scenario: expanding reveals the full removal-note line
- **WHEN** a collapsed removal-note line row is expanded
- **THEN** the full line is shown, wrapped to the pane width
