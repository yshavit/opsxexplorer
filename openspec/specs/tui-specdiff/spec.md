# tui-specdiff Specification

## Purpose

Defines the right pane: how a selected change's computed spec diff is presented — one tab per capability, a collapsible tree of requirement diffs with word-level inline highlighting, and the pane's empty and error states.

## Requirements

### Requirement: The right pane shows the spec diff of the change selected in the left pane
The right pane SHALL display the per-requirement spec diff of whichever change is currently selected in the left pane, and SHALL update to the newly selected change whenever the selection moves. The pane SHALL derive its content afresh from the current selection rather than from content computed for an earlier selection.

#### Scenario: a change is selected
- **WHEN** the user selects a change in the left pane
- **THEN** the right pane shows that change's spec diff

#### Scenario: selection moves to a different change
- **WHEN** the user moves the left pane's cursor from one change to another
- **THEN** the right pane replaces its contents with the newly selected change's spec diff

#### Scenario: a non-change row is selected
- **WHEN** the left pane's cursor is on a row that is not a change, such as the archived section header or a placeholder row
- **THEN** the right pane shows an explanatory placeholder instead of a diff, and shows no tab bar

### Requirement: Each capability a change touches is presented as a tab
The right pane SHALL present one tab per capability the selected change carries a delta spec for, in the same stable alphabetical order the capabilities are enumerated in, and SHALL display the diff of exactly one capability — the selected tab — at a time. A tab bar SHALL be rendered whenever the change touches at least one capability, including when it touches exactly one, so that the pane's layout does not shift with the number of capabilities. When the selection moves to a different change, the first tab SHALL be selected.

#### Scenario: a change touching several capabilities
- **WHEN** the selected change carries delta specs for more than one capability
- **THEN** the pane shows one tab per capability in alphabetical order, with the first selected and its diff displayed

#### Scenario: a change touching one capability
- **WHEN** the selected change carries a delta spec for exactly one capability
- **THEN** the pane still shows a tab bar, containing that one tab

#### Scenario: switching to another change resets the tab
- **WHEN** the user has selected a later tab and then moves the left pane's selection to a different change
- **THEN** the first tab of the newly selected change is selected

### Requirement: Bracket keys switch between capability tabs
When the right pane holds focus, the system SHALL move the tab selection to the next capability on `]` and to the previous capability on `[`. Tab selection SHALL stop at the ends rather than wrapping around, and SHALL have no effect when the change touches a single capability.

#### Scenario: moving to the next tab
- **WHEN** the right pane holds focus, a tab other than the last is selected, and the user presses `]`
- **THEN** the next capability's tab is selected and its diff is displayed

#### Scenario: moving to the previous tab
- **WHEN** the right pane holds focus, a tab other than the first is selected, and the user presses `[`
- **THEN** the previous capability's tab is selected and its diff is displayed

#### Scenario: pressing past the last tab
- **WHEN** the last tab is selected and the user presses `]`
- **THEN** the last tab remains selected

#### Scenario: pressing before the first tab
- **WHEN** the first tab is selected and the user presses `[`
- **THEN** the first tab remains selected

### Requirement: Requirements are shown grouped under operation headings, in the order the diff reports them
Within a tab, the pane SHALL render requirement entries in the order the computed diff reports them, and SHALL introduce each run of entries sharing an operation with a display-only group heading naming that operation. A group heading SHALL NOT be rendered for an operation the capability's diff has no entries for.

#### Scenario: a capability with several operations
- **WHEN** a capability's diff contains added, modified and removed entries
- **THEN** the pane shows an added group heading followed by its entries, then a modified group heading followed by its entries, then a removed group heading followed by its entries

#### Scenario: an operation with no entries
- **WHEN** a capability's diff contains no entries for an operation
- **THEN** no group heading for that operation is shown

### Requirement: Every requirement row carries a gutter marker for its operation
Because a requirement can be scrolled away from its group heading, each requirement row SHALL be self-identifying: it SHALL carry a marker in a fixed gutter column, distinct per operation — added, modified, removed and renamed each having their own marker — so that the operation is readable from the row alone.

#### Scenario: an added requirement
- **WHEN** a requirement was added by the delta
- **THEN** its row carries the added marker in the gutter

#### Scenario: a modified requirement
- **WHEN** a requirement was modified by the delta
- **THEN** its row carries the modified marker in the gutter

#### Scenario: a removed requirement
- **WHEN** a requirement was removed by the delta
- **THEN** its row carries the removed marker in the gutter

