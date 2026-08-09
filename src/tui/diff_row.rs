use std::collections::HashSet;
use std::mem::discriminant;

use crate::diff::{CapabilityDiff, Operation, Piece, RequirementDiff};

use super::layout;

/// Identifies a row's collapse state by name rather than by its position in
/// the flattened list, so the set survives re-flattening when the tree's
/// shape changes (see design.md). `Purpose` addresses the capability-level
/// purpose row, which has no requirement name to hang off of.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RowKey {
    Purpose {
        capability: String,
    },
    Requirement {
        capability: String,
        requirement: String,
    },
    Scenario {
        capability: String,
        requirement: String,
        scenario: String,
    },
}

/// A single visible row in the right pane's flattened tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffRow<'a> {
    /// A display-only heading introducing a run of entries sharing an operation.
    GroupHeading(&'a Operation),
    /// A display-only heading introducing the purpose comparison below it.
    PurposeHeading(&'a Piece),
    /// The purpose comparison's text fits in full on one line: no collapse
    /// affordance, but still selectable (see design.md).
    PurposeFull(&'a Piece),
    /// The purpose comparison is collapsible: either its text doesn't fit
    /// (`Added`/`Changed`), or it's a wholesale replacement, which is always
    /// collapsible regardless of length.
    Purpose {
        piece: &'a Piece,
        expanded: bool,
        key: RowKey,
    },
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
        matches!(
            self,
            DiffRow::Requirement { .. }
                | DiffRow::Scenario { .. }
                | DiffRow::Purpose { .. }
                | DiffRow::PurposeFull(_)
        )
    }

    /// The collapse-state key for a selectable row, so key-based toggling
    /// doesn't need to re-derive it from surrounding context. `PurposeFull`
    /// falls through to `None` deliberately: it has nothing to toggle (see
    /// design.md).
    pub fn key(&self) -> Option<&RowKey> {
        match self {
            DiffRow::Requirement { key, .. }
            | DiffRow::Scenario { key, .. }
            | DiffRow::Purpose { key, .. } => Some(key),
            _ => None,
        }
    }

    pub fn expanded(&self) -> Option<bool> {
        match self {
            DiffRow::Requirement { expanded, .. }
            | DiffRow::Scenario { expanded, .. }
            | DiffRow::Purpose { expanded, .. } => Some(*expanded),
            _ => None,
        }
    }
}

/// Flattens a capability's diff into the rows the right pane renders and
/// navigates over, given which rows the collapse-state set currently marks
/// as expanded and the pane's current width (needed only to decide whether
/// the purpose row, if any, can collapse at all — see design.md). Errors are
/// surfaced as `Notice` rows above the tree, the purpose comparison (if any)
/// follows them, and a group heading is emitted only when the run of entries
/// it introduces is non-empty (see design.md, spec.md).
pub fn flatten<'a>(
    diff: &'a CapabilityDiff,
    expanded: &HashSet<RowKey>,
    width: usize,
) -> Vec<DiffRow<'a>> {
    let mut rows = Vec::new();

    for error in &diff.errors {
        rows.push(DiffRow::Notice(error.to_string()));
    }

    if let Some(piece) = &diff.purpose {
        push_purpose(&mut rows, diff, piece, expanded, width);
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

/// Pushes the purpose heading and its one content row. A wholesale
/// replacement is always collapsible; an insertion or an ordinary edit is
/// collapsible only when its current text, trimmed of trailing whitespace,
/// doesn't fit the row's available width at `width` — the same budget the
/// collapsed row's own truncation uses (`layout::purpose_available`), so the
/// two decisions can never disagree (see design.md).
fn push_purpose<'a>(
    rows: &mut Vec<DiffRow<'a>>,
    diff: &'a CapabilityDiff,
    piece: &'a Piece,
    expanded: &HashSet<RowKey>,
    width: usize,
) {
    rows.push(DiffRow::PurposeHeading(piece));

    let key = RowKey::Purpose {
        capability: diff.capability.clone(),
    };
    let purpose_expanded = expanded.contains(&key);

    let current_text = match piece {
        Piece::Added { delta } | Piece::Changed { delta, .. } => Some(delta.as_str()),
        _ => None,
    };

    match current_text {
        Some(text) if text.trim_end().chars().count() <= layout::purpose_available(width) => {
            rows.push(DiffRow::PurposeFull(piece));
        }
        _ => rows.push(DiffRow::Purpose {
            piece,
            expanded: purpose_expanded,
            key,
        }),
    }
}

