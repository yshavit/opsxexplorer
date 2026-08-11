use std::collections::HashSet;
use std::mem::discriminant;

use crate::diff::{CapabilityDiff, Operation, Piece, RequirementDiff, UnrecognizedSection};

use super::layout;

/// Which document an unrecognised section was parsed from. Carried on the
/// row key because a capability's two lists are addressed independently: two
/// sections can share a title, within one list or across the two, so a
/// section's identity is its position in its own origin's list — the same
/// reason `RowKey::RemovalNoteLine` is addressed by line index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Origin {
    /// The change's own delta spec.
    Delta,
    /// The capability's spec of record.
    Base,
}

/// Identifies a row's collapse state by name rather than by its position in
/// the flattened list, so the set survives re-flattening when the tree's
/// shape changes (see `2026-08-08-tui-specdiff/design.md`). `Purpose`
/// addresses the capability-level purpose row, which has no requirement name
/// to hang off of.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RowKey {
    Purpose {
        capability: String,
    },
    Intro {
        capability: String,
        requirement: String,
    },
    /// One line of a removed requirement's removal note, addressed by its
    /// index in `note.lines()` (stable across re-flattens, since the note's
    /// text doesn't change while the requirement is on screen).
    RemovalNoteLine {
        capability: String,
        requirement: String,
        line: usize,
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
    /// One unrecognised section, addressed by its index within its own
    /// origin's list on the capability's diff.
    UnrecognizedSection {
        capability: String,
        origin: Origin,
        index: usize,
    },
}

/// A single visible row in the right pane's flattened tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffRow<'a> {
    /// A display-only heading introducing a run of entries sharing an operation.
    GroupHeading(&'a Operation),
    /// A display-only heading introducing the purpose comparison below it.
    PurposeHeading(&'a Piece),
    /// A paragraph-shaped comparison (the capability's purpose, or a
    /// requirement's intro) whose text fits in full on one line: no collapse
    /// affordance, but still selectable (see
    /// `2026-08-09-render-purpose/design.md`). `indent` is the
    /// row's nesting depth: 0 for purpose, 1 for a requirement's intro.
    ParagraphFull {
        piece: &'a Piece,
        indent: usize,
    },
    /// A paragraph-shaped comparison that is collapsible: either its text
    /// doesn't fit at its own `indent`, or it's a wholesale replacement,
    /// which is always collapsible regardless of length.
    Paragraph {
        piece: &'a Piece,
        expanded: bool,
        key: RowKey,
        indent: usize,
    },
    /// One line of a removed requirement's removal note, shown in full: no
    /// collapse affordance, but still selectable, mirroring `ParagraphFull`
    /// (see `2026-08-10-spec-model-removals/design.md`).
    RemovalNoteFull {
        text: &'a str,
        indent: usize,
    },
    /// One line of a removed requirement's removal note that doesn't fit at
    /// `indent` and is therefore collapsible, mirroring `Paragraph`.
    RemovalNoteLine {
        text: &'a str,
        expanded: bool,
        key: RowKey,
        indent: usize,
    },
    Requirement {
        name: &'a str,
        op: &'a Operation,
        expanded: bool,
        key: RowKey,
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
    /// A display-only heading introducing one origin's run of unrecognised
    /// sections below it.
    UnrecognizedSectionsHeading(Origin),
    /// One unrecognised section, shaped like a `Requirement` row: its title,
    /// collapsible and collapsed by default, with its body on the
    /// `UnrecognizedSectionBody` row directly below when expanded.
    UnrecognizedSection {
        title: &'a str,
        expanded: bool,
        key: RowKey,
    },
    /// An expanded unrecognised section's body, shown in full and unstyled:
    /// never excerpted, truncated, or collapsible in its own right (see
    /// `2026-08-10-unrecognized-sections-in-base/design.md`).
    UnrecognizedSectionBody {
        text: &'a str,
    },
}

impl DiffRow<'_> {
    pub fn is_selectable(&self) -> bool {
        matches!(
            self,
            DiffRow::Requirement { .. }
                | DiffRow::Scenario { .. }
                | DiffRow::Paragraph { .. }
                | DiffRow::ParagraphFull { .. }
                | DiffRow::RemovalNoteLine { .. }
                | DiffRow::RemovalNoteFull { .. }
                | DiffRow::UnrecognizedSection { .. }
        )
    }

    /// The collapse-state key for a selectable row, so key-based toggling
    /// doesn't need to re-derive it from surrounding context. `ParagraphFull`
    /// falls through to `None` deliberately: it has nothing to toggle (see
    /// `2026-08-09-render-purpose/design.md`).
    pub fn key(&self) -> Option<&RowKey> {
        match self {
            DiffRow::Requirement { key, .. }
            | DiffRow::Scenario { key, .. }
            | DiffRow::Paragraph { key, .. }
            | DiffRow::RemovalNoteLine { key, .. }
            | DiffRow::UnrecognizedSection { key, .. } => Some(key),
            _ => None,
        }
    }

    pub fn expanded(&self) -> Option<bool> {
        match self {
            DiffRow::Requirement { expanded, .. }
            | DiffRow::Scenario { expanded, .. }
            | DiffRow::Paragraph { expanded, .. }
            | DiffRow::RemovalNoteLine { expanded, .. }
            | DiffRow::UnrecognizedSection { expanded, .. } => Some(*expanded),
            _ => None,
        }
    }
}

/// Flattens a capability's diff into the rows the right pane renders and
/// navigates over, given which rows the collapse-state set currently marks
/// as expanded and the pane's current width (needed only to decide whether
/// the purpose row, if any, can collapse at all — see
/// `2026-08-09-render-purpose/design.md`). Errors are surfaced as `Notice`
/// rows above the tree, the purpose comparison (if any) follows them, and a
/// group heading is emitted only when the run of entries it introduces is
/// non-empty (see `2026-08-08-tui-specdiff/design.md`, spec.md).
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
        push_requirement(&mut rows, diff, req, expanded, width);
    }

    push_unrecognized_sections(
        &mut rows,
        diff,
        Origin::Delta,
        &diff.delta_unrecognized_sections,
        expanded,
    );
    push_unrecognized_sections(
        &mut rows,
        diff,
        Origin::Base,
        &diff.base_unrecognized_sections,
        expanded,
    );

    rows
}