#### Scenario: a renamed requirement
- **WHEN** a requirement was renamed by the delta
- **THEN** its row carries the renamed marker in the gutter, distinct from the other three

#### Scenario: a requirement scrolled away from its heading
- **WHEN** a requirement's group heading has scrolled out of view
- **THEN** the requirement's own row still identifies its operation

### Requirement: A renamed requirement shows both names, in one entry
A renamed requirement SHALL be shown once, displaying both its former name and its new name, rather than as two entries or as a modification whose name happens to differ. Where the same delta renames a requirement and also changes its content, that content comparison SHALL be shown under the single renamed entry.

#### Scenario: rename alone
- **WHEN** a requirement is renamed and nothing else about it changed
- **THEN** one row is shown carrying both the former and the new name

#### Scenario: rename combined with a modification
- **WHEN** a requirement is renamed and its content also changed
- **THEN** one row is shown carrying both names, and expanding it reveals the content comparison

### Requirement: A requirement's intro and each of its scenarios carry a marker for their own state
The state of a requirement's content is reported below the requirement level, and does not follow from the requirement's operation: within one modified requirement, one scenario may be unchanged, another changed, another added, and another present only in the spec of record. The pane SHALL therefore give the intro block and each scenario its own marker in a gutter column, reflecting that piece's own state rather than inheriting the requirement's. A piece present in the spec of record and unmentioned by the delta SHALL carry a marker distinct from the removed marker, since the two mean different things, and SHALL NOT be styled as a removal.

#### Scenario: mixed states within one modified requirement
- **WHEN** a modified requirement has an unchanged scenario, a changed scenario, an added scenario and a scenario present only in the spec of record
- **THEN** each of those scenario rows carries a marker for its own state, and they are not all shown as modified

#### Scenario: unmentioned is not shown as removal
- **WHEN** a scenario is present in the spec of record and unmentioned by the delta
- **THEN** its marker is distinct from the removed marker and it is not styled as removed content

### Requirement: An unmentioned intro is visibly distinguished from an unchanged one
An intro block that the delta does not mention renders the spec of record's text — the same text an unchanged intro renders — so without a distinguishing treatment the delta's silence would be invisible. The pane SHALL therefore mark an unmentioned intro block with the unmentioned marker in the gutter **and** render its text in a de-emphasised style, so it cannot be mistaken for content the delta restated.

#### Scenario: intro the delta did not mention
- **WHEN** a modified requirement's intro is unmentioned by the delta
- **THEN** the intro block carries the unmentioned marker and its text is de-emphasised

#### Scenario: intro the delta restated unchanged
- **WHEN** a modified requirement's intro is restated by the delta and is unchanged
- **THEN** the intro block does not carry the unmentioned marker and its text is not de-emphasised

### Requirement: Changed content is shown as one inline word-level diff, not as before-and-after blocks
For a piece whose content changed and whose difference is reported as word-level runs, the pane SHALL render a single reflowed passage in which deleted and inserted runs are interleaved in place, each styled distinctly from unchanged text and from each other, rather than rendering the spec of record's text and the delta's text as two separate blocks. Deleted and inserted text SHALL both be visible in that one passage.

This applies to every piece reported with runs. A piece reported instead as a wholesale replacement — because the two texts are too dissimilar for an inline reading to help — is rendered as stacked before-and-after text (see "A wholesale replacement is shown as stacked before-and-after text"). The pane SHALL NOT make that judgement itself: it renders whichever form the piece was reported in.

#### Scenario: a sentence appended to a long paragraph
- **WHEN** a changed piece is the spec of record's text with one further sentence appended
- **THEN** the passage is shown once, with only the appended sentence styled as an insertion

#### Scenario: an edit in the middle of a line
- **WHEN** a changed piece differs from the spec of record's only in a few words
- **THEN** those words are shown as a deletion and an insertion in place, with the surrounding text shown unchanged and shown only once

#### Scenario: both sides are visible
- **WHEN** a piece's content is reported with runs
- **THEN** both the removed text and the added text are visible in the rendered passage

### Requirement: A wholesale replacement is shown as stacked before-and-after text
A piece reported as a wholesale replacement has no runs to interleave, and the two texts are by construction too dissimilar for interleaving to have helped. The pane SHALL render such a piece as the spec of record's text styled as a deletion, followed by the delta's text styled as an insertion, each beginning on its own line so the two read as consecutive passages rather than as one run-on sentence. Both texts SHALL be shown in full.

