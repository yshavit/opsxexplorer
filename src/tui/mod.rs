mod app;
mod diff_row;
mod help;
mod layout;
mod row;
mod wrap;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{
    Block, BorderType, Clear, List, ListItem, Padding, Paragraph, Scrollbar, ScrollbarOrientation,
    ScrollbarState,
};

use crate::changes::Changes;
use crate::diff::Operation;

use app::{App, Focus, PaneView};
use diff_row::DiffRow;
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
            if is_quit_key(&key) {
                return Ok(());
            }
            app.handle_key(key);
        }
    }
}

fn is_quit_key(key: &KeyEvent) -> bool {
    key.code == KeyCode::Char('q') && key.modifiers == KeyModifiers::NONE
}

fn render(frame: &mut Frame, app: &mut App) {
    let left_width = left_pane_width(frame.area().width, &app.all_rows());
    let [left, right] =
        Layout::horizontal([Constraint::Length(left_width), Constraint::Min(0)])
            .areas(frame.area());

    render_left_pane(frame, left, app);
    render_right_pane(frame, right, app);

    if app.help_open() {
        render_help_modal(frame, frame.area(), app);
    }
}

fn render_left_pane(frame: &mut Frame, area: Rect, app: &mut App) {
    let dim = app.help_open();
    let mut border_style = focus_border_style(app.focus() == Focus::Left);
    if dim {
        border_style = desaturate_style(border_style);
    }
    let block = Block::bordered()
        .title("Changes")
        .border_style(border_style);
    let inner = block.inner(area);
    let inner_width = inner.width as usize;

    let rows = app.rows();
    let widest = widest_row_width(&app.all_rows());
    let max_scroll = widest.saturating_sub(inner_width);
    let effective_offset = app.h_scroll().min(max_scroll);
    let items: Vec<ListItem<'static>> = rows
        .iter()
        .map(|row| row_to_list_item(row, effective_offset, dim))
        .collect();

    app.set_max_h_scroll(max_scroll);
    app.set_left_viewport_rows(inner.height as usize);

    let list = List::new(items)
        .block(block)
        .highlight_style(Modifier::REVERSED);
    frame.render_stateful_widget(list, area, app.list_state());

    if max_scroll == 0 {
        return;
    }

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
    let dim = app.help_open();
    let mut border_style = focus_border_style(app.focus() == Focus::Right);
    if dim {
        border_style = desaturate_style(border_style);
    }

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
    let dim = app.help_open();
    let block = Block::bordered()
        .border_style(border_style)
        .title(tab_bar_title(names, selected));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if let Some(msg) = tab_error {
        let mut style = Style::new().fg(Color::Red);
        if dim {
            style = desaturate_style(style);
        }
        frame.render_widget(Paragraph::new(msg).style(style), inner);
        app.set_line_offset(0);
        app.set_max_line_offset(0);
        render_right_scrollbar(frame, area, 0, 0);
        return;
    }

    let inner_width = inner.width as usize;
    let inner_height = inner.height as usize;
    let cursor = app.cursor();

    let (mut lines, selected_range, reveal_end) =
        build_diff_lines(&app.diff_rows(), inner_width, cursor);
    if dim {
        lines = lines.into_iter().map(desaturate_line).collect();
    }

    let max_line_offset = lines.len().saturating_sub(inner_height);
    let offset = clamp_offset(
        app.line_offset(),
        max_line_offset,
        selected_range,
        reveal_end,
        inner_height,
    );
    app.set_line_offset(offset);
    app.set_max_line_offset(max_line_offset);
    app.set_right_viewport_rows(inner_height);

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

/// `content_length` is `max_offset + 1`, the count of valid scroll positions,
/// matching the left pane's horizontal scrollbar handling (`render_left_pane`).
/// Unlike that one, this scrollbar is hidden entirely when there's nothing to
/// scroll, rather than rendered in an empty/full state.
fn render_right_scrollbar(frame: &mut Frame, area: Rect, offset: usize, max_line_offset: usize) {
    if max_line_offset == 0 {
        return;
    }
    let mut scrollbar_state = ScrollbarState::new(max_line_offset + 1).position(offset);
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);
    frame.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
}