/// Pushes one origin's heading and one row per unrecognised section it
/// carries, or nothing at all when it carries none. The two origins are
/// pushed in a fixed order (delta's first) rather than one derived from
/// document position: they come from unrelated documents, so there is no
/// order between them to derive. Unlike a paragraph or removal-note row,
/// there's no fits-or-collapses check — an unrecognised section's body is
/// opaque content with no meaningful shorter excerpt to fall back to, so the
/// row is always collapsible and expands straight to its full body (see
/// `2026-08-10-unrecognized-sections-in-base/design.md`).
fn push_unrecognized_sections<'a>(
    rows: &mut Vec<DiffRow<'a>>,
    diff: &'a CapabilityDiff,
    origin: Origin,
    sections: &'a [UnrecognizedSection],
    expanded: &HashSet<RowKey>,
) {
    if sections.is_empty() {
        return;
    }

    rows.push(DiffRow::UnrecognizedSectionsHeading(origin));
    for (index, section) in sections.iter().enumerate() {
        let key = RowKey::UnrecognizedSection {
            capability: diff.capability.clone(),
            origin,
            index,
        };
        let section_expanded = expanded.contains(&key);
        rows.push(DiffRow::UnrecognizedSection {
            title: &section.title,
            expanded: section_expanded,
            key,
        });
        if section_expanded {
            rows.push(DiffRow::UnrecognizedSectionBody {
                text: &section.body,
            });
        }
    }
}

