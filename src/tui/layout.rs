use std::ops::Range;

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::diff::{Operation, Piece, Run};

use super::diff_row::{DiffRow, paragraph_text};
use super::wrap::{chars_to_spans, wrap_spans};

/// Width, in columns, of the gutter column: one marker character plus one
/// column of separating space.
const GUTTER_WIDTH: usize = 2;
/// Columns of indent per nesting level (requirement -> piece -> body).
const INDENT_UNIT: usize = 2;

/// The fixed prefix a *collapsed* paragraph row renders before its content:
/// disclosure triangle, pillcrow, and their separating spaces.
const PARAGRAPH_COLLAPSED_PREFIX: &str = "▸ ¶ ";

/// How many characters of text a paragraph row (purpose, or a requirement's
/// intro) has room for at pane width `width` and nesting depth `indent`,
/// before deciding whether it needs to collapse at all. Budgeted against the
/// collapsed row's own prefix width (not the plain `¶ ` prefix a fitting row
/// would actually render) plus the row's own indent, so
/// `diff_row::flatten`'s fits-check and this module's own collapsed-row
/// truncation always agree on the same number (see
/// `2026-08-09-render-purpose/design.md`). Purpose is always at indent 0 (a
/// sibling of `Requirement`); a requirement's intro is always at indent 1
/// (nested under it).
pub(crate) fn paragraph_available(width: usize, indent: usize) -> usize {
    width
        .saturating_sub(GUTTER_WIDTH)
        .saturating_sub(PARAGRAPH_COLLAPSED_PREFIX.chars().count())
        .saturating_sub(indent * INDENT_UNIT)
}

/// A literal character-slice-plus-ellipsis truncation, indifferent to word
/// boundaries — deliberately dumber than `wrap_spans`, for the one row in
/// this pane whose collapsed form is an exact-width excerpt rather than a
/// wrapped paragraph (see `2026-08-09-render-purpose/design.md`).
pub(crate) fn truncate_chars(text: &str, width: usize) -> String {
    let mut truncated: String = text.chars().take(width.saturating_sub(1)).collect();
    truncated.push('…');
    truncated
}

/// Builds a row's spans, wraps them to the space left after the gutter and
/// indent, and lays out the result as full lines: the marker prefixes only
/// the first line, continuation lines get a blank gutter, and every line is
/// left-padded by the row's indent (see
/// `2026-08-08-tui-specdiff/design.md`). A collapsed `Paragraph`
/// row is the one exception: its content is an exact-width excerpt or
/// placeholder, not a wrapped paragraph, so it bypasses `wrap_spans`
/// entirely (see `collapsed_paragraph_lines`).
pub fn row_lines(row: &DiffRow, width: usize) -> Vec<Line<'static>> {
    if let DiffRow::Paragraph {
        piece,
        expanded: false,
        indent,
        ..
    } = row
    {
        return collapsed_paragraph_lines(piece, width, *indent);
    }

    if let DiffRow::RemovalNoteLine {
        text,
        expanded: false,
        indent,
        ..
    } = row
    {
        return collapsed_removal_note_lines(text, width, *indent);
    }

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

/// The placeholder a collapsed, wholesale-replacement paragraph row shows in
/// place of any excerpt of either text — a short slice of just the new text
/// would misrepresent a rewrite as an ordinary edit (see
/// `2026-08-09-render-purpose/design.md`).
const REPLACED_PARAGRAPH_PLACEHOLDER: &str = "Expand to view diff";

/// Renders a collapsed paragraph row (purpose, or a requirement's intro at
/// `indent`) as a single, unwrapped line: `indent * INDENT_UNIT` columns of
/// blank padding, then the fixed `▸ ¶ ` prefix, followed by either a literal
/// character-slice excerpt of the piece's current text (every variant except
/// `Replaced`, via `paragraph_text`) or the italic placeholder (`Replaced`),
/// each truncated with `truncate_chars` only when it doesn't already fit
/// (see `2026-08-09-render-purpose/design.md`). The excerpt is
/// de-emphasised when `piece` is `Unmentioned`, matching the expanded path's
/// own dimming.
fn collapsed_paragraph_lines(piece: &Piece, width: usize, indent: usize) -> Vec<Line<'static>> {
    let (marker, marker_style) = piece_marker(piece);
    let budget = paragraph_available(width, indent);

    let content = match paragraph_text(piece) {
        Some(text) => {
            let trimmed = text.trim_end();
            let style = if matches!(piece, Piece::Unmentioned { .. }) {
                marker_style.add_modifier(Modifier::DIM)
            } else {
                marker_style
            };
            Span::styled(truncate_chars(trimmed, budget), style)
        }
        None => {
            let style = modified_style().add_modifier(Modifier::ITALIC);
            let text = if REPLACED_PARAGRAPH_PLACEHOLDER.chars().count() <= budget {
                REPLACED_PARAGRAPH_PLACEHOLDER.to_string()
            } else {
                truncate_chars(REPLACED_PARAGRAPH_PLACEHOLDER, budget)
            };
            Span::styled(text, style)
        }
    };

    // Disclosure triangle unstyled (matching `Requirement`/`Scenario`'s own
    // `expand_arrow` convention), pillcrow carrying the piece's marker color
    // (matching the expanded path's "¶" convention).
    vec![Line::from(vec![
        Span::raw(" ".repeat(indent * INDENT_UNIT)),
        Span::styled(marker, marker_style),
        Span::raw(" "),
        Span::raw(expand_arrow(false)),
        Span::styled("¶", marker_style),
        Span::raw(" "),
        content,
    ])]
}

