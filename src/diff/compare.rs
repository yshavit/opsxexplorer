use std::collections::{HashMap, HashSet};

use crate::diff::model::{Piece, Run, ScenarioDiff};
use crate::diff::runs::runs;
use crate::specs::Requirement;

/// Below this similarity, a piece is reported as a wholesale replacement
/// rather than as runs. Measured as
/// `2 × equal_bytes ÷ (base_bytes + delta_bytes)` over the coalesced runs.
///
/// Calibrated against every changed piece in this repo's own archive:
///
/// | similarity | piece | verdict |
/// |---|---|---|
/// | 0.238 | `tui-specdiff` rename, *Focus moves between the two panes* intro | replace |
/// | 0.483 | `changelist-archived-ordering` intro | keep inline |
/// | 0.909 | `tui-keybinding-improvements`, *normal exit restores terminal* | keep inline |
///
/// 0.238 → 0.483 is the largest gap in that distribution (0.245; the next
/// largest is 0.095), so the threshold sits in the widest empty band the
/// data offers. Full table and the rejected alternatives:
/// `2026-08-08-diff-legibility/design.md`.
const INLINE_DIFF_MIN_SIMILARITY: f32 = 0.35;

/// The fraction of the two texts' combined bytes that the coalesced runs
/// report as equal. Derived from the runs `runs()` already returned rather
/// than a second `TextDiff` pass, so the measure and the reported runs can
/// never disagree (see `2026-08-08-diff-legibility/design.md`).
fn similarity(base: &str, delta: &str, runs: &[Run]) -> f32 {
    let equal_bytes: usize = runs
        .iter()
        .map(|run| match run {
            Run::Equal { base, .. } => base.len(),
            Run::Delete { .. } | Run::Insert { .. } => 0,
        })
        .sum();
    let total_bytes = base.len() + delta.len();
    if total_bytes == 0 {
        return 1.0;
    }
    (2 * equal_bytes) as f32 / total_bytes as f32
}

