use crossterm::event::{KeyCode, KeyEvent};
use ratatui::widgets::ListState;

use crate::changes::Changes;

use super::row::{self, Row};

pub struct App {
    changes: Changes,
    archived_expanded: bool,
    list_state: ListState,
}

impl App {
    pub fn new(changes: Changes) -> Self {
        App {
            changes,
            archived_expanded: false,
            list_state: ListState::default().with_selected(Some(0)),
        }
    }

    pub fn rows(&self) -> Vec<Row<'_>> {
        row::flatten(
            &self.changes.active,
            &self.changes.archived,
            self.archived_expanded,
        )
    }

    pub fn list_state(&mut self) -> &mut ListState {
        &mut self.list_state
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.move_selection(-1),
            KeyCode::Down | KeyCode::Char('j') => self.move_selection(1),
            KeyCode::Enter | KeyCode::Char(' ') => self.toggle_archived_at_cursor(),
            _ => {}
        }
    }

    fn move_selection(&mut self, delta: isize) {
        let rows = self.rows();
        let current = self.list_state.selected().unwrap_or(0);
        self.list_state
            .select(Some(next_selectable(&rows, current, delta)));
    }

    fn toggle_archived_at_cursor(&mut self) {
        let rows = self.rows();
        let Some(selected) = self.list_state.selected() else {
            return;
        };
        if !matches!(rows.get(selected), Some(Row::ArchivedHeader { .. })) {
            return;
        }

        self.archived_expanded = !self.archived_expanded;
        if !self.archived_expanded {
            let rows = self.rows();
            if let Some(idx) = archived_header_index(&rows) {
                self.list_state.select(Some(idx));
            }
        }
    }
}

/// Moves `current` by `delta` rows, skipping a single adjacent placeholder row.
/// Clamps at the ends of `rows` rather than wrapping.
fn next_selectable(rows: &[Row], current: usize, delta: isize) -> usize {
    let len = rows.len() as isize;
    if len == 0 {
        return current;
    }

    let mut new = current as isize + delta;
    if new < 0 || new >= len {
        return current;
    }
    if !rows[new as usize].is_selectable() {
        new += delta;
        if new < 0 || new >= len {
            return current;
        }
    }
    new as usize
}

fn archived_header_index(rows: &[Row]) -> Option<usize> {
    rows.iter()
        .position(|r| matches!(r, Row::ArchivedHeader { .. }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::changes::Change;

    #[test]
    fn skips_placeholder_moving_down() {
        let active: Vec<Change> = Vec::new();
        let archived = vec![Change("archive/2026-01-01-x".to_string())];
        // [Placeholder(no active), ArchivedHeader, ...] collapsed, so only 2 rows.
        let rows = row::flatten(&active, &archived, false);
        assert_eq!(next_selectable(&rows, 0, 1), 1);
    }

    #[test]
    fn skips_placeholder_moving_up() {
        let active = vec![Change("a".to_string())];
        let archived: Vec<Change> = Vec::new();
        // Expanded with no archived changes: [Active, ArchivedHeader, Placeholder(no archived)]
        let rows = row::flatten(&active, &archived, true);
        assert_eq!(next_selectable(&rows, 2, -1), 1);
    }

    #[test]
    fn clamps_at_start_and_end() {
        let active = vec![Change("a".to_string())];
        let archived: Vec<Change> = Vec::new();
        let rows = row::flatten(&active, &archived, false);
        assert_eq!(next_selectable(&rows, 0, -1), 0);
        let last = rows.len() - 1;
        assert_eq!(next_selectable(&rows, last, 1), last);
    }

    #[test]
    fn collapsing_snaps_selection_to_header_regardless_of_prior_selection() {
        let active = vec![Change("a".to_string())];
        let archived = vec![
            Change("archive/2026-01-01-x".to_string()),
            Change("archive/2026-01-02-y".to_string()),
        ];
        let rows_collapsed = row::flatten(&active, &archived, false);
        // Header sits right after active rows; unaffected by whichever archived
        // child was selected before collapsing.
        assert_eq!(archived_header_index(&rows_collapsed), Some(1));
    }
}
