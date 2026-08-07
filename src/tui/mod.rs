mod app;
mod row;

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, List, ListItem};

use crate::changes::Changes;

use app::App;
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
    render_right_pane(frame, right);
}

fn render_left_pane(frame: &mut Frame, area: Rect, app: &mut App) {
    let items: Vec<ListItem<'static>> = app.rows().iter().map(row_to_list_item).collect();

    let list = List::new(items)
        .block(Block::bordered().title("Changes"))
        .highlight_style(Modifier::REVERSED);

    frame.render_stateful_widget(list, area, app.list_state());
}

fn render_right_pane(frame: &mut Frame, area: Rect) {
    frame.render_widget(Block::bordered(), area);
}

fn row_to_list_item(row: &Row) -> ListItem<'static> {
    let indent = if is_indented(row) { "  " } else { "" };
    match row {
        Row::Active(change) => {
            ListItem::new(format!("{indent}{}", format_name(change.display_name())))
        }
        Row::ArchivedHeader { expanded } => {
            let marker = if *expanded { "▾" } else { "▸" };
            ListItem::new(format!("{marker} archived"))
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
            ListItem::new(Line::from(spans))
        }
        Row::Placeholder { text, .. } => ListItem::new(format!("{indent}{text}")),
    }
}

/// Formats a change's raw, hyphenated name for display.
fn format_name(name: &str) -> String {
    name.replace('-', " ")
}

fn is_indented(row: &Row) -> bool {
    matches!(row, Row::Archived(_)) || matches!(row, Row::Placeholder { indented: true, .. })
}
