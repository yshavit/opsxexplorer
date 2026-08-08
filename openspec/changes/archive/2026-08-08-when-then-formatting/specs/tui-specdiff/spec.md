## ADDED Requirements

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

## MODIFIED Requirements

### Requirement: Content is rendered as text, uniformly, with no markdown formatting applied
Rendering markdown would strip the very markup that the word-level comparison addresses, so styled markdown and word-level highlighting cannot both be applied to a diffed passage. The pane SHALL render every requirement's intro and scenario bodies as plain text with only diff and state styling applied, whether or not that content sits inside a changed passage, so that the same content looks the same in every position. The one deliberate exception is a scenario body's leading `WHEN`/`THEN` bullet keyword, which is rewritten rather than shown as literal `**` characters (see "WHEN/THEN bullet keywords render without markdown asterisks"); no other markdown markup is exempted.

#### Scenario: markup inside a changed passage
- **WHEN** a changed passage contains markdown markup such as emphasis or list markers, other than a scenario body's leading `WHEN`/`THEN` bullet keyword
- **THEN** the markup characters are shown as text and only diff styling is applied

#### Scenario: markup inside an unchanged passage
- **WHEN** an unchanged passage contains the same markdown markup, other than a scenario body's leading `WHEN`/`THEN` bullet keyword
- **THEN** it is rendered the same way as it would be inside a changed passage
