## MODIFIED Requirements

### Requirement: Archived changes sorted alphabetically, displayed with date
When expanded, the archived section SHALL list archived changes ordered by their `YYYY-MM-DD` date prefix descending (most recent date first). Archived changes sharing the same date SHALL be ordered by the timestamp of the commit that first introduced the change's directory in git history, descending (most recently introduced first); a change whose introducing commit cannot be resolved (for example, an uncommitted change, or no enclosing git repository) SHALL sort as more recent than any change whose introducing commit can be resolved. Archived changes that remain tied after applying both the date and commit-timestamp comparisons SHALL be ordered by their full directory name (date prefix included), ascending. Each archived change SHALL be displayed as its date followed by its change name (date prefix removed from the name portion), with the date rendered in a visually de-emphasized (dimmed) style relative to the change name.

#### Scenario: alphabetical order matches chronological order
- **WHEN** the repo has archived changes `2026-01-03-foo` and `2026-06-19-bar`
- **THEN** the left pane lists `2026-06-19 bar` before `2026-01-03 foo` (most recent date first)

#### Scenario: same-date tie broken by introducing commit, most recent first
- **WHEN** two archived changes share the same date prefix but their directories were introduced by different commits
- **THEN** the change whose directory was introduced by the more recent commit is listed first

#### Scenario: unresolvable introducing commit sorts as most recent
- **WHEN** an archived change shares its date with another archived change, but the first change's introducing commit cannot be resolved (for example, its directory is uncommitted, or there is no enclosing git repository)
- **THEN** the change with no resolvable introducing commit is listed before the one whose introducing commit is resolvable

#### Scenario: unresolvable timestamp does not remove the change from the list
- **WHEN** an archived change's introducing commit cannot be resolved
- **THEN** the change still appears in the archived list

#### Scenario: directory name is the final tiebreaker, ascending
- **WHEN** two archived changes share the same date and the same (or equally unresolvable) introducing-commit timestamp
- **THEN** they are ordered by their full directory name, ascending

#### Scenario: date is visually de-emphasized
- **WHEN** an archived change row is rendered
- **THEN** the date portion is styled distinctly (dimmed) from the change name portion

#### Scenario: malformed archive name has no date to show
- **WHEN** an archived change's directory name does not have a well-formed date prefix
- **THEN** the left pane displays it using its change name only, with no date shown
