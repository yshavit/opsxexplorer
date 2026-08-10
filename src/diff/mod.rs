mod compare;
mod error;
mod model;
mod runs;

pub use error::DiffError;
pub use model::{CapabilityDiff, Operation, Piece, RequirementDiff, Run, ScenarioDiff};

use std::collections::{HashMap, HashSet};

use crate::specs::{DeltaEntry, DeltaOp, Requirement, SpecPair};

/// Compares a change's delta entries for `capability` against the spec of
/// record `pair` supplies, producing the per-requirement diff. Pure: no file
/// reads, no git, no styling. Errors are per-entry, collected rather than
/// short-circuiting, so one bad entry never suppresses the requirements that
/// are fine (see `2026-08-08-spec-diff/design.md`).
pub fn diff(capability: &str, pair: &SpecPair) -> CapabilityDiff {
    let base_index = build_base_index(pair);

    let rename_targets: HashSet<&str> = pair.delta.renames.iter().map(|r| r.to.as_str()).collect();
    let mut modified_for_rename: HashMap<&str, &DeltaEntry> = HashMap::new();

    let mut requirements = Vec::new();
    let mut errors = Vec::new();

    // Added: emit directly, no base lookup.
    for entry in pair.delta.entries.iter().filter(|e| e.op == DeltaOp::Added) {
        requirements.push(RequirementDiff {
            name: entry.requirement.name.clone(),
            op: Operation::Added,
            intro: Piece::Added {
                delta: entry.requirement.intro.clone(),
            },
            scenarios: entry
                .requirement
                .scenarios
                .iter()
                .map(|s| ScenarioDiff {
                    name: s.name.clone(),
                    body: Piece::Added {
                        delta: s.body.clone(),
                    },
                })
                .collect(),
        });
    }

    // Modified: entries whose name is a rename's `to` are set aside for the
    // renamed group; the rest are compared against the base directly.
    for entry in pair
        .delta
        .entries
        .iter()
        .filter(|e| e.op == DeltaOp::Modified)
    {
        let name = entry.requirement.name.as_str();
        if rename_targets.contains(name) {
            modified_for_rename.entry(name).or_insert(entry);
            continue;
        }
        match base_index.get(name) {
            Some(base_req) => {
                let (intro, scenarios) = compare::compare_requirement(base_req, &entry.requirement);
                requirements.push(RequirementDiff {
                    name: name.to_string(),
                    op: Operation::Modified,
                    intro,
                    scenarios,
                });
            }
            None => errors.push(missing_base(capability, name)),
        }
    }

    // Removed: the delta entry has no body of its own; the base's intro and
    // scenarios are recovered and reported as deletions.
    for entry in pair
        .delta
        .entries
        .iter()
        .filter(|e| e.op == DeltaOp::Removed)
    {
        let name = entry.requirement.name.as_str();
        match base_index.get(name) {
            Some(base_req) => {
                requirements.push(RequirementDiff {
                    name: name.to_string(),
                    op: Operation::Removed,
                    intro: Piece::Deleted {
                        base: base_req.intro.clone(),
                    },
                    scenarios: base_req
                        .scenarios
                        .iter()
                        .map(|s| ScenarioDiff {
                            name: s.name.clone(),
                            body: Piece::Deleted {
                                base: s.body.clone(),
                            },
                        })
                        .collect(),
                });
            }
            None => errors.push(missing_base(capability, name)),
        }
    }

    // Renamed: compared against the MODIFIED entry set aside above, or
    // against an empty requirement when there is none — the uniform rule
    // then yields Unmentioned for the intro and every base scenario.
    for rename in &pair.delta.renames {
        let from = rename.from.as_str();
        let to = rename.to.as_str();
        match base_index.get(from) {
            Some(base_req) => {
                let empty = empty_requirement(to);
                let delta_req = modified_for_rename
                    .get(to)
                    .map_or(&empty, |e| &e.requirement);
                let (intro, scenarios) = compare::compare_requirement(base_req, delta_req);
                requirements.push(RequirementDiff {
                    name: to.to_string(),
                    op: Operation::Renamed {
                        from: from.to_string(),
                    },
                    intro,
                    scenarios,
                });
            }
            None => errors.push(missing_base(capability, from)),
        }
    }

    CapabilityDiff {
        capability: capability.to_string(),
        requirements,
        errors,
        purpose: purpose_diff(pair),
        unrecognized_sections: pair.delta.unrecognized_sections.clone(),
    }
}