/// Renders the `?` help modal as a `Clear` + bordered popup over the two-pane
/// layout, sized to its content and capped to the available frame (see
/// design.md - Decision 4). The content itself is fully static — there's
/// nothing left to expand or collapse — so the popup's width never varies;
/// only its height ever clamps, and only when the frame is too short to fit
/// everything, in which case the content scrolls line by line.
fn render_help_modal(frame: &mut Frame, area: Rect, app: &mut App) {
    let lines = help::lines();

    // Columns of padding on *each* of the left/right edges (spec.md);
    // `Padding::horizontal` below applies this to both sides, so the width
    // computation must account for it twice, not once — a mismatch here
    // previously left the widest line's tail clipped by exactly that gap.
    const H_PADDING: u16 = 2;
    let content_width = lines.iter().map(line_width).max().unwrap_or(0) as u16 + H_PADDING * 2;
    let content_height = lines.len() as u16;
    let (popup_width, popup_height) = popup_size(content_width, content_height, area);
    let popup_area = centered_rect(popup_width, popup_height, area);

    let block = Block::bordered()
        .title("Keybindings")
        .on_light_blue()
        .border_style(Style::new().fg(Color::DarkGray))
        .border_type(BorderType::Rounded)
        .padding(Padding::horizontal(H_PADDING));
    let inner = block.inner(popup_area);
    let inner_height = inner.height as usize;

    let max_line_offset = lines.len().saturating_sub(inner_height);
    let offset = app.help_line_offset().min(max_line_offset);
    app.set_help_line_offset(offset);
    app.set_help_max_line_offset(max_line_offset);
    app.set_help_viewport_rows(inner_height);

    frame.render_widget(Clear, popup_area);
    frame.render_widget(block, popup_area);
    let paragraph = Paragraph::new(Text::from(lines)).scroll((offset as u16, 0));
    frame.render_widget(paragraph, inner);

    render_right_scrollbar(frame, popup_area, offset, max_line_offset);
}

fn line_width(line: &Line) -> usize {
    line.spans.iter().map(|s| s.content.chars().count()).sum()
}

/// The popup's `(width, height)`: exactly `content` plus its own border, unless
/// that exceeds what `area` has room for (minus a small margin so the popup
/// never touches the frame's edge), in which case it's capped to the available
/// space (spec.md - "The modal is sized to its content, capped to the
/// available frame").
fn popup_size(content_width: u16, content_height: u16, area: Rect) -> (u16, u16) {
    const MARGIN: u16 = 2;
    const BORDERS: u16 = 2;
    let max_width = area.width.saturating_sub(MARGIN * 2).max(BORDERS + 1);
    let max_height = area.height.saturating_sub(MARGIN * 2).max(BORDERS + 1);
    let width = (content_width + BORDERS).min(max_width);
    let height = (content_height + BORDERS).min(max_height);
    (width, height)
}

/// Centers a `width`x`height` rect within `area`, clamping to `area`'s own
/// bounds so it never renders larger than the frame it overlays.
fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    let x = area.x + (area.width - width) / 2;
    let y = area.y + (area.height - height) / 2;
    Rect::new(x, y, width, height)
}

/// Lays out a tab's rows into rendered lines, and reports which line range
/// belongs to the row at `cursor` (for highlighting) plus how far its
/// revealed content — the rows expanding it just uncovered — extends (for
/// scrolling). A blank line is inserted before every group heading except
/// the first, and each heading renders as a small bordered box rather than a
/// plain line, so a capability's operation groups (Added/Modified/...) read
/// as visually distinct sections rather than running together — the cursor
/// is unaffected since it addresses `DiffRow`s, not rendered lines, and
/// neither the spacer nor the box's extra border lines are rows.
///
/// `reveal_end` is `selected_range`'s own `end` widened through whatever the
/// cursor's row owns: a `Requirement`'s intro and scenario headers (and any
/// of *their* expanded bodies), or a `Scenario`'s own body. It stops at the
/// next row that isn't part of that — another requirement, a group heading,
/// or a notice. Without this, expanding the bottom-most row on screen reveals
/// content the offset-clamp never learns about, since it only ever checked
/// the cursor row's own single-row span.
fn build_diff_lines(
    rows: &[DiffRow],
    width: usize,
    cursor: usize,
) -> (Vec<Line<'static>>, Option<(usize, usize)>, usize) {
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut selected_range: Option<(usize, usize)> = None;
    let mut reveal_end = 0;
    let mut seen_group_heading = false;
    let mut in_cursor_block = false;
    let mut cursor_is_requirement = false;

    for (i, row) in rows.iter().enumerate() {
        if in_cursor_block {
            let leaves_block = match row {
                DiffRow::Intro { .. } | DiffRow::Body { .. } => false,
                DiffRow::Scenario { .. } => !cursor_is_requirement,
                DiffRow::Requirement { .. } | DiffRow::GroupHeading(_) | DiffRow::Notice(_) => true,
            };
            if leaves_block {
                reveal_end = lines.len();
                in_cursor_block = false;
            }
        }

        let row_lines = if let DiffRow::GroupHeading(op) = row {
            if seen_group_heading {
                lines.push(Line::default());
            }
            seen_group_heading = true;
            group_heading_box(op, width, requirement_count_after(rows, i))
        } else {
            layout::row_lines(row, width)
        };

        let start = lines.len();
        if i == cursor {
            let end = start + row_lines.len();
            selected_range = Some((start, end));
            reveal_end = end;
            in_cursor_block = true;
            cursor_is_requirement = matches!(row, DiffRow::Requirement { .. });
        }
        lines.extend(row_lines);
    }
    if in_cursor_block {
        reveal_end = lines.len();
    }

    (lines, selected_range, reveal_end)
}

