use std::ops::Range;

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::diff::{Operation, Piece, Run};

use super::diff_row::DiffRow;
use super::wrap::{chars_to_spans, wrap_spans};

/// Width, in columns, of the gutter column: one marker character plus one
/// column of separating space.
const GUTTER_WIDTH: usize = 2;
/// Columns of indent per nesting level (requirement -> piece -> body).
const INDENT_UNIT: usize = 2;

/// Builds a row's spans, wraps them to the space left after the gutter and
/// indent, and lays out the result as full lines: the marker prefixes only
/// the first line, continuation lines get a blank gutter, and every line is
/// left-padded by the row's indent (see design.md).
pub fn row_lines(row: &DiffRow, width: usize) -> Vec<Line<'static>> {
    let (marker, marker_style) = gutter_marker(row);
    let indent = indent_depth(row) * INDENT_UNIT;
    let available = width.saturating_sub(GUTTER_WIDTH + indent).max(1);

    let wrapped = wrap_spans(content_spans(row), available);

    wrapped
        .into_iter()
        .enumerate()
        .map(|(i, spans)| {
            let mut line_spans = Vec::with_capacity(spans.len() + 2);
            line_spans.push(Span::raw(" ".repeat(indent)));
            if i == 0 {
                line_spans.push(Span::styled(marker, marker_style));
                line_spans.push(Span::raw(" "));
            } else {
                line_spans.push(Span::raw(" ".repeat(GUTTER_WIDTH)));
            }
            line_spans.extend(spans);
            Line::from(line_spans)
        })
        .collect()
}

fn indent_depth(row: &DiffRow) -> usize {
    match row {
        DiffRow::GroupHeading(_) | DiffRow::Requirement { .. } | DiffRow::Notice(_) => 0,
        DiffRow::Intro { .. } | DiffRow::Scenario { .. } => 1,
        DiffRow::Body { .. } => 2,
    }
}

fn gutter_marker(row: &DiffRow) -> (&'static str, Style) {
    match row {
        DiffRow::GroupHeading(_) | DiffRow::Body { .. } | DiffRow::Notice(_) => {
            (" ", Style::default())
        }
        DiffRow::Requirement { op, .. } => requirement_marker(op),
        DiffRow::Intro { piece } => piece_marker(piece),
        DiffRow::Scenario { body, .. } => piece_marker(body),
    }
}

/// The gutter marker for a requirement row, one per `Operation` variant.
fn requirement_marker(op: &Operation) -> (&'static str, Style) {
    let style = operation_style(op);
    match op {
        Operation::Added => ("+", style),
        Operation::Modified => ("~", style),
        Operation::Removed => ("-", style),
        Operation::Renamed { .. } => ("»", style),
    }
}

/// The color associated with an operation, shared by the requirement marker
/// and the group heading box so the two never drift apart. Renamed shares
/// Modified's color rather than getting its own: cyan read too close to the
/// pane focus highlight, and the marker glyph (`»` vs `~`) already
/// distinguishes the two operations on its own.
pub(crate) fn operation_style(op: &Operation) -> Style {
    match op {
        Operation::Added => added_style(),
        Operation::Modified | Operation::Renamed { .. } => modified_style(),
        Operation::Removed => removed_style(),
    }
}

/// The gutter marker for a piece row (intro or scenario), one per `Piece`
/// variant. `?` (unmentioned) is dimmed, deliberately not a shade of red:
/// it means the delta said nothing, not that content was removed.
fn piece_marker(piece: &Piece) -> (&'static str, Style) {
    match piece {
        Piece::Unchanged { .. } => (" ", Style::default()),
        Piece::Added { .. } => ("+", added_style()),
        Piece::Deleted { .. } => ("-", removed_style()),
        Piece::Changed { .. } => ("~", modified_style()),
        Piece::Unmentioned { .. } => ("?", Style::new().add_modifier(Modifier::DIM)),
    }
}

fn added_style() -> Style {
    Style::new().fg(Color::Green)
}

fn removed_style() -> Style {
    Style::new().fg(Color::Red)
}

fn modified_style() -> Style {
    Style::new().fg(Color::Yellow)
}