/// Computes the capability's purpose comparison: `None` if the delta carries
/// no purpose section or restates the spec of record's purpose unchanged;
/// `Piece::Added` for an insertion (the spec of record's purpose is empty,
/// or there is no spec of record at all); otherwise the same
/// changed-vs-replaced judgement used for a requirement's intro. An
/// insertion is detected directly (an empty base) rather than routed through
/// `compare::changed_or_unchanged`, which never produces `Piece::Added` — it
/// treats an empty base exactly like an ordinary edit, all-`Insert` runs and
/// all (see spec-diff's "purpose is compared... using the same
/// changed-vs-replaced judgement" requirement, which reports an insertion as
/// its own outcome distinct from a changed/replaced comparison).
fn purpose_diff(pair: &SpecPair) -> Option<Piece> {
    let delta_purpose = pair.delta.purpose.as_deref()?;
    let base_purpose = pair
        .base
        .as_ref()
        .and_then(|s| s.purpose.as_deref())
        .unwrap_or_default();

    if base_purpose == delta_purpose {
        return None;
    }
    if base_purpose.is_empty() {
        return Some(Piece::Added {
            delta: delta_purpose.to_string(),
        });
    }
    match compare::changed_or_unchanged(base_purpose, delta_purpose) {
        Piece::Unchanged { .. } => None,
        piece => Some(piece),
    }
}

fn build_base_index(pair: &SpecPair) -> HashMap<&str, &Requirement> {
    let mut index = HashMap::new();
    if let Some(base) = &pair.base {
        for requirement in &base.requirements {
            index
                .entry(requirement.name.as_str())
                .or_insert(requirement);
        }
    }
    index
}

fn empty_requirement(name: &str) -> Requirement {
    Requirement {
        name: name.to_string(),
        intro: String::new(),
        scenarios: Vec::new(),
    }
}