/// Renders a collapsed removal-note line row: same shape as
/// `collapsed_paragraph_lines`, but the source text is always a plain line
/// (never a wholesale-replacement placeholder), so this is simpler — a
/// character-exact truncated excerpt of the line, taken after stripping any
/// leading `**Reason**`/`**Migration**` down to the bare keyword (see
/// `strip_reason_migration_prefix`), so the excerpt never shows literal `**`
/// characters even though it's a truncation, not the full styled line.
fn collapsed_removal_note_lines(text: &str, width: usize, indent: usize) -> Vec<Line<'static>> {
    let (marker, marker_style) = removal_note_marker();
    let budget = paragraph_available(width, indent);
    let trimmed = text.trim_end();
    let (deasterisked, keyword) = strip_reason_migration_prefix(trimmed);
    let excerpt = truncate_chars(&deasterisked, budget);

    let content = match keyword {
        Some(keyword) if excerpt.starts_with(keyword) => {
            let rest = excerpt[keyword.len()..].to_string();
            vec![
                Span::styled(keyword.to_string(), marker_style.patch(when_then_style())),
                Span::styled(rest, marker_style),
            ]
        }
        _ => vec![Span::styled(excerpt, marker_style)],
    };

    let mut spans = vec![
        Span::raw(" ".repeat(indent * INDENT_UNIT)),
        Span::styled(marker, marker_style),
        Span::raw(" "),
        Span::raw(expand_arrow(false)),
        Span::styled("¶", marker_style),
        Span::raw(" "),
    ];
    spans.extend(content);
    vec![Line::from(spans)]
}

fn indent_depth(row: &DiffRow) -> usize {
    match row {
        DiffRow::ParagraphFull { indent, .. }
        | DiffRow::Paragraph { indent, .. }
        | DiffRow::RemovalNoteFull { indent, .. }
        | DiffRow::RemovalNoteLine { indent, .. } => *indent,
        DiffRow::GroupHeading(_)
        | DiffRow::PurposeHeading(_)
        | DiffRow::Requirement { .. }
        | DiffRow::Notice(_)
        | DiffRow::UnrecognizedSectionsHeading(_)
        | DiffRow::UnrecognizedSection { .. } => 0,
        DiffRow::Scenario { .. } | DiffRow::UnrecognizedSectionBody { .. } => 1,
        DiffRow::Body { .. } => 2,
    }
}

fn gutter_marker(row: &DiffRow) -> (&'static str, Style) {
    match row {
        DiffRow::GroupHeading(_)
        | DiffRow::PurposeHeading(_)
        | DiffRow::Body { .. }
        | DiffRow::Notice(_)
        | DiffRow::UnrecognizedSectionsHeading(_)
        | DiffRow::UnrecognizedSection { .. }
        | DiffRow::UnrecognizedSectionBody { .. } => (" ", Style::default()),
        DiffRow::Requirement { op, .. } => requirement_marker(op),
        DiffRow::Scenario { body, .. } => piece_marker(body),
        DiffRow::ParagraphFull { piece, .. } => piece_marker(piece),
        DiffRow::Paragraph { piece, .. } => piece_marker(piece),
        DiffRow::RemovalNoteFull { .. } | DiffRow::RemovalNoteLine { .. } => removal_note_marker(),
    }
}

/// The gutter marker for a removed requirement's removal-note line: the same
/// marker glyph and colour as a modified operation and a changed piece —
/// distinct from both the added and the deletion styling used elsewhere in a
/// removed requirement's rows (see `2026-08-10-spec-model-removals/design.md`).
fn removal_note_marker() -> (&'static str, Style) {
    ("~", modified_style())
}

/// The gutter marker for a requirement row, one per `Operation` variant.
fn requirement_marker(op: &Operation) -> (&'static str, Style) {
    let style = operation_style(op);
    match op {
        Operation::Added => ("+", style),
        Operation::Modified => ("~", style),
        Operation::Removed { .. } => ("-", style),
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
        Operation::Removed { .. } => removed_marker_style(),
    }
}

/// The gutter marker for a piece row (intro or scenario), one per `Piece`
/// variant. `?` (unmentioned) is dimmed, deliberately not a shade of red:
/// it means the delta said nothing, not that content was removed.
fn piece_marker(piece: &Piece) -> (&'static str, Style) {
    match piece {
        Piece::Unchanged { .. } => (" ", Style::default()),
        Piece::Added { .. } => ("+", added_style()),
        Piece::Deleted { .. } => ("-", removed_marker_style()),
        Piece::Changed { .. } | Piece::Replaced { .. } => ("~", modified_style()),
        Piece::Unmentioned { .. } => ("?", Style::new().add_modifier(Modifier::DIM)),
    }
}

