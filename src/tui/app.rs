use crossterm::event::{KeyCode, KeyEvent};
use ratatui::widgets::ListState;

use crate::changes::Changes;

use super::row::{self, Row};

pub struct App {
    changes: Changes,
    archived_expanded: bool,
    list_state: ListState,
    h_scroll: usize,
    /// `max_scroll` as computed by the most recent render, cached so `$`/`End` can jump
    /// straight to it without `handle_key` needing to know the pane's rendered width.
    max_h_scroll: usize,
}

impl App {
    pub fn new(changes: Changes) -> Self {
        App {
            changes,
            archived_expanded: false,
            list_state: ListState::default().with_selected(Some(0)),
            h_scroll: 0,
            max_h_scroll: 0,
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

    pub fn h_scroll(&self) -> usize {
        self.h_scroll
    }

    /// Called by the left pane's renderer after computing the current `max_scroll`, so
    /// `$`/`End` can jump straight to it on the next keypress.
    pub fn set_max_h_scroll(&mut self, max_h_scroll: usize) {
        self.max_h_scroll = max_h_scroll;
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.move_selection(-1),
            KeyCode::Down | KeyCode::Char('j') => self.move_selection(1),
            KeyCode::Enter | KeyCode::Char(' ') => self.toggle_archived_at_cursor(),
            KeyCode::Left | KeyCode::Char('h') => {
                self.h_scroll = self.h_scroll.saturating_sub(1);
            }
            KeyCode::Right | KeyCode::Char('l') => {
                // Clamp against the cached max, not just saturating_add: otherwise repeated
                // presses past the visible edge accumulate an invisible overshoot that later
                // `h`/left presses have to silently unwind before any movement is visible.
                self.h_scroll = self.h_scroll.saturating_add(1).min(self.max_h_scroll);
            }
            KeyCode::Home | KeyCode::Char('^') => self.h_scroll = 0,
            KeyCode::End | KeyCode::Char('$') => self.h_scroll = self.max_h_scroll,
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
    use crossterm::event::KeyModifiers;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    /// A directory containing an empty `openspec/changes`, just enough for `Changes::discover`
    /// to succeed with no active/archived changes. Removed on drop.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let id = COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir()
                .join(format!("opsxexplorer-app-test-{}-{id}", std::process::id()));
            std::fs::create_dir_all(path.join("openspec/changes")).unwrap();
            TempDir(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn empty_app() -> App {
        let dir = TempDir::new();
        App::new(Changes::discover(dir.path()).unwrap())
    }

    #[test]
    fn l_and_right_scroll_right() {
        let mut app = empty_app();
        app.set_max_h_scroll(10);
        app.handle_key(key(KeyCode::Char('l')));
        assert_eq!(app.h_scroll(), 1);
        app.handle_key(key(KeyCode::Right));
        assert_eq!(app.h_scroll(), 2);
    }

    #[test]
    fn l_and_right_clamp_to_cached_max_scroll_without_overshoot() {
        let mut app = empty_app();
        app.set_max_h_scroll(2);
        for _ in 0..7 {
            app.handle_key(key(KeyCode::Char('l')));
        }
        assert_eq!(
            app.h_scroll(),
            2,
            "should not overshoot past the cached max"
        );

        // A single `h` press afterward must move immediately: no invisible overshoot to unwind.
        app.handle_key(key(KeyCode::Char('h')));
        assert_eq!(app.h_scroll(), 1);
    }

    #[test]
    fn h_and_left_scroll_left() {
        let mut app = empty_app();
        app.h_scroll = 2;
        app.handle_key(key(KeyCode::Char('h')));
        assert_eq!(app.h_scroll(), 1);
        app.handle_key(key(KeyCode::Left));
        assert_eq!(app.h_scroll(), 0);
    }

    #[test]
    fn h_and_left_saturate_at_zero() {
        let mut app = empty_app();
        app.handle_key(key(KeyCode::Char('h')));
        assert_eq!(app.h_scroll(), 0);
        app.handle_key(key(KeyCode::Left));
        assert_eq!(app.h_scroll(), 0);
    }

    #[test]
    fn caret_and_home_jump_to_zero() {
        let mut app = empty_app();
        app.h_scroll = 5;
        app.handle_key(key(KeyCode::Char('^')));
        assert_eq!(app.h_scroll(), 0);
        app.h_scroll = 5;
        app.handle_key(key(KeyCode::Home));
        assert_eq!(app.h_scroll(), 0);
    }

    #[test]
    fn dollar_and_end_jump_to_cached_max_scroll() {
        let mut app = empty_app();
        app.set_max_h_scroll(7);
        app.handle_key(key(KeyCode::Char('$')));
        assert_eq!(app.h_scroll(), 7);
        app.h_scroll = 0;
        app.handle_key(key(KeyCode::End));
        assert_eq!(app.h_scroll(), 7);
    }

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