/// The style for a scenario body's `WHEN`/`THEN` bullet keyword, layered on
/// top of whatever style the keyword's characters already carry (e.g. a
/// word-diff run's color) rather than replacing it.
fn when_then_style() -> Style {
    Style::new().add_modifier(Modifier::BOLD)
}

fn content_spans(row: &DiffRow) -> Vec<Span<'static>> {
    match row {
        DiffRow::GroupHeading(op) => {
            vec![Span::styled(
                heading_text(op).to_string(),
                Style::new().add_modifier(Modifier::BOLD),
            )]
        }
        DiffRow::Requirement {
            name, op, expanded, ..
        } => {
            let mut spans = vec![
                Span::raw(expand_arrow(*expanded)),
                Span::styled("REQ", operation_style(op)),
                Span::raw(" "),
            ];
            match op {
                Operation::Renamed { from } => {
                    spans.push(Span::styled(
                        from.clone(),
                        Style::new().add_modifier(Modifier::DIM),
                    ));
                    spans.push(Span::styled(" → ", modified_style()));
                    spans.push(Span::raw((*name).to_string()));
                }
                _ => spans.push(Span::raw((*name).to_string())),
            }
            spans
        }
        DiffRow::Intro { piece } => {
            let (_, marker_style) = piece_marker(piece);
            let mut spans = vec![Span::styled("¶", marker_style), Span::raw(" ")];
            spans.extend(piece_spans(piece));
            if matches!(piece, Piece::Unmentioned { .. }) {
                dim(spans)
            } else {
                spans
            }
        }
        DiffRow::Scenario {
            name,
            body,
            expanded,
            ..
        } => {
            let (_, marker_style) = piece_marker(body);
            vec![
                Span::raw(expand_arrow(*expanded)),
                Span::styled("§", marker_style),
                Span::raw(" "),
                Span::raw((*name).to_string()),
            ]
        }
        DiffRow::Body { piece } => style_when_then(piece_spans(piece)),
        DiffRow::Notice(text) => vec![Span::styled(text.clone(), Style::new().fg(Color::Red))],
    }
}

fn expand_arrow(expanded: bool) -> &'static str {
    if expanded { "▾ " } else { "▸ " }
}

pub(crate) fn heading_text(op: &Operation) -> &'static str {
    match op {
        Operation::Added => "Added",
        Operation::Modified => "Modified",
        Operation::Removed => "Removed",
        Operation::Renamed { .. } => "Renamed",
    }
}

fn dim(spans: Vec<Span<'static>>) -> Vec<Span<'static>> {
    spans
        .into_iter()
        .map(|s| Span::styled(s.content, s.style.add_modifier(Modifier::DIM)))
        .collect()
}

fn piece_spans(piece: &Piece) -> Vec<Span<'static>> {
    match piece {
        Piece::Unchanged { text } => vec![Span::raw(text.clone())],
        Piece::Added { delta } => vec![Span::styled(delta.clone(), added_style())],
        Piece::Deleted { base } => vec![Span::styled(base.clone(), removed_style())],
        Piece::Unmentioned { base } => vec![Span::raw(base.clone())],
        Piece::Changed { base, delta, runs } => changed_spans(base, delta, runs),
    }
}

/// Builds one passage's spans by walking `runs` in order: `Equal` slices the
/// `delta` range (the two sides are equal by construction), `Delete` slices
/// `base` styled as a deletion, `Insert` slices `delta` styled as an
/// insertion. Slices go through `str::get`, so a run whose range does not
/// land on a char boundary in the supplied string yields an empty span
/// instead of panicking (see design.md).
fn changed_spans(base: &str, delta: &str, runs: &[Run]) -> Vec<Span<'static>> {
    runs.iter()
        .map(|run| match run {
            Run::Equal { delta: d, .. } => Span::raw(slice(delta, d)),
            Run::Delete { base: b } => Span::styled(slice(base, b), removed_style()),
            Run::Insert { delta: d } => Span::styled(slice(delta, d), added_style()),
        })
        .collect()
}

fn slice(s: &str, range: &Range<usize>) -> String {
    s.get(range.clone()).unwrap_or("").to_string()
}

