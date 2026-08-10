use std::ops::Range;

use similar::{ChangeTag, TextDiff};

use crate::diff::model::Run;

/// Word-level diff runs between `base` and `delta`, as byte-offset ranges
/// into the two strings exactly as supplied — no trimming, whitespace
/// collapsing, re-wrapping or other normalisation (see
/// `2026-08-08-spec-diff/design.md`). Adjacent
/// changes sharing a tag are merged into a single run, and a whitespace-only
/// equal run with a change on both sides is coalesced away rather than left
/// to anchor the diff (see `2026-08-08-diff-legibility/design.md`).
pub(crate) fn runs(base: &str, delta: &str) -> Vec<Run> {
    let diff = TextDiff::from_words(base, delta);
    let mut out: Vec<Run> = Vec::new();
    let mut base_pos = 0usize;
    let mut delta_pos = 0usize;

    for change in diff.iter_all_changes() {
        let len = change.value().len();
        let run = match change.tag() {
            ChangeTag::Equal => {
                let run = Run::Equal {
                    base: base_pos..base_pos + len,
                    delta: delta_pos..delta_pos + len,
                };
                base_pos += len;
                delta_pos += len;
                run
            }
            ChangeTag::Delete => {
                let run = Run::Delete {
                    base: base_pos..base_pos + len,
                };
                base_pos += len;
                run
            }
            ChangeTag::Insert => {
                let run = Run::Insert {
                    delta: delta_pos..delta_pos + len,
                };
                delta_pos += len;
                run
            }
        };
        push_merging(&mut out, run);
    }

    coalesce_whitespace_anchors(base, out)
}

/// An `Equal` run whose text is entirely whitespace, contains no line break,
/// and has a non-`Equal` run on both sides carries no information as an
/// anchor: dissolving it lets its span be reported as one deletion and one
/// insertion instead of two of each. A run at either end of the sequence has
/// no change on that side, so it is never dissolved. A line break is excluded
/// because line structure is meaningful in this content (a scenario body's
/// `WHEN`/`THEN` bullets); dissolving it could span one deletion across two
/// bullets.
fn is_whitespace_anchor(base: &str, runs: &[Run], i: usize) -> bool {
    let Run::Equal { base: b, .. } = &runs[i] else {
        return false;
    };
    if i == 0 || i + 1 == runs.len() {
        return false;
    }
    let text = &base[b.clone()];
    !text.is_empty() && !text.contains('\n') && text.chars().all(char::is_whitespace)
}

/// Removes whitespace-only anchor runs, coalescing each resulting span into a
/// single `Delete` of its base text followed by a single `Insert` of its
/// delta text. Works at region granularity — splitting an individual equal
/// run into a delete/insert pair and relying on adjacent same-tag merging
/// does not work, since nothing then ends up adjacent to a same-tag neighbour
/// (see `2026-08-08-diff-legibility/design.md`, Decisions). This can only
/// ever destroy `Equal` runs, never
/// fabricate one, so it preserves reconstruction: base order among deletes
/// and delta order among inserts are both unchanged.
fn coalesce_whitespace_anchors(base: &str, runs: Vec<Run>) -> Vec<Run> {
    let n = runs.len();
    let survives = |runs: &[Run], i: usize| {
        matches!(runs[i], Run::Equal { .. }) && !is_whitespace_anchor(base, runs, i)
    };

    let mut out = Vec::new();
    let mut i = 0;
    while i < n {
        if survives(&runs, i) {
            out.push(runs[i].clone());
            i += 1;
            continue;
        }
        let start = i;
        while i < n && !survives(&runs, i) {
            i += 1;
        }
        out.extend(coalesce_span(&runs[start..i]));
    }
    out
}

/// Merges a span containing no surviving anchor into at most one `Delete`
/// (covering every base-advancing run in the span) followed by at most one
/// `Insert` (covering every delta-advancing run). The runs within a span are
/// contiguous in each side's positions by construction, so the merged range
/// is simply the min start to the max end.
fn coalesce_span(span: &[Run]) -> Vec<Run> {
    let mut base_range: Option<Range<usize>> = None;
    let mut delta_range: Option<Range<usize>> = None;

    for run in span {
        match run {
            Run::Equal { base, delta } => {
                extend(&mut base_range, base);
                extend(&mut delta_range, delta);
            }
            Run::Delete { base } => extend(&mut base_range, base),
            Run::Insert { delta } => extend(&mut delta_range, delta),
        }
    }

    let mut out = Vec::new();
    if let Some(base) = base_range {
        out.push(Run::Delete { base });
    }
    if let Some(delta) = delta_range {
        out.push(Run::Insert { delta });
    }
    out
}

fn extend(acc: &mut Option<Range<usize>>, r: &Range<usize>) {
    match acc {
        None => *acc = Some(r.clone()),
        Some(a) => {
            a.start = a.start.min(r.start);
            a.end = a.end.max(r.end);
        }
    }
}

