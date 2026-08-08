mod app;
mod diff_row;
mod layout;
mod row;
mod wrap;

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{
    Block, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
};

use crate::changes::Changes;

use app::{App, Focus, PaneView};
use row::Row;

pub fn run() -> color_eyre::Result<()> {
    let cwd = std::env::current_dir()?;
    let changes = Changes::discover(&cwd)?;
    let mut app = App::new(changes);

    let mut terminal = ratatui::init();
    let result = event_loop(&mut terminal, &mut app);
    ratatui::restore();
    result
}

fn event_loop(terminal: &mut ratatui::DefaultTerminal, app: &mut App) -> color_eyre::Result<()> {
    loop {
        terminal.draw(|frame| render(frame, app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            if key.code == KeyCode::Char('q') && key.modifiers.contains(KeyModifiers::CONTROL) {
                return Ok(());
            }
            app.handle_key(key);
        }
    }
}

fn render(frame: &mut Frame, app: &mut App) {
    let [left, right] =
        Layout::horizontal([Constraint::Percentage(35), Constraint::Percentage(65)])
            .areas(frame.area());

    render_left_pane(frame, left, app);
    render_right_pane(frame, right, app);
}

fn render_left_pane(frame: &mut Frame, area: Rect, app: &mut App) {
    let block = Block::bordered()
        .title("Changes")
        .border_style(focus_border_style(app.focus() == Focus::Left));
    let inner_width = block.inner(area).width as usize;

    let rows = app.rows();
    let widest = widest_row_width(&rows);
    let max_scroll = widest.saturating_sub(inner_width);
    let effective_offset = app.h_scroll().min(max_scroll);
    let items: Vec<ListItem<'static>> = rows
        .iter()
        .map(|row| row_to_list_item(row, effective_offset))
        .collect();

    app.set_max_h_scroll(max_scroll);

    let list = List::new(items)
        .block(block)
        .highlight_style(Modifier::REVERSED);
    frame.render_stateful_widget(list, area, app.list_state());

    // `ScrollbarState::content_length` is the count of valid scroll positions (its own `last()`
    // sets `position = content_length - 1`), not the raw content width: our valid positions are
    // exactly `0..=max_scroll`, so content_length is `max_scroll + 1`. Using `widest` directly
    // would size the thumb as if a full extra viewport of overscroll were reachable beyond
    // `max_scroll` (matching Paragraph-style scrolling, where the last line can scroll to the
    // top), which we never allow — that made the thumb look far smaller, and move far less,
    // than the actual (small) scrollable range warranted.
    let mut scrollbar_state = ScrollbarState::new(max_scroll + 1)
        .position(effective_offset)
        .viewport_content_length(inner_width);
    let scrollbar = Scrollbar::new(ScrollbarOrientation::HorizontalBottom);
    frame.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
}

fn render_right_pane(frame: &mut Frame, area: Rect, app: &mut App) {
    let border_style = focus_border_style(app.focus() == Focus::Right);

    match app.pane_view() {
        PaneView::NotAChange => render_message_pane(
            frame,
            area,
            border_style,
            "Select a change to see its spec diff.",
        ),
        PaneView::PaneError(msg) => render_message_pane(frame, area, border_style, &msg),
        PaneView::NoCapabilities => render_message_pane(
            frame,
            area,
            border_style,
            "This change has no spec changes.",
        ),
        PaneView::Tabs {
            names,
            selected,
            tab_error,
        } => render_diff_tabs(frame, area, app, &names, selected, tab_error, border_style),
    }
}

/// Renders the pane's placeholder and error states: a bordered box with no
/// tab bar and a single message, used whenever there is no diff tree to show.
fn render_message_pane(frame: &mut Frame, area: Rect, border_style: Style, message: &str) {
    let block = Block::bordered().border_style(border_style);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(Paragraph::new(message.to_string()), inner);
}

/// Renders the pane's normal state: a tab bar in the border title and,
/// below it, either the selected tab's error notice or its flattened,
/// wrapped, scrollable diff tree.
fn render_diff_tabs(
    frame: &mut Frame,
    area: Rect,
    app: &mut App,
    names: &[String],
    selected: usize,
    tab_error: Option<String>,
    border_style: Style,
) {
    let block = Block::bordered()
        .border_style(border_style)
        .title(tab_bar_title(names, selected));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if let Some(msg) = tab_error {
        frame.render_widget(
            Paragraph::new(msg).style(Style::new().fg(Color::Red)),
            inner,
        );
        app.set_line_offset(0);
        app.set_max_line_offset(0);
        render_right_scrollbar(frame, area, 0, 0);
        return;
    }

    let inner_width = inner.width as usize;
    let inner_height = inner.height as usize;
    let cursor = app.cursor();

    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut selected_range: Option<(usize, usize)> = None;
    for (i, row) in app.diff_rows().iter().enumerate() {
        let row_lines = layout::row_lines(row, inner_width);
        let start = lines.len();
        if i == cursor {
            selected_range = Some((start, start + row_lines.len()));
        }
        lines.extend(row_lines);
    }

    let max_line_offset = lines.len().saturating_sub(inner_height);
    let mut offset = app.line_offset().min(max_line_offset);
    if let Some((start, end)) = selected_range {
        if start < offset {
            offset = start;
        } else if inner_height > 0 && end > offset + inner_height {
            offset = end.saturating_sub(inner_height);
        }
    }
    app.set_line_offset(offset);
    app.set_max_line_offset(max_line_offset);

    if let Some((start, end)) = selected_range {
        for line in &mut lines[start..end] {
            line.style = line
                .style
                .patch(Style::new().add_modifier(Modifier::REVERSED));
        }
    }

    let paragraph = Paragraph::new(Text::from(lines)).scroll((offset as u16, 0));
    frame.render_widget(paragraph, inner);

    render_right_scrollbar(frame, area, offset, max_line_offset);
}

/// Mirrors the left pane's horizontal scrollbar handling (`render_left_pane`):
/// `content_length` is `max_offset + 1`, the count of valid scroll positions,
/// and the scrollbar is rendered even when there is nothing to scroll.
fn render_right_scrollbar(frame: &mut Frame, area: Rect, offset: usize, max_line_offset: usize) {
    let mut scrollbar_state = ScrollbarState::new(max_line_offset + 1).position(offset);
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);
    frame.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
}