The two texts SHALL carry the same deletion and insertion styling that a deleted run and an inserted run carry inside an inline diff, so that the colours mean the same thing everywhere in the pane. Each text SHALL wrap to the pane width under the same rules as any other content, and the piece SHALL carry the same gutter marker as any other changed piece, since a replacement is a modification and not a removal.

#### Scenario: a replaced piece renders both texts
- **WHEN** a piece is reported as a wholesale replacement
- **THEN** the spec of record's text is shown in full styled as a deletion, and the delta's text is shown in full styled as an insertion

#### Scenario: the two texts do not run together
- **WHEN** a replaced piece is rendered
- **THEN** the delta's text begins on a line of its own rather than continuing the line the spec of record's text ended on

#### Scenario: a replaced piece is marked as modified
- **WHEN** a replaced piece is rendered
- **THEN** its gutter marker is the one a changed piece carries, not the one a removed piece carries

#### Scenario: a long replaced piece wraps
- **WHEN** either text of a replaced piece is wider than the pane
- **THEN** it wraps to the pane width, keeping its styling across the break, with no horizontal scrolling required

### Requirement: Content wraps to the pane width and is never scrolled horizontally
The right pane SHALL wrap content to its available width rather than clipping it or offering horizontal scrolling, so that no part of a requirement's text is unreachable at any pane width. Wrapping SHALL preserve the styling of the text it breaks, including a word-diff run that straddles a wrap point. A row that wraps SHALL have its continuation lines indented to align beneath the start of the row's own text, and the gutter column SHALL be left blank on continuation lines so that a marker is never mistaken for a second entry.

#### Scenario: a line longer than the pane
- **WHEN** a requirement's text is wider than the pane
- **THEN** it is wrapped across multiple lines and all of it is readable without scrolling sideways

#### Scenario: a long requirement name
- **WHEN** a requirement's name is wider than the pane
- **THEN** the name wraps like body text rather than being truncated

#### Scenario: styling survives a wrap
- **WHEN** an inserted or deleted run spans a wrap point
- **THEN** the text on both sides of the break keeps that run's styling

#### Scenario: continuation lines are aligned and ungutter-marked
- **WHEN** a row with a gutter marker wraps onto further lines
- **THEN** those lines start beneath the first line's text and carry no gutter marker

### Requirement: Content is rendered as text, uniformly, with no markdown formatting applied
Rendering markdown would strip the very markup that the word-level comparison addresses, so styled markdown and word-level highlighting cannot both be applied to a diffed passage. The pane SHALL render every requirement's intro and scenario bodies as plain text with only diff and state styling applied, whether or not that content sits inside a changed passage, so that the same content looks the same in every position. The one deliberate exception is a scenario body's leading `WHEN`/`THEN` bullet keyword, which is rewritten rather than shown as literal `**` characters (see "WHEN/THEN bullet keywords render without markdown asterisks"); no other markdown markup is exempted.

#### Scenario: markup inside a changed passage
- **WHEN** a changed passage contains markdown markup such as emphasis or list markers, other than a scenario body's leading `WHEN`/`THEN` bullet keyword
- **THEN** the markup characters are shown as text and only diff styling is applied

#### Scenario: markup inside an unchanged passage
- **WHEN** an unchanged passage contains the same markdown markup, other than a scenario body's leading `WHEN`/`THEN` bullet keyword
- **THEN** it is rendered the same way as it would be inside a changed passage

### Requirement: WHEN/THEN bullet keywords render without markdown asterisks
A scenario body bullet whose text begins with the markdown-bold form `**WHEN**` or `**THEN**` SHALL be rendered with that keyword styled (not as literal `**` characters) rather than showing the surrounding asterisks. The keyword's rendered style SHALL come from a single, dedicated styling definition, so that the visual treatment (bold, or something else) can be changed without touching the rest of the scenario-body rendering path.

This applies only to the leading `WHEN` / `THEN` bullet keyword. Other markdown emphasis appearing elsewhere in requirement or scenario text is unaffected by this requirement and continues to render as literal characters, consistent with this pane's existing decision not to render markdown generally.

#### Scenario: a scenario body's WHEN bullet
- **WHEN** a scenario's body contains a bullet beginning `- **WHEN** ...`
- **THEN** the rendered row shows `WHEN` styled, with no `**` characters around it

#### Scenario: a scenario body's THEN bullet
- **WHEN** a scenario's body contains a bullet beginning `- **THEN** ...`
- **THEN** the rendered row shows `THEN` styled, with no `**` characters around it