/// Counts how many requirements the group heading at `heading_index`
/// introduces, so its label can read "Added Requirement" vs "Added
/// Requirements". Scans the run's own top-level `Requirement` rows, skipping
/// their (possibly expanded) children, and stops at the next group heading
/// or notice — the same boundary `build_diff_lines`'s block-detection uses.
fn requirement_count_after(rows: &[DiffRow], heading_index: usize) -> usize {
    rows[heading_index + 1..]
        .iter()
        .take_while(|row| !matches!(row, DiffRow::GroupHeading(_) | DiffRow::Notice(_)))
        .filter(|row| matches!(row, DiffRow::Requirement { .. }))
        .count()
}

/// Computes the render-time vertical scroll offset: clamps the stored offset
/// to the current content length, then adjusts it so the cursor row is
/// visible and, so far as the viewport allows, so is everything its
/// expansion revealed (`reveal_end`, from `build_diff_lines`) — not just the
/// cursor row's own single line, which is all the previous version checked
/// and is why expanding the bottom-most row on screen used to leave the
/// newly revealed content unreachable.
fn clamp_offset(
    stored_offset: usize,
    max_line_offset: usize,
    selected_range: Option<(usize, usize)>,
    reveal_end: usize,
    inner_height: usize,
) -> usize {
    let mut offset = stored_offset.min(max_line_offset);
    if let Some((start, _)) = selected_range {
        if start < offset {
            offset = start;
        } else if inner_height > 0 && reveal_end > offset + inner_height {
            // Scroll down far enough to reveal as much of the cursor row's
            // content as fits, but never past `start` — if the revealed
            // content is itself taller than the viewport, keep the row's
            // own top in view rather than jumping straight to its tail.
            offset = reveal_end.saturating_sub(inner_height).min(start);
        }
    }
    offset
}

/// Renders an operation's group heading as a small bordered box with the
/// label itself colored per `Operation` (via `layout::operation_style`,
/// matching the requirement marker's own color), so a heading has enough
/// visual weight to read as a section boundary rather than another content
/// line. Degrades to a single unboxed line when the pane is too narrow for
/// a box to make sense.
fn group_heading_box(op: &Operation, width: usize, count: usize) -> Vec<Line<'static>> {
    let border_style = layout::operation_style(op);
    let label_style = border_style.add_modifier(Modifier::BOLD);
    let label = heading_label(op, count);
    let label_width = label.chars().count();

    const BORDER_WIDTH: usize = 2; // the left and right border columns
    const PREFIX_WIDTH: usize = 1; // the space between the border and the label
    let min_width = BORDER_WIDTH + PREFIX_WIDTH + label_width;

    if width < min_width {
        return vec![Line::from(Span::styled(label, label_style))];
    }

    let inner_width = width - BORDER_WIDTH;
    let pad = inner_width.saturating_sub(PREFIX_WIDTH + label_width);

    vec![
        Line::from(Span::styled(
            format!("╭{}╮", "─".repeat(inner_width)),
            border_style,
        )),
        Line::from(vec![
            Span::styled("│ ", border_style),
            Span::styled(label, label_style),
            Span::raw(" ".repeat(pad)),
            Span::styled("│", border_style),
        ]),
        Line::from(Span::styled(
            format!("╰{}╯", "─".repeat(inner_width)),
            border_style,
        )),
    ]
}

