use std::collections::HashSet;
use std::mem::discriminant;

use crate::diff::{CapabilityDiff, Operation, Piece, RequirementDiff};

/// Identifies a row's collapse state by name rather than by its position in
/// the flattened list, so the set survives re-flattening when the tree's
/// shape changes (see design.md). `scenario: None` addresses a requirement
/// row; `scenario: Some(_)` addresses one of its scenario rows.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RowKey {
    pub capability: String,
    pub requirement: String,
    pub scenario: Option<String>,
}

/// A single visible row in the right pane's flattened tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffRow<'a> {
    /// A display-only heading introducing a run of entries sharing an operation.
    GroupHeading(&'a Operation),
    Requirement {
        name: &'a str,
        op: &'a Operation,
        expanded: bool,
        key: RowKey,
    },
    Intro {
        piece: &'a Piece,
    },
    Scenario {
        name: &'a str,
        body: &'a Piece,
        expanded: bool,
        key: RowKey,
    },
    Body {
        piece: &'a Piece,
    },
    Notice(String),
}

impl DiffRow<'_> {
    pub fn is_selectable(&self) -> bool {
        matches!(self, DiffRow::Requirement { .. } | DiffRow::Scenario { .. })
    }

    /// The collapse-state key for a selectable row, so key-based toggling
    /// doesn't need to re-derive it from surrounding context.
    pub fn key(&self) -> Option<&RowKey> {
        match self {
            DiffRow::Requirement { key, .. } | DiffRow::Scenario { key, .. } => Some(key),
            _ => None,
        }
    }

    pub fn expanded(&self) -> Option<bool> {
        match self {
            DiffRow::Requirement { expanded, .. } | DiffRow::Scenario { expanded, .. } => {
                Some(*expanded)
            }
            _ => None,
        }
    }
}

/// Flattens a capability's diff into the rows the right pane renders and
/// navigates over, given which rows the collapse-state set currently marks
/// as expanded. Errors are surfaced as `Notice` rows above the tree, and a
/// group heading is emitted only when the run of entries it introduces is
/// non-empty (see design.md, spec.md).
pub fn flatten<'a>(diff: &'a CapabilityDiff, expanded: &HashSet<RowKey>) -> Vec<DiffRow<'a>> {
    let mut rows = Vec::new();

    for error in &diff.errors {
        rows.push(DiffRow::Notice(error.to_string()));
    }

    let mut last_kind = None;
    for req in &diff.requirements {
        if last_kind != Some(discriminant(&req.op)) {
            rows.push(DiffRow::GroupHeading(&req.op));
            last_kind = Some(discriminant(&req.op));
        }
        push_requirement(&mut rows, diff, req, expanded);
    }

    rows
}

