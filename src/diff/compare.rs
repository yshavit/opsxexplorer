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
/// data offers. Full table and the rejected alternatives: the
/// `diff-legibility` change's design.md.
const INLINE_DIFF_MIN_SIMILARITY: f32 = 0.35;

/// The fraction of the two texts' combined bytes that the coalesced runs
/// report as equal. Derived from the runs `runs()` already returned rather
/// than a second `TextDiff` pass, so the measure and the reported runs can
/// never disagree (see design.md).
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
/// presence means authoritative for that piece (see design.md). A piece too
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

    // --- diff-legibility 3.5-3.8: Piece::Replaced ---
    //
    // Fixtures below are inlined from this repo's own archive (see
    // design.md's calibration table) rather than read from the archive
    // directory, so the test survives this change's own archiving.

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
}