fn push_requirement<'a>(
    rows: &mut Vec<DiffRow<'a>>,
    diff: &'a CapabilityDiff,
    req: &'a RequirementDiff,
    expanded: &HashSet<RowKey>,
) {
    let req_key = RowKey::Requirement {
        capability: diff.capability.clone(),
        requirement: req.name.clone(),
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
        let scenario_key = RowKey::Scenario {
            capability: diff.capability.clone(),
            requirement: req.name.clone(),
            scenario: scenario.name.clone(),
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

    /// Wide enough that no fixture's purpose text in this module needs to
    /// collapse unless a test deliberately narrows it.
    const WIDE: usize = 200;

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

    fn capability_diff(requirements: Vec<RequirementDiff>) -> CapabilityDiff {
        CapabilityDiff {
            capability: "cap".to_string(),
            requirements,
            errors: vec![],
            purpose: None,
        }
    }

    fn req_key(capability: &str, requirement: &str) -> RowKey {
        RowKey::Requirement {
            capability: capability.to_string(),
            requirement: requirement.to_string(),
        }
    }

    fn scenario_key(capability: &str, requirement: &str, scenario: &str) -> RowKey {
        RowKey::Scenario {
            capability: capability.to_string(),
            requirement: requirement.to_string(),
            scenario: scenario.to_string(),
        }
    }

    fn purpose_key(capability: &str) -> RowKey {
        RowKey::Purpose {
            capability: capability.to_string(),
        }
    }

    #[test]
    fn everything_collapsed_yields_one_row_per_requirement_plus_headings() {
        let diff = capability_diff(vec![
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
        ]);
        let rows = flatten(&diff, &HashSet::new(), WIDE);
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
        let diff = capability_diff(vec![requirement(
            "Req",
            Operation::Modified,
            unchanged("the intro"),
            vec![
                scenario("first", unchanged("first body")),
                scenario("second", unchanged("second body")),
            ],
        )]);
        let mut expanded = HashSet::new();
        expanded.insert(req_key("cap", "Req"));

        let rows = flatten(&diff, &expanded, WIDE);
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
        let diff = capability_diff(vec![requirement(
            "Req",
            Operation::Modified,
            unchanged("intro"),
            vec![scenario("only", unchanged("scenario body"))],
        )]);
        let mut expanded = HashSet::new();
        expanded.insert(req_key("cap", "Req"));
        expanded.insert(scenario_key("cap", "Req", "only"));

        let rows = flatten(&diff, &expanded, WIDE);
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
        let diff = capability_diff(vec![requirement(
            "Only Added",
            Operation::Added,
            unchanged("intro"),
            vec![],
        )]);
        let rows = flatten(&diff, &HashSet::new(), WIDE);
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
        let mut diff = capability_diff(vec![requirement(
            "Fine",
            Operation::Added,
            unchanged("intro"),
            vec![],
        )]);
        diff.errors.push(DiffError::MissingBaseRequirement {
            capability: "cap".to_string(),
            requirement: "Ghost".to_string(),
        });
        let rows = flatten(&diff, &HashSet::new(), WIDE);
        assert!(matches!(rows[0], DiffRow::Notice(_)));
        assert!(matches!(rows[1], DiffRow::GroupHeading(Operation::Added)));
        assert!(matches!(rows[2], DiffRow::Requirement { name: "Fine", .. }));
    }

    #[test]
    fn only_requirement_scenario_and_purpose_rows_are_selectable() {
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
        assert!(
            DiffRow::Purpose {
                piece: &added_purpose("x"),
                expanded: false,
                key: purpose_key("cap"),
            }
            .is_selectable()
        );
        assert!(DiffRow::PurposeFull(&added_purpose("x")).is_selectable());
        assert!(!DiffRow::GroupHeading(&Operation::Added).is_selectable());
        assert!(!DiffRow::PurposeHeading(&added_purpose("x")).is_selectable());
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

    // --- purpose rows (render-purpose 3.4) ---

    fn added_purpose(text: &str) -> Piece {
        Piece::Added {
            delta: text.to_string(),
        }
    }

    fn changed_purpose(delta: &str) -> Piece {
        Piece::Changed {
            base: "old".to_string(),
            delta: delta.to_string(),
            runs: vec![],
        }
    }

    fn replaced_purpose() -> Piece {
        Piece::Replaced {
            base: "old text".to_string(),
            delta: "new text".to_string(),
        }
    }

    fn diff_with_purpose(piece: Piece) -> CapabilityDiff {
        let mut diff = capability_diff(vec![requirement(
            "Req",
            Operation::Added,
            unchanged("intro"),
            vec![],
        )]);
        diff.purpose = Some(piece);
        diff
    }

    #[test]
    fn absent_purpose_emits_no_purpose_rows() {
        let diff = capability_diff(vec![requirement(
            "Req",
            Operation::Added,
            unchanged("intro"),
            vec![],
        )]);
        let rows = flatten(&diff, &HashSet::new(), WIDE);
        assert!(!rows.iter().any(|r| matches!(r, DiffRow::PurposeHeading(_))));
        assert!(!rows.iter().any(|r| matches!(r, DiffRow::PurposeFull(_))));
        assert!(!rows.iter().any(|r| matches!(r, DiffRow::Purpose { .. })));
    }

    #[test]
    fn purpose_rows_sit_after_notices_and_before_the_first_group_heading() {
        let mut diff = diff_with_purpose(added_purpose("short"));
        diff.errors.push(DiffError::MissingBaseRequirement {
            capability: "cap".to_string(),
            requirement: "Ghost".to_string(),
        });
        let rows = flatten(&diff, &HashSet::new(), WIDE);
        assert!(matches!(rows[0], DiffRow::Notice(_)));
        assert!(matches!(rows[1], DiffRow::PurposeHeading(_)));
        assert!(matches!(rows[2], DiffRow::PurposeFull(_)));
        assert!(matches!(rows[3], DiffRow::GroupHeading(Operation::Added)));
    }

    #[test]
    fn fitting_added_purpose_text_yields_purpose_full() {
        let diff = diff_with_purpose(added_purpose("short purpose text"));
        let rows = flatten(&diff, &HashSet::new(), WIDE);
        assert!(matches!(rows[1], DiffRow::PurposeFull(_)));
        assert!(!rows.iter().any(|r| matches!(r, DiffRow::Purpose { .. })));
    }

    #[test]
    fn non_fitting_changed_purpose_text_yields_purpose() {
        let long_text = "a very long purpose ".repeat(20);
        let diff = diff_with_purpose(changed_purpose(&long_text));
        let rows = flatten(&diff, &HashSet::new(), WIDE);
        assert!(matches!(rows[1], DiffRow::Purpose { .. }));
        assert!(!rows.iter().any(|r| matches!(r, DiffRow::PurposeFull(_))));
    }

    #[test]
    fn replaced_purpose_always_yields_purpose_even_when_short() {
        // `replaced_purpose`'s texts are short enough to trivially fit `WIDE`,
        // but a wholesale replacement is never rendered as `PurposeFull`.
        let diff = diff_with_purpose(replaced_purpose());
        let rows = flatten(&diff, &HashSet::new(), WIDE);
        assert!(matches!(rows[1], DiffRow::Purpose { .. }));
        assert!(!rows.iter().any(|r| matches!(r, DiffRow::PurposeFull(_))));
    }

    #[test]
    fn purpose_fulls_key_and_expanded_are_none() {
        let row = DiffRow::PurposeFull(&added_purpose("x"));
        assert!(row.key().is_none());
        assert!(row.expanded().is_none());
    }

    #[test]
    fn purposes_collapse_state_follows_the_expanded_set() {
        let long_text = "a very long purpose ".repeat(20);
        let diff = diff_with_purpose(changed_purpose(&long_text));

        let rows = flatten(&diff, &HashSet::new(), WIDE);
        assert_eq!(rows[1].expanded(), Some(false));

        let mut expanded = HashSet::new();
        expanded.insert(purpose_key("cap"));
        let rows = flatten(&diff, &expanded, WIDE);
        assert_eq!(rows[1].expanded(), Some(true));
    }

    #[test]
    fn fits_boundary_responds_to_width() {
        let diff = diff_with_purpose(added_purpose("0123456789"));

        let rows = flatten(&diff, &HashSet::new(), WIDE);
        assert!(matches!(rows[1], DiffRow::PurposeFull(_)));

        // Narrow the pane down to nothing: the same text no longer fits.
        let rows = flatten(&diff, &HashSet::new(), 0);
        assert!(matches!(rows[1], DiffRow::Purpose { .. }));
    }
}