fn missing_base(capability: &str, requirement: &str) -> DiffError {
    DiffError::MissingBaseRequirement {
        capability: capability.to_string(),
        requirement: requirement.to_string(),
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use std::path::Path;

    use crate::specs::SpecPair;
    use crate::specs::parse::{parse_delta, parse_spec};

    /// Builds a `SpecPair` from markdown source, so diff tests read as spec
    /// text rather than hand-constructed nested structs.
    pub(crate) fn pair_from_markdown(delta_md: &str, base_md: Option<&str>) -> SpecPair {
        let delta = parse_delta(Path::new("test-delta"), delta_md).unwrap();
        let base = base_md.map(|md| parse_spec(Path::new("test-base"), md).unwrap());
        SpecPair { delta, base }
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::pair_from_markdown;
    use super::*;

    // --- 4.8: ordering ---

    #[test]
    fn operations_are_grouped_added_modified_removed_renamed() {
        let base_md = "## Requirements\n\n\
            ### Requirement: To Modify\n\
            Intro.\n\n\
            #### Scenario: A\n\
            - **WHEN** a\n\
            - **THEN** a2\n\n\
            ### Requirement: To Remove\n\
            Intro.\n\n\
            #### Scenario: B\n\
            - **WHEN** b\n\
            - **THEN** b2\n\n\
            ### Requirement: Old Name\n\
            Intro.\n\n\
            #### Scenario: C\n\
            - **WHEN** c\n\
            - **THEN** c2\n";
        let delta_md = "## ADDED Requirements\n\n\
            ### Requirement: New One\n\
            #### Scenario: X\n\
            - **WHEN** x\n\
            - **THEN** x2\n\n\
            ## REMOVED Requirements\n\n\
            ### Requirement: To Remove\n\n\
            ## MODIFIED Requirements\n\n\
            ### Requirement: To Modify\n\
            Intro EDITED.\n\n\
            #### Scenario: A\n\
            - **WHEN** a\n\
            - **THEN** a2\n\n\
            ## RENAMED Requirements\n\n\
            - FROM: `### Requirement: Old Name`\n\
            - TO: `### Requirement: New Name`\n";
        let pair = pair_from_markdown(delta_md, Some(base_md));
        let result = diff("cap", &pair);
        assert!(result.errors.is_empty());
        let ops: Vec<&Operation> = result.requirements.iter().map(|r| &r.op).collect();
        assert_eq!(
            ops,
            vec![
                &Operation::Added,
                &Operation::Modified,
                &Operation::Removed,
                &Operation::Renamed {
                    from: "Old Name".to_string()
                },
            ]
        );
        assert_eq!(result.requirements[0].name, "New One");
        assert_eq!(result.requirements[1].name, "To Modify");
        assert_eq!(result.requirements[2].name, "To Remove");
        assert_eq!(result.requirements[3].name, "New Name");
    }

    #[test]
    fn additions_within_a_group_follow_delta_order() {
        let delta_md = "## ADDED Requirements\n\n\
            ### Requirement: First\n\
            #### Scenario: A\n\
            - **WHEN** a\n\
            - **THEN** a2\n\n\
            ### Requirement: Second\n\
            #### Scenario: B\n\
            - **WHEN** b\n\
            - **THEN** b2\n\n\
            ### Requirement: Third\n\
            #### Scenario: C\n\
            - **WHEN** c\n\
            - **THEN** c2\n";
        let pair = pair_from_markdown(delta_md, None);
        let result = diff("cap", &pair);
        let names: Vec<&str> = result
            .requirements
            .iter()
            .map(|r| r.name.as_str())
            .collect();
        assert_eq!(names, vec!["First", "Second", "Third"]);
    }

    // --- 4.9: determinism ---

    #[test]
    fn comparing_the_same_pair_twice_is_deterministic() {
        let base_md = "## Requirements\n\n\
            ### Requirement: Foo\n\
            Intro.\n\n\
            #### Scenario: A\n\
            - **WHEN** a\n\
            - **THEN** a2\n\n\
            #### Scenario: B\n\
            - **WHEN** b\n\
            - **THEN** b2\n";
        let delta_md = "## MODIFIED Requirements\n\n\
            ### Requirement: Foo\n\
            Intro EDITED.\n\n\
            #### Scenario: A\n\
            - **WHEN** a\n\
            - **THEN** a2\n\n\
            #### Scenario: C\n\
            - **WHEN** c\n\
            - **THEN** c2\n";
        let pair = pair_from_markdown(delta_md, Some(base_md));
        let first = diff("cap", &pair);
        let second = diff("cap", &pair);
        assert_eq!(first, second);
    }

    // --- 4.10: untouched requirements ---

    #[test]
    fn untouched_requirements_produce_no_entry() {
        let base_md = "## Requirements\n\n\
            ### Requirement: Touched\n\
            Intro.\n\n\
            #### Scenario: A\n\
            - **WHEN** a\n\
            - **THEN** a2\n\n\
            ### Requirement: Untouched\n\
            Intro.\n\n\
            #### Scenario: B\n\
            - **WHEN** b\n\
            - **THEN** b2\n";
        let delta_md = "## MODIFIED Requirements\n\n\
            ### Requirement: Touched\n\
            Intro EDITED.\n\n\
            #### Scenario: A\n\
            - **WHEN** a\n\
            - **THEN** a2\n";
        let pair = pair_from_markdown(delta_md, Some(base_md));
        let result = diff("cap", &pair);
        assert_eq!(result.requirements.len(), 1);
        assert_eq!(result.requirements[0].name, "Touched");
    }

    // --- 4.8a: duplicate names, first-wins ---

    #[test]
    fn duplicate_requirement_and_scenario_names_resolve_first_wins_without_panicking() {
        let base_md = "## Requirements\n\n\
            ### Requirement: Dup\n\
            First intro.\n\n\
            #### Scenario: S\n\
            - **WHEN** a\n\
            - **THEN** a2\n\n\
            ### Requirement: Dup\n\
            Second intro.\n\n\
            #### Scenario: S\n\
            - **WHEN** z\n\
            - **THEN** z2\n";
        let delta_md = "## MODIFIED Requirements\n\n\
            ### Requirement: Dup\n\
            First intro EDITED.\n\n\
            #### Scenario: S\n\
            - **WHEN** a\n\
            - **THEN** a2\n";
        let pair = pair_from_markdown(delta_md, Some(base_md));
        let result = diff("cap", &pair);
        assert!(result.errors.is_empty());
        assert_eq!(result.requirements.len(), 1);
        let req = &result.requirements[0];
        // Matched against the base's first "Dup" occurrence, not the second.
        match &req.intro {
            Piece::Changed { base, .. } => assert_eq!(base, "First intro."),
            other => panic!("expected Changed, got {other:?}"),
        }
        assert_eq!(req.scenarios.len(), 1);
    }

    // --- 5.1: missing-base errors per operation ---

    #[test]
    fn modified_entry_naming_unknown_requirement_is_a_displayable_error() {
        let base_md = "## Requirements\n\n### Requirement: Real\n#### Scenario: A\n- **WHEN** a\n- **THEN** a2\n";
        let delta_md = "## MODIFIED Requirements\n\n### Requirement: Typo'd Name\n#### Scenario: A\n- **WHEN** a\n- **THEN** a2\n";
        let pair = pair_from_markdown(delta_md, Some(base_md));
        let result = diff("cap", &pair);
        assert!(result.requirements.is_empty());
        assert_eq!(
            result.errors,
            vec![DiffError::MissingBaseRequirement {
                capability: "cap".to_string(),
                requirement: "Typo'd Name".to_string(),
            }]
        );
    }

    #[test]
    fn removed_entry_naming_unknown_requirement_is_a_displayable_error() {
        let base_md = "## Requirements\n\n### Requirement: Real\n#### Scenario: A\n- **WHEN** a\n- **THEN** a2\n";
        let delta_md = "## REMOVED Requirements\n\n### Requirement: Typo'd Name\n";
        let pair = pair_from_markdown(delta_md, Some(base_md));
        let result = diff("cap", &pair);
        assert!(result.requirements.is_empty());
        assert_eq!(
            result.errors,
            vec![DiffError::MissingBaseRequirement {
                capability: "cap".to_string(),
                requirement: "Typo'd Name".to_string(),
            }]
        );
    }

    #[test]
    fn renamed_entry_naming_unknown_former_name_is_a_displayable_error() {
        let base_md = "## Requirements\n\n### Requirement: Real\n#### Scenario: A\n- **WHEN** a\n- **THEN** a2\n";
        let delta_md = "## RENAMED Requirements\n\n\
            - FROM: `### Requirement: Typo'd Name`\n\
            - TO: `### Requirement: New Name`\n";
        let pair = pair_from_markdown(delta_md, Some(base_md));
        let result = diff("cap", &pair);
        assert!(result.requirements.is_empty());
        assert_eq!(
            result.errors,
            vec![DiffError::MissingBaseRequirement {
                capability: "cap".to_string(),
                requirement: "Typo'd Name".to_string(),
            }]
        );
    }

    // --- 5.2: partial success ---

    #[test]
    fn one_bad_entry_does_not_suppress_the_sound_ones() {
        let base_md = "## Requirements\n\n\
            ### Requirement: Good One\n\
            Intro.\n\n\
            #### Scenario: A\n\
            - **WHEN** a\n\
            - **THEN** a2\n\n\
            ### Requirement: Also Good\n\
            Intro.\n\n\
            #### Scenario: B\n\
            - **WHEN** b\n\
            - **THEN** b2\n";
        let delta_md = "## ADDED Requirements\n\n\
            ### Requirement: Brand New\n\
            #### Scenario: X\n\
            - **WHEN** x\n\
            - **THEN** x2\n\n\
            ## MODIFIED Requirements\n\n\
            ### Requirement: Good One\n\
            Intro EDITED.\n\n\
            #### Scenario: A\n\
            - **WHEN** a\n\
            - **THEN** a2\n\n\
            ### Requirement: Nonexistent\n\
            #### Scenario: Z\n\
            - **WHEN** z\n\
            - **THEN** z2\n\n\
            ## REMOVED Requirements\n\n\
            ### Requirement: Also Good\n";
        let pair = pair_from_markdown(delta_md, Some(base_md));
        let result = diff("cap", &pair);
        assert_eq!(
            result.errors,
            vec![DiffError::MissingBaseRequirement {
                capability: "cap".to_string(),
                requirement: "Nonexistent".to_string(),
            }]
        );
        let names: Vec<&str> = result
            .requirements
            .iter()
            .map(|r| r.name.as_str())
            .collect();
        assert_eq!(names, vec!["Brand New", "Good One", "Also Good"]);
    }

    // --- 6.3/6.4: real fixture, reproduced inline so it survives this
    // change's own archiving (see `2026-08-08-spec-diff/design.md` and
    // `2026-08-08-spec-diff/tasks.md` 6.3) ---

    #[test]
    fn horizontal_scrolling_change_diffs_as_expected_against_its_base() {
        let base_md = "## Requirements\n\n\
            ### Requirement: Archived changes are grouped under a collapsible section\n\
            The left pane SHALL show a single `archived` row after the active changes. The archived row SHALL be collapsed on launch. When expanded, it SHALL reveal the archived changes as rows beneath it; when collapsed, those rows SHALL NOT appear in the list.\n\n\
            #### Scenario: archived row collapsed by default\n\
            - **WHEN** the application starts\n\
            - **THEN** the `archived` row is present and collapsed, and no individual archived changes are shown\n\n\
            #### Scenario: expanding reveals archived changes\n\
            - **WHEN** the user expands the `archived` row\n\
            - **THEN** the archived changes appear as rows immediately beneath it\n\n\
            #### Scenario: collapsing hides archived changes\n\
            - **WHEN** the user collapses an expanded `archived` row\n\
            - **THEN** the archived changes beneath it no longer appear in the list\n";

        let delta_md = "## ADDED Requirements\n\n\
            ### Requirement: Left pane scrolls horizontally as a single unit\n\
            When any row's content is wider than the left pane, the user SHALL be able to scroll the pane's content horizontally. Scrolling SHALL apply a single offset to every row — active changes, the `archived` header, and archived changes alike — so all rows shift together and stay vertically aligned. The offset SHALL be moved one column at a time with `h`/`l` and the left/right arrow keys, and jumped to the leftmost or rightmost extent with `^`/Home and `$`/End respectively.\n\n\
            #### Scenario: scrolling right with l\n\
            - **WHEN** the cursor is anywhere in the left pane and the user presses `l`\n\
            - **THEN** the pane's content shifts one column to the left (revealing content further right), identically across all rows\n\n\
            #### Scenario: scrolling right with the right arrow key\n\
            - **WHEN** the user presses the right arrow key\n\
            - **THEN** the pane's content scrolls right by one column, identically to pressing `l`\n\n\
            #### Scenario: scrolling left with h\n\
            - **WHEN** the pane is scrolled right and the user presses `h`\n\
            - **THEN** the pane's content shifts one column back toward the left, identically across all rows\n\n\
            #### Scenario: scrolling left with the left arrow key\n\
            - **WHEN** the pane is scrolled right and the user presses the left arrow key\n\
            - **THEN** the pane's content scrolls left by one column, identically to pressing `h`\n\n\
            #### Scenario: jumping to the leftmost extent\n\
            - **WHEN** the pane is scrolled right and the user presses `^` (or Home)\n\
            - **THEN** the pane's content returns to its unscrolled, leftmost position\n\n\
            #### Scenario: jumping to the rightmost extent\n\
            - **WHEN** the user presses `$` (or End)\n\
            - **THEN** the pane scrolls to the furthest position needed to reveal the end of its widest row\n\n\
            #### Scenario: scrolling past content has no further effect\n\
            - **WHEN** the pane is already scrolled to its leftmost or rightmost extent and the user scrolls further in that direction\n\
            - **THEN** the scroll position does not move further\n\n\
            ### Requirement: Horizontal scroll position is indicated with a scrollbar\n\
            The left pane SHALL show a horizontal scrollbar reflecting the current scroll offset relative to the widest row's content. The scrollbar SHALL only indicate scrollable content when at least one row is wider than the pane.\n\n\
            #### Scenario: all rows fit within the pane\n\
            - **WHEN** every visible row's content fits within the left pane's width\n\
            - **THEN** the horizontal scrollbar shows no scrollable range\n\n\
            #### Scenario: a row is wider than the pane\n\
            - **WHEN** at least one visible row's content is wider than the left pane\n\
            - **THEN** the horizontal scrollbar reflects that there is additional content and shows the current scroll position within it\n\n\
            ### Requirement: Horizontal scroll offset persists across selection and section toggling, clamped to current content\n\
            The horizontal scroll offset SHALL NOT reset when the cursor moves to a different row or when the `archived` section is expanded or collapsed. The offset SHALL instead be clamped, at render time, to the maximum scroll needed for the currently visible rows at the pane's current width — so it self-corrects whenever the visible content or the pane's width changes, without an explicit reset.\n\n\
            #### Scenario: moving the cursor does not reset horizontal scroll\n\
            - **WHEN** the pane is scrolled right and the user moves the cursor to a different row\n\
            - **THEN** the pane remains scrolled to the same position\n\n\
            #### Scenario: scrolled past a shorter row's content\n\
            - **WHEN** the pane is scrolled right past the length of a given row's content\n\
            - **THEN** that row renders with no visible text, while rows with content reaching that far still show their content\n\n\
            #### Scenario: collapsing the archived section reduces the available scroll range\n\
            - **WHEN** the pane is scrolled right to reveal content only present in an expanded archived row, and the user collapses the `archived` section\n\
            - **THEN** the scroll offset is reduced to the maximum needed for the now-visible rows, if it exceeded that maximum\n\n\
            #### Scenario: widening the pane reduces the available scroll range\n\
            - **WHEN** the pane is scrolled right and the terminal is resized wider such that the previously-scrolled content now fits\n\
            - **THEN** the scroll offset is reduced accordingly, down to zero if all content now fits\n\n\
            ## MODIFIED Requirements\n\n\
            ### Requirement: Archived changes are grouped under a collapsible section\n\
            The left pane SHALL show a single `archived` row after the active changes. The archived row SHALL be collapsed on launch. When expanded, it SHALL reveal the archived changes as rows beneath it; when collapsed, those rows SHALL NOT appear in the list. While collapsed, the `archived` row SHALL render with an underline style; while expanded, it SHALL NOT.\n\n\
            #### Scenario: archived row collapsed by default\n\
            - **WHEN** the application starts\n\
            - **THEN** the `archived` row is present and collapsed, and no individual archived changes are shown\n\n\
            #### Scenario: expanding reveals archived changes\n\
            - **WHEN** the user expands the `archived` row\n\
            - **THEN** the archived changes appear as rows immediately beneath it\n\n\
            #### Scenario: collapsing hides archived changes\n\
            - **WHEN** the user collapses an expanded `archived` row\n\
            - **THEN** the archived changes beneath it no longer appear in the list\n\n\
            #### Scenario: collapsed row is underlined\n\
            - **WHEN** the `archived` row is collapsed\n\
            - **THEN** it renders with an underline style\n\n\
            #### Scenario: expanded row is not underlined\n\
            - **WHEN** the `archived` row is expanded\n\
            - **THEN** it renders without an underline style\n\n\
            #### Scenario: underline persists under horizontal scroll\n\
            - **WHEN** the `archived` row is collapsed and the pane is scrolled horizontally such that only part of the row's text is visible\n\
            - **THEN** the visible portion of the row's text still renders underlined\n";

        let pair = pair_from_markdown(delta_md, Some(base_md));
        let result = diff("tui-changelist", &pair);
        assert!(result.errors.is_empty());
        assert_eq!(result.requirements.len(), 4);

        // 6.4: the three ADDED requirements come first, in delta order, ahead of the modified one.
        assert_eq!(result.requirements[0].op, Operation::Added);
        assert_eq!(
            result.requirements[0].name,
            "Left pane scrolls horizontally as a single unit"
        );
        assert_eq!(result.requirements[1].op, Operation::Added);
        assert_eq!(
            result.requirements[1].name,
            "Horizontal scroll position is indicated with a scrollbar"
        );
        assert_eq!(result.requirements[2].op, Operation::Added);
        assert_eq!(
            result.requirements[2].name,
            "Horizontal scroll offset persists across selection and section toggling, clamped to current content"
        );

        let modified = &result.requirements[3];
        assert_eq!(modified.op, Operation::Modified);
        assert_eq!(
            modified.name,
            "Archived changes are grouped under a collapsible section"
        );

        // 6.3: intro is Changed with exactly one trailing Insert run and no Delete runs.
        match &modified.intro {
            Piece::Changed { runs, .. } => {
                assert!(
                    !runs.iter().any(|r| matches!(r, Run::Delete { .. })),
                    "expected no delete runs in the intro diff, got {runs:?}"
                );
                let insert_count = runs
                    .iter()
                    .filter(|r| matches!(r, Run::Insert { .. }))
                    .count();
                assert_eq!(
                    insert_count, 1,
                    "expected exactly one insert run, got {runs:?}"
                );
                assert!(
                    matches!(runs.last(), Some(Run::Insert { .. })),
                    "expected the insert run to trail the diff, got {runs:?}"
                );
            }
            other => panic!("expected the intro to be Changed, got {other:?}"),
        }

        // Added scenarios (delta order) are reported ahead of the unchanged
        // ones (base order), since added outranks unchanged.
        let scenario_names: Vec<&str> =
            modified.scenarios.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(
            scenario_names,
            vec![
                "collapsed row is underlined",
                "expanded row is not underlined",
                "underline persists under horizontal scroll",
                "archived row collapsed by default",
                "expanding reveals archived changes",
                "collapsing hides archived changes",
            ]
        );
        for name in [
            "archived row collapsed by default",
            "expanding reveals archived changes",
            "collapsing hides archived changes",
        ] {
            let scenario = modified.scenarios.iter().find(|s| s.name == name).unwrap();
            assert!(
                matches!(scenario.body, Piece::Unchanged { .. }),
                "expected {name:?} to be Unchanged, got {:?}",
                scenario.body
            );
        }
        for name in [
            "collapsed row is underlined",
            "expanded row is not underlined",
            "underline persists under horizontal scroll",
        ] {
            let scenario = modified.scenarios.iter().find(|s| s.name == name).unwrap();
            assert!(
                matches!(scenario.body, Piece::Added { .. }),
                "expected {name:?} to be Added, got {:?}",
                scenario.body
            );
        }
        assert!(
            !modified
                .scenarios
                .iter()
                .any(|s| matches!(s.body, Piece::Unmentioned { .. }))
        );
        assert!(
            !modified
                .scenarios
                .iter()
                .any(|s| matches!(s.body, Piece::Deleted { .. }))
        );
    }

    // --- 5.3: base: None reaching this layer via a rename-only delta ---

    #[test]
    fn rename_only_delta_with_no_base_produces_missing_base_error_not_a_panic() {
        let delta_md = "## RENAMED Requirements\n\n\
            - FROM: `### Requirement: Old Name`\n\
            - TO: `### Requirement: New Name`\n";
        let pair = pair_from_markdown(delta_md, None);
        assert!(pair.base.is_none());
        let result = diff("cap", &pair);
        assert!(result.requirements.is_empty());
        assert_eq!(
            result.errors,
            vec![DiffError::MissingBaseRequirement {
                capability: "cap".to_string(),
                requirement: "Old Name".to_string(),
            }]
        );
    }

    // --- purpose comparison (render-purpose 1.3) ---

    const REQS_ONLY: &str = "## MODIFIED Requirements\n\n\
        ### Requirement: Foo\n\
        Intro.\n\n\
        #### Scenario: A\n\
        - **WHEN** a\n\
        - **THEN** a2\n";
    const BASE_REQS: &str = "## Requirements\n\n\
        ### Requirement: Foo\n\
        Intro.\n\n\
        #### Scenario: A\n\
        - **WHEN** a\n\
        - **THEN** a2\n";

    fn delta_with_purpose(purpose_md: &str) -> String {
        format!("## Purpose\n\n{purpose_md}\n\n{REQS_ONLY}")
    }

    fn base_with_purpose(purpose_md: &str) -> String {
        format!("## Purpose\n\n{purpose_md}\n\n{BASE_REQS}")
    }

    #[test]
    fn delta_with_no_purpose_section_reports_no_purpose_comparison() {
        let pair = pair_from_markdown(REQS_ONLY, Some(BASE_REQS));
        assert_eq!(diff("cap", &pair).purpose, None);
    }

    #[test]
    fn delta_purpose_against_no_spec_of_record_is_reported_as_added() {
        let delta_md = delta_with_purpose("A brand new purpose.");
        let pair = pair_from_markdown(&delta_md, None);
        match diff("cap", &pair).purpose {
            Some(Piece::Added { delta }) => assert_eq!(delta, "A brand new purpose."),
            other => panic!("expected Some(Piece::Added), got {other:?}"),
        }
    }

    #[test]
    fn delta_purpose_against_base_with_no_purpose_section_is_reported_as_added() {
        let delta_md = delta_with_purpose("A brand new purpose.");
        let pair = pair_from_markdown(&delta_md, Some(BASE_REQS));
        match diff("cap", &pair).purpose {
            Some(Piece::Added { delta }) => assert_eq!(delta, "A brand new purpose."),
            other => panic!("expected Some(Piece::Added), got {other:?}"),
        }
    }

    #[test]
    fn delta_purpose_equal_to_base_purpose_reports_no_comparison() {
        let delta_md = delta_with_purpose("The same purpose.");
        let base_md = base_with_purpose("The same purpose.");
        let pair = pair_from_markdown(&delta_md, Some(&base_md));
        assert_eq!(diff("cap", &pair).purpose, None);
    }

    #[test]
    fn delta_purpose_differing_from_base_purpose_is_reported_changed() {
        let delta_md = delta_with_purpose("The base purpose text EDITED.");
        let base_md = base_with_purpose("The base purpose text.");
        let pair = pair_from_markdown(&delta_md, Some(&base_md));
        match diff("cap", &pair).purpose {
            Some(Piece::Changed { base, delta, .. }) => {
                assert_eq!(base, "The base purpose text.");
                assert_eq!(delta, "The base purpose text EDITED.");
            }
            other => panic!("expected Some(Piece::Changed), got {other:?}"),
        }
    }

    #[test]
    fn delta_purpose_wholly_dissimilar_from_base_purpose_is_reported_replaced() {
        // Same pair used by `compare`'s own `Piece::Replaced` calibration
        // (`2026-08-08-diff-legibility/design.md`), reproduced inline so
        // this test doesn't depend on that module's private test fixtures.
        let base_purpose = "The left pane SHALL hold keyboard input focus for the duration of the application's runtime. There SHALL be no mechanism to move focus to the right pane.";
        let delta_purpose = "Exactly one pane SHALL hold keyboard input focus at any time. The left pane SHALL hold focus when the application starts. The system SHALL move focus to the other pane when the user presses Tab, and SHALL indicate which pane currently holds focus visually. A key pressed while a pane holds focus SHALL be handled by that pane, except for keys the application handles globally.";
        let delta_md = delta_with_purpose(delta_purpose);
        let base_md = base_with_purpose(base_purpose);
        let pair = pair_from_markdown(&delta_md, Some(&base_md));
        match diff("cap", &pair).purpose {
            Some(Piece::Replaced { .. }) => {}
            other => panic!("expected Some(Piece::Replaced), got {other:?}"),
        }
    }

    #[test]
    fn purpose_comparison_is_deterministic() {
        let delta_md = delta_with_purpose("The base purpose text EDITED.");
        let base_md = base_with_purpose("The base purpose text.");
        let pair = pair_from_markdown(&delta_md, Some(&base_md));
        let first = diff("cap", &pair).purpose;
        let second = diff("cap", &pair).purpose;
        assert_eq!(first, second);
    }

    // --- unrecognised sections carried through (unrecognized-spec-sections 2.3) ---

    #[test]
    fn unrecognised_sections_are_carried_through_unchanged() {
        let delta_md = "## FIRST BOGUS\n\ntext\n\n\
            ## ADDED Requirements\n\n\
            ### Requirement: Foo\n\
            Intro.\n\n\
            #### Scenario: A\n\
            - **WHEN** a\n\
            - **THEN** a2\n\n\
            ## SECOND BOGUS\n\ntext\n";
        let pair = pair_from_markdown(delta_md, None);
        let result = diff("cap", &pair);
        assert_eq!(
            result.unrecognized_sections,
            vec!["FIRST BOGUS", "SECOND BOGUS"]
        );
    }

    #[test]
    fn no_unrecognised_sections_yields_empty_vec_on_the_diff() {
        let pair = pair_from_markdown(REQS_ONLY, Some(BASE_REQS));
        assert!(diff("cap", &pair).unrecognized_sections.is_empty());
    }
}