pub(crate) fn added_style() -> Style {
    Style::new().fg(Color::Green)
}

/// The style for removed text content (a deletion run, a deleted piece, or
/// the base side of a replacement): red.
fn removed_text_style() -> Style {
    removed_marker_style()
}

/// The style for removed markers and labels — gutter markers, the `REQ`
/// label.
fn removed_marker_style() -> Style {
    Style::new().fg(Color::Red)
}

pub(crate) fn modified_style() -> Style {
    Style::new().fg(Color::Yellow)
}

/// The style for the "Unknown sections" heading and its content, a
/// hand-picked purple distinct from every other color used in the pane (see
/// `2026-08-10-unrecognized-spec-sections/design.md`).
pub(crate) fn unrecognised_style() -> Style {
    Style::new().fg(Color::Rgb(147, 51, 234))
}

/// The style for a scenario body's `WHEN`/`THEN`/`AND` bullet keyword,
/// layered on top of whatever style the keyword's characters already carry
/// (e.g. a word-diff run's color) rather than replacing it.
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
                    spans.push(Span::styled((*name).to_string(), operation_style(op)));
                }
                _ => spans.push(Span::styled((*name).to_string(), operation_style(op))),
            }
            spans
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
                Span::styled((*name).to_string(), marker_style),
            ]
        }
        DiffRow::Body { piece } => style_when_then(piece_spans(piece)),
        DiffRow::Notice(text) => vec![Span::styled(text.clone(), Style::new().fg(Color::Red))],
        // Display-only; always rendered through `purpose_heading_box`
        // instead (see `build_diff_lines`), never through this path.
        DiffRow::PurposeHeading(_) => vec![],
        // Display-only; always rendered through
        // `unrecognized_sections_heading` instead (see `build_diff_lines`),
        // never through this path.
        DiffRow::UnrecognizedSectionsHeading(_) => vec![],
        // The title carries no colour of its own: the heading above it
        // already says which origin the section came from, and the section's
        // own text is content the tool didn't interpret (see
        // `2026-08-10-unrecognized-sections-in-base/design.md`).
        DiffRow::UnrecognizedSection {
            title, expanded, ..
        } => vec![
            Span::raw(expand_arrow(*expanded)),
            Span::raw((*title).to_string()),
        ],
        DiffRow::UnrecognizedSectionBody { text } => vec![Span::raw((*text).to_string())],
        DiffRow::ParagraphFull { piece, .. } => {
            let (_, marker_style) = piece_marker(piece);
            let mut spans = vec![Span::styled("¶", marker_style), Span::raw(" ")];
            spans.extend(piece_spans(piece));
            if matches!(piece, Piece::Unmentioned { .. }) {
                dim(spans)
            } else {
                spans
            }
        }
        // `expanded: false` is intercepted earlier in `row_lines` via
        // `collapsed_paragraph_lines`; only the expanded case reaches here.
        DiffRow::Paragraph {
            piece, expanded, ..
        } => {
            let (_, marker_style) = piece_marker(piece);
            let mut spans = vec![
                Span::raw(expand_arrow(*expanded)),
                Span::styled("¶", marker_style),
                Span::raw(" "),
            ];
            spans.extend(piece_spans(piece));
            if matches!(piece, Piece::Unmentioned { .. }) {
                dim(spans)
            } else {
                spans
            }
        }
        DiffRow::RemovalNoteFull { text, .. } => {
            let (_, marker_style) = removal_note_marker();
            let mut spans = vec![Span::styled("¶", marker_style), Span::raw(" ")];
            spans.extend(removal_note_line_spans(text));
            spans
        }
        // `expanded: false` is intercepted earlier in `row_lines` via
        // `collapsed_removal_note_lines`; only the expanded case reaches here.
        DiffRow::RemovalNoteLine { text, expanded, .. } => {
            let (_, marker_style) = removal_note_marker();
            let mut spans = vec![
                Span::raw(expand_arrow(*expanded)),
                Span::styled("¶", marker_style),
                Span::raw(" "),
            ];
            spans.extend(removal_note_line_spans(text));
            spans
        }
    }
}

/// Builds one removal-note line's spans: a leading `**Reason**` or
/// `**Migration**` keyword is de-asterisked and styled the same way a
/// scenario body's `WHEN`/`THEN`/`AND` bullet keyword is (see
/// `style_when_then`); any other line renders as-is. The whole line carries
/// modification styling — the removal note has no base counterpart to diff
/// against, so it is never insertion- or deletion-styled.
fn removal_note_line_spans(text: &str) -> Vec<Span<'static>> {
    let (_, marker_style) = removal_note_marker();
    let (deasterisked, keyword) = strip_reason_migration_prefix(text);
    match keyword {
        Some(keyword) => {
            let rest = deasterisked[keyword.len()..].to_string();
            vec![
                Span::styled(keyword.to_string(), marker_style.patch(when_then_style())),
                Span::styled(rest, marker_style),
            ]
        }
        None => vec![Span::styled(deasterisked, marker_style)],
    }
}

