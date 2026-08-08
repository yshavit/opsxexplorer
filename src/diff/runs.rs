use similar::{ChangeTag, TextDiff};

use crate::diff::model::Run;

/// Word-level diff runs between `base` and `delta`, as byte-offset ranges
/// into the two strings exactly as supplied — no trimming, whitespace
/// collapsing, re-wrapping or other normalisation (see design.md). Adjacent
/// changes sharing a tag are merged into a single run.
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

    out
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
}
