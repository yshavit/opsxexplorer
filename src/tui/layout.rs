use std::ops::Range;

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::diff::{Operation, Piece, Run};

use super::diff_row::DiffRow;
use super::wrap::wrap_spans;

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
    match op {
        Operation::Added => ("+", added_style()),
        Operation::Modified => ("~", modified_style()),
        Operation::Removed => ("-", removed_style()),
        Operation::Renamed { .. } => ("»", renamed_style()),
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

fn renamed_style() -> Style {
    Style::new().fg(Color::Cyan)
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
            let mut spans = vec![Span::raw(expand_arrow(*expanded))];
            match op {
                Operation::Renamed { from } => spans.push(Span::raw(format!("{from} → {name}"))),
                _ => spans.push(Span::raw((*name).to_string())),
            }
            spans
        }
        DiffRow::Intro { piece } => {
            let spans = piece_spans(piece);
            if matches!(piece, Piece::Unmentioned { .. }) {
                dim(spans)
            } else {
                spans
            }
        }
        DiffRow::Scenario { name, expanded, .. } => {
            vec![
                Span::raw(expand_arrow(*expanded)),
                Span::raw((*name).to_string()),
            ]
        }
        DiffRow::Body { piece } => piece_spans(piece),
        DiffRow::Notice(text) => vec![Span::styled(text.clone(), Style::new().fg(Color::Red))],
    }
}

fn expand_arrow(expanded: bool) -> &'static str {
    if expanded { "▾ " } else { "▸ " }
}

fn heading_text(op: &Operation) -> &'static str {
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
}