/// "Added Requirement" vs "Added Requirements" — a group heading names what
/// it groups, pluralized by how many requirements it actually introduces.
fn heading_label(op: &Operation, count: usize) -> String {
    let noun = if count == 1 {
        "Requirement"
    } else {
        "Requirements"
    };
    format!("{} {noun}", layout::heading_text(op))
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
        Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD)
    } else {
        Style::new().add_modifier(Modifier::DIM)
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
            vec![Span::styled(format!("{marker} archived/"), Style::new())]
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

/// The left pane's width: the lesser of its default 35%-of-frame share and
/// the widest row's content width plus a one-column buffer plus the pane's
/// two border columns, computed over `all_rows` (every row, archived
/// included, regardless of expand state) so the width doesn't change as the
/// archived section is toggled (see design.md).
fn left_pane_width(frame_width: u16, all_rows: &[Row]) -> u16 {
    const BUFFER_AND_BORDERS: usize = 1 + 2;
    let proportional_share = frame_width as usize * 35 / 100;
    let content_width = widest_row_width(all_rows) + BUFFER_AND_BORDERS;
    proportional_share.min(content_width) as u16
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

fn row_to_list_item(row: &Row, h_scroll: usize, dim: bool) -> ListItem<'static> {
    let line = Line::from(skip_chars(row_spans(row), h_scroll));
    let line = if dim { desaturate_line(line) } else { line };
    ListItem::new(line)
}

/// The flat neutral tone every colored span is reduced to while the help modal is
/// open, replacing whichever color it originally carried. A single post-process
/// step over already-styled output, rather than a "dimmed" parameter threaded
/// through every color-producing call site (see design.md - Decision 5).
const DESATURATED_COLOR: Color = Color::Gray;

fn desaturate_style(style: Style) -> Style {
    Style {
        fg: style.fg.map(|_| DESATURATED_COLOR),
        bg: style.bg.map(|_| DESATURATED_COLOR),
        ..style
    }
}

fn desaturate_span(span: Span<'static>) -> Span<'static> {
    Span::styled(span.content, desaturate_style(span.style))
}

fn desaturate_line(line: Line<'static>) -> Line<'static> {
    let style = desaturate_style(line.style);
    Line::from(
        line.spans
            .into_iter()
            .map(desaturate_span)
            .collect::<Vec<_>>(),
    )
    .style(style)
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
    fn q_with_no_modifier_is_the_quit_key() {
        assert!(is_quit_key(&KeyEvent::new(
            KeyCode::Char('q'),
            KeyModifiers::NONE
        )));
    }

    #[test]
    fn ctrl_q_is_no_longer_the_quit_key() {
        assert!(!is_quit_key(&KeyEvent::new(
            KeyCode::Char('q'),
            KeyModifiers::CONTROL
        )));
    }

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
    fn left_pane_width_shrinks_to_narrow_content_plus_buffer_and_borders() {
        let a = Change("a".to_string());
        let rows = vec![Row::Active(&a), Row::ArchivedHeader { expanded: false }];
        let frame_width = 100;
        let width = left_pane_width(frame_width, &rows);
        let expected = widest_row_width(&rows) as u16 + 1 + 2;
        assert_eq!(width, expected);
        assert!((width as usize) < frame_width as usize * 35 / 100);
    }

    #[test]
    fn left_pane_width_caps_at_the_proportional_share_for_wide_content() {
        let long = Change("a".repeat(200));
        let rows = vec![Row::Active(&long)];
        let frame_width = 100;
        let width = left_pane_width(frame_width, &rows);
        assert_eq!(width as usize, frame_width as usize * 35 / 100);
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
    fn archived_header_label_has_a_trailing_slash_collapsed_and_expanded() {
        for expanded in [false, true] {
            let spans = row_spans(&Row::ArchivedHeader { expanded });
            let text: String = spans.iter().map(|s| s.content.as_ref()).collect();
            assert!(text.ends_with("archived/"), "expected {text:?} to end with archived/");
        }
    }

    #[test]
    fn collapsed_and_expanded_archived_header_differ_only_in_the_marker() {
        let collapsed = row_spans(&Row::ArchivedHeader { expanded: false });
        let expanded = row_spans(&Row::ArchivedHeader { expanded: true });

        let collapsed_text: String = collapsed.iter().map(|s| s.content.as_ref()).collect();
        let expanded_text: String = expanded.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(collapsed_text, "▸ archived/");
        assert_eq!(expanded_text, "▾ archived/");

        assert_eq!(
            collapsed.iter().map(|s| s.style).collect::<Vec<_>>(),
            expanded.iter().map(|s| s.style).collect::<Vec<_>>()
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

    // --- build_diff_lines: group-heading spacers ---

    fn req_row<'a>(name: &'a str, op: &'a crate::diff::Operation) -> DiffRow<'a> {
        req_row_expanded(name, op, false)
    }

    fn req_row_expanded<'a>(
        name: &'a str,
        op: &'a crate::diff::Operation,
        expanded: bool,
    ) -> DiffRow<'a> {
        DiffRow::Requirement {
            name,
            op,
            expanded,
            key: diff_row::RowKey {
                capability: "cap".to_string(),
                requirement: name.to_string(),
                scenario: None,
            },
        }
    }

    fn scenario_row<'a>(
        name: &'a str,
        body: &'a crate::diff::Piece,
        expanded: bool,
    ) -> DiffRow<'a> {
        DiffRow::Scenario {
            name,
            body,
            expanded,
            key: diff_row::RowKey {
                capability: "cap".to_string(),
                requirement: "Req".to_string(),
                scenario: Some(name.to_string()),
            },
        }
    }

    fn unchanged_piece(text: &str) -> crate::diff::Piece {
        crate::diff::Piece::Unchanged {
            text: text.to_string(),
        }
    }

    fn line_text(line: &Line<'static>) -> String {
        line.spans.iter().map(|s| s.content.as_ref()).collect()
    }

    #[test]
    fn no_blank_line_before_the_first_group_heading() {
        let added = crate::diff::Operation::Added;
        let rows = vec![DiffRow::GroupHeading(&added), req_row("A", &added)];
        let (lines, _, _) = build_diff_lines(&rows, 40, 0);
        // 3 box lines (top/content/bottom) + 1 requirement line, no leading blank.
        assert_eq!(lines.len(), 4);
        assert!(line_text(&lines[0]).starts_with('╭'));
    }

    #[test]
    fn blank_line_and_box_appear_before_a_later_group_heading_and_cursor_range_accounts_for_them() {
        let added = crate::diff::Operation::Added;
        let modified = crate::diff::Operation::Modified;
        let rows = vec![
            DiffRow::GroupHeading(&added),
            req_row("A", &added),
            DiffRow::GroupHeading(&modified),
            req_row("B", &modified),
        ];

        // Cursor on "B", the last row (index 3 in `rows`).
        let (lines, selected_range, _) = build_diff_lines(&rows, 30, 3);

        // box(A): top/content/bottom, req(A), blank, box(B): top/content/bottom, req(B)
        assert_eq!(lines.len(), 9);
        assert!(line_text(&lines[0]).starts_with('╭'));
        assert!(line_text(&lines[4]).trim().is_empty());
        assert!(line_text(&lines[5]).starts_with('╭'));
        assert!(line_text(&lines[6]).contains("Modified"));
        assert!(line_text(&lines[7]).starts_with('╰'));

        // "B"'s rendered line is pushed later than a naive row-count would
        // predict, because of the inserted spacer and the box's own border lines.
        assert_eq!(selected_range, Some((8, 9)));
    }

    #[test]
    fn group_heading_box_content_never_overflows_its_own_border() {
        // "Modified Requirements" (plural) is the longest heading label; a
        // width just wide enough for a box (but not comfortably so) must not
        // let the content line (border + label) run past the box's own
        // top/bottom border.
        let modified = crate::diff::Operation::Modified;
        for width in 10..=30 {
            let lines = group_heading_box(&modified, width, 2);
            let border_width = line_text(&lines[0]).chars().count();
            for line in &lines {
                assert!(
                    line_text(line).chars().count() <= border_width,
                    "content line wider than the box's own border at width {width}"
                );
            }
        }
    }

    #[test]
    fn group_heading_falls_back_to_a_plain_line_when_too_narrow_for_a_box() {
        let added = crate::diff::Operation::Added;
        let lines = group_heading_box(&added, 6, 1);
        assert_eq!(lines.len(), 1);
        assert_eq!(line_text(&lines[0]), "Added Requirement");
    }

    #[test]
    fn group_heading_label_pluralizes_by_requirement_count() {
        let added = crate::diff::Operation::Added;
        assert_eq!(heading_label(&added, 1), "Added Requirement");
        assert_eq!(heading_label(&added, 0), "Added Requirements");
        assert_eq!(heading_label(&added, 2), "Added Requirements");
    }

    #[test]
    fn requirement_count_after_counts_only_the_runs_own_top_level_requirements() {
        let added = crate::diff::Operation::Added;
        let modified = crate::diff::Operation::Modified;
        let intro = unchanged_piece("intro");
        let body = unchanged_piece("body");
        let rows = vec![
            DiffRow::GroupHeading(&added),
            req_row_expanded("A", &added, true),
            DiffRow::Intro { piece: &intro },
            scenario_row("S1", &body, true),
            DiffRow::Body { piece: &body },
            req_row("B", &added),
            DiffRow::GroupHeading(&modified),
            req_row("C", &modified),
        ];
        assert_eq!(requirement_count_after(&rows, 0), 2); // A, B
        assert_eq!(requirement_count_after(&rows, 6), 1); // C
    }

    // --- build_diff_lines: reveal_end (scroll-to-show-expanded-content bug) ---

    #[test]
    fn reveal_end_extends_through_an_expanded_requirements_intro_and_scenarios() {
        let added = crate::diff::Operation::Added;
        let intro = unchanged_piece("intro text");
        let body1 = unchanged_piece("body one");
        let rows = vec![
            req_row_expanded("A", &added, true),
            DiffRow::Intro { piece: &intro },
            scenario_row("S1", &body1, false),
            req_row("B", &added),
        ];

        // Cursor on "A" (index 0), which we just expanded.
        let (_, selected_range, reveal_end) = build_diff_lines(&rows, 40, 0);

        assert_eq!(selected_range, Some((0, 1)));
        // Extends through the intro and scenario header (lines 1 and 2),
        // stopping right before "B" starts at line 3.
        assert_eq!(reveal_end, 3);
    }

    #[test]
    fn reveal_end_extends_through_a_scenarios_body_but_stops_at_the_next_scenario() {
        let added = crate::diff::Operation::Added;
        let intro = unchanged_piece("intro");
        let body1 = unchanged_piece("body one");
        let body2 = unchanged_piece("body two");
        let rows = vec![
            req_row_expanded("A", &added, true),
            DiffRow::Intro { piece: &intro },
            scenario_row("S1", &body1, true),
            DiffRow::Body { piece: &body1 },
            scenario_row("S2", &body2, false),
        ];

        // Cursor on "S1" (index 2), which we just expanded.
        let (_, selected_range, reveal_end) = build_diff_lines(&rows, 40, 2);

        assert_eq!(selected_range, Some((2, 3)));
        // Extends through its own body (line 3), stopping before "S2" at line 4
        // — a sibling scenario is not part of what "S1" revealed.
        assert_eq!(reveal_end, 4);
    }

    #[test]
    fn reveal_end_matches_selected_range_end_when_nothing_new_was_revealed() {
        let added = crate::diff::Operation::Added;
        let rows = vec![req_row("A", &added), req_row("B", &added)];
        let (_, selected_range, reveal_end) = build_diff_lines(&rows, 40, 0);
        let (_, end) = selected_range.unwrap();
        assert_eq!(reveal_end, end);
    }

    // --- clamp_offset: the actual scroll-position fix ---

    #[test]
    fn clamp_offset_scrolls_down_to_reveal_content_an_expansion_uncovered() {
        // The cursor row (a single line at index 6) was already visible with
        // offset 3 in a 5-line viewport ([3, 8)). Expanding it grew what
        // follows out to line 15 — well past the viewport — which the old
        // logic never saw, since it only checked the cursor row's own end (7).
        let offset = clamp_offset(3, 20, Some((6, 7)), 15, 5);
        // Scrolls down so the row's own top becomes the first visible line,
        // showing as much of the newly revealed content as fits beneath it.
        assert_eq!(offset, 6);
    }

    #[test]
    fn clamp_offset_is_a_no_op_when_the_revealed_content_already_fits() {
        let offset = clamp_offset(3, 20, Some((6, 7)), 8, 5);
        assert_eq!(offset, 3);
    }

    #[test]
    fn clamp_offset_still_scrolls_up_to_show_a_cursor_row_above_the_viewport() {
        let offset = clamp_offset(10, 20, Some((2, 3)), 3, 5);
        assert_eq!(offset, 2);
    }

    #[test]
    fn clamp_offset_prioritizes_the_cursor_rows_own_top_when_revealed_content_cant_fully_fit() {
        // The cursor row is the very first row, and the block it reveals is
        // much taller than the viewport. There's nowhere to scroll without
        // hiding the row's own header, so the offset stays put — the rest
        // becomes reachable as the cursor moves further into the block.
        let offset = clamp_offset(0, 20, Some((0, 1)), 20, 5);
        assert_eq!(offset, 0);
    }

    // --- help modal: popup sizing ---

    #[test]
    fn popup_sizes_to_content_when_it_fits_the_frame() {
        let area = Rect::new(0, 0, 80, 40);
        assert_eq!(popup_size(30, 10, area), (32, 12));
    }

    #[test]
    fn popup_clamps_to_the_available_frame_when_content_overflows() {
        let area = Rect::new(0, 0, 20, 10);
        // max width/height is the frame minus a 2-column margin on each side.
        assert_eq!(popup_size(100, 100, area), (16, 6));
    }

    // --- help modal: desaturation pass ---

    #[test]
    fn desaturate_strips_colors_but_preserves_modifiers() {
        let style = Style::new()
            .fg(Color::Red)
            .bg(Color::Blue)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED);
        let result = desaturate_style(style);
        assert_eq!(result.fg, Some(DESATURATED_COLOR));
        assert_eq!(result.bg, Some(DESATURATED_COLOR));
        assert!(result.add_modifier.contains(Modifier::BOLD));
        assert!(result.add_modifier.contains(Modifier::UNDERLINED));
    }

    #[test]
    fn desaturate_leaves_unset_colors_unset() {
        let style = Style::new().add_modifier(Modifier::DIM);
        let result = desaturate_style(style);
        assert_eq!(result.fg, None);
        assert_eq!(result.bg, None);
        assert!(result.add_modifier.contains(Modifier::DIM));
    }

    #[test]
    fn desaturate_line_applies_to_every_span_and_the_lines_own_style() {
        let line = Line::from(vec![
            Span::styled("a", Style::new().fg(Color::Green)),
            Span::styled("b", Style::new().fg(Color::Red)),
        ])
        .style(Style::new().bg(Color::Yellow));
        let result = desaturate_line(line);
        assert_eq!(result.style.bg, Some(DESATURATED_COLOR));
        for span in &result.spans {
            assert_eq!(span.style.fg, Some(DESATURATED_COLOR));
        }
    }

    // --- render_right_scrollbar: hidden when there's nothing to scroll ---

    #[test]
    fn scrollbar_is_hidden_when_nothing_to_scroll() {
        use ratatui::Terminal;
        use ratatui::backend::TestBackend;

        let mut terminal = Terminal::new(TestBackend::new(10, 5)).unwrap();
        terminal
            .draw(|f| render_right_scrollbar(f, f.area(), 0, 0))
            .unwrap();
        let buf = terminal.backend().buffer();
        assert!(
            buf.content().iter().all(|cell| cell.symbol() == " "),
            "expected nothing to be drawn when max_line_offset is 0"
        );
    }

    #[test]
    fn scrollbar_is_shown_when_there_is_something_to_scroll() {
        use ratatui::Terminal;
        use ratatui::backend::TestBackend;

        let mut terminal = Terminal::new(TestBackend::new(10, 5)).unwrap();
        terminal
            .draw(|f| render_right_scrollbar(f, f.area(), 0, 3))
            .unwrap();
        let buf = terminal.backend().buffer();
        assert!(
            buf.content().iter().any(|cell| cell.symbol() != " "),
            "expected the scrollbar to draw something when max_line_offset is nonzero"
        );
    }

    // --- help modal: popup width must fit the widest line past its own padding ---

    #[test]
    fn widest_entry_line_is_not_clipped_by_the_popups_own_padding() {
        use crate::changes::Changes;
        use ratatui::Terminal;
        use ratatui::backend::TestBackend;

        let dir = std::env::temp_dir().join(format!(
            "opsxexplorer-help-width-test-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(dir.join("openspec/changes")).unwrap();
        let mut app = App::new(Changes::discover(&dir).unwrap());
        app.handle_key(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE));

        // Wide enough that the popup renders at its natural content width,
        // not clamped by the frame.
        let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
        terminal.draw(|f| render(f, &mut app)).unwrap();
        let buf = terminal.backend().buffer();

        let widest_line_text = help::lines()
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|s| s.content.as_ref())
                    .collect::<String>()
            })
            .max_by_key(|text| text.chars().count())
            .unwrap();

        let rendered_contains_it = (0..buf.area.height).any(|y| {
            let row: String = (0..buf.area.width).map(|x| buf[(x, y)].symbol()).collect();
            row.contains(widest_line_text.trim_end())
        });
        assert!(
            rendered_contains_it,
            "expected the widest content line ({widest_line_text:?}) to render in full, \
             not clipped by the popup's border/padding"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    // --- left pane: content-driven width and scrollbar visibility ---

    fn temp_repo_with_changes(active: &[&str], archived: &[&str]) -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "opsxexplorer-left-pane-width-test-{}-{id}",
            std::process::id()
        ));
        std::fs::create_dir_all(dir.join("openspec/changes")).unwrap();
        for name in active {
            std::fs::create_dir_all(dir.join("openspec/changes").join(name)).unwrap();
        }
        if !archived.is_empty() {
            std::fs::create_dir_all(dir.join("openspec/changes/archive")).unwrap();
            for name in archived {
                std::fs::create_dir_all(dir.join("openspec/changes/archive").join(name)).unwrap();
            }
        }
        dir
    }

    /// The column at which the right pane's own left border starts, found by
    /// locating the top row's `┐┌` seam between the two adjacent blocks.
    fn right_pane_left_border_column(buf: &ratatui::buffer::Buffer) -> usize {
        let symbols: Vec<&str> = (0..buf.area.width).map(|x| buf[(x, 0)].symbol()).collect();
        symbols
            .windows(2)
            .position(|w| w[0] == "┐" && w[1] == "┌")
            .map(|i| i + 1)
            .expect("expected a seam between the left and right panes' top borders")
    }

    fn bottom_row_has_scrollbar_glyphs(buf: &ratatui::buffer::Buffer) -> bool {
        let bottom_y = buf.area.height - 1;
        (0..buf.area.width).any(|x| {
            let s = buf[(x, bottom_y)].symbol();
            s == "←" || s == "→" || s == "█"
        })
    }

    #[test]
    fn left_pane_width_is_identical_whether_archived_is_expanded_or_collapsed() {
        use crate::changes::Changes;
        use ratatui::Terminal;
        use ratatui::backend::TestBackend;

        let dir =
            temp_repo_with_changes(&["a"], &["2026-01-01-a-rather-long-archived-change-name"]);
        let mut app = App::new(Changes::discover(&dir).unwrap());

        let mut terminal = Terminal::new(TestBackend::new(80, 10)).unwrap();
        terminal.draw(|f| render(f, &mut app)).unwrap();
        let collapsed_border_col = right_pane_left_border_column(terminal.backend().buffer());

        let rows = app.rows();
        let header_idx = rows
            .iter()
            .position(|r| matches!(r, Row::ArchivedHeader { .. }))
            .unwrap();
        app.list_state().select(Some(header_idx));
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(matches!(
            app.rows()[header_idx],
            Row::ArchivedHeader { expanded: true }
        ));

        terminal.draw(|f| render(f, &mut app)).unwrap();
        let expanded_border_col = right_pane_left_border_column(terminal.backend().buffer());

        assert_eq!(
            collapsed_border_col, expanded_border_col,
            "left pane width must not change when the archived section is toggled"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn left_pane_scrollbar_hidden_when_content_fits() {
        use crate::changes::Changes;
        use ratatui::Terminal;
        use ratatui::backend::TestBackend;

        let dir = temp_repo_with_changes(&["a", "b"], &[]);
        let mut app = App::new(Changes::discover(&dir).unwrap());
        let mut terminal = Terminal::new(TestBackend::new(80, 10)).unwrap();
        terminal.draw(|f| render(f, &mut app)).unwrap();

        assert!(
            !bottom_row_has_scrollbar_glyphs(terminal.backend().buffer()),
            "expected no scrollbar when every row (including the collapsed archived header) fits"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn left_pane_scrollbar_shown_when_content_exceeds_the_proportional_cap() {
        use crate::changes::Changes;
        use ratatui::Terminal;
        use ratatui::backend::TestBackend;

        let long_name = format!("2026-01-01-{}", "a".repeat(80));
        let dir = temp_repo_with_changes(&["a"], &[&long_name]);
        let mut app = App::new(Changes::discover(&dir).unwrap());

        let rows = app.rows();
        let header_idx = rows
            .iter()
            .position(|r| matches!(r, Row::ArchivedHeader { .. }))
            .unwrap();
        app.list_state().select(Some(header_idx));
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        let mut terminal = Terminal::new(TestBackend::new(80, 10)).unwrap();
        terminal.draw(|f| render(f, &mut app)).unwrap();

        assert!(
            bottom_row_has_scrollbar_glyphs(terminal.backend().buffer()),
            "expected a scrollbar once the widest row exceeds the pane's proportional cap"
        );

        std::fs::remove_dir_all(&dir).ok();
    }
}