/// Builds the tab bar as styled spans for `Block::title`, rather than using
/// ratatui's `Tabs` widget, which renders into a `Rect` of its own and can't
/// draw into a block's border (see design.md).
fn tab_bar_title(names: &[String], selected: usize) -> Line<'static> {
    let mut spans = Vec::with_capacity(names.len() * 2);
    for (i, name) in names.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw(" │ "));
        }
        let style = if i == selected {
            Style::new().add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
        };
        spans.push(Span::styled(name.clone(), style));
    }
    Line::from(spans)
}

fn focus_border_style(focused: bool) -> Style {
    if focused {
        Style::new().add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    }
}

/// Builds a row's full-width styled spans, before any horizontal scroll offset is applied.
/// This is the single source of truth for a row's display text: both width measurement
/// (`row_display_width`) and offset application (`row_to_list_item`) build on it, so they
/// can't drift out of sync with each other or with what's actually rendered.
fn row_spans(row: &Row) -> Vec<Span<'static>> {
    let indent = if is_indented(row) { "  " } else { "" };
    match row {
        Row::Active(change) => vec![Span::raw(format!(
            "{indent}{}",
            format_name(change.display_name())
        ))],
        Row::ArchivedHeader { expanded } => {
            let marker = if *expanded { "▾" } else { "▸" };
            let mut style = Style::new();
            if !*expanded {
                style = style.add_modifier(Modifier::UNDERLINED);
            }
            vec![Span::styled(format!("{marker} archived"), style)]
        }
        Row::Archived(change) => {
            let mut spans = vec![Span::raw(indent)];
            match change.archive_date() {
                Some(date) => {
                    spans.push(Span::styled(
                        format!("{date} "),
                        Style::new().add_modifier(Modifier::DIM),
                    ));
                    spans.push(Span::raw(format_name(change.display_name())));
                }
                None => spans.push(Span::raw(format_name(change.display_name()))),
            }
            spans
        }
        Row::Placeholder { text, .. } => vec![Span::raw(format!("{indent}{text}"))],
    }
}

/// A row's total display width in columns, matching what `row_spans` renders. Row content is
/// plain ASCII/kebab-case names and dates plus the `▸`/`▾` markers, so character count and
/// rendered column width coincide here (see design.md - Risks / Trade-offs).
fn row_display_width(row: &Row) -> usize {
    row_spans(row)
        .iter()
        .map(|span| span.content.chars().count())
        .sum()
}

fn widest_row_width(rows: &[Row]) -> usize {
    rows.iter().map(row_display_width).max().unwrap_or(0)
}