/// Rewrites a scenario body's leading `- **WHEN**` / `- **THEN**` bullet
/// keyword from its markdown-bold source form into a de-asterisked, styled
/// keyword, matched at the start of each line on the row's flattened
/// character stream so a keyword split across spans by word-diff
/// highlighting is still caught. Every other character keeps its original
/// style; the keyword's characters layer `when_then_style()` on top of
/// theirs rather than replacing it (see design.md).
fn style_when_then(spans: Vec<Span<'static>>) -> Vec<Span<'static>> {
    let chars: Vec<(char, Style)> = spans
        .into_iter()
        .flat_map(|span| {
            let style = span.style;
            span.content
                .chars()
                .collect::<Vec<_>>()
                .into_iter()
                .map(move |c| (c, style))
        })
        .collect();

    let mut out: Vec<(char, Style)> = Vec::with_capacity(chars.len());
    let mut i = 0;
    let mut at_line_start = true;
    while i < chars.len() {
        if at_line_start && let Some(keyword) = bullet_keyword(&chars[i..]) {
            out.push(chars[i]); // "-"
            out.push(chars[i + 1]); // " "
            for (k, kc) in keyword.chars().enumerate() {
                let (_, style) = chars[i + 4 + k];
                out.push((kc, style.patch(when_then_style())));
            }
            i += 10; // "- **WHEN**" / "- **THEN**"
            at_line_start = false;
            continue;
        }
        let (c, style) = chars[i];
        out.push((c, style));
        at_line_start = c == '\n';
        i += 1;
    }

    chars_to_spans(out)
}

/// Returns `"WHEN"` or `"THEN"` if `chars` opens with the literal bullet
/// `- **WHEN**` / `- **THEN**`, matched on character content alone (the
/// styles of the individual characters don't matter for the match).
fn bullet_keyword(chars: &[(char, Style)]) -> Option<&'static str> {
    for keyword in ["WHEN", "THEN"] {
        let pattern: Vec<char> = format!("- **{keyword}**").chars().collect();
        if chars.len() >= pattern.len() && chars.iter().zip(&pattern).all(|(&(c, _), &p)| c == p) {
            return Some(keyword);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_operation_gets_its_expected_marker() {
        assert_eq!(requirement_marker(&Operation::Added).0, "+");
        assert_eq!(requirement_marker(&Operation::Modified).0, "~");
        assert_eq!(requirement_marker(&Operation::Removed).0, "-");
        assert_eq!(
            requirement_marker(&Operation::Renamed {
                from: "x".to_string()
            })
            .0,
            "»"
        );
        // All four markers are distinct.
        let markers = [
            requirement_marker(&Operation::Added).0,
            requirement_marker(&Operation::Modified).0,
            requirement_marker(&Operation::Removed).0,
            requirement_marker(&Operation::Renamed {
                from: "x".to_string(),
            })
            .0,
        ];
        let unique: std::collections::HashSet<_> = markers.iter().collect();
        assert_eq!(unique.len(), 4);
    }

    #[test]
    fn each_piece_variant_gets_its_expected_marker() {
        assert_eq!(
            piece_marker(&Piece::Unchanged {
                text: "x".to_string()
            })
            .0,
            " "
        );
        assert_eq!(
            piece_marker(&Piece::Added {
                delta: "x".to_string()
            })
            .0,
            "+"
        );
        assert_eq!(
            piece_marker(&Piece::Deleted {
                base: "x".to_string()
            })
            .0,
            "-"
        );
        assert_eq!(
            piece_marker(&Piece::Changed {
                base: "x".to_string(),
                delta: "y".to_string(),
                runs: vec![],
            })
            .0,
            "~"
        );
        let (marker, style) = piece_marker(&Piece::Unmentioned {
            base: "x".to_string(),
        });
        assert_eq!(marker, "?");
        assert!(style.add_modifier.contains(Modifier::DIM));
    }

    #[test]
    fn unmentioned_intro_is_marked_and_dimmed_while_unchanged_is_neither() {
        let unmentioned = DiffRow::Intro {
            piece: &Piece::Unmentioned {
                base: "same text".to_string(),
            },
        };
        let (marker, marker_style) = gutter_marker(&unmentioned);
        assert_eq!(marker, "?");
        assert!(marker_style.add_modifier.contains(Modifier::DIM));
        let spans = content_spans(&unmentioned);
        assert!(
            spans
                .iter()
                .all(|s| s.style.add_modifier.contains(Modifier::DIM))
        );

        let unchanged = DiffRow::Intro {
            piece: &Piece::Unchanged {
                text: "same text".to_string(),
            },
        };
        let (marker, marker_style) = gutter_marker(&unchanged);
        assert_eq!(marker, " ");
        assert!(!marker_style.add_modifier.contains(Modifier::DIM));
        let spans = content_spans(&unchanged);
        assert!(
            !spans
                .iter()
                .any(|s| s.style.add_modifier.contains(Modifier::DIM))
        );
    }

    #[test]
    fn changed_piece_renders_deleted_and_inserted_text_once_each() {
        let runs = vec![
            Run::Equal {
                base: 0..4,
                delta: 0..4,
            },
            Run::Delete { base: 4..9 },
            Run::Insert { delta: 4..8 },
            Run::Equal {
                base: 9..15,
                delta: 8..14,
            },
        ];
        let base = "the quick brown fox";
        let delta = "the slow brown fox";
        let spans = changed_spans(base, delta, &runs);

        let deleted_text: String = spans
            .iter()
            .filter(|s| s.style == removed_style())
            .map(|s| s.content.as_ref())
            .collect();
        let inserted_text: String = spans
            .iter()
            .filter(|s| s.style == added_style())
            .map(|s| s.content.as_ref())
            .collect();
        assert_eq!(deleted_text, "quick");
        assert_eq!(inserted_text, "slow");

        let deleted_count = spans.iter().filter(|s| s.style == removed_style()).count();
        let inserted_count = spans.iter().filter(|s| s.style == added_style()).count();
        assert_eq!(deleted_count, 1);
        assert_eq!(inserted_count, 1);
    }

    #[test]
    fn changed_span_with_bad_range_is_empty_not_panicking() {
        // A range that does not land on a char boundary of a multi-byte string.
        let base = "cafe\u{301}"; // "café" (combining acute), 6 bytes
        let runs = vec![Run::Delete { base: 0..5 }]; // 5 is not a char boundary
        let spans = changed_spans(base, "", &runs);
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].content.as_ref(), "");
    }

    #[test]
    fn continuation_lines_carry_no_marker_and_align_under_first_line() {
        let piece = Piece::Unchanged {
            text: "one two three four five six seven eight".to_string(),
        };
        let row = DiffRow::Body { piece: &piece };
        let lines = row_lines(&row, 12);
        assert!(
            lines.len() > 1,
            "expected the body to wrap onto several lines"
        );

        // Body indent is 2 levels * 2 columns = 4, plus a blank 2-column gutter.
        let expected_prefix = " ".repeat(4 + GUTTER_WIDTH);
        for line in &lines[1..] {
            let prefix: String = line
                .spans
                .iter()
                .flat_map(|s| s.content.chars())
                .take(expected_prefix.chars().count())
                .collect();
            assert_eq!(prefix, expected_prefix);
        }

        // First line carries the (blank, for Body) marker at the same indent.
        let first_prefix: String = lines[0]
            .spans
            .iter()
            .flat_map(|s| s.content.chars())
            .take(expected_prefix.chars().count())
            .collect();
        assert_eq!(first_prefix, expected_prefix);
    }

    fn dummy_key(requirement: &str) -> super::super::diff_row::RowKey {
        super::super::diff_row::RowKey {
            capability: "cap".to_string(),
            requirement: requirement.to_string(),
            scenario: None,
        }
    }

    #[test]
    fn requirement_row_marker_differs_by_operation() {
        let row = DiffRow::Requirement {
            name: "Some Requirement",
            op: &Operation::Added,
            expanded: false,
            key: dummy_key("Some Requirement"),
        };
        let lines = row_lines(&row, 40);
        let text: String = lines[0].spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(text.contains('+'));
        assert!(text.contains("Some Requirement"));
    }

    #[test]
    fn renamed_requirement_shows_both_names() {
        let row = DiffRow::Requirement {
            name: "New Name",
            op: &Operation::Renamed {
                from: "Old Name".to_string(),
            },
            expanded: false,
            key: dummy_key("New Name"),
        };
        let lines = row_lines(&row, 60);
        let text: String = lines[0].spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(text.contains("Old Name"));
        assert!(text.contains("New Name"));
    }

    #[test]
    fn renamed_requirement_dims_the_old_name_and_colors_the_arrow_like_modified() {
        let row = DiffRow::Requirement {
            name: "New Name",
            op: &Operation::Renamed {
                from: "Old Name".to_string(),
            },
            expanded: false,
            key: dummy_key("New Name"),
        };
        let spans = &row_lines(&row, 60)[0].spans;

        let old_name = spans
            .iter()
            .find(|s| s.content.as_ref() == "Old Name")
            .expect("expected a span exactly matching the old name");
        assert!(old_name.style.add_modifier.contains(Modifier::DIM));

        let arrow = spans
            .iter()
            .find(|s| s.content.as_ref() == " → ")
            .expect("expected a span exactly matching the arrow");
        assert_eq!(arrow.style, modified_style());

        let new_name = spans
            .iter()
            .find(|s| s.content.as_ref() == "New Name")
            .expect("expected a span exactly matching the new name");
        assert!(!new_name.style.add_modifier.contains(Modifier::DIM));
        assert_ne!(new_name.style, modified_style());
    }

    #[test]
    fn when_then_bullets_lose_their_asterisks_and_gain_bold() {
        let piece = Piece::Unchanged {
            text: "- **WHEN** a\n- **THEN** b".to_string(),
        };
        let row = DiffRow::Body { piece: &piece };
        let spans = content_spans(&row);

        let text: String = spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(text, "- WHEN a\n- THEN b");
        assert!(!text.contains('*'));

        let when = spans
            .iter()
            .find(|s| s.content.as_ref() == "WHEN")
            .expect("expected a span exactly matching WHEN");
        assert!(when.style.add_modifier.contains(Modifier::BOLD));

        let then = spans
            .iter()
            .find(|s| s.content.as_ref() == "THEN")
            .expect("expected a span exactly matching THEN");
        assert!(then.style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn when_then_styling_survives_word_level_diff_highlighting() {
        let runs = vec![
            Run::Equal {
                base: 0..11,
                delta: 0..11,
            },
            Run::Delete { base: 11..12 },
            Run::Insert { delta: 11..12 },
            Run::Equal {
                base: 12..25,
                delta: 12..25,
            },
        ];
        let base = "- **WHEN** a\n- **THEN** b";
        let delta = "- **WHEN** x\n- **THEN** b";
        let piece = Piece::Changed {
            base: base.to_string(),
            delta: delta.to_string(),
            runs,
        };
        let row = DiffRow::Body { piece: &piece };
        let spans = content_spans(&row);

        let text: String = spans.iter().map(|s| s.content.as_ref()).collect();
        // Both sides of the word-level diff are visible: the deleted "a"
        // and the inserted "x" that replaced it.
        assert_eq!(text, "- WHEN ax\n- THEN b");

        // The inserted "x" that replaced "a" keeps the diff's insertion
        // color, unaffected by the keyword rewrite elsewhere in the row.
        let inserted = spans
            .iter()
            .find(|s| s.content.as_ref() == "x")
            .expect("expected a span exactly matching the inserted character");
        assert_eq!(inserted.style, added_style());

        // The WHEN/THEN keywords are still de-asterisked and bold.
        let when = spans
            .iter()
            .find(|s| s.content.as_ref() == "WHEN")
            .expect("expected a span exactly matching WHEN");
        assert!(when.style.add_modifier.contains(Modifier::BOLD));
        let then = spans
            .iter()
            .find(|s| s.content.as_ref() == "THEN")
            .expect("expected a span exactly matching THEN");
        assert!(then.style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn bold_text_elsewhere_in_a_body_is_left_untouched() {
        let piece = Piece::Unchanged {
            text: "this passage has **bold** text, not a bullet".to_string(),
        };
        let row = DiffRow::Body { piece: &piece };
        let spans = content_spans(&row);

        let text: String = spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(text, "this passage has **bold** text, not a bullet");
        assert!(
            spans
                .iter()
                .all(|s| !s.style.add_modifier.contains(Modifier::BOLD))
        );
    }
}