#### Scenario: styling does not disturb word-level diff highlighting
- **WHEN** a scenario body is a changed piece whose word-level diff run boundary falls within or after a `**WHEN**` or `**THEN**` bullet
- **THEN** the insertion/deletion styling for that run is shown correctly, unaffected by the keyword's own styling

#### Scenario: bold text elsewhere is untouched
- **WHEN** requirement or scenario text contains `**bold**` emphasis that is not a leading `WHEN`/`THEN` bullet keyword
- **THEN** it continues to render with its literal `**` characters, unchanged by this requirement

### Requirement: Requirements and scenarios are collapsible, and collapsed by default
The pane SHALL open with every requirement collapsed, so that a capability's diff first reads as a one-row-per-requirement summary. Expanding a requirement SHALL reveal its intro block and its scenario headers, with every scenario collapsed. Expanding a scenario SHALL reveal that scenario's body. Collapsing SHALL restore the prior state. A row's expanded or collapsed state SHALL be indicated on the row itself.

#### Scenario: pane opens collapsed
- **WHEN** a capability's diff is first displayed
- **THEN** each requirement is shown as a single collapsed row, with no intro block or scenario rows visible

#### Scenario: expanding a requirement
- **WHEN** a collapsed requirement is expanded
- **THEN** its intro block and its scenario headers become visible, and its scenarios are collapsed

#### Scenario: expanding a scenario
- **WHEN** a collapsed scenario is expanded
- **THEN** its body becomes visible

#### Scenario: collapsing a requirement
- **WHEN** an expanded requirement is collapsed
- **THEN** its intro block and scenario rows are hidden again

### Requirement: Only collapsible rows are selectable
The right pane SHALL place its cursor only on rows that can be collapsed or expanded — requirement rows and scenario rows — and SHALL skip group headings, intro blocks, scenario bodies and notice rows when moving the cursor, so that every cursor position is one where the toggle keys do something.

#### Scenario: cursor skips a group heading
- **WHEN** the cursor moves across an operation group heading
- **THEN** it lands on the next requirement row rather than on the heading

#### Scenario: cursor skips content rows
- **WHEN** the cursor moves through an expanded requirement's intro block or an expanded scenario's body
- **THEN** it lands on the next requirement or scenario row rather than on the content

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

### Requirement: A change with no delta specs shows an explanatory message, not an error
A change directory that carries no delta specs is a normal state, not a failure. The right pane SHALL render an explanatory message for such a change and SHALL NOT render a tab bar, an empty tree, or an error notice.

#### Scenario: change with no delta specs
- **WHEN** the selected change carries no delta specs
- **THEN** the pane shows a message saying the change has no spec changes, with no tab bar and no error

### Requirement: A capability whose specs cannot be read shows an error notice in its own tab
Failure to load or parse one capability's specs SHALL be reported as a notice inside that capability's tab, naming the capability and describing the failure, and SHALL leave the change's other tabs displaying normally. The pane SHALL NOT crash, blank itself, or drop the tab.

#### Scenario: one capability of several fails to load
- **WHEN** one capability's delta spec cannot be parsed and the change's other capabilities are sound
- **THEN** that capability's tab shows an error notice and the other tabs show their diffs normally

#### Scenario: an enumerated capability with no spec document
- **WHEN** a capability is enumerated for the change but has no spec document
- **THEN** its tab shows an error notice describing that

#### Scenario: a capability with no spec of record
- **WHEN** a capability's delta modifies or removes a requirement and the capability has no spec of record at the diff base
- **THEN** its tab shows an error notice describing that, distinct from the notice for an absent spec document

### Requirement: A requirement that cannot be diffed shows an error notice alongside the requirements that could
An entry naming a requirement absent from the spec of record fails on its own, and the capability's other requirements are still computed. The pane SHALL render such an error as a notice within the capability's tab, naming the requirement, **and** SHALL render the capability's successfully computed requirements in the same view.

#### Scenario: one entry names an unknown requirement
- **WHEN** one of a capability's entries names a requirement absent from the spec of record and its other entries are sound
- **THEN** the tab shows an error notice naming that requirement together with the tree of the requirements that were computed

### Requirement: Errors are located structurally, never by line number
Source positions are not available for the spec documents being read, so the pane SHALL locate an error by the structure it occurred in — the capability, the requirements section, and the requirement name where known — and SHALL NOT display a line or column number, which would imply a precision it does not have.

#### Scenario: a malformed delta spec
- **WHEN** a delta spec fails to parse under a named requirements section
- **THEN** the notice identifies the capability, the section and, where known, the requirement, and shows no line number