fn push_requirement<'a>(
    rows: &mut Vec<DiffRow<'a>>,
    diff: &'a CapabilityDiff,
    req: &'a RequirementDiff,
    expanded: &HashSet<RowKey>,
) {
    let req_key = RowKey {
        capability: diff.capability.clone(),
        requirement: req.name.clone(),
        scenario: None,
    };
    let req_expanded = expanded.contains(&req_key);
    rows.push(DiffRow::Requirement {
        name: &req.name,
        op: &req.op,
        expanded: req_expanded,
        key: req_key,
    });

    if !req_expanded {
        return;
    }

    rows.push(DiffRow::Intro { piece: &req.intro });

    for scenario in &req.scenarios {
        let scenario_key = RowKey {
            capability: diff.capability.clone(),
            requirement: req.name.clone(),
            scenario: Some(scenario.name.clone()),
        };
        let scenario_expanded = expanded.contains(&scenario_key);
        rows.push(DiffRow::Scenario {
            name: &scenario.name,
            body: &scenario.body,
            expanded: scenario_expanded,
            key: scenario_key,
        });
        if scenario_expanded {
            rows.push(DiffRow::Body {
                piece: &scenario.body,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::{DiffError, ScenarioDiff};

    fn scenario(name: &str, body: Piece) -> ScenarioDiff {
        ScenarioDiff {
            name: name.to_string(),
            body,
        }
    }

    fn requirement(
        name: &str,
        op: Operation,
        intro: Piece,
        scenarios: Vec<ScenarioDiff>,
    ) -> RequirementDiff {
        RequirementDiff {
            name: name.to_string(),
            op,
            intro,
            scenarios,
        }
    }

    fn unchanged(text: &str) -> Piece {
        Piece::Unchanged {
            text: text.to_string(),
        }
    }

    fn req_key(capability: &str, requirement: &str) -> RowKey {
        RowKey {
            capability: capability.to_string(),
            requirement: requirement.to_string(),
            scenario: None,
        }
    }

    fn scenario_key(capability: &str, requirement: &str, scenario: &str) -> RowKey {
        RowKey {
            capability: capability.to_string(),
            requirement: requirement.to_string(),
            scenario: Some(scenario.to_string()),
        }
    }

    #[test]
    fn everything_collapsed_yields_one_row_per_requirement_plus_headings() {
        let diff = CapabilityDiff {
            capability: "cap".to_string(),
            requirements: vec![
                requirement(
                    "Added One",
                    Operation::Added,
                    unchanged("intro"),
                    vec![scenario("s1", unchanged("body"))],
                ),
                requirement(
                    "Modified One",
                    Operation::Modified,
                    unchanged("intro"),
                    vec![scenario("s2", unchanged("body"))],
                ),
            ],
            errors: vec![],
        };
        let rows = flatten(&diff, &HashSet::new());
        assert_eq!(
            rows,
            vec![
                DiffRow::GroupHeading(&Operation::Added),
                DiffRow::Requirement {
                    name: "Added One",
                    op: &Operation::Added,
                    expanded: false,
                    key: req_key("cap", "Added One"),
                },
                DiffRow::GroupHeading(&Operation::Modified),
                DiffRow::Requirement {
                    name: "Modified One",
                    op: &Operation::Modified,
                    expanded: false,
                    key: req_key("cap", "Modified One"),
                },
            ]
        );
    }

    #[test]
    fn expanding_a_requirement_reveals_intro_and_collapsed_scenario_headers() {
        let diff = CapabilityDiff {
            capability: "cap".to_string(),
            requirements: vec![requirement(
                "Req",
                Operation::Modified,
                unchanged("the intro"),
                vec![
                    scenario("first", unchanged("first body")),
                    scenario("second", unchanged("second body")),
                ],
            )],
            errors: vec![],
        };
        let mut expanded = HashSet::new();
        expanded.insert(RowKey {
            capability: "cap".to_string(),
            requirement: "Req".to_string(),
            scenario: None,
        });

        let rows = flatten(&diff, &expanded);
        assert_eq!(
            rows,
            vec![
                DiffRow::GroupHeading(&Operation::Modified),
                DiffRow::Requirement {
                    name: "Req",
                    op: &Operation::Modified,
                    expanded: true,
                    key: req_key("cap", "Req"),
                },
                DiffRow::Intro {
                    piece: &unchanged("the intro"),
                },
                DiffRow::Scenario {
                    name: "first",
                    body: &unchanged("first body"),
                    expanded: false,
                    key: scenario_key("cap", "Req", "first"),
                },
                DiffRow::Scenario {
                    name: "second",
                    body: &unchanged("second body"),
                    expanded: false,
                    key: scenario_key("cap", "Req", "second"),
                },
            ]
        );
    }

    #[test]
    fn expanding_a_scenario_reveals_its_body() {
        let diff = CapabilityDiff {
            capability: "cap".to_string(),
            requirements: vec![requirement(
                "Req",
                Operation::Modified,
                unchanged("intro"),
                vec![scenario("only", unchanged("scenario body"))],
            )],
            errors: vec![],
        };
        let mut expanded = HashSet::new();
        expanded.insert(RowKey {
            capability: "cap".to_string(),
            requirement: "Req".to_string(),
            scenario: None,
        });
        expanded.insert(RowKey {
            capability: "cap".to_string(),
            requirement: "Req".to_string(),
            scenario: Some("only".to_string()),
        });

        let rows = flatten(&diff, &expanded);
        assert_eq!(
            rows,
            vec![
                DiffRow::GroupHeading(&Operation::Modified),
                DiffRow::Requirement {
                    name: "Req",
                    op: &Operation::Modified,
                    expanded: true,
                    key: req_key("cap", "Req"),
                },
                DiffRow::Intro {
                    piece: &unchanged("intro"),
                },
                DiffRow::Scenario {
                    name: "only",
                    body: &unchanged("scenario body"),
                    expanded: true,
                    key: scenario_key("cap", "Req", "only"),
                },
                DiffRow::Body {
                    piece: &unchanged("scenario body"),
                },
            ]
        );
    }

    #[test]
    fn operation_with_no_entries_emits_no_heading() {
        let diff = CapabilityDiff {
            capability: "cap".to_string(),
            requirements: vec![requirement(
                "Only Added",
                Operation::Added,
                unchanged("intro"),
                vec![],
            )],
            errors: vec![],
        };
        let rows = flatten(&diff, &HashSet::new());
        assert_eq!(rows.len(), 2);
        assert!(matches!(rows[0], DiffRow::GroupHeading(Operation::Added)));
        assert!(
            !rows
                .iter()
                .any(|r| matches!(r, DiffRow::GroupHeading(Operation::Modified)))
        );
        assert!(
            !rows
                .iter()
                .any(|r| matches!(r, DiffRow::GroupHeading(Operation::Removed)))
        );
    }

    #[test]
    fn errors_and_requirements_appear_together() {
        let diff = CapabilityDiff {
            capability: "cap".to_string(),
            requirements: vec![requirement(
                "Fine",
                Operation::Added,
                unchanged("intro"),
                vec![],
            )],
            errors: vec![DiffError::MissingBaseRequirement {
                capability: "cap".to_string(),
                requirement: "Ghost".to_string(),
            }],
        };
        let rows = flatten(&diff, &HashSet::new());
        assert!(matches!(rows[0], DiffRow::Notice(_)));
        assert!(matches!(rows[1], DiffRow::GroupHeading(Operation::Added)));
        assert!(matches!(rows[2], DiffRow::Requirement { name: "Fine", .. }));
    }

    #[test]
    fn only_requirement_and_scenario_rows_are_selectable() {
        assert!(
            DiffRow::Requirement {
                name: "x",
                op: &Operation::Added,
                expanded: false,
                key: req_key("cap", "x"),
            }
            .is_selectable()
        );
        assert!(
            DiffRow::Scenario {
                name: "x",
                body: &unchanged("x"),
                expanded: false,
                key: scenario_key("cap", "x", "x"),
            }
            .is_selectable()
        );
        assert!(!DiffRow::GroupHeading(&Operation::Added).is_selectable());
        assert!(
            !DiffRow::Intro {
                piece: &unchanged("x"),
            }
            .is_selectable()
        );
        assert!(
            !DiffRow::Body {
                piece: &unchanged("x"),
            }
            .is_selectable()
        );
        assert!(!DiffRow::Notice("x".to_string()).is_selectable());
    }
}
