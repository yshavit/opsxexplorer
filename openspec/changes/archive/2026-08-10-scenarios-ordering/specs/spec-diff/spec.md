## RENAMED Requirements

- FROM: `### Requirement: A modified requirement's scenarios are matched by name and ordered base-first`
- TO: `### Requirement: A modified requirement's scenarios are matched by name and ordered by category`

## MODIFIED Requirements

### Requirement: A modified requirement's scenarios are matched by name and ordered by category
The system SHALL match a modified entry's scenarios to the spec of record's scenarios by scenario name, independently of their position in either document. Each matched pair SHALL be reported as unchanged when the two bodies are equal, and as changed or replaced otherwise (by the same legibility judgement applied to any other piece), carrying the comparison of the two bodies. A scenario present only in the delta entry SHALL be reported as added. A scenario present only in the spec of record and not restated by the delta entry SHALL be reported as unmentioned, carrying the spec of record's content.

Scenarios SHALL be reported grouped by category, in the fixed order added, then modified (changed or replaced), then unmentioned, then unchanged, so that the pieces most relevant to a reviewer are surfaced first and the least relevant last. Within a category, ties SHALL be broken by the delta entry's order for the added category, and by the spec of record's order for every other category. Comparing the same modified entry against the same spec of record more than once SHALL produce the same order each time.

#### Scenario: scenario restated with an edit

- **WHEN** a modified entry restates a scenario under a name the spec of record also uses, with a different body
- **THEN** that scenario is reported as changed, carrying the comparison of the two bodies

#### Scenario: scenario restated unchanged

- **WHEN** a modified entry restates a scenario with a body equal to the spec of record's
- **THEN** that scenario is reported as unchanged

#### Scenario: scenario new in the delta

- **WHEN** a modified entry contains a scenario whose name the spec of record's requirement does not use
- **THEN** that scenario is reported as added

#### Scenario: reported order is base order then delta-only

- **WHEN** a modified entry restates every scenario it shares with the spec of record and adds no new scenarios, listing the shared ones in an order different from the spec of record
- **THEN** the scenarios are reported in the spec of record's order, since none of them fall in the added category

#### Scenario: scenarios are grouped by category

- **WHEN** a modified entry has some scenarios added, some changed, some left unmentioned, and some unchanged
- **THEN** the added scenarios are reported first, followed by the changed or replaced scenarios, then the unmentioned scenarios, then the unchanged scenarios

#### Scenario: added scenarios within their category follow the delta's order

- **WHEN** a modified entry adds several scenarios not present in the spec of record
- **THEN** those scenarios are reported in the order the delta entry lists them, ahead of every other category

#### Scenario: modified scenarios within their category follow the spec of record's order

- **WHEN** a modified entry restates several scenarios with edited bodies, listing them in an order different from the spec of record
- **THEN** the changed or replaced scenarios are reported in the spec of record's order, not the delta entry's order

#### Scenario: unmentioned scenarios within their category follow the spec of record's order

- **WHEN** a modified entry omits several of the spec of record's scenarios
- **THEN** the unmentioned scenarios are reported in the spec of record's order

#### Scenario: unchanged scenarios within their category follow the spec of record's order

- **WHEN** a modified entry restates several scenarios with bodies equal to the spec of record's, in an order different from the spec of record
- **THEN** the unchanged scenarios are reported in the spec of record's order, not the delta entry's order

#### Scenario: repeated comparison is stable

- **WHEN** the same modified entry is compared against the same spec of record twice
- **THEN** both comparisons report the scenarios in the same category order and the same order within each category

### Requirement: A removed requirement's content is recovered from the spec of record
A delta's removal entry names a requirement without restating its content, but the user is shown what is being removed. The system SHALL take a removed requirement's intro and scenarios from the spec of record and SHALL report all of them as pure deletions, in the spec of record's order.

#### Scenario: removal shows the base's content

- **WHEN** a delta removes a requirement that the spec of record defines with an intro and several scenarios
- **THEN** that intro and those scenarios are reported as deleted content, taken from the spec of record

#### Scenario: removal reports no content of its own

- **WHEN** a removal entry is compared
- **THEN** nothing from the delta entry's own body contributes to the reported content, since a removal entry has none

#### Scenario: removed scenarios follow the spec of record's order

- **WHEN** a delta removes a requirement whose spec of record scenarios are all in the deleted category alike
- **THEN** those scenarios are reported in the spec of record's order