/// Extends the last run in place when it shares a tag and abuts the new one,
/// rather than pushing a new adjacent run of the same kind.
fn push_merging(out: &mut Vec<Run>, run: Run) {
    match (out.last_mut(), &run) {
        (
            Some(Run::Equal { base: b, delta: d }),
            Run::Equal {
                base: nb,
                delta: nd,
            },
        ) if b.end == nb.start && d.end == nd.start => {
            b.end = nb.end;
            d.end = nd.end;
        }
        (Some(Run::Delete { base: b }), Run::Delete { base: nb }) if b.end == nb.start => {
            b.end = nb.end;
        }
        (Some(Run::Insert { delta: d }), Run::Insert { delta: nd }) if d.end == nd.start => {
            d.end = nd.end;
        }
        _ => out.push(run),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reconstruct_base(base: &str, runs: &[Run]) -> String {
        let mut out = String::new();
        for run in runs {
            match run {
                Run::Equal { base: r, .. } => out.push_str(&base[r.clone()]),
                Run::Delete { base: r } => out.push_str(&base[r.clone()]),
                Run::Insert { .. } => {}
            }
        }
        out
    }

    fn reconstruct_delta(delta: &str, runs: &[Run]) -> String {
        let mut out = String::new();
        for run in runs {
            match run {
                Run::Equal { delta: r, .. } => out.push_str(&delta[r.clone()]),
                Run::Insert { delta: r } => out.push_str(&delta[r.clone()]),
                Run::Delete { .. } => {}
            }
        }
        out
    }

    fn assert_reconstructs(base: &str, delta: &str) {
        let runs = runs(base, delta);
        assert_eq!(
            reconstruct_base(base, &runs),
            base,
            "base did not reconstruct for base={base:?} delta={delta:?}"
        );
        assert_eq!(
            reconstruct_delta(delta, &runs),
            delta,
            "delta did not reconstruct for base={base:?} delta={delta:?}"
        );
        assert_equal_runs_address_identical_text(base, delta, &runs);
    }

    /// Reconstruction alone cannot catch a mis-aligned equal run: it only
    /// checks that concatenating the right spans reproduces each side, not
    /// that an `Equal` run's two spans actually agree with each other.
    fn assert_equal_runs_address_identical_text(base: &str, delta: &str, runs: &[Run]) {
        for run in runs {
            if let Run::Equal { base: b, delta: d } = run {
                assert_eq!(
                    &base[b.clone()],
                    &delta[d.clone()],
                    "equal run addressed different text on each side for base={base:?} delta={delta:?}: {run:?}"
                );
            }
        }
    }

    // --- 2.2: reconstruction invariant ---

    #[test]
    fn reconstruction_invariant_holds_across_cases() {
        assert_reconstructs(
            "line one\nline two\nline three\n",
            "line one\nline TWO\nline three\n",
        );
        assert_reconstructs(
            "changed start here\nrest unchanged\n",
            "DIFFERENT start\nrest unchanged\n",
        );
        assert_reconstructs(
            "rest unchanged\nchanged end here\n",
            "rest unchanged\nDIFFERENT end\n",
        );
        assert_reconstructs(
            "start unchanged\nmiddle original\nend unchanged\n",
            "start unchanged\nmiddle EDITED\nend unchanged\n",
        );
        assert_reconstructs("", "some new content");
        assert_reconstructs("some old content", "");
        assert_reconstructs("identical text here", "identical text here");
    }

    // --- 2.3: word granularity ---

    #[test]
    fn edits_are_word_level_not_line_level() {
        let base = "the quick brown fox jumps over the lazy dog in the morning light";
        let delta = "the quick brown fox leaps over the lazy dog in the morning light";
        let runs = runs(base, delta);

        // There should be equal runs covering the untouched prefix and suffix,
        // not a single whole-line delete plus insert.
        assert!(
            runs.iter()
                .any(|r| matches!(r, Run::Equal { base: b, .. } if base[b.clone()].contains("the quick brown fox")))
        );
        assert!(
            runs.iter()
                .any(|r| matches!(r, Run::Equal { base: b, .. } if base[b.clone()].contains("over the lazy dog")))
        );
        assert!(
            runs.iter()
                .any(|r| matches!(r, Run::Delete { base: b } if base[b.clone()] == *"jumps"))
        );
        assert!(
            runs.iter()
                .any(|r| matches!(r, Run::Insert { delta: d } if delta[d.clone()] == *"leaps"))
        );
    }

    // --- 2.4: UTF-8 safety ---

    #[test]
    fn run_boundaries_are_char_boundary_safe_on_multibyte_utf8() {
        let base = "the base spec says A → B and uses an em dash — here";
        let delta = "the base spec says A → C and uses an em dash — there";
        let runs = runs(base, delta);
        for run in &runs {
            match run {
                Run::Equal { base: b, delta: d } => {
                    assert!(base.is_char_boundary(b.start) && base.is_char_boundary(b.end));
                    assert!(delta.is_char_boundary(d.start) && delta.is_char_boundary(d.end));
                    let _ = &base[b.clone()];
                    let _ = &delta[d.clone()];
                }
                Run::Delete { base: b } => {
                    assert!(base.is_char_boundary(b.start) && base.is_char_boundary(b.end));
                    let _ = &base[b.clone()];
                }
                Run::Insert { delta: d } => {
                    assert!(delta.is_char_boundary(d.start) && delta.is_char_boundary(d.end));
                    let _ = &delta[d.clone()];
                }
            }
        }
        assert_reconstructs(base, delta);
    }

    // --- 2.5: trailing append ---

    #[test]
    fn trailing_append_yields_one_insert_and_no_delete() {
        let base = "The spec of record says one thing about the feature.";
        let delta = format!("{base} And this sentence was appended after it.");
        let runs = runs(base, &delta);

        assert!(
            !runs.iter().any(|r| matches!(r, Run::Delete { .. })),
            "expected no delete runs, got {runs:?}"
        );
        let insert_count = runs
            .iter()
            .filter(|r| matches!(r, Run::Insert { .. }))
            .count();
        assert_eq!(
            insert_count, 1,
            "expected exactly one insert run, got {runs:?}"
        );
        assert_reconstructs(base, &delta);
    }

    // --- diff-legibility 2.4: whitespace no longer anchors a diff ---

    #[test]
    fn whitespace_between_two_word_edits_does_not_anchor_the_diff() {
        let base = "- **WHEN** the user quits the application via Ctrl+Q";
        let delta = "- **WHEN** the user quits the application by pressing `q`";
        let runs = runs(base, delta);

        let deletes: Vec<&Run> = runs
            .iter()
            .filter(|r| matches!(r, Run::Delete { .. }))
            .collect();
        let inserts: Vec<&Run> = runs
            .iter()
            .filter(|r| matches!(r, Run::Insert { .. }))
            .collect();
        assert_eq!(
            deletes.len(),
            1,
            "expected exactly one delete run, got {runs:?}"
        );
        assert_eq!(
            inserts.len(),
            1,
            "expected exactly one insert run, got {runs:?}"
        );

        let Run::Delete { base: b } = deletes[0] else {
            unreachable!()
        };
        let Run::Insert { delta: d } = inserts[0] else {
            unreachable!()
        };
        assert_eq!(&base[b.clone()], "via Ctrl+Q");
        assert_eq!(&delta[d.clone()], "by pressing `q`");

        assert_reconstructs(base, delta);
    }

    // --- diff-legibility 2.5: non-whitespace equal text still anchors ---

    #[test]
    fn non_whitespace_equal_text_still_anchors_the_diff() {
        let base = "a b c d e";
        let delta = "a X c Y e";
        let runs = runs(base, delta);

        assert!(
            runs.iter()
                .any(|r| matches!(r, Run::Equal { base: b, .. } if base[b.clone()].contains('c'))),
            "expected \"c\" to still anchor the diff, got {runs:?}"
        );
        let deletes: Vec<&Run> = runs
            .iter()
            .filter(|r| matches!(r, Run::Delete { .. }))
            .collect();
        let inserts: Vec<&Run> = runs
            .iter()
            .filter(|r| matches!(r, Run::Insert { .. }))
            .collect();
        assert_eq!(
            deletes.len(),
            2,
            "expected two distinct deletes, got {runs:?}"
        );
        assert_eq!(
            inserts.len(),
            2,
            "expected two distinct inserts, got {runs:?}"
        );

        assert_reconstructs(base, delta);
    }

    // --- diff-legibility 2.6: a whitespace anchor containing a newline still anchors ---

    #[test]
    fn whitespace_anchor_containing_a_newline_still_anchors() {
        let base = "- **WHEN** a\n- **THEN** b";
        let delta = "- **WHEN** x\n- **THEN** y";
        let runs = runs(base, delta);

        // The scenario body's WHEN and THEN lines both change; the newline
        // between them must still separate the two into distinct edits
        // rather than collapsing into one deletion spanning both bullets.
        assert!(
            runs.iter()
                .any(|r| matches!(r, Run::Equal { base: b, .. } if base[b.clone()].contains('\n'))),
            "expected the newline-containing equal run to still anchor, got {runs:?}"
        );
        let deletes: Vec<&Run> = runs
            .iter()
            .filter(|r| matches!(r, Run::Delete { .. }))
            .collect();
        let inserts: Vec<&Run> = runs
            .iter()
            .filter(|r| matches!(r, Run::Insert { .. }))
            .collect();
        assert_eq!(
            deletes.len(),
            2,
            "expected two distinct deletes, got {runs:?}"
        );
        assert_eq!(
            inserts.len(),
            2,
            "expected two distinct inserts, got {runs:?}"
        );

        assert_reconstructs(base, delta);
    }
}