/// Drops `offset` characters cumulatively across `spans`, in order, keeping each remaining
/// fragment attached to its original span's style. A span can be partially or fully consumed;
/// fully-consumed spans are dropped. Character-based (not byte-based) so it never panics on
/// multi-byte UTF-8 content (e.g. the `▸`/`▾` markers), unlike raw byte-index slicing.
fn skip_chars(spans: Vec<Span<'static>>, mut offset: usize) -> Vec<Span<'static>> {
    let mut result = Vec::with_capacity(spans.len());
    for span in spans {
        let len = span.content.chars().count();
        if offset >= len {
            offset -= len;
            continue;
        }
        let remaining: String = span.content.chars().skip(offset).collect();
        result.push(Span::styled(remaining, span.style));
        offset = 0;
    }
    result
}

fn row_to_list_item(row: &Row, h_scroll: usize) -> ListItem<'static> {
    ListItem::new(Line::from(skip_chars(row_spans(row), h_scroll)))
}

/// Formats a change's raw, hyphenated name for display.
fn format_name(name: &str) -> String {
    name.replace('-', " ")
}

fn is_indented(row: &Row) -> bool {
    matches!(row, Row::Archived(_)) || matches!(row, Row::Placeholder { indented: true, .. })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::changes::Change;

    #[test]
    fn widest_row_width_picks_the_longest_row() {
        let a = Change("a".to_string());
        let bb = Change("bb".to_string());
        let rows = vec![Row::Active(&a), Row::Active(&bb)];
        assert_eq!(
            widest_row_width(&rows),
            row_display_width(&Row::Active(&bb))
        );
    }

    #[test]
    fn widest_row_width_prefers_archived_row_with_date_and_indent() {
        let active = Change("x".to_string());
        let archived = Change("archive/2026-01-01-a-very-long-archived-change-name".to_string());
        let rows = vec![Row::Active(&active), Row::Archived(&archived)];
        assert_eq!(
            widest_row_width(&rows),
            row_display_width(&Row::Archived(&archived))
        );
        assert!(widest_row_width(&rows) > row_display_width(&Row::Active(&active)));
    }

    #[test]
    fn widest_row_width_considers_placeholder_rows() {
        let rows = vec![
            Row::Placeholder {
                text: "(no active changes)",
                indented: false,
            },
            Row::ArchivedHeader { expanded: true },
            Row::Placeholder {
                text: "(no archived changes)",
                indented: true,
            },
        ];
        let expected = rows.iter().map(row_display_width).max().unwrap();
        assert_eq!(widest_row_width(&rows), expected);
    }

    #[test]
    fn widest_row_width_of_no_rows_is_zero() {
        assert_eq!(widest_row_width(&[]), 0);
    }

    #[test]
    fn skip_chars_blank_when_offset_exceeds_content() {
        let spans = vec![Span::raw("short")];
        let result = skip_chars(spans, 100);
        assert!(result.is_empty());
    }

    #[test]
    fn skip_chars_within_a_single_span() {
        let spans = vec![Span::raw("hello")];
        let result = skip_chars(spans, 2);
        let text: String = result.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(text, "llo");
    }

    #[test]
    fn archived_header_scrolls_through_marker_without_panicking() {
        for expanded in [false, true] {
            let row = Row::ArchivedHeader { expanded };
            let width = row_display_width(&row);
            for offset in 0..=(width + 3) {
                let spans = skip_chars(row_spans(&row), offset);
                let remaining: usize = spans.iter().map(|s| s.content.chars().count()).sum();
                assert_eq!(remaining, width.saturating_sub(offset));
            }
        }
    }

    #[test]
    fn collapsed_archived_header_is_underlined_and_expanded_is_not() {
        let collapsed = row_spans(&Row::ArchivedHeader { expanded: false });
        assert!(
            collapsed
                .iter()
                .all(|s| s.style.add_modifier.contains(Modifier::UNDERLINED))
        );

        let expanded = row_spans(&Row::ArchivedHeader { expanded: true });
        assert!(
            expanded
                .iter()
                .all(|s| !s.style.add_modifier.contains(Modifier::UNDERLINED))
        );
    }

    #[test]
    fn collapsed_archived_header_underline_persists_when_scrolled() {
        let spans = row_spans(&Row::ArchivedHeader { expanded: false });
        let scrolled = skip_chars(spans, 3);
        assert!(!scrolled.is_empty());
        assert!(
            scrolled
                .iter()
                .all(|s| s.style.add_modifier.contains(Modifier::UNDERLINED))
        );
    }

    #[test]
    fn archived_row_date_stays_dimmed_when_scrolled_partway_through_it() {
        let change = Change("archive/2026-01-01-my-change".to_string());
        let row = Row::Archived(&change);
        let spans = row_spans(&row);
        let indent_len = spans[0].content.chars().count();

        // Land partway into the date span (which starts right after the indent).
        let offset = indent_len + 3;
        let result = skip_chars(spans, offset);

        assert!(!result.is_empty());
        assert!(result[0].style.add_modifier.contains(Modifier::DIM));
        assert!(result[0].content.starts_with("6-01-01"));
    }
}