/// The rule applied everywhere: absence in the delta means unmentioned,
/// presence means authoritative for that piece (see
/// `2026-08-08-spec-diff/design.md`). A piece too
/// dissimilar for an inline reading is reported as a wholesale replacement
/// instead, unless either side is empty — there is then nothing to compare
/// and no interleaving to avoid.
pub(crate) fn changed_or_unchanged(base: &str, delta: &str) -> Piece {
    if delta == base {
        return Piece::Unchanged {
            text: delta.to_string(),
        };
    }

    let runs = runs(base, delta);
    let is_replacement = !base.is_empty()
        && !delta.is_empty()
        && similarity(base, delta, &runs) < INLINE_DIFF_MIN_SIMILARITY;

    if is_replacement {
        Piece::Replaced {
            base: base.to_string(),
            delta: delta.to_string(),
        }
    } else {
        Piece::Changed {
            base: base.to_string(),
            delta: delta.to_string(),
            runs,
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

/// A scenario's place in the reported order: category first (added, then
/// modified, then unmentioned, then unchanged), then a tie-break index within
/// that category (see `compare_requirement`).
///
/// `Piece::Deleted` never reaches this function — `compare_requirement` only
/// ever constructs `Added`, `Changed`, `Replaced`, `Unmentioned` or
/// `Unchanged` pieces; `Deleted` is produced solely by the separate removal
/// path in `diff::diff`, which recovers a whole removed requirement's
/// scenarios from the spec of record and does not call this function.
fn category_rank(piece: &Piece) -> u8 {
    match piece {
        Piece::Added { .. } => 0,
        Piece::Changed { .. } | Piece::Replaced { .. } => 1,
        Piece::Unmentioned { .. } => 2,
        Piece::Unchanged { .. } => 3,
        Piece::Deleted { .. } => {
            unreachable!("compare_requirement never produces a Deleted piece")
        }
    }
}

/// Compares a base requirement's content against a delta requirement's,
/// applying the uniform rule to the intro and matching scenarios by name.
/// Scenarios are reported grouped by category — added, then modified
/// (changed or replaced), then unmentioned, then unchanged — so the pieces
/// most relevant to a reviewer come first. Within a category, ties are
/// broken by the delta's order for added scenarios and by the base's order
/// for every other category. Duplicate names resolve first-wins on both
/// sides (see `2026-08-08-spec-diff/design.md`).
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

    let mut scenarios: Vec<(ScenarioDiff, (u8, usize))> = Vec::new();
    let mut seen: HashSet<&str> = HashSet::new();

    for (index, base_scenario) in base.scenarios.iter().enumerate() {
        if !seen.insert(base_scenario.name.as_str()) {
            continue;
        }
        let body = match delta_by_name.get(base_scenario.name.as_str()) {
            Some(delta_scenario) => changed_or_unchanged(&base_scenario.body, &delta_scenario.body),
            None => Piece::Unmentioned {
                base: base_scenario.body.clone(),
            },
        };
        let key = (category_rank(&body), index);
        scenarios.push((
            ScenarioDiff {
                name: base_scenario.name.clone(),
                body,
            },
            key,
        ));
    }

    for (index, delta_scenario) in delta.scenarios.iter().enumerate() {
        if !seen.insert(delta_scenario.name.as_str()) {
            continue;
        }
        let body = Piece::Added {
            delta: delta_scenario.body.clone(),
        };
        let key = (category_rank(&body), index);
        scenarios.push((
            ScenarioDiff {
                name: delta_scenario.name.clone(),
                body,
            },
            key,
        ));
    }

    scenarios.sort_by_key(|(_, key)| *key);

    (intro, scenarios.into_iter().map(|(s, _)| s).collect())
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
        // Unmentioned outranks unchanged, so D is reported first.
        assert_eq!(req.scenarios[0].name, "D");
        assert_eq!(
            req.scenarios[0].body,
            Piece::Unmentioned {
                base: "- **WHEN** d\n- **THEN** d2".to_string()
            }
        );
        assert!(matches!(req.scenarios[1].body, Piece::Unchanged { .. }));
        assert!(matches!(req.scenarios[2].body, Piece::Unchanged { .. }));
        assert!(matches!(req.scenarios[3].body, Piece::Unchanged { .. }));
        assert!(
            !req.scenarios
                .iter()
                .any(|s| matches!(s.body, Piece::Deleted { .. }))
        );
        assert_eq!(req.op, Operation::Modified);
    }

    // --- diff-legibility 3.5-3.8: Piece::Replaced ---
    //
    // Fixtures below are inlined from this repo's own archive (see
    // `2026-08-08-diff-legibility/design.md`'s calibration table) rather
    // than read from the archive directory, so the test survives this
    // change's own archiving.

    // `tui-specdiff`'s rename of "Left pane holds input focus" to "Focus
    // moves between the two panes" — the archive's worst-scoring piece,
    // similarity 0.238.
    const RENAME_INTRO_BASE: &str = "The left pane SHALL hold keyboard input focus for the duration of the application's runtime. There SHALL be no mechanism to move focus to the right pane.";
    const RENAME_INTRO_DELTA: &str = "Exactly one pane SHALL hold keyboard input focus at any time. The left pane SHALL hold focus when the application starts. The system SHALL move focus to the other pane when the user presses Tab, and SHALL indicate which pane currently holds focus visually. A key pressed while a pane holds focus SHALL be handled by that pane, except for keys the application handles globally.";

    #[test]
    fn a_substantially_rewritten_intro_is_reported_as_replaced() {
        match changed_or_unchanged(RENAME_INTRO_BASE, RENAME_INTRO_DELTA) {
            Piece::Replaced { .. } => {}
            other => panic!("expected Replaced, got {other:?}"),
        }
    }

    #[test]
    fn the_next_nearest_pieces_are_still_reported_as_changed() {
        // `tui-specdiff`'s "user presses any key" scenario body, ~0.503.
        let scenario_a_base = "- **WHEN** the user presses a key while the application is running\n- **THEN** the key is handled by the left pane, since no other pane can hold focus";
        let scenario_a_delta = "- **WHEN** the user presses a key that both panes bind, while one of them holds focus\n- **THEN** the key is handled by the pane that holds focus, and the other pane's state is unchanged";

        // `tui-specdiff`'s "application launches" scenario body, ~0.585.
        let scenario_b_base = "- **WHEN** the application starts\n- **THEN** keyboard input is directed to the left pane";
        let scenario_b_delta = "- **WHEN** the application starts\n- **THEN** the left pane holds keyboard input focus, and this is visually indicated";

        // `changelist-archived-ordering`'s intro, ~0.483.
        let ordering_intro_base = "When expanded, the archived section SHALL list archived changes sorted alphabetically by their full directory name (date prefix included). Each SHALL be displayed as its date followed by its change name (date prefix removed from the name portion), with the date rendered in a visually de-emphasized (dimmed) style relative to the change name.";
        let ordering_intro_delta = "When expanded, the archived section SHALL list archived changes ordered by their `YYYY-MM-DD` date prefix descending (most recent date first). Archived changes sharing the same date SHALL be ordered by the timestamp of the commit that first introduced the change's directory in git history, descending (most recently introduced first); a change whose introducing commit cannot be resolved (for example, an uncommitted change, or no enclosing git repository) SHALL sort as more recent than any change whose introducing commit can be resolved. Archived changes that remain tied after applying both the date and commit-timestamp comparisons SHALL be ordered by their full directory name (date prefix included), ascending. Each archived change SHALL be displayed as its date followed by its change name (date prefix removed from the name portion), with the date rendered in a visually de-emphasized (dimmed) style relative to the change name.";

        for (base, delta) in [
            (scenario_a_base, scenario_a_delta),
            (scenario_b_base, scenario_b_delta),
            (ordering_intro_base, ordering_intro_delta),
        ] {
            match changed_or_unchanged(base, delta) {
                Piece::Changed { .. } => {}
                other => panic!("expected Changed for base={base:?}, got {other:?}"),
            }
        }
    }

    #[test]
    fn an_empty_side_is_never_reported_as_replaced() {
        match changed_or_unchanged(
            "",
            "a wholly new intro with nothing in common to match against",
        ) {
            Piece::Changed { base, delta, .. } => {
                assert_eq!(base, "");
                assert_eq!(
                    delta,
                    "a wholly new intro with nothing in common to match against"
                );
            }
            other => panic!("expected Changed, got {other:?}"),
        }
    }

    #[test]
    fn a_replaced_piece_carries_both_texts_unmodified() {
        match changed_or_unchanged(RENAME_INTRO_BASE, RENAME_INTRO_DELTA) {
            Piece::Replaced { base, delta } => {
                assert_eq!(base, RENAME_INTRO_BASE);
                assert_eq!(delta, RENAME_INTRO_DELTA);
            }
            other => panic!("expected Replaced, got {other:?}"),
        }
    }

    // --- scenarios-ordering: category-based scenario order ---

    #[test]
    fn scenarios_are_grouped_by_category() {
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
            - **THEN** c2\n";
        // A is restated unchanged, B is restated with an edit, C is omitted
        // (unmentioned), and D is new (added).
        let delta_md = "## MODIFIED Requirements\n\n\
            ### Requirement: Foo\n\
            Intro.\n\n\
            #### Scenario: A\n\
            - **WHEN** a\n\
            - **THEN** a2\n\n\
            #### Scenario: B\n\
            - **WHEN** b\n\
            - **THEN** b2 with a small edit appended\n\n\
            #### Scenario: D\n\
            - **WHEN** d\n\
            - **THEN** d2\n";
        let pair = pair_from_markdown(delta_md, Some(base_md));
        let result = diff("cap", &pair);
        let req = only_requirement(&result);

        let names: Vec<&str> = req.scenarios.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["D", "B", "C", "A"]);
        assert!(matches!(req.scenarios[0].body, Piece::Added { .. }));
        assert!(matches!(req.scenarios[1].body, Piece::Changed { .. }));
        assert!(matches!(req.scenarios[2].body, Piece::Unmentioned { .. }));
        assert!(matches!(req.scenarios[3].body, Piece::Unchanged { .. }));
    }

    #[test]
    fn added_scenarios_within_their_category_follow_the_deltas_order() {
        let base_md = "## Requirements\n\n\
            ### Requirement: Foo\n\
            Intro.\n\n\
            #### Scenario: Keep\n\
            - **WHEN** k\n\
            - **THEN** k2\n";
        // Delta lists Keep first, then Zeta, then Alpha — Zeta and Alpha are
        // both new, in that order, even though alphabetically Alpha comes
        // first and Keep sits ahead of both in the base.
        let delta_md = "## MODIFIED Requirements\n\n\
            ### Requirement: Foo\n\
            Intro.\n\n\
            #### Scenario: Keep\n\
            - **WHEN** k\n\
            - **THEN** k2\n\n\
            #### Scenario: Zeta\n\
            - **WHEN** z\n\
            - **THEN** z2\n\n\
            #### Scenario: Alpha\n\
            - **WHEN** al\n\
            - **THEN** al2\n";
        let pair = pair_from_markdown(delta_md, Some(base_md));
        let result = diff("cap", &pair);
        let req = only_requirement(&result);

        let names: Vec<&str> = req.scenarios.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["Zeta", "Alpha", "Keep"]);
    }

    #[test]
    fn modified_scenarios_within_their_category_follow_the_base_order_not_the_deltas() {
        let base_md = "## Requirements\n\n\
            ### Requirement: Foo\n\
            Intro.\n\n\
            #### Scenario: X\n\
            - **WHEN** x\n\
            - **THEN** x2\n\n\
            #### Scenario: Y\n\
            - **WHEN** y\n\
            - **THEN** y2\n";
        // Delta restates both with edits, listing Y before X — the reverse
        // of the base's order.
        let delta_md = "## MODIFIED Requirements\n\n\
            ### Requirement: Foo\n\
            Intro.\n\n\
            #### Scenario: Y\n\
            - **WHEN** y\n\
            - **THEN** y2 with an edit appended\n\n\
            #### Scenario: X\n\
            - **WHEN** x\n\
            - **THEN** x2 with an edit appended\n";
        let pair = pair_from_markdown(delta_md, Some(base_md));
        let result = diff("cap", &pair);
        let req = only_requirement(&result);

        let names: Vec<&str> = req.scenarios.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["X", "Y"]);
        assert!(
            req.scenarios
                .iter()
                .all(|s| matches!(s.body, Piece::Changed { .. }))
        );
    }

    #[test]
    fn unchanged_scenarios_within_their_category_follow_the_base_order_not_the_deltas() {
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
            - **THEN** c2\n";
        // Delta restates all three unchanged, in a different order than the
        // base, and also adds Z — proving the unchanged trio still sorts by
        // base order rather than by this delta order.
        let delta_md = "## MODIFIED Requirements\n\n\
            ### Requirement: Foo\n\
            Intro.\n\n\
            #### Scenario: Z\n\
            - **WHEN** z\n\
            - **THEN** z2\n\n\
            #### Scenario: C\n\
            - **WHEN** c\n\
            - **THEN** c2\n\n\
            #### Scenario: A\n\
            - **WHEN** a\n\
            - **THEN** a2\n\n\
            #### Scenario: B\n\
            - **WHEN** b\n\
            - **THEN** b2\n";
        let pair = pair_from_markdown(delta_md, Some(base_md));
        let result = diff("cap", &pair);
        let req = only_requirement(&result);

        let names: Vec<&str> = req.scenarios.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["Z", "A", "B", "C"]);
    }

    #[test]
    fn repeated_comparison_of_a_modified_entry_is_stable() {
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
            - **THEN** b2 with an edit appended\n\n\
            #### Scenario: A\n\
            - **WHEN** a\n\
            - **THEN** a2\n\n\
            #### Scenario: C\n\
            - **WHEN** c\n\
            - **THEN** c2\n";
        let pair = pair_from_markdown(delta_md, Some(base_md));
        let first = diff("cap", &pair);
        let second = diff("cap", &pair);
        assert_eq!(
            only_requirement(&first).scenarios,
            only_requirement(&second).scenarios
        );
    }

    #[test]
    fn removed_requirements_scenarios_stay_in_the_spec_of_records_order() {
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
            - **THEN** c2\n";
        let delta_md = "## REMOVED Requirements\n\n\
            ### Requirement: Foo\n";
        let pair = pair_from_markdown(delta_md, Some(base_md));
        let result = diff("cap", &pair);
        let req = only_requirement(&result);

        assert_eq!(
            req.op,
            Operation::Removed {
                note: String::new()
            }
        );
        let names: Vec<&str> = req.scenarios.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["A", "B", "C"]);
        assert!(
            req.scenarios
                .iter()
                .all(|s| matches!(s.body, Piece::Deleted { .. }))
        );
    }
}
