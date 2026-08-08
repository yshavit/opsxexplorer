use std::collections::{HashMap, HashSet};

use crate::diff::model::{Piece, ScenarioDiff};
use crate::diff::runs::runs;
use crate::specs::Requirement;

/// The rule applied everywhere: absence in the delta means unmentioned,
/// presence means authoritative for that piece (see design.md).
fn changed_or_unchanged(base: &str, delta: &str) -> Piece {
    if delta == base {
        Piece::Unchanged {
            text: delta.to_string(),
        }
    } else {
        Piece::Changed {
            base: base.to_string(),
            delta: delta.to_string(),
            runs: runs(base, delta),
        }
    }
}

/// An omitted intro and an emptied one are indistinguishable in the source
/// (both parse to `""`), so both yield `Unmentioned` rather than a deletion.
fn intro_piece(base: &str, delta: &str) -> Piece {
    if delta.is_empty() {
        Piece::Unmentioned {
            base: base.to_string(),
        }
    } else {
        changed_or_unchanged(base, delta)
    }
}

/// Compares a base requirement's content against a delta requirement's,
/// applying the uniform rule to the intro and matching scenarios by name.
/// Base scenarios are emitted first, in base order (matched or
/// `Unmentioned`), followed by delta-only scenarios in delta order.
/// Duplicate names resolve first-wins on both sides (see design.md).
pub(crate) fn compare_requirement(
    base: &Requirement,
    delta: &Requirement,
) -> (Piece, Vec<ScenarioDiff>) {
    let intro = intro_piece(&base.intro, &delta.intro);

    let mut delta_by_name = HashMap::new();
    for scenario in &delta.scenarios {
        delta_by_name
            .entry(scenario.name.as_str())
            .or_insert(scenario);
    }

    let mut scenarios = Vec::new();
    let mut seen: HashSet<&str> = HashSet::new();

    for base_scenario in &base.scenarios {
        if !seen.insert(base_scenario.name.as_str()) {
            continue;
        }
        let body = match delta_by_name.get(base_scenario.name.as_str()) {
            Some(delta_scenario) => changed_or_unchanged(&base_scenario.body, &delta_scenario.body),
            None => Piece::Unmentioned {
                base: base_scenario.body.clone(),
            },
        };
        scenarios.push(ScenarioDiff {
            name: base_scenario.name.clone(),
            body,
        });
    }

    for delta_scenario in &delta.scenarios {
        if !seen.insert(delta_scenario.name.as_str()) {
            continue;
        }
        scenarios.push(ScenarioDiff {
            name: delta_scenario.name.clone(),
            body: Piece::Added {
                delta: delta_scenario.body.clone(),
            },
        });
    }

    (intro, scenarios)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::test_support::pair_from_markdown;
    use crate::diff::{Operation, diff};

    fn only_requirement(
        capability_diff: &crate::diff::CapabilityDiff,
    ) -> &crate::diff::RequirementDiff {
        assert!(capability_diff.errors.is_empty());
        assert_eq!(capability_diff.requirements.len(), 1);
        &capability_diff.requirements[0]
    }

    // --- 3.1: intro rule ---

    #[test]
    fn intro_omitted_from_delta_is_unmentioned_carrying_base() {
        let base_md = "## Requirements\n\n\
            ### Requirement: Foo\n\
            The base intro.\n\n\
            #### Scenario: A\n\
            - **WHEN** a\n\
            - **THEN** a2\n";
        let delta_md = "## MODIFIED Requirements\n\n\
            ### Requirement: Foo\n\
            #### Scenario: A\n\
            - **WHEN** a\n\
            - **THEN** a2\n";
        let pair = pair_from_markdown(delta_md, Some(base_md));
        let result = diff("cap", &pair);
        let req = only_requirement(&result);
        assert_eq!(
            req.intro,
            Piece::Unmentioned {
                base: "The base intro.".to_string()
            }
        );
    }

    #[test]
    fn intro_restated_unchanged_is_reported_unchanged() {
        let base_md = "## Requirements\n\n\
            ### Requirement: Foo\n\
            The base intro.\n\n\
            #### Scenario: A\n\
            - **WHEN** a\n\
            - **THEN** a2\n";
        let delta_md = "## MODIFIED Requirements\n\n\
            ### Requirement: Foo\n\
            The base intro.\n\n\
            #### Scenario: A\n\
            - **WHEN** a\n\
            - **THEN** a2\n";
        let pair = pair_from_markdown(delta_md, Some(base_md));
        let result = diff("cap", &pair);
        let req = only_requirement(&result);
        assert_eq!(
            req.intro,
            Piece::Unchanged {
                text: "The base intro.".to_string()
            }
        );
    }

    #[test]
    fn intro_edited_is_reported_changed_with_runs() {
        let base_md = "## Requirements\n\n\
            ### Requirement: Foo\n\
            The base intro text.\n\n\
            #### Scenario: A\n\
            - **WHEN** a\n\
            - **THEN** a2\n";
        let delta_md = "## MODIFIED Requirements\n\n\
            ### Requirement: Foo\n\
            The base intro EDITED.\n\n\
            #### Scenario: A\n\
            - **WHEN** a\n\
            - **THEN** a2\n";
        let pair = pair_from_markdown(delta_md, Some(base_md));
        let result = diff("cap", &pair);
        let req = only_requirement(&result);
        match &req.intro {
            Piece::Changed { base, delta, .. } => {
                assert_eq!(base, "The base intro text.");
                assert_eq!(delta, "The base intro EDITED.");
            }
            other => panic!("expected Changed, got {other:?}"),
        }
    }

    // --- 3.3: scenario reordering is a no-op ---

    #[test]
    fn reordering_restated_scenarios_changes_nothing() {
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
            Intro.\n\n\
            #### Scenario: B\n\
            - **WHEN** b\n\
            - **THEN** b2\n\n\
            #### Scenario: A\n\
            - **WHEN** a\n\
            - **THEN** a2\n";
        let pair = pair_from_markdown(delta_md, Some(base_md));
        let result = diff("cap", &pair);
        let req = only_requirement(&result);
        let names: Vec<&str> = req.scenarios.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["A", "B"]);
        assert!(
            req.scenarios
                .iter()
                .all(|s| matches!(s.body, Piece::Unchanged { .. }))
        );
    }

    // --- 3.4: subset case ---

    #[test]
    fn subset_of_scenarios_restated_leaves_one_unmentioned() {
        let base_md = "## Requirements\n\n\
            ### Requirement: Foo\n\
            Intro.\n\n\
            #### Scenario: A\n\
            - **WHEN** a\n\
            - **THEN** a2\n\n\
            #### Scenario: B\n\
            - **WHEN** b\n\
            - **THEN** b2\n\n\
            #### Scenario: C\n\
            - **WHEN** c\n\
            - **THEN** c2\n\n\
            #### Scenario: D\n\
            - **WHEN** d\n\
            - **THEN** d2\n";
        let delta_md = "## MODIFIED Requirements\n\n\
            ### Requirement: Foo\n\
            Intro.\n\n\
            #### Scenario: A\n\
            - **WHEN** a\n\
            - **THEN** a2\n\n\
            #### Scenario: B\n\
            - **WHEN** b\n\
            - **THEN** b2\n\n\
            #### Scenario: C\n\
            - **WHEN** c\n\
            - **THEN** c2\n";
        let pair = pair_from_markdown(delta_md, Some(base_md));
        let result = diff("cap", &pair);
        let req = only_requirement(&result);
        assert_eq!(req.scenarios.len(), 4);
        assert!(matches!(req.scenarios[0].body, Piece::Unchanged { .. }));
        assert!(matches!(req.scenarios[1].body, Piece::Unchanged { .. }));
        assert!(matches!(req.scenarios[2].body, Piece::Unchanged { .. }));
        assert_eq!(req.scenarios[3].name, "D");
        assert_eq!(
            req.scenarios[3].body,
            Piece::Unmentioned {
                base: "- **WHEN** d\n- **THEN** d2".to_string()
            }
        );
        assert!(
            !req.scenarios
                .iter()
                .any(|s| matches!(s.body, Piece::Deleted { .. }))
        );
        assert_eq!(req.op, Operation::Modified);
    }
}