/// Extracts a piece's "current text" — the single passage of ordinary text
/// it renders when collapsed to an excerpt. Every `Piece` variant except
/// `Replaced` is exactly one such passage; a wholesale replacement has no
/// single "current text" and is always collapsible (see
/// `2026-08-09-unified-intro-collapsing/design.md`).
pub(crate) fn paragraph_text(piece: &Piece) -> Option<&str> {
    match piece {
        Piece::Unchanged { text } => Some(text),
        Piece::Added { delta } => Some(delta),
        Piece::Deleted { base } => Some(base),
        Piece::Unmentioned { base } => Some(base),
        Piece::Changed { delta, .. } => Some(delta),
        Piece::Replaced { .. } => None,
    }
}

/// Pushes a paragraph-shaped row (a purpose comparison or a requirement's
/// intro) at the given `key`/`indent`. A wholesale replacement is always
/// collapsible; any other piece is collapsible only when its current text,
/// trimmed of trailing whitespace, doesn't fit the row's available width at
/// `width`/`indent` — the same budget the collapsed row's own truncation
/// uses (`layout::paragraph_available`), so the two decisions can never
/// disagree (see `2026-08-09-render-purpose/design.md`).
fn push_paragraph_row<'a>(
    rows: &mut Vec<DiffRow<'a>>,
    piece: &'a Piece,
    key: RowKey,
    indent: usize,
    width: usize,
    expanded: &HashSet<RowKey>,
) {
    let row_expanded = expanded.contains(&key);

    match paragraph_text(piece) {
        Some(text)
            if text.trim_end().chars().count() <= layout::paragraph_available(width, indent) =>
        {
            rows.push(DiffRow::ParagraphFull { piece, indent });
        }
        _ => rows.push(DiffRow::Paragraph {
            piece,
            expanded: row_expanded,
            key,
            indent,
        }),
    }
}

/// Pushes one row per non-blank line of a removed requirement's removal
/// note, directly above the requirement's intro row, in document order. Each
/// line is at the same indent as the intro row (1), and follows the same
/// fits-in-one-line-else-collapsible convention `push_paragraph_row` uses for
/// a `Piece` (see `2026-08-10-spec-model-removals/design.md`); a blank line produces no
/// row of its own, and a note that is empty (a bare removal) produces none.
fn push_removal_note_rows<'a>(
    rows: &mut Vec<DiffRow<'a>>,
    diff: &'a CapabilityDiff,
    req: &'a RequirementDiff,
    note: &'a str,
    expanded: &HashSet<RowKey>,
    width: usize,
) {
    const INDENT: usize = 1;
    for (i, line) in note.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let key = RowKey::RemovalNoteLine {
            capability: diff.capability.clone(),
            requirement: req.name.clone(),
            line: i,
        };
        if line.trim_end().chars().count() <= layout::paragraph_available(width, INDENT) {
            rows.push(DiffRow::RemovalNoteFull {
                text: line,
                indent: INDENT,
            });
        } else {
            rows.push(DiffRow::RemovalNoteLine {
                text: line,
                expanded: expanded.contains(&key),
                key,
                indent: INDENT,
            });
        }
    }
}

/// Pushes the purpose heading and its one content row.
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
    push_paragraph_row(rows, piece, key, 0, width, expanded);
}