/// Returns `"Reason"` or `"Migration"` if `text` opens with the literal
/// markdown-bold form `**Reason**` or `**Migration**`.
fn reason_migration_keyword(text: &str) -> Option<&'static str> {
    for keyword in ["Reason", "Migration"] {
        if text.starts_with(&format!("**{keyword}**")) {
            return Some(keyword);
        }
    }
    None
}

/// Strips a removal-note line's leading `**Reason**`/`**Migration**`
/// markdown-bold wrapper down to the bare keyword, so both the fully
/// rendered line (`removal_note_line_spans`) and its collapsed excerpt
/// (`collapsed_removal_note_lines`) show the same de-asterisked text —
/// keyword stripping applies in both states, unlike a collapsed intro row's
/// excerpt, which has no keyword convention of its own to strip. Returns the
/// de-asterisked text and, if a keyword was found, the keyword itself so a
/// caller can style just that prefix.
fn strip_reason_migration_prefix(text: &str) -> (String, Option<&'static str>) {
    match reason_migration_keyword(text) {
        Some(keyword) => {
            let prefix_len = keyword.len() + 4; // "**" + keyword + "**"
            (format!("{keyword}{}", &text[prefix_len..]), Some(keyword))
        }
        None => (text.to_string(), None),
    }
}

fn expand_arrow(expanded: bool) -> &'static str {
    if expanded { "▾ " } else { "▸ " }
}

pub(crate) fn heading_text(op: &Operation) -> &'static str {
    match op {
        Operation::Added => "Added",
        Operation::Modified => "Modified",
        Operation::Removed { .. } => "Removed",
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
        Piece::Deleted { base } => vec![Span::styled(base.clone(), removed_text_style())],
        Piece::Unmentioned { base } => vec![Span::raw(base.clone())],
        Piece::Changed { base, delta, runs } => changed_spans(base, delta, runs),
        Piece::Replaced { base, delta } => vec![
            Span::styled(base.clone(), removed_text_style()),
            Span::raw("\n"),
            Span::styled(delta.clone(), added_style()),
        ],
    }
}

/// Builds one passage's spans by walking `runs` in order: `Equal` slices the
/// `delta` range (the two sides are equal by construction), `Delete` slices
/// `base` styled as a deletion, `Insert` slices `delta` styled as an
/// insertion. Slices go through `str::get`, so a run whose range does not
/// land on a char boundary in the supplied string yields an empty span
/// instead of panicking (see `2026-08-08-tui-specdiff/design.md`).
fn changed_spans(base: &str, delta: &str, runs: &[Run]) -> Vec<Span<'static>> {
    runs.iter()
        .map(|run| match run {
            Run::Equal { delta: d, .. } => Span::raw(slice(delta, d)),
            Run::Delete { base: b } => Span::styled(slice(base, b), removed_text_style()),
            Run::Insert { delta: d } => Span::styled(slice(delta, d), added_style()),
        })
        .collect()
}

fn slice(s: &str, range: &Range<usize>) -> String {
    s.get(range.clone()).unwrap_or("").to_string()
}

/// Rewrites a scenario body's leading `- **WHEN**` / `- **THEN**` /
/// `- **AND**` bullet keyword from its markdown-bold source form into a
/// de-asterisked, styled keyword, matched at the start of each line on the
/// row's flattened character stream so a keyword split across spans by
/// word-diff highlighting is still caught. Every other character keeps its
/// original style; the keyword's characters layer `when_then_style()` on
/// top of theirs rather than replacing it (see
/// `2026-08-08-when-then-formatting/design.md`).
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
            i += 4 + keyword.len() + 2; // "- **" + keyword + "**"
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