fn push_requirement<'a>(
    rows: &mut Vec<DiffRow<'a>>,
    diff: &'a CapabilityDiff,
    req: &'a RequirementDiff,
    expanded: &HashSet<RowKey>,
    width: usize,
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

    if let Operation::Removed { note } = &req.op {
        push_removal_note_rows(rows, diff, req, note, expanded, width);
    }

    let intro_key = RowKey::Intro {
        capability: diff.capability.clone(),
        requirement: req.name.clone(),
    };
    push_paragraph_row(rows, &req.intro, intro_key, 1, width, expanded);

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
            delta_unrecognized_sections: vec![],
            base_unrecognized_sections: vec![],
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

    fn intro_key(capability: &str, requirement: &str) -> RowKey {
        RowKey::Intro {
            capability: capability.to_string(),
            requirement: requirement.to_string(),
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
                DiffRow::ParagraphFull {
                    piece: &unchanged("the intro"),
                    indent: 1,
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
                DiffRow::ParagraphFull {
                    piece: &unchanged("intro"),
                    indent: 1,
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
                .any(|r| matches!(r, DiffRow::GroupHeading(Operation::Removed { .. })))
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
    fn only_requirement_scenario_and_paragraph_rows_are_selectable() {
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
            DiffRow::Paragraph {
                piece: &added_purpose("x"),
                expanded: false,
                key: purpose_key("cap"),
                indent: 0,
            }
            .is_selectable()
        );
        assert!(
            DiffRow::ParagraphFull {
                piece: &added_purpose("x"),
                indent: 0,
            }
            .is_selectable()
        );
        assert!(
            DiffRow::ParagraphFull {
                piece: &unchanged("x"),
                indent: 1,
            }
            .is_selectable()
        );
        assert!(!DiffRow::GroupHeading(&Operation::Added).is_selectable());
        assert!(!DiffRow::PurposeHeading(&added_purpose("x")).is_selectable());
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
        assert!(
            !rows
                .iter()
                .any(|r| matches!(r, DiffRow::ParagraphFull { .. }))
        );
        assert!(!rows.iter().any(|r| matches!(r, DiffRow::Paragraph { .. })));
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
        assert!(matches!(rows[2], DiffRow::ParagraphFull { indent: 0, .. }));
        assert!(matches!(rows[3], DiffRow::GroupHeading(Operation::Added)));
    }

    #[test]
    fn fitting_added_purpose_text_yields_purpose_full() {
        let diff = diff_with_purpose(added_purpose("short purpose text"));
        let rows = flatten(&diff, &HashSet::new(), WIDE);
        assert!(matches!(rows[1], DiffRow::ParagraphFull { indent: 0, .. }));
        assert!(!rows.iter().any(|r| matches!(r, DiffRow::Paragraph { .. })));
    }

    #[test]
    fn non_fitting_changed_purpose_text_yields_purpose() {
        let long_text = "a very long purpose ".repeat(20);
        let diff = diff_with_purpose(changed_purpose(&long_text));
        let rows = flatten(&diff, &HashSet::new(), WIDE);
        assert!(matches!(rows[1], DiffRow::Paragraph { indent: 0, .. }));
        assert!(
            !rows
                .iter()
                .any(|r| matches!(r, DiffRow::ParagraphFull { .. }))
        );
    }

    #[test]
    fn replaced_purpose_always_yields_purpose_even_when_short() {
        // `replaced_purpose`'s texts are short enough to trivially fit `WIDE`,
        // but a wholesale replacement is never rendered as `ParagraphFull`.
        let diff = diff_with_purpose(replaced_purpose());
        let rows = flatten(&diff, &HashSet::new(), WIDE);
        assert!(matches!(rows[1], DiffRow::Paragraph { indent: 0, .. }));
        assert!(
            !rows
                .iter()
                .any(|r| matches!(r, DiffRow::ParagraphFull { .. }))
        );
    }

    #[test]
    fn purpose_fulls_key_and_expanded_are_none() {
        let row = DiffRow::ParagraphFull {
            piece: &added_purpose("x"),
            indent: 0,
        };
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
        assert!(matches!(rows[1], DiffRow::ParagraphFull { indent: 0, .. }));

        // Narrow the pane down to nothing: the same text no longer fits.
        let rows = flatten(&diff, &HashSet::new(), 0);
        assert!(matches!(rows[1], DiffRow::Paragraph { indent: 0, .. }));
    }

    // --- intro rows (unified-intro-collapsing) ---

    fn diff_with_intro(intro: Piece) -> CapabilityDiff {
        capability_diff(vec![requirement("Req", Operation::Modified, intro, vec![])])
    }

    #[test]
    fn intro_at_indent_1_fits_checks_against_the_narrower_indent_1_budget() {
        // Long enough to not fit purpose's indent-0 budget either, but chosen
        // so that at a width where indent 0 would just barely fit, indent 1
        // (two extra columns of indent) does not.
        let text = "a".repeat(layout::paragraph_available(40, 0));
        let diff = diff_with_intro(unchanged(&text));
        let mut expanded = HashSet::new();
        expanded.insert(req_key("cap", "Req"));

        let rows = flatten(&diff, &expanded, 40);
        let intro_row = rows
            .iter()
            .find(|r| matches!(r, DiffRow::ParagraphFull { .. } | DiffRow::Paragraph { .. }))
            .expect("expected an intro row");
        assert!(
            matches!(intro_row, DiffRow::Paragraph { indent: 1, .. }),
            "expected the intro to be collapsible at indent 1: {intro_row:?}"
        );
    }

    #[test]
    fn intro_with_deleted_piece_gets_a_truncatable_excerpt() {
        let long_text = "a very long deleted intro ".repeat(20);
        let diff = diff_with_intro(Piece::Deleted {
            base: long_text.clone(),
        });
        let mut expanded = HashSet::new();
        expanded.insert(req_key("cap", "Req"));

        let rows = flatten(&diff, &expanded, 40);
        assert!(
            rows.iter()
                .any(|r| matches!(r, DiffRow::Paragraph { indent: 1, .. })),
            "expected a collapsible intro row rather than an always-collapsible placeholder-only row"
        );
    }

    #[test]
    fn intro_with_unmentioned_piece_gets_a_truncatable_excerpt() {
        let long_text = "a very long unmentioned intro ".repeat(20);
        let diff = diff_with_intro(Piece::Unmentioned {
            base: long_text.clone(),
        });
        let mut expanded = HashSet::new();
        expanded.insert(req_key("cap", "Req"));

        let rows = flatten(&diff, &expanded, 40);
        assert!(
            rows.iter()
                .any(|r| matches!(r, DiffRow::Paragraph { indent: 1, .. })),
            "expected a collapsible intro row rather than an always-collapsible placeholder-only row"
        );
    }

    #[test]
    fn intro_row_key_participates_in_the_expanded_set() {
        let long_text = "a very long intro ".repeat(20);
        let diff = diff_with_intro(unchanged(&long_text));
        let mut expanded = HashSet::new();
        expanded.insert(req_key("cap", "Req"));

        let rows = flatten(&diff, &expanded, 40);
        let intro_row = rows
            .iter()
            .find(|r| matches!(r, DiffRow::Paragraph { indent: 1, .. }))
            .expect("expected a collapsible intro row");
        assert_eq!(intro_row.expanded(), Some(false));
        assert_eq!(intro_row.key(), Some(&intro_key("cap", "Req")));

        expanded.insert(intro_key("cap", "Req"));
        let rows = flatten(&diff, &expanded, 40);
        let intro_row = rows
            .iter()
            .find(|r| matches!(r, DiffRow::Paragraph { indent: 1, .. }))
            .expect("expected a collapsible intro row");
        assert_eq!(intro_row.expanded(), Some(true));
    }

    // --- removal-note rows (spec-model-removals) ---

    fn diff_with_removal_note(note: &str) -> CapabilityDiff {
        capability_diff(vec![requirement(
            "Req",
            Operation::Removed {
                note: note.to_string(),
            },
            unchanged("base intro"),
            vec![],
        )])
    }

    fn removal_note_texts<'a>(rows: &[DiffRow<'a>]) -> Vec<&'a str> {
        rows.iter()
            .filter_map(|r| match r {
                DiffRow::RemovalNoteFull { text, .. } => Some(*text),
                DiffRow::RemovalNoteLine { text, .. } => Some(*text),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn removal_note_lines_render_above_the_intro_in_document_order() {
        let note = "**Reason**: no longer needed.\n**Migration**: use the new thing instead.";
        let diff = diff_with_removal_note(note);
        let mut expanded = HashSet::new();
        expanded.insert(req_key("cap", "Req"));

        let rows = flatten(&diff, &expanded, WIDE);
        assert_eq!(
            removal_note_texts(&rows),
            vec![
                "**Reason**: no longer needed.",
                "**Migration**: use the new thing instead.",
            ]
        );

        // Both lines sit directly above the intro row, in the order they
        // appear, despite sharing one markdown paragraph in the source.
        let reason_idx = rows
            .iter()
            .position(|r| matches!(r, DiffRow::RemovalNoteFull { text, .. } if *text == "**Reason**: no longer needed."))
            .expect("expected a Reason row");
        let migration_idx = rows
            .iter()
            .position(|r| matches!(r, DiffRow::RemovalNoteFull { text, .. } if *text == "**Migration**: use the new thing instead."))
            .expect("expected a Migration row");
        let intro_idx = rows
            .iter()
            .position(|r| {
                matches!(
                    r,
                    DiffRow::ParagraphFull { indent: 1, .. } | DiffRow::Paragraph { indent: 1, .. }
                )
            })
            .expect("expected an intro row");
        assert!(reason_idx < migration_idx);
        assert!(migration_idx < intro_idx);
    }

    #[test]
    fn a_blank_line_in_the_removal_note_produces_no_row() {
        let note = "**Reason**: no longer needed.\n\n**Migration**: use the new thing instead.";
        let diff = diff_with_removal_note(note);
        let mut expanded = HashSet::new();
        expanded.insert(req_key("cap", "Req"));

        let rows = flatten(&diff, &expanded, WIDE);
        assert_eq!(
            removal_note_texts(&rows),
            vec![
                "**Reason**: no longer needed.",
                "**Migration**: use the new thing instead.",
            ]
        );
    }

    #[test]
    fn an_unrecognised_removal_note_line_still_renders() {
        let note = "Just some plain text, not Reason or Migration.";
        let diff = diff_with_removal_note(note);
        let mut expanded = HashSet::new();
        expanded.insert(req_key("cap", "Req"));

        let rows = flatten(&diff, &expanded, WIDE);
        assert_eq!(removal_note_texts(&rows), vec![note]);
    }

    #[test]
    fn a_bare_removal_produces_no_removal_note_rows() {
        let diff = diff_with_removal_note("");
        let mut expanded = HashSet::new();
        expanded.insert(req_key("cap", "Req"));

        let rows = flatten(&diff, &expanded, WIDE);
        assert!(removal_note_texts(&rows).is_empty());
        // The intro row is the first row shown after the group heading and
        // the requirement row.
        assert!(matches!(
            rows[2],
            DiffRow::ParagraphFull { indent: 1, .. } | DiffRow::Paragraph { indent: 1, .. }
        ));
    }

    #[test]
    fn a_long_removal_note_line_is_collapsible_and_starts_collapsed() {
        let long_line = format!("**Reason**: {}", "a very long reason ".repeat(20));
        let diff = diff_with_removal_note(&long_line);
        let mut expanded = HashSet::new();
        expanded.insert(req_key("cap", "Req"));

        let rows = flatten(&diff, &expanded, 40);
        let note_row = rows
            .iter()
            .find(|r| matches!(r, DiffRow::RemovalNoteLine { .. }))
            .expect("expected a collapsible removal-note row");
        assert_eq!(note_row.expanded(), Some(false));
    }

    #[test]
    fn a_short_removal_note_line_renders_in_full() {
        let diff = diff_with_removal_note("**Reason**: short.");
        let mut expanded = HashSet::new();
        expanded.insert(req_key("cap", "Req"));

        let rows = flatten(&diff, &expanded, WIDE);
        assert!(
            rows.iter()
                .any(|r| matches!(r, DiffRow::RemovalNoteFull { .. }))
        );
        assert!(
            !rows
                .iter()
                .any(|r| matches!(r, DiffRow::RemovalNoteLine { .. }))
        );
    }

    #[test]
    fn removal_note_rows_are_selectable_and_carry_a_stable_key() {
        let full = DiffRow::RemovalNoteFull {
            text: "**Reason**: short.",
            indent: 1,
        };
        assert!(full.is_selectable());
        assert!(full.key().is_none());
        assert!(full.expanded().is_none());

        let key = RowKey::RemovalNoteLine {
            capability: "cap".to_string(),
            requirement: "Req".to_string(),
            line: 0,
        };
        let collapsible = DiffRow::RemovalNoteLine {
            text: "**Reason**: a very long line that will not fit",
            expanded: false,
            key: key.clone(),
            indent: 1,
        };
        assert!(collapsible.is_selectable());
        assert_eq!(collapsible.key(), Some(&key));
        assert_eq!(collapsible.expanded(), Some(false));
    }

    // --- unrecognised sections (unrecognized-sections-in-base 3.6) ---

    fn sections(titles: Vec<&str>) -> Vec<UnrecognizedSection> {
        titles
            .into_iter()
            .map(|title| UnrecognizedSection {
                title: title.to_string(),
                body: format!("body of {title}"),
            })
            .collect()
    }

    fn diff_with_unrecognized_sections(
        delta_titles: Vec<&str>,
        base_titles: Vec<&str>,
    ) -> CapabilityDiff {
        let mut diff = capability_diff(vec![requirement(
            "Req",
            Operation::Added,
            unchanged("intro"),
            vec![],
        )]);
        diff.delta_unrecognized_sections = sections(delta_titles);
        diff.base_unrecognized_sections = sections(base_titles);
        diff
    }

    fn section_key(origin: Origin, index: usize) -> RowKey {
        RowKey::UnrecognizedSection {
            capability: "cap".to_string(),
            origin,
            index,
        }
    }

    fn heading_origins(rows: &[DiffRow]) -> Vec<Origin> {
        rows.iter()
            .filter_map(|r| match r {
                DiffRow::UnrecognizedSectionsHeading(origin) => Some(*origin),
                _ => None,
            })
            .collect()
    }

    fn section_titles<'a>(rows: &[DiffRow<'a>]) -> Vec<&'a str> {
        rows.iter()
            .filter_map(|r| match r {
                DiffRow::UnrecognizedSection { title, .. } => Some(*title),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn delta_sourced_sections_render_under_their_own_heading_below_requirement_groups() {
        let diff = diff_with_unrecognized_sections(vec!["Bogus One", "Bogus Two"], vec![]);
        let rows = flatten(&diff, &HashSet::new(), WIDE);

        // The heading and its two rows sit at the very end, after every group
        // heading and requirement row.
        assert!(matches!(rows[0], DiffRow::GroupHeading(Operation::Added)));
        assert_eq!(
            rows[rows.len() - 3],
            DiffRow::UnrecognizedSectionsHeading(Origin::Delta)
        );
        assert_eq!(section_titles(&rows), vec!["Bogus One", "Bogus Two"]);
        assert_eq!(heading_origins(&rows), vec![Origin::Delta]);
    }

    #[test]
    fn base_sourced_sections_render_under_their_own_heading() {
        let diff = diff_with_unrecognized_sections(vec![], vec!["From The Base"]);
        let rows = flatten(&diff, &HashSet::new(), WIDE);

        assert_eq!(heading_origins(&rows), vec![Origin::Base]);
        assert_eq!(section_titles(&rows), vec!["From The Base"]);
    }

    #[test]
    fn both_origins_render_delta_first_then_base() {
        let diff = diff_with_unrecognized_sections(vec!["Delta One"], vec!["Base One"]);
        let rows = flatten(&diff, &HashSet::new(), WIDE);

        assert_eq!(heading_origins(&rows), vec![Origin::Delta, Origin::Base]);
        assert_eq!(section_titles(&rows), vec!["Delta One", "Base One"]);

        let tail = &rows[rows.len() - 4..];
        assert_eq!(
            tail,
            [
                DiffRow::UnrecognizedSectionsHeading(Origin::Delta),
                DiffRow::UnrecognizedSection {
                    title: "Delta One",
                    expanded: false,
                    key: section_key(Origin::Delta, 0),
                },
                DiffRow::UnrecognizedSectionsHeading(Origin::Base),
                DiffRow::UnrecognizedSection {
                    title: "Base One",
                    expanded: false,
                    key: section_key(Origin::Base, 0),
                },
            ]
        );
    }

    #[test]
    fn no_unrecognized_sections_renders_neither_heading_nor_row() {
        let diff = diff_with_unrecognized_sections(vec![], vec![]);
        let rows = flatten(&diff, &HashSet::new(), WIDE);
        assert!(heading_origins(&rows).is_empty());
        assert!(section_titles(&rows).is_empty());
        assert!(
            !rows
                .iter()
                .any(|r| matches!(r, DiffRow::UnrecognizedSectionBody { .. }))
        );
    }

    #[test]
    fn an_unrecognized_section_row_is_selectable_and_starts_collapsed() {
        let diff = diff_with_unrecognized_sections(vec!["Bogus"], vec![]);
        let rows = flatten(&diff, &HashSet::new(), WIDE);
        let section_row = rows
            .iter()
            .find(|r| matches!(r, DiffRow::UnrecognizedSection { .. }))
            .expect("expected an UnrecognizedSection row");

        assert!(section_row.is_selectable());
        assert_eq!(section_row.key(), Some(&section_key(Origin::Delta, 0)));
        assert_eq!(section_row.expanded(), Some(false));
        // Collapsed: no body row of its own.
        assert!(
            !rows
                .iter()
                .any(|r| matches!(r, DiffRow::UnrecognizedSectionBody { .. }))
        );

        let heading_row = rows
            .iter()
            .find(|r| matches!(r, DiffRow::UnrecognizedSectionsHeading(_)))
            .expect("expected an UnrecognizedSectionsHeading row");
        assert!(!heading_row.is_selectable());
        assert_eq!(heading_row.key(), None);
    }

    #[test]
    fn expanding_a_section_reveals_its_full_body_however_long() {
        let long_body = "a very long unrecognised body ".repeat(50);
        let mut diff = diff_with_unrecognized_sections(vec!["Bogus"], vec![]);
        diff.delta_unrecognized_sections[0].body = long_body.clone();

        let mut expanded = HashSet::new();
        expanded.insert(section_key(Origin::Delta, 0));

        // Narrow pane: the body is still carried in full, with no
        // fits-or-collapses check of its own.
        let rows = flatten(&diff, &expanded, 20);
        let body_row = rows
            .iter()
            .find(|r| matches!(r, DiffRow::UnrecognizedSectionBody { .. }))
            .expect("expected an UnrecognizedSectionBody row");
        match body_row {
            DiffRow::UnrecognizedSectionBody { text } => assert_eq!(*text, long_body),
            other => panic!("expected UnrecognizedSectionBody, got {other:?}"),
        }
        assert!(!body_row.is_selectable());
        assert_eq!(body_row.expanded(), None);
    }

    #[test]
    fn each_section_row_carries_its_own_index_keyed_collapse_state() {
        // Two sections sharing a title, one per origin: only the index-keyed
        // row the expanded set names opens.
        let diff = diff_with_unrecognized_sections(vec!["Same", "Same"], vec!["Same"]);
        let mut expanded = HashSet::new();
        expanded.insert(section_key(Origin::Delta, 1));

        let rows = flatten(&diff, &expanded, WIDE);
        let states: Vec<Option<bool>> = rows
            .iter()
            .filter(|r| matches!(r, DiffRow::UnrecognizedSection { .. }))
            .map(|r| r.expanded())
            .collect();
        assert_eq!(states, vec![Some(false), Some(true), Some(false)]);
        assert_eq!(
            rows.iter()
                .filter(|r| matches!(r, DiffRow::UnrecognizedSectionBody { .. }))
                .count(),
            1
        );
    }
}