/// Returns `"WHEN"`, `"THEN"`, or `"AND"` if `chars` opens with the literal
/// bullet `- **WHEN**` / `- **THEN**` / `- **AND**`, matched on character
/// content alone (the styles of the individual characters don't matter for
/// the match).
fn bullet_keyword(chars: &[(char, Style)]) -> Option<&'static str> {
    for keyword in ["WHEN", "THEN", "AND"] {
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
        assert_eq!(
            requirement_marker(&Operation::Removed {
                note: String::new()
            })
            .0,
            "-"
        );
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
            requirement_marker(&Operation::Removed {
                note: String::new(),
            })
            .0,
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
        // A replacement is a modification, not a removal: it carries the
        // same marker and style as `Changed`, not `Deleted`'s.
        assert_eq!(
            piece_marker(&Piece::Replaced {
                base: "x".to_string(),
                delta: "y".to_string(),
            }),
            piece_marker(&Piece::Changed {
                base: "x".to_string(),
                delta: "y".to_string(),
                runs: vec![],
            })
        );
        let (marker, style) = piece_marker(&Piece::Unmentioned {
            base: "x".to_string(),
        });
        assert_eq!(marker, "?");
        assert!(style.add_modifier.contains(Modifier::DIM));
    }

    #[test]
    fn unmentioned_intro_is_marked_and_dimmed_while_unchanged_is_neither() {
        let unmentioned = DiffRow::ParagraphFull {
            piece: &Piece::Unmentioned {
                base: "same text".to_string(),
            },
            indent: 1,
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

        let unchanged = DiffRow::ParagraphFull {
            piece: &Piece::Unchanged {
                text: "same text".to_string(),
            },
            indent: 1,
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
            .filter(|s| s.style == removed_text_style())
            .map(|s| s.content.as_ref())
            .collect();
        let inserted_text: String = spans
            .iter()
            .filter(|s| s.style == added_style())
            .map(|s| s.content.as_ref())
            .collect();
        assert_eq!(deleted_text, "quick");
        assert_eq!(inserted_text, "slow");

        let deleted_count = spans
            .iter()
            .filter(|s| s.style == removed_text_style())
            .count();
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
    fn replaced_piece_renders_both_texts_in_full_on_separate_lines_with_deletion_and_insertion_styling()
     {
        let piece = Piece::Replaced {
            base: "the old text in full".to_string(),
            delta: "the new text in full".to_string(),
        };
        let spans = piece_spans(&piece);

        let text: String = spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(text, "the old text in full\nthe new text in full");

        // The delta text begins on a line of its own: a raw "\n" separates
        // the two, matching wrap.rs's forced-break token.
        let newline_index = spans
            .iter()
            .position(|s| s.content.as_ref() == "\n")
            .expect("expected a raw newline span between the two texts");

        let before: String = spans[..newline_index]
            .iter()
            .map(|s| s.content.as_ref())
            .collect();
        let after: String = spans[newline_index + 1..]
            .iter()
            .map(|s| s.content.as_ref())
            .collect();
        assert_eq!(before, "the old text in full");
        assert_eq!(after, "the new text in full");

        assert!(
            spans[..newline_index]
                .iter()
                .all(|s| s.style == removed_text_style())
        );
        assert!(
            spans[newline_index + 1..]
                .iter()
                .all(|s| s.style == added_style())
        );
    }

    #[test]
    fn replaced_piece_wraps_and_keeps_styling_across_the_break() {
        let piece = Piece::Replaced {
            base: "one two three four five six seven".to_string(),
            delta: "eight nine ten eleven twelve thirteen".to_string(),
        };
        let row = DiffRow::Body { piece: &piece };
        let width = 10;
        let lines = row_lines(&row, width);

        assert!(
            lines.len() > 2,
            "expected both texts to wrap onto several lines"
        );

        // No rendered line exceeds the pane width — no horizontal scrolling
        // is required to read a replacement.
        for line in &lines {
            let line_len: usize = line.spans.iter().flat_map(|s| s.content.chars()).count();
            assert!(
                line_len <= width,
                "line exceeds pane width {width}: {line:?}"
            );
        }

        // Every word carries the style of the text it came from, wrap or no
        // wrap. `base` and `delta` share no words, so this also confirms the
        // wrap didn't garble which word came from which side.
        for line in &lines {
            for span in &line.spans {
                let word = span.content.trim();
                if word.is_empty() {
                    continue;
                }
                if piece_base_words().contains(&word) {
                    assert_eq!(
                        span.style,
                        removed_text_style(),
                        "base word {word:?} mis-styled"
                    );
                } else if piece_delta_words().contains(&word) {
                    assert_eq!(span.style, added_style(), "delta word {word:?} mis-styled");
                }
            }
        }

        // The two texts don't run together: no line mixes a base word with a
        // delta word, and every base-word line precedes every delta-word line.
        let last_base_line = lines.iter().rposition(|l| {
            l.spans
                .iter()
                .any(|s| piece_base_words().contains(&s.content.trim()))
        });
        let first_delta_line = lines.iter().position(|l| {
            l.spans
                .iter()
                .any(|s| piece_delta_words().contains(&s.content.trim()))
        });
        let (last_base_line, first_delta_line) = (
            last_base_line.expect("expected at least one base word"),
            first_delta_line.expect("expected at least one delta word"),
        );
        assert!(
            last_base_line < first_delta_line,
            "expected base text to fully precede delta text: last_base_line={last_base_line} first_delta_line={first_delta_line}"
        );
    }

    fn piece_base_words() -> [&'static str; 7] {
        ["one", "two", "three", "four", "five", "six", "seven"]
    }

    fn piece_delta_words() -> [&'static str; 6] {
        ["eight", "nine", "ten", "eleven", "twelve", "thirteen"]
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
        super::super::diff_row::RowKey::Requirement {
            capability: "cap".to_string(),
            requirement: requirement.to_string(),
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
    fn renamed_requirement_dims_the_old_name_and_colors_the_arrow_and_new_name_like_modified() {
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

        // The arrow and the new name now share `modified_style()`, so
        // `chars_to_spans` coalesces them into one span rather than keeping
        // them separate.
        let arrow_and_new_name = spans
            .iter()
            .find(|s| s.content.as_ref() == " → New Name")
            .expect("expected the arrow and new name merged into one span");
        assert!(
            !arrow_and_new_name
                .style
                .add_modifier
                .contains(Modifier::DIM)
        );
        assert_eq!(arrow_and_new_name.style, modified_style());
    }

    #[test]
    fn when_then_bullets_lose_their_asterisks_and_gain_bold() {
        let piece = Piece::Unchanged {
            text: "- **WHEN** a\n- **THEN** b\n- **AND** c".to_string(),
        };
        let row = DiffRow::Body { piece: &piece };
        let spans = content_spans(&row);

        let text: String = spans.iter().map(|s| s.content.as_ref()).collect();
        // The "AND" bullet is one character shorter than "WHEN"/"THEN" (3
        // letters vs. 4); if the rewrite still advanced by the length that
        // fits a 4-letter keyword, the trailing " c" would be corrupted.
        assert_eq!(text, "- WHEN a\n- THEN b\n- AND c");
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

        let and = spans
            .iter()
            .find(|s| s.content.as_ref() == "AND")
            .expect("expected a span exactly matching AND");
        assert!(and.style.add_modifier.contains(Modifier::BOLD));
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
                base: 12..37,
                delta: 12..37,
            },
        ];
        let base = "- **WHEN** a\n- **THEN** b\n- **AND** c";
        let delta = "- **WHEN** x\n- **THEN** b\n- **AND** c";
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
        assert_eq!(text, "- WHEN ax\n- THEN b\n- AND c");

        // The inserted "x" that replaced "a" keeps the diff's insertion
        // color, unaffected by the keyword rewrite elsewhere in the row.
        let inserted = spans
            .iter()
            .find(|s| s.content.as_ref() == "x")
            .expect("expected a span exactly matching the inserted character");
        assert_eq!(inserted.style, added_style());

        // The WHEN/THEN/AND keywords are still de-asterisked and bold.
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
        let and = spans
            .iter()
            .find(|s| s.content.as_ref() == "AND")
            .expect("expected a span exactly matching AND");
        assert!(and.style.add_modifier.contains(Modifier::BOLD));
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

    // --- paragraph row rendering (render-purpose 5.6, unified-intro-collapsing) ---

    fn purpose_key() -> super::super::diff_row::RowKey {
        super::super::diff_row::RowKey::Purpose {
            capability: "cap".to_string(),
        }
    }

    fn intro_key() -> super::super::diff_row::RowKey {
        super::super::diff_row::RowKey::Intro {
            capability: "cap".to_string(),
            requirement: "Req".to_string(),
        }
    }

    #[test]
    fn truncate_chars_takes_width_minus_one_chars_and_appends_ellipsis() {
        assert_eq!(truncate_chars("abcdefgh", 5), "abcd…");
        assert_eq!(truncate_chars("abc", 1), "…");
    }

    #[test]
    fn purpose_full_renders_full_text_with_no_ellipsis_and_no_arrow() {
        let piece = Piece::Added {
            delta: "short purpose text".to_string(),
        };
        let row = DiffRow::ParagraphFull {
            piece: &piece,
            indent: 0,
        };
        let lines = row_lines(&row, 80);
        let text: String = lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .map(|s| s.content.as_ref())
            .collect();
        assert!(text.contains("short purpose text"));
        assert!(!text.contains('…'));
        assert!(!text.contains('▸'));
        assert!(!text.contains('▾'));
    }

    #[test]
    fn collapsed_added_purpose_is_one_line_ending_in_ellipsis_sized_to_available_width() {
        let long_text = "abcdefghijklmnopqrstuvwxyz".repeat(3);
        let piece = Piece::Added {
            delta: long_text.clone(),
        };
        let width = 30;
        let row = DiffRow::Paragraph {
            piece: &piece,
            expanded: false,
            key: purpose_key(),
            indent: 0,
        };
        let lines = row_lines(&row, width);
        assert_eq!(lines.len(), 1);
        let excerpt = lines[0].spans.last().unwrap().content.as_ref();
        assert!(excerpt.ends_with('…'));
        let budget = paragraph_available(width, 0);
        assert_eq!(excerpt, truncate_chars(&long_text, budget));
    }

    #[test]
    fn collapsed_purpose_truncation_ignores_word_boundaries() {
        let text = "one two three four five six seven eight nine ten".to_string();
        let piece = Piece::Changed {
            base: "old".to_string(),
            delta: text.clone(),
            runs: vec![],
        };
        // Chosen so the budget's cut point lands inside "three", not on a
        // preceding space.
        let width = 17;
        let row = DiffRow::Paragraph {
            piece: &piece,
            expanded: false,
            key: purpose_key(),
            indent: 0,
        };
        let lines = row_lines(&row, width);
        let excerpt = lines[0].spans.last().unwrap().content.as_ref();
        assert_eq!(excerpt, "one two th…");
        let budget = paragraph_available(width, 0);
        assert_eq!(excerpt, truncate_chars(&text, budget));
    }

    #[test]
    fn collapsed_replaced_purpose_shows_italicized_placeholder_not_an_excerpt() {
        let piece = Piece::Replaced {
            base: "old text in full".to_string(),
            delta: "new text in full".to_string(),
        };
        let row = DiffRow::Paragraph {
            piece: &piece,
            expanded: false,
            key: purpose_key(),
            indent: 0,
        };
        let lines = row_lines(&row, 40);
        let last = lines[0].spans.last().unwrap();
        assert_eq!(last.content.as_ref(), "Expand to view diff");
        assert!(last.style.add_modifier.contains(Modifier::ITALIC));
        assert!(!last.content.contains("old text"));
        assert!(!last.content.contains("new text"));
    }

    #[test]
    fn collapsed_replaced_purpose_placeholder_truncates_only_at_extreme_widths() {
        let piece = Piece::Replaced {
            base: "old".to_string(),
            delta: "new".to_string(),
        };
        let row = DiffRow::Paragraph {
            piece: &piece,
            expanded: false,
            key: purpose_key(),
            indent: 0,
        };

        let wide_lines = row_lines(&row, 40);
        let wide_content = wide_lines[0].spans.last().unwrap().content.as_ref();
        assert_eq!(wide_content, "Expand to view diff");
        assert!(!wide_content.contains('…'));

        let narrow_lines = row_lines(&row, 6);
        let narrow_content = narrow_lines[0].spans.last().unwrap().content.as_ref();
        assert!(narrow_content.ends_with('…'));
        assert!(narrow_content.chars().count() < "Expand to view diff".chars().count());
    }

    #[test]
    fn expanded_changed_purpose_uses_the_same_interleaved_run_styling_as_an_intro_would() {
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
        let piece = Piece::Changed {
            base: base.to_string(),
            delta: delta.to_string(),
            runs,
        };
        let row = DiffRow::Paragraph {
            piece: &piece,
            expanded: true,
            key: purpose_key(),
            indent: 0,
        };
        let spans = content_spans(&row);

        let deleted_text: String = spans
            .iter()
            .filter(|s| s.style == removed_text_style())
            .map(|s| s.content.as_ref())
            .collect();
        let inserted_text: String = spans
            .iter()
            .filter(|s| s.style == added_style())
            .map(|s| s.content.as_ref())
            .collect();
        assert_eq!(deleted_text, "quick");
        assert_eq!(inserted_text, "slow");
    }

    #[test]
    fn expanded_replaced_purpose_stacks_both_texts_like_any_other_replacement() {
        let piece = Piece::Replaced {
            base: "the old text in full".to_string(),
            delta: "the new text in full".to_string(),
        };
        let row = DiffRow::Paragraph {
            piece: &piece,
            expanded: true,
            key: purpose_key(),
            indent: 0,
        };
        let spans = content_spans(&row);
        let text: String = spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(text.contains("the old text in full"));
        assert!(text.contains("the new text in full"));
        assert!(text.contains('\n'));
    }

    #[test]
    fn resizing_changes_the_collapsed_truncation_point_for_added_purpose() {
        let text = "abcdefghijklmnopqrstuvwxyz".repeat(2);
        let piece = Piece::Added {
            delta: text.clone(),
        };
        let row = DiffRow::Paragraph {
            piece: &piece,
            expanded: false,
            key: purpose_key(),
            indent: 0,
        };

        let narrow = row_lines(&row, 20);
        let wide = row_lines(&row, 40);
        let narrow_excerpt = narrow[0].spans.last().unwrap().content.as_ref();
        let wide_excerpt = wide[0].spans.last().unwrap().content.as_ref();
        assert_ne!(narrow_excerpt, wide_excerpt);
        assert!(wide_excerpt.chars().count() > narrow_excerpt.chars().count());
    }

    // --- intro row rendering (unified-intro-collapsing) ---

    #[test]
    fn paragraph_available_subtracts_indent_at_a_couple_of_widths() {
        assert_eq!(
            paragraph_available(40, 1),
            paragraph_available(40, 0) - INDENT_UNIT
        );
        assert_eq!(
            paragraph_available(80, 1),
            paragraph_available(80, 0) - INDENT_UNIT
        );
    }

    #[test]
    fn collapsed_paragraph_lines_at_indent_1_renders_leading_padding() {
        let long_text = "abcdefghijklmnopqrstuvwxyz".repeat(3);
        let piece = Piece::Unchanged { text: long_text };
        let width = 30;
        let row = DiffRow::Paragraph {
            piece: &piece,
            expanded: false,
            key: intro_key(),
            indent: 1,
        };
        let lines = row_lines(&row, width);
        assert_eq!(lines.len(), 1);
        let prefix: String = lines[0]
            .spans
            .first()
            .expect("expected a leading padding span")
            .content
            .as_ref()
            .to_string();
        assert_eq!(prefix, " ".repeat(INDENT_UNIT));
    }

    #[test]
    fn collapsed_unmentioned_intro_excerpt_is_de_emphasised() {
        let long_text = "a very long unmentioned intro ".repeat(10);
        let piece = Piece::Unmentioned { base: long_text };
        let row = DiffRow::Paragraph {
            piece: &piece,
            expanded: false,
            key: intro_key(),
            indent: 1,
        };
        let lines = row_lines(&row, 30);
        let excerpt = lines[0].spans.last().unwrap();
        assert!(excerpt.content.ends_with('…'));
        assert!(excerpt.style.add_modifier.contains(Modifier::DIM));
    }

    // --- removal-note line rendering (spec-model-removals) ---

    fn removal_note_key() -> super::super::diff_row::RowKey {
        super::super::diff_row::RowKey::RemovalNoteLine {
            capability: "cap".to_string(),
            requirement: "Req".to_string(),
            line: 0,
        }
    }

    #[test]
    fn a_reason_line_is_deasterisked_and_styled_bold() {
        let row = DiffRow::RemovalNoteFull {
            text: "**Reason**: no longer needed.",
            indent: 1,
        };
        let spans = content_spans(&row);

        let text: String = spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(text, "¶ Reason: no longer needed.");
        assert!(!text.contains('*'));

        let keyword = spans
            .iter()
            .find(|s| s.content.as_ref() == "Reason")
            .expect("expected a span exactly matching Reason");
        assert!(keyword.style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn a_migration_line_is_deasterisked_and_styled_bold() {
        let row = DiffRow::RemovalNoteFull {
            text: "**Migration**: use the new thing instead.",
            indent: 1,
        };
        let spans = content_spans(&row);

        let text: String = spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(text, "¶ Migration: use the new thing instead.");
        assert!(!text.contains('*'));

        let keyword = spans
            .iter()
            .find(|s| s.content.as_ref() == "Migration")
            .expect("expected a span exactly matching Migration");
        assert!(keyword.style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn an_unrecognised_removal_note_line_renders_without_keyword_stripping() {
        let row = DiffRow::RemovalNoteFull {
            text: "Just plain text, not Reason or Migration.",
            indent: 1,
        };
        let spans = content_spans(&row);
        let text: String = spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(text, "¶ Just plain text, not Reason or Migration.");
        assert!(
            !spans
                .iter()
                .any(|s| s.style.add_modifier.contains(Modifier::BOLD))
        );
    }

    #[test]
    fn removal_note_rows_use_modification_styling_distinct_from_added_and_deleted() {
        let (marker, style) = removal_note_marker();
        assert_eq!(marker, "~");
        assert_eq!(style, modified_style());
        assert_ne!(style, added_style());
        assert_ne!(style, removed_marker_style());
    }

    #[test]
    fn a_short_removal_note_line_renders_in_full_with_no_ellipsis_or_triangle() {
        let row = DiffRow::RemovalNoteFull {
            text: "**Reason**: short.",
            indent: 1,
        };
        let lines = row_lines(&row, 80);
        let text: String = lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .map(|s| s.content.as_ref())
            .collect();
        assert!(text.contains("Reason: short."));
        assert!(!text.contains('…'));
        assert!(!text.contains('▸'));
        assert!(!text.contains('▾'));
    }

    #[test]
    fn a_long_removal_note_line_collapsed_shows_a_truncated_excerpt_with_a_triangle() {
        let long_line = format!("**Reason**: {}", "a very long reason ".repeat(20));
        let row = DiffRow::RemovalNoteLine {
            text: &long_line,
            expanded: false,
            key: removal_note_key(),
            indent: 1,
        };
        let width = 30;
        let lines = row_lines(&row, width);
        assert_eq!(lines.len(), 1);
        let text: String = lines[0].spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(text.contains('▸'));
        assert!(text.ends_with('…'));

        // The excerpt is a character-exact truncation of the de-asterisked
        // text (`Reason: ...`, not `**Reason**: ...`), so its length matches
        // the same budget the intro row's own excerpt is measured against.
        let budget = paragraph_available(width, 1);
        let pillcrow_idx = lines[0]
            .spans
            .iter()
            .position(|s| s.content.as_ref() == "¶")
            .expect("expected a pillcrow span");
        let excerpt: String = lines[0].spans[pillcrow_idx + 2..]
            .iter()
            .flat_map(|s| s.content.chars())
            .collect();
        assert_eq!(excerpt.chars().count(), budget);
        assert!(excerpt.starts_with("Reason:"));
    }

    #[test]
    fn a_collapsed_removal_note_line_de_asterisks_the_reason_keyword_even_when_truncated() {
        let long_line = format!("**Reason**: {}", "a very long reason ".repeat(20));
        let row = DiffRow::RemovalNoteLine {
            text: &long_line,
            expanded: false,
            key: removal_note_key(),
            indent: 1,
        };
        let lines = row_lines(&row, 30);
        let text: String = lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .map(|s| s.content.as_ref())
            .collect();
        assert!(
            !text.contains('*'),
            "expected no literal ** in the collapsed excerpt, got {text:?}"
        );
        assert!(text.contains("Reason:"));

        let keyword = lines[0]
            .spans
            .iter()
            .find(|s| s.content.as_ref() == "Reason")
            .expect("expected a span exactly matching Reason");
        assert!(keyword.style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn expanding_a_removal_note_line_reveals_the_full_wrapped_line() {
        let long_line = format!("**Reason**: {}", "a very long reason ".repeat(20));
        let row = DiffRow::RemovalNoteLine {
            text: &long_line,
            expanded: true,
            key: removal_note_key(),
            indent: 1,
        };
        let lines = row_lines(&row, 30);
        assert!(
            lines.len() > 1,
            "expected the line to wrap onto several lines"
        );
        let text: String = lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .map(|s| s.content.as_ref())
            .collect();
        assert!(text.contains("Reason:"));
        assert!(!text.contains('*'));
        assert!(text.contains('▾'));
    }
}
