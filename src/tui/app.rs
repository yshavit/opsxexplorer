use std::collections::HashSet;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::widgets::ListState;

use crate::changes::{Change, Changes, ChangesError};
use crate::diff::{self, CapabilityDiff};
use crate::specs::{self, SpecError};

use super::diff_row::{self, DiffRow, RowKey};
use super::row::{self, Row};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Left,
    Right,
}

/// Which capability's diff, if any, `load_diff_state` produced for the
/// currently selected left-pane row. `Loaded` holds one entry per
/// capability, in the order `specs::capabilities` enumerated them, each
/// either the computed diff or the error that kept it from loading — one
/// bad capability costs its own tab, not the pane (see
/// `2026-08-08-tui-specdiff/design.md`).
enum DiffPaneState {
    /// The left-pane cursor is not on a change row.
    NotAChange,
    /// `Changes::resolve`/`views`/`specs::capabilities` failed for the whole change.
    PaneError(String),
    /// The change carries no delta specs at all.
    NoCapabilities,
    Loaded(Vec<(String, Result<CapabilityDiff, SpecError>)>),
}

/// A rendering-friendly summary of the right pane's current state, built
/// fresh from `DiffPaneState` on every call so it never drifts from it.
pub enum PaneView {
    NotAChange,
    PaneError(String),
    NoCapabilities,
    Tabs {
        names: Vec<String>,
        selected: usize,
        /// Set when the selected tab's capability failed to load; its
        /// sibling tabs are unaffected and still render normally.
        tab_error: Option<String>,
    },
}

pub struct App {
    changes: Changes,
    archived_expanded: bool,
    list_state: ListState,
    h_scroll: usize,
    /// `max_scroll` as computed by the most recent render, cached so `$`/`End` can jump
    /// straight to it without `handle_key` needing to know the pane's rendered width.
    max_h_scroll: usize,
    /// The left pane's visible row count as of the most recent render, cached so
    /// `Ctrl+d`/`Ctrl+u` can jump by half of it without `handle_key` needing to know
    /// the pane's rendered height.
    left_viewport_rows: usize,

    focus: Focus,
    diff_state: DiffPaneState,
    tab: usize,
    expanded: HashSet<RowKey>,
    cursor: usize,
    line_offset: usize,
    /// `max_line_offset` as computed by the most recent render, cached for the same
    /// reason as `max_h_scroll` (see `2026-08-08-tui-specdiff/design.md`).
    max_line_offset: usize,
    /// The right pane's visible row count as of the most recent render, mirroring
    /// `left_viewport_rows`.
    right_viewport_rows: usize,
    /// The right pane's inner content width as of the most recent render, so
    /// `diff_rows()` can decide (via `diff_row::flatten`) whether the purpose
    /// row fits without the caller needing to pass it through explicitly.
    /// Mirrors `right_viewport_rows`'s one-frame-lag pattern (see
    /// `2026-08-09-render-purpose/design.md`).
    right_pane_width: usize,

    help_open: bool,
    help_line_offset: usize,
    /// `max_line_offset` as computed by the most recent render, mirroring
    /// `max_h_scroll` (see `2026-08-09-keybinding-help/design.md`): the
    /// modal's content is fixed, but its available height isn't, so this
    /// still needs to come from the renderer.
    help_max_line_offset: usize,
    /// The help modal's visible row count as of the most recent render,
    /// mirroring `right_viewport_rows`.
    help_viewport_rows: usize,
}

impl App {
    pub fn new(changes: Changes) -> Self {
        let mut app = App {
            changes,
            archived_expanded: false,
            list_state: ListState::default().with_selected(Some(0)),
            h_scroll: 0,
            max_h_scroll: 0,
            left_viewport_rows: 0,

            focus: Focus::Left,
            diff_state: DiffPaneState::NotAChange,
            tab: 0,
            expanded: HashSet::new(),
            cursor: 0,
            line_offset: 0,
            max_line_offset: 0,
            right_viewport_rows: 0,
            right_pane_width: 0,

            help_open: false,
            help_line_offset: 0,
            help_max_line_offset: 0,
            help_viewport_rows: 0,
        };
        app.recompute_diff();
        app
    }

    pub fn rows(&self) -> Vec<Row<'_>> {
        row::flatten(
            &self.changes.active,
            &self.changes.archived,
            self.archived_expanded,
        )
    }

    /// Every row as if the archived section were expanded, regardless of
    /// `archived_expanded`'s actual state. Used solely for width/scroll-max
    /// computation, so those don't vary with whether the section happens to
    /// be collapsed (see
    /// `2026-08-09-tui-changelist-improvements/design.md`).
    pub fn all_rows(&self) -> Vec<Row<'_>> {
        row::flatten(&self.changes.active, &self.changes.archived, true)
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

    /// Called by the left pane's renderer after computing its inner height, so
    /// `Ctrl+d`/`Ctrl+u` can jump by half of it.
    pub fn set_left_viewport_rows(&mut self, rows: usize) {
        self.left_viewport_rows = rows;
    }

    pub fn focus(&self) -> Focus {
        self.focus
    }

    /// The flattened, navigable rows of the currently selected tab's diff.
    /// Empty when there is no successfully loaded diff to show (see `pane_view`
    /// for why: not a change, a pane-level error, no capabilities, or the
    /// selected tab itself failed to load).
    pub fn diff_rows(&self) -> Vec<DiffRow<'_>> {
        match self.current_capability_diff() {
            Some(diff) => diff_row::flatten(diff, &self.expanded, self.right_pane_width),
            None => Vec::new(),
        }
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn line_offset(&self) -> usize {
        self.line_offset
    }

    pub fn set_line_offset(&mut self, offset: usize) {
        self.line_offset = offset;
    }

    /// Called by the right pane's renderer after computing the current
    /// `max_line_offset`, mirroring `set_max_h_scroll`. Cached for parity
    /// with the left pane even though nothing currently reads it back; the
    /// field itself is what a future jump-to-end key would clamp against.
    pub fn set_max_line_offset(&mut self, max_line_offset: usize) {
        self.max_line_offset = max_line_offset;
    }

    /// Called by the right pane's renderer after computing its inner height, mirroring
    /// `set_left_viewport_rows`.
    pub fn set_right_viewport_rows(&mut self, rows: usize) {
        self.right_viewport_rows = rows;
    }

    /// Called by the right pane's renderer after computing its inner width,
    /// alongside `set_right_viewport_rows`.
    pub fn set_right_pane_width(&mut self, width: usize) {
        self.right_pane_width = width;
    }

    pub fn help_open(&self) -> bool {
        self.help_open
    }

    pub fn help_line_offset(&self) -> usize {
        self.help_line_offset
    }

    pub fn set_help_line_offset(&mut self, offset: usize) {
        self.help_line_offset = offset;
    }

    /// Called by the modal's renderer after computing the current `max_line_offset`,
    /// mirroring `set_max_h_scroll`.
    pub fn set_help_max_line_offset(&mut self, max_line_offset: usize) {
        self.help_max_line_offset = max_line_offset;
    }

    /// Called by the modal's renderer after computing its inner height, mirroring
    /// `set_right_viewport_rows`.
    pub fn set_help_viewport_rows(&mut self, rows: usize) {
        self.help_viewport_rows = rows;
    }

    pub fn pane_view(&self) -> PaneView {
        match &self.diff_state {
            DiffPaneState::NotAChange => PaneView::NotAChange,
            DiffPaneState::PaneError(msg) => PaneView::PaneError(msg.clone()),
            DiffPaneState::NoCapabilities => PaneView::NoCapabilities,
            DiffPaneState::Loaded(tabs) => PaneView::Tabs {
                names: tabs.iter().map(|(name, _)| name.clone()).collect(),
                selected: self.tab,
                tab_error: tabs
                    .get(self.tab)
                    .and_then(|(_, result)| result.as_ref().err())
                    .map(|e| e.to_string()),
            },
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        if key.code == KeyCode::Char('?') {
            self.toggle_help();
            return;
        }
        if self.help_open {
            self.handle_help_key(key);
            return;
        }
        if key.code == KeyCode::Tab {
            self.focus = match self.focus {
                Focus::Left => Focus::Right,
                Focus::Right => Focus::Left,
            };
            return;
        }
        match self.focus {
            Focus::Left => self.handle_left_key(key),
            Focus::Right => self.handle_right_key(key),
        }
    }

    /// Opens or closes the help modal, resetting scroll to the top on open — the
    /// modal has no state of its own to preserve, unlike the panes it overlays.
    fn toggle_help(&mut self) {
        self.help_open = !self.help_open;
        if self.help_open {
            self.help_line_offset = 0;
        }
    }

    /// Line-by-line scrolling, not selection-based: the modal has nothing
    /// selectable, so `j`/`k`/`Ctrl+d`/`Ctrl+u` move the viewport itself
    /// rather than a cursor, the same way `h`/`l` scroll the left pane. This
    /// also means the very first line is always reachable, unlike scrolling
    /// that's clamped to a selectable row's position.
    fn handle_help_key(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('d') => self.scroll_help(half_page(self.help_viewport_rows)),
                KeyCode::Char('u') => self.scroll_help(-half_page(self.help_viewport_rows)),
                _ => {}
            }
            return;
        }
        match key.code {
            KeyCode::Esc => self.help_open = false,
            KeyCode::Up | KeyCode::Char('k') => self.scroll_help(-1),
            KeyCode::Down | KeyCode::Char('j') => self.scroll_help(1),
            _ => {}
        }
    }

    fn scroll_help(&mut self, delta: isize) {
        let new =
            (self.help_line_offset as isize + delta).clamp(0, self.help_max_line_offset as isize);
        self.help_line_offset = new as usize;
    }

    fn handle_left_key(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('d') => self.move_selection(half_page(self.left_viewport_rows)),
                KeyCode::Char('u') => self.move_selection(-half_page(self.left_viewport_rows)),
                _ => {}
            }
            return;
        }
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.move_selection(-1),
            KeyCode::Down | KeyCode::Char('j') => self.move_selection(1),
            KeyCode::Enter | KeyCode::Char(' ') => self.activate_at_cursor(),
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

    fn handle_right_key(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('d') => self.move_cursor(half_page(self.right_viewport_rows)),
                KeyCode::Char('u') => self.move_cursor(-half_page(self.right_viewport_rows)),
                _ => {}
            }
            return;
        }
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.move_cursor(-1),
            KeyCode::Down | KeyCode::Char('j') => self.move_cursor(1),
            KeyCode::Enter | KeyCode::Char(' ') => self.toggle_cursor_row(),
            KeyCode::Right | KeyCode::Char('l') => self.set_cursor_row_expanded(true),
            KeyCode::Left | KeyCode::Char('h') => self.set_cursor_row_expanded(false),
            KeyCode::Char(']') => self.move_tab(1),
            KeyCode::Char('[') => self.move_tab(-1),
            _ => {}
        }
    }

    fn move_selection(&mut self, delta: isize) {
        let rows = self.rows();
        let current = self.list_state.selected().unwrap_or(0);
        self.list_state
            .select(Some(next_selectable(&rows, current, delta)));
        self.recompute_diff();
    }

    /// Enter/Space on the cursor's row: toggles the archived section when the cursor is
    /// on its header (unchanged), or moves focus to the right pane when the cursor is on
    /// a change row, since there's now something for the right pane to show.
    fn activate_at_cursor(&mut self) {
        let rows = self.rows();
        let Some(selected) = self.list_state.selected() else {
            return;
        };
        match rows.get(selected) {
            Some(Row::ArchivedHeader { .. }) => self.toggle_archived_at_cursor(),
            Some(Row::Active(_) | Row::Archived(_)) => self.focus = Focus::Right,
            _ => {}
        }
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
        self.recompute_diff();
    }

    fn move_cursor(&mut self, delta: isize) {
        let rows = self.diff_rows();
        self.cursor = next_selectable_row(&rows, self.cursor, delta);
    }

    fn toggle_cursor_row(&mut self) {
        let Some((key, expanded)) = self.cursor_row_key_and_state() else {
            return;
        };
        self.set_expanded(key, !expanded);
    }

    fn set_cursor_row_expanded(&mut self, expand: bool) {
        let Some((key, _)) = self.cursor_row_key_and_state() else {
            return;
        };
        self.set_expanded(key, expand);
    }

    fn cursor_row_key_and_state(&self) -> Option<(RowKey, bool)> {
        let rows = self.diff_rows();
        let row = rows.get(self.cursor)?;
        Some((row.key()?.clone(), row.expanded().unwrap_or(false)))
    }

    fn set_expanded(&mut self, key: RowKey, expand: bool) {
        if expand {
            self.expanded.insert(key);
        } else {
            self.expanded.remove(&key);
        }
    }

    fn move_tab(&mut self, delta: isize) {
        let DiffPaneState::Loaded(tabs) = &self.diff_state else {
            return;
        };
        let len = tabs.len() as isize;
        if len == 0 {
            return;
        }
        let new = (self.tab as isize + delta).clamp(0, len - 1) as usize;
        if new != self.tab {
            self.tab = new;
            self.reset_cursor_to_first_selectable();
        }
    }

    /// Recomputes the right pane's diff state for whatever the left pane's
    /// cursor is currently on. Called whenever the left-pane selection
    /// moves — never per frame, since this does real I/O (see
    /// `2026-08-08-tui-specdiff/design.md`).
    fn recompute_diff(&mut self) {
        let change = {
            let rows = self.rows();
            self.list_state
                .selected()
                .and_then(|i| rows.get(i))
                .and_then(row_change)
                .cloned()
        };

        self.tab = 0;
        self.expanded.clear();

        self.diff_state = match &change {
            None => DiffPaneState::NotAChange,
            Some(change) => match load_diff_state(&self.changes, change) {
                Ok(state) => state,
                Err(e) => DiffPaneState::PaneError(e.to_string()),
            },
        };

        self.reset_cursor_to_first_selectable();
    }

    fn current_capability_diff(&self) -> Option<&CapabilityDiff> {
        match &self.diff_state {
            DiffPaneState::Loaded(tabs) => tabs.get(self.tab).and_then(|(_, r)| r.as_ref().ok()),
            _ => None,
        }
    }

    fn reset_cursor_to_first_selectable(&mut self) {
        let rows = self.diff_rows();
        self.cursor = rows.iter().position(|r| r.is_selectable()).unwrap_or(0);
        self.line_offset = 0;
    }
}

/// Loads every capability the change touches and diffs each one, isolating
/// failures per capability so one bad load doesn't cost its siblings. Only
/// the pane-wide steps (resolving the change, opening its views, enumerating
/// its capabilities) short-circuit the whole change (see
/// `2026-08-08-tui-specdiff/design.md`).
fn load_diff_state(changes: &Changes, change: &Change) -> Result<DiffPaneState, ChangesError> {
    let view = changes.resolve(change)?;
    let views = changes.views(&view)?;
    let names = specs::capabilities(&views)?;
    if names.is_empty() {
        return Ok(DiffPaneState::NoCapabilities);
    }
    let tabs = names
        .into_iter()
        .map(|name| {
            let result = specs::load(&views, &name).map(|pair| diff::diff(&name, &pair));
            (name, result)
        })
        .collect();
    Ok(DiffPaneState::Loaded(tabs))
}

fn row_change<'a>(row: &Row<'a>) -> Option<&'a Change> {
    match row {
        Row::Active(c) | Row::Archived(c) => Some(c),
        _ => None,
    }
}

/// Half of a pane's visible row count, as a movement delta for `Ctrl+d`/`Ctrl+u`.
/// Row-count-based, not line-count-based, so it reuses the same clamped movement
/// path as single-step `j`/`k` (see
/// `2026-08-08-tui-keybinding-improvements/design.md`).
fn half_page(viewport_rows: usize) -> isize {
    (viewport_rows / 2) as isize
}

/// Moves `current` by up to `delta.abs()` selectable rows in `delta`'s direction,
/// skipping placeholder rows along the way. Walks one row at a time (rather than
/// jumping straight by `delta`) so a jump larger than 1 — as `Ctrl+d`/`Ctrl+u` use —
/// lands on the furthest reachable selectable row instead of overshooting past the
/// end and snapping back to `current`. Clamps at the ends of `rows` rather than
/// wrapping.
fn next_selectable(rows: &[Row], current: usize, delta: isize) -> usize {
    if rows.is_empty() {
        return current;
    }
    let len = rows.len() as isize;
    let step = if delta >= 0 { 1 } else { -1 };
    let mut idx = current as isize;
    let mut last_selectable = current as isize;
    let mut remaining = delta.abs();
    while remaining > 0 {
        let next = idx + step;
        if next < 0 || next >= len {
            break;
        }
        idx = next;
        if rows[idx as usize].is_selectable() {
            last_selectable = idx;
            remaining -= 1;
        }
    }
    last_selectable as usize
}

/// Moves `current` by up to `delta.abs()` selectable rows in `delta`'s direction,
/// skipping any run of non-selectable rows (group headings, intro/body content,
/// notices) along the way. Mirrors `next_selectable`'s one-row-at-a-time walk, for
/// the same reason: a jump larger than 1 must land on the furthest reachable
/// selectable row, not overshoot and snap back to `current`. Clamps at the ends
/// rather than wrapping.
fn next_selectable_row(rows: &[DiffRow], current: usize, delta: isize) -> usize {
    if rows.is_empty() {
        return current;
    }
    let len = rows.len() as isize;
    let step = if delta >= 0 { 1 } else { -1 };
    let mut idx = current as isize;
    let mut last_selectable = current as isize;
    let mut remaining = delta.abs();
    while remaining > 0 {
        let next = idx + step;
        if next < 0 || next >= len {
            break;
        }
        idx = next;
        if rows[idx as usize].is_selectable() {
            last_selectable = idx;
            remaining -= 1;
        }
    }
    last_selectable as usize
}

fn archived_header_index(rows: &[Row]) -> Option<usize> {
    rows.iter()
        .position(|r| matches!(r, Row::ArchivedHeader { .. }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::changes::Change;
    use crate::diff::{DiffError, Operation, Piece, RequirementDiff, ScenarioDiff};
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
    fn all_rows_is_unaffected_by_archived_expanded_toggle() {
        let mut app = empty_app();
        app.changes.archived = vec![
            Change("archive/2026-01-01-x".to_string()),
            Change("archive/2026-01-02-y".to_string()),
        ];
        assert!(!app.archived_expanded);
        let collapsed: Vec<String> = app.all_rows().iter().map(|r| format!("{r:?}")).collect();

        app.archived_expanded = true;
        let expanded: Vec<String> = app.all_rows().iter().map(|r| format!("{r:?}")).collect();

        assert_eq!(collapsed, expanded);
        // Sanity check: `all_rows` actually includes the archived changes,
        // not just an accident of both states being empty.
        assert!(collapsed.iter().any(|s| s.starts_with("Archived(")));
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

    // --- right pane: focus routing and key handling ---

    fn unchanged(text: &str) -> Piece {
        Piece::Unchanged {
            text: text.to_string(),
        }
    }

    fn sample_diff(capability: &str) -> CapabilityDiff {
        CapabilityDiff {
            capability: capability.to_string(),
            requirements: vec![RequirementDiff {
                name: "Req".to_string(),
                op: Operation::Added,
                intro: unchanged("intro"),
                scenarios: vec![ScenarioDiff {
                    name: "Scenario".to_string(),
                    body: unchanged("body"),
                }],
            }],
            errors: vec![],
            purpose: None,
        }
    }

    /// Builds an app whose right pane already holds the given tabs, bypassing
    /// `recompute_diff`'s filesystem/git access so key-handling tests stay fast
    /// and self-contained.
    fn app_with_loaded(tabs: Vec<(&str, CapabilityDiff)>) -> App {
        app_with_loaded_results(
            tabs.into_iter()
                .map(|(name, diff)| (name, Ok(diff)))
                .collect(),
        )
    }

    /// As `app_with_loaded`, but allows individual tabs to be pre-failed, so
    /// key-handling and rendering-state tests can exercise a mix of sound and
    /// broken capabilities without touching the filesystem.
    fn app_with_loaded_results(tabs: Vec<(&str, Result<CapabilityDiff, SpecError>)>) -> App {
        let mut app = empty_app();
        app.diff_state = DiffPaneState::Loaded(
            tabs.into_iter()
                .map(|(name, result)| (name.to_string(), result))
                .collect(),
        );
        app.tab = 0;
        app.reset_cursor_to_first_selectable();
        app
    }

    #[test]
    fn moving_left_pane_selection_recomputes_diff_from_a_real_change() {
        use git2::{Repository, Signature};

        let dir = TempDir::new();
        let repo = Repository::init(dir.path()).unwrap();
        let write = |rel: &str, content: &str| {
            let full = dir.path().join(rel);
            std::fs::create_dir_all(full.parent().unwrap()).unwrap();
            std::fs::write(full, content).unwrap();
        };
        write(
            "openspec/changes/add-thing/specs/cap/spec.md",
            "## ADDED Requirements\n\n### Requirement: New\n#### Scenario: it works\n- **WHEN** a\n- **THEN** b\n",
        );
        let mut index = repo.index().unwrap();
        index
            .add_path(Path::new("openspec/changes/add-thing/specs/cap/spec.md"))
            .unwrap();
        index.write().unwrap();
        let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
        let sig = Signature::now("Test", "test@example.com").unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
            .unwrap();

        let app = App::new(Changes::discover(dir.path()).unwrap());
        // `App::new` already recomputes for the initial selection (index 0),
        // which is the one active change we just wrote.
        match app.pane_view() {
            PaneView::Tabs { names, .. } => assert_eq!(names, vec!["cap".to_string()]),
            _ => panic!("expected a loaded tab for the active change"),
        }
        let rows = app.diff_rows();
        assert!(
            rows.iter()
                .any(|r| matches!(r, DiffRow::Requirement { name: "New", .. }))
        );
    }

    #[test]
    fn a_change_with_no_delta_specs_yields_no_capabilities_and_no_tabs() {
        let dir = TempDir::new();
        let repo = git2::Repository::init(dir.path()).unwrap();
        let full = dir
            .path()
            .join("openspec/changes/proposal-only/proposal.md");
        std::fs::create_dir_all(full.parent().unwrap()).unwrap();
        std::fs::write(&full, "x").unwrap();
        let mut index = repo.index().unwrap();
        index
            .add_path(Path::new("openspec/changes/proposal-only/proposal.md"))
            .unwrap();
        index.write().unwrap();
        let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
        let sig = git2::Signature::now("Test", "test@example.com").unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
            .unwrap();

        let app = App::new(Changes::discover(dir.path()).unwrap());
        assert!(matches!(app.pane_view(), PaneView::NoCapabilities));
        assert!(app.diff_rows().is_empty());
    }

    #[test]
    fn one_capability_failing_to_load_leaves_its_siblings_rendering_normally() {
        let mut app = app_with_loaded_results(vec![
            (
                "bad",
                Err(SpecError::MissingSpecDocument {
                    capability: "bad".to_string(),
                }),
            ),
            ("good", Ok(sample_diff("good"))),
        ]);

        // The first (selected) tab reports its own error and no rows.
        match app.pane_view() {
            PaneView::Tabs {
                names, tab_error, ..
            } => {
                assert_eq!(names, vec!["bad".to_string(), "good".to_string()]);
                assert!(tab_error.is_some());
            }
            _ => panic!("expected Tabs, got a pane view for a different state"),
        }
        assert!(app.diff_rows().is_empty());

        // Its sibling tab is unaffected and renders its diff normally.
        app.focus = Focus::Right;
        app.handle_key(key(KeyCode::Char(']')));
        match app.pane_view() {
            PaneView::Tabs { tab_error, .. } => assert!(tab_error.is_none()),
            _ => panic!("expected Tabs, got a pane view for a different state"),
        }
        assert!(
            app.diff_rows()
                .iter()
                .any(|r| matches!(r, DiffRow::Requirement { .. }))
        );
    }

    #[test]
    fn tab_round_trips_focus() {
        let mut app = empty_app();
        assert_eq!(app.focus(), Focus::Left);
        app.handle_key(key(KeyCode::Tab));
        assert_eq!(app.focus(), Focus::Right);
        app.handle_key(key(KeyCode::Tab));
        assert_eq!(app.focus(), Focus::Left);
    }

    #[test]
    fn right_pane_keys_are_inert_while_left_pane_holds_focus() {
        let mut app = app_with_loaded(vec![("cap", sample_diff("cap"))]);
        assert_eq!(app.focus(), Focus::Left);
        let cursor_before = app.cursor;
        app.handle_key(key(KeyCode::Char('l'))); // expand, if it were routed to the right pane
        assert_eq!(
            app.cursor, cursor_before,
            "right pane cursor must not move while the left pane is focused"
        );
        assert!(
            app.expanded.is_empty(),
            "right pane collapse state must not change while the left pane is focused"
        );
    }

    #[test]
    fn left_pane_keys_are_inert_while_right_pane_holds_focus() {
        let mut app = app_with_loaded(vec![("cap", sample_diff("cap"))]);
        app.focus = Focus::Right;
        let left_selection_before = app.list_state.selected();
        app.handle_key(key(KeyCode::Char('j')));
        assert_eq!(
            app.list_state.selected(),
            left_selection_before,
            "left pane selection must not move while the right pane is focused"
        );
    }

    #[test]
    fn cursor_skips_group_headings_bodies_and_notices_but_stops_on_the_intro_row() {
        let mut diff = sample_diff("cap");
        diff.errors.push(DiffError::MissingBaseRequirement {
            capability: "cap".to_string(),
            requirement: "Ghost".to_string(),
        });
        let mut app = app_with_loaded(vec![("cap", diff)]);
        app.focus = Focus::Right;

        // Expand the requirement and its scenario so the intro and body rows exist too.
        app.expanded.insert(RowKey::Requirement {
            capability: "cap".to_string(),
            requirement: "Req".to_string(),
        });
        app.expanded.insert(RowKey::Scenario {
            capability: "cap".to_string(),
            requirement: "Req".to_string(),
            scenario: "Scenario".to_string(),
        });
        app.reset_cursor_to_first_selectable();

        let rows = app.diff_rows();
        // Sanity check: the tree actually contains every non-selectable kind under test.
        assert!(rows.iter().any(|r| matches!(r, DiffRow::Notice(_))));
        assert!(rows.iter().any(|r| matches!(r, DiffRow::GroupHeading(_))));
        assert!(rows.iter().any(|r| matches!(r, DiffRow::Body { .. })));

        // Cursor starts on the (only) requirement row.
        assert!(matches!(rows[app.cursor], DiffRow::Requirement { .. }));

        // Moving down lands on the intro row: it is selectable, per the same
        // exception the purpose row already has (see
        // `2026-08-09-unified-intro-collapsing/design.md`).
        app.handle_key(key(KeyCode::Char('j')));
        let rows = app.diff_rows();
        assert!(matches!(
            rows[app.cursor],
            DiffRow::Paragraph { .. } | DiffRow::ParagraphFull { .. }
        ));

        // Moving down again lands on the Scenario row.
        app.handle_key(key(KeyCode::Char('j')));
        let rows = app.diff_rows();
        assert!(matches!(rows[app.cursor], DiffRow::Scenario { .. }));

        // Moving down again has nowhere selectable to go (only Body follows): clamped.
        let cursor_before = app.cursor;
        app.handle_key(key(KeyCode::Char('j')));
        assert_eq!(app.cursor, cursor_before);

        // Moving up from the Scenario row returns to the intro row, then the
        // Requirement row.
        app.handle_key(key(KeyCode::Char('k')));
        let rows = app.diff_rows();
        assert!(matches!(
            rows[app.cursor],
            DiffRow::Paragraph { .. } | DiffRow::ParagraphFull { .. }
        ));

        app.handle_key(key(KeyCode::Char('k')));
        let rows = app.diff_rows();
        assert!(matches!(rows[app.cursor], DiffRow::Requirement { .. }));

        // Moving up again has nowhere selectable to go (only the heading/notice precede it).
        let cursor_before = app.cursor;
        app.handle_key(key(KeyCode::Char('k')));
        assert_eq!(app.cursor, cursor_before);
    }

    #[test]
    fn tab_selection_stops_at_both_ends() {
        let mut app = app_with_loaded(vec![
            ("alpha", sample_diff("alpha")),
            ("beta", sample_diff("beta")),
        ]);
        app.focus = Focus::Right;

        app.handle_key(key(KeyCode::Char('[')));
        assert_eq!(app.tab, 0, "already at the first tab");

        app.handle_key(key(KeyCode::Char(']')));
        assert_eq!(app.tab, 1);

        app.handle_key(key(KeyCode::Char(']')));
        assert_eq!(app.tab, 1, "already at the last tab");

        app.handle_key(key(KeyCode::Char('[')));
        assert_eq!(app.tab, 0);
    }

    #[test]
    fn tab_selection_is_a_no_op_for_a_single_capability() {
        let mut app = app_with_loaded(vec![("only", sample_diff("only"))]);
        app.focus = Focus::Right;

        app.handle_key(key(KeyCode::Char(']')));
        assert_eq!(app.tab, 0);
        app.handle_key(key(KeyCode::Char('[')));
        assert_eq!(app.tab, 0);
    }

    #[test]
    fn toggle_and_directional_expand_collapse_change_row_state() {
        let mut app = app_with_loaded(vec![("cap", sample_diff("cap"))]);
        app.focus = Focus::Right;

        assert!(!matches!(
            app.diff_rows()[app.cursor].expanded(),
            Some(true)
        ));

        app.handle_key(key(KeyCode::Char('l')));
        assert_eq!(app.diff_rows()[app.cursor].expanded(), Some(true));

        app.handle_key(key(KeyCode::Char('h')));
        assert_eq!(app.diff_rows()[app.cursor].expanded(), Some(false));

        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.diff_rows()[app.cursor].expanded(), Some(true));
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.diff_rows()[app.cursor].expanded(), Some(false));
    }

    // --- purpose row wiring (render-purpose 6.3-6.6) ---

    fn diff_with_purpose(capability: &str, piece: Piece) -> CapabilityDiff {
        let mut diff = sample_diff(capability);
        diff.purpose = Some(piece);
        diff
    }

    #[test]
    fn toggle_keys_on_a_purpose_full_row_are_inert() {
        let diff = diff_with_purpose(
            "cap",
            Piece::Added {
                delta: "short".to_string(),
            },
        );
        let mut app = app_with_loaded(vec![("cap", diff)]);
        app.focus = Focus::Right;
        app.set_right_pane_width(200); // wide enough that "short" fits.
        app.reset_cursor_to_first_selectable();

        let rows = app.diff_rows();
        assert!(
            matches!(rows[app.cursor], DiffRow::ParagraphFull { .. }),
            "expected cursor to start on the ParagraphFull row"
        );

        let cursor_before = app.cursor;
        let expanded_before = app.expanded.clone();

        for code in [
            KeyCode::Enter,
            KeyCode::Char(' '),
            KeyCode::Char('l'),
            KeyCode::Char('h'),
        ] {
            app.handle_key(key(code));
        }

        assert_eq!(app.cursor, cursor_before);
        assert_eq!(app.expanded, expanded_before);
    }

    #[test]
    fn toggle_keys_expand_and_collapse_a_non_fitting_purpose_row() {
        let text = "abcdefghijklmnopqrstuvwxyz".repeat(3);
        let diff = diff_with_purpose("cap", Piece::Added { delta: text });
        let mut app = app_with_loaded(vec![("cap", diff)]);
        app.focus = Focus::Right;
        app.set_right_pane_width(10); // narrow enough that the text doesn't fit.
        app.reset_cursor_to_first_selectable();

        assert!(matches!(
            app.diff_rows()[app.cursor],
            DiffRow::Paragraph { .. }
        ));
        assert_eq!(app.diff_rows()[app.cursor].expanded(), Some(false));

        app.handle_key(key(KeyCode::Char('l')));
        assert_eq!(app.diff_rows()[app.cursor].expanded(), Some(true));

        app.handle_key(key(KeyCode::Char('h')));
        assert_eq!(app.diff_rows()[app.cursor].expanded(), Some(false));

        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.diff_rows()[app.cursor].expanded(), Some(true));
    }

    #[test]
    fn purpose_notice_and_requirements_render_in_order_and_cursor_reaches_the_purpose_row() {
        let mut diff = sample_diff("cap");
        diff.errors.push(DiffError::MissingBaseRequirement {
            capability: "cap".to_string(),
            requirement: "Ghost".to_string(),
        });
        diff.purpose = Some(Piece::Added {
            delta: "short purpose".to_string(),
        });
        let mut app = app_with_loaded(vec![("cap", diff)]);
        app.focus = Focus::Right;
        app.set_right_pane_width(200);
        app.reset_cursor_to_first_selectable();

        let rows = app.diff_rows();
        assert!(matches!(rows[0], DiffRow::Notice(_)));
        assert!(matches!(rows[1], DiffRow::PurposeHeading(_)));
        assert!(matches!(rows[2], DiffRow::ParagraphFull { .. }));
        assert!(matches!(rows[3], DiffRow::GroupHeading(Operation::Added)));

        // The cursor starts on the first selectable row, skipping both the
        // notice and the purpose heading, landing directly on the purpose row.
        assert!(matches!(rows[app.cursor], DiffRow::ParagraphFull { .. }));

        app.handle_key(key(KeyCode::Char('j')));
        let rows = app.diff_rows();
        assert!(matches!(rows[app.cursor], DiffRow::Requirement { .. }));
    }

    #[test]
    fn cursor_reaches_a_non_fitting_purpose_row_too() {
        let text = "abcdefghijklmnopqrstuvwxyz".repeat(3);
        let diff = diff_with_purpose("cap", Piece::Added { delta: text });
        let mut app = app_with_loaded(vec![("cap", diff)]);
        app.focus = Focus::Right;
        app.set_right_pane_width(10);
        app.reset_cursor_to_first_selectable();

        assert!(matches!(
            app.diff_rows()[app.cursor],
            DiffRow::Paragraph { .. }
        ));
    }

    #[test]
    fn resizing_the_right_pane_across_the_fits_boundary_changes_purpose_row_kind() {
        let text = "abcdefghijklmnopqrstuvwxyz".repeat(3);
        let diff = diff_with_purpose("cap", Piece::Added { delta: text });
        let mut app = app_with_loaded(vec![("cap", diff)]);
        app.focus = Focus::Right;

        app.set_right_pane_width(10); // narrow: doesn't fit.
        app.reset_cursor_to_first_selectable();
        let cursor = app.cursor;
        assert!(matches!(app.diff_rows()[cursor], DiffRow::Paragraph { .. }));

        app.set_right_pane_width(200); // wide: fits.
        assert!(matches!(
            app.diff_rows()[cursor],
            DiffRow::ParagraphFull { .. }
        ));
    }

    // --- intro row wiring (unified-intro-collapsing) ---

    fn diff_with_intro(capability: &str, intro: Piece) -> CapabilityDiff {
        let mut diff = sample_diff(capability);
        diff.requirements[0].intro = intro;
        diff
    }

    fn expand_only_requirement(app: &mut App, capability: &str) {
        app.expanded.insert(RowKey::Requirement {
            capability: capability.to_string(),
            requirement: "Req".to_string(),
        });
    }

    #[test]
    fn toggle_keys_on_a_fitting_intro_row_are_inert() {
        let diff = diff_with_intro(
            "cap",
            Piece::Added {
                delta: "short".to_string(),
            },
        );
        let mut app = app_with_loaded(vec![("cap", diff)]);
        app.focus = Focus::Right;
        app.set_right_pane_width(200); // wide enough that "short" fits.
        expand_only_requirement(&mut app, "cap");
        app.reset_cursor_to_first_selectable();
        app.handle_key(key(KeyCode::Char('j'))); // move from Requirement onto the intro row.

        let rows = app.diff_rows();
        assert!(
            matches!(rows[app.cursor], DiffRow::ParagraphFull { indent: 1, .. }),
            "expected cursor to stop on the intro's ParagraphFull row"
        );

        let cursor_before = app.cursor;
        let expanded_before = app.expanded.clone();

        for code in [
            KeyCode::Enter,
            KeyCode::Char(' '),
            KeyCode::Char('l'),
            KeyCode::Char('h'),
        ] {
            app.handle_key(key(code));
        }

        assert_eq!(app.cursor, cursor_before);
        assert_eq!(app.expanded, expanded_before);
    }

    #[test]
    fn cursor_stops_on_a_non_fitting_intro_row_too() {
        let text = "abcdefghijklmnopqrstuvwxyz".repeat(3);
        let diff = diff_with_intro("cap", Piece::Added { delta: text });
        let mut app = app_with_loaded(vec![("cap", diff)]);
        app.focus = Focus::Right;
        app.set_right_pane_width(10); // narrow enough that the text doesn't fit.
        expand_only_requirement(&mut app, "cap");
        app.reset_cursor_to_first_selectable();
        app.handle_key(key(KeyCode::Char('j'))); // move from Requirement onto the intro row.

        assert!(matches!(
            app.diff_rows()[app.cursor],
            DiffRow::Paragraph { indent: 1, .. }
        ));
        assert_eq!(app.diff_rows()[app.cursor].expanded(), Some(false));

        app.handle_key(key(KeyCode::Char('l')));
        assert_eq!(app.diff_rows()[app.cursor].expanded(), Some(true));

        app.handle_key(key(KeyCode::Char('h')));
        assert_eq!(app.diff_rows()[app.cursor].expanded(), Some(false));
    }

    // --- left pane: Enter/Space focus-shift ---

    fn app_with_active_changes(names: &[&str]) -> App {
        let mut app = empty_app();
        app.changes.active = names.iter().map(|n| Change(n.to_string())).collect();
        app
    }

    #[test]
    fn enter_on_an_active_change_row_moves_focus_right() {
        let mut app = app_with_active_changes(&["a", "b"]);
        app.list_state.select(Some(0));
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.focus(), Focus::Right);
    }

    #[test]
    fn space_on_an_archived_change_row_moves_focus_right() {
        let mut app = empty_app();
        app.changes.archived = vec![Change("archive/2026-01-01-x".to_string())];
        app.archived_expanded = true;
        let rows = app.rows();
        let idx = rows
            .iter()
            .position(|r| matches!(r, Row::Archived(_)))
            .unwrap();
        app.list_state.select(Some(idx));
        app.handle_key(key(KeyCode::Char(' ')));
        assert_eq!(app.focus(), Focus::Right);
    }

    #[test]
    fn enter_on_the_archived_header_does_not_move_focus_but_still_toggles() {
        let mut app = empty_app();
        let rows = app.rows();
        let idx = rows
            .iter()
            .position(|r| matches!(r, Row::ArchivedHeader { .. }))
            .unwrap();
        app.list_state.select(Some(idx));
        assert!(!app.archived_expanded);

        app.handle_key(key(KeyCode::Enter));

        assert_eq!(app.focus(), Focus::Left);
        assert!(app.archived_expanded, "header should still toggle");
    }

    // --- half-page movement (Ctrl+u / Ctrl+d) ---

    fn ctrl_key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::CONTROL)
    }

    #[test]
    fn ctrl_d_moves_left_pane_selection_down_by_half_the_viewport() {
        let names: Vec<String> = (0..20).map(|i| format!("change-{i}")).collect();
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        let mut app = app_with_active_changes(&refs);
        app.list_state.select(Some(0));
        app.set_left_viewport_rows(10);

        app.handle_key(ctrl_key(KeyCode::Char('d')));

        assert_eq!(app.list_state.selected(), Some(5));
    }

    #[test]
    fn ctrl_u_moves_left_pane_selection_up_by_half_the_viewport() {
        let names: Vec<String> = (0..20).map(|i| format!("change-{i}")).collect();
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        let mut app = app_with_active_changes(&refs);
        app.list_state.select(Some(15));
        app.set_left_viewport_rows(10);

        app.handle_key(ctrl_key(KeyCode::Char('u')));

        assert_eq!(app.list_state.selected(), Some(10));
    }

    #[test]
    fn ctrl_d_clamps_at_the_last_selectable_row_in_the_left_pane() {
        let names: Vec<String> = (0..5).map(|i| format!("change-{i}")).collect();
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        let mut app = app_with_active_changes(&refs);
        app.list_state.select(Some(0));
        // Half-page (10) far exceeds the 6 selectable rows (5 active + header).
        app.set_left_viewport_rows(20);

        app.handle_key(ctrl_key(KeyCode::Char('d')));

        let last = app.rows().len() - 1;
        assert_eq!(app.list_state.selected(), Some(last));
    }

    #[test]
    fn ctrl_d_moves_right_pane_cursor_down_by_half_the_viewport() {
        let mut diff = sample_diff("cap");
        diff.requirements = (0..20)
            .map(|i| RequirementDiff {
                name: format!("Req{i}"),
                op: Operation::Added,
                intro: unchanged("intro"),
                scenarios: vec![],
            })
            .collect();
        let mut app = app_with_loaded(vec![("cap", diff)]);
        app.focus = Focus::Right;
        let start = app.cursor; // first selectable row (past the group heading)
        app.set_right_viewport_rows(10);

        app.handle_key(ctrl_key(KeyCode::Char('d')));

        assert_eq!(app.cursor, start + 5);
    }

    #[test]
    fn ctrl_u_moves_right_pane_cursor_up_by_half_the_viewport() {
        let mut diff = sample_diff("cap");
        diff.requirements = (0..20)
            .map(|i| RequirementDiff {
                name: format!("Req{i}"),
                op: Operation::Added,
                intro: unchanged("intro"),
                scenarios: vec![],
            })
            .collect();
        let mut app = app_with_loaded(vec![("cap", diff)]);
        app.focus = Focus::Right;
        app.cursor = app.diff_rows().len() - 1; // last row (Req19), selectable
        app.set_right_viewport_rows(10);

        app.handle_key(ctrl_key(KeyCode::Char('u')));

        assert_eq!(app.cursor, app.diff_rows().len() - 1 - 5);
    }

    #[test]
    fn ctrl_d_clamps_at_the_last_selectable_row_in_the_right_pane() {
        let mut diff = sample_diff("cap");
        diff.requirements = (0..3)
            .map(|i| RequirementDiff {
                name: format!("Req{i}"),
                op: Operation::Added,
                intro: unchanged("intro"),
                scenarios: vec![],
            })
            .collect();
        let mut app = app_with_loaded(vec![("cap", diff)]);
        app.focus = Focus::Right;
        // Half-page (10) far exceeds the 3 selectable requirement rows.
        app.set_right_viewport_rows(20);

        app.handle_key(ctrl_key(KeyCode::Char('d')));

        let last = app.diff_rows().len() - 1;
        assert_eq!(app.cursor, last);
    }

    #[test]
    fn ctrl_d_does_not_move_left_pane_selection_while_right_pane_holds_focus() {
        let mut app = app_with_active_changes(&["a", "b", "c"]);
        app.list_state.select(Some(0));
        app.set_left_viewport_rows(10);
        app.focus = Focus::Right;

        app.handle_key(ctrl_key(KeyCode::Char('d')));

        assert_eq!(
            app.list_state.selected(),
            Some(0),
            "left pane selection must not move while the right pane is focused"
        );
    }

    // --- help modal ---

    #[test]
    fn question_mark_opens_and_closes_the_help_modal() {
        let mut app = empty_app();
        assert!(!app.help_open());
        app.handle_key(key(KeyCode::Char('?')));
        assert!(app.help_open());
        app.handle_key(key(KeyCode::Char('?')));
        assert!(!app.help_open());
    }

    #[test]
    fn esc_also_closes_the_help_modal() {
        let mut app = empty_app();
        app.handle_key(key(KeyCode::Char('?')));
        assert!(app.help_open());
        app.handle_key(key(KeyCode::Esc));
        assert!(!app.help_open());
    }

    #[test]
    fn opening_the_modal_resets_scroll_to_the_top() {
        let mut app = empty_app();
        app.handle_key(key(KeyCode::Char('?')));
        app.set_help_max_line_offset(20);
        app.handle_key(key(KeyCode::Char('j')));
        app.handle_key(key(KeyCode::Char('j')));
        assert_eq!(app.help_line_offset(), 2);

        app.handle_key(key(KeyCode::Char('?'))); // close
        app.handle_key(key(KeyCode::Char('?'))); // reopen
        assert_eq!(app.help_line_offset(), 0);
    }

    #[test]
    fn j_and_k_scroll_the_help_modal_by_one_line() {
        let mut app = empty_app();
        app.handle_key(key(KeyCode::Char('?')));
        app.set_help_max_line_offset(20);

        app.handle_key(key(KeyCode::Char('j')));
        assert_eq!(app.help_line_offset(), 1);
        app.handle_key(key(KeyCode::Down));
        assert_eq!(app.help_line_offset(), 2);

        app.handle_key(key(KeyCode::Char('k')));
        assert_eq!(app.help_line_offset(), 1);
        app.handle_key(key(KeyCode::Up));
        assert_eq!(app.help_line_offset(), 0);
    }

    #[test]
    fn help_scroll_clamps_at_both_ends_without_needing_a_selectable_row() {
        let mut app = empty_app();
        app.handle_key(key(KeyCode::Char('?')));
        app.set_help_max_line_offset(2);

        // The very top (offset 0) is directly reachable, unlike a
        // selection-based scroll that could get stuck below it.
        app.handle_key(key(KeyCode::Char('k')));
        assert_eq!(app.help_line_offset(), 0, "should clamp at the top");

        for _ in 0..5 {
            app.handle_key(key(KeyCode::Char('j')));
        }
        assert_eq!(app.help_line_offset(), 2, "should clamp at the bottom");
    }

    #[test]
    fn ctrl_d_and_ctrl_u_scroll_the_help_modal_by_half_a_page() {
        let mut app = empty_app();
        app.handle_key(key(KeyCode::Char('?')));
        app.set_help_viewport_rows(10);
        app.set_help_max_line_offset(20);

        app.handle_key(ctrl_key(KeyCode::Char('d')));
        assert_eq!(app.help_line_offset(), 5);

        app.handle_key(ctrl_key(KeyCode::Char('u')));
        assert_eq!(app.help_line_offset(), 0);
    }

    #[test]
    fn pane_scroll_expand_and_tab_keys_are_inert_while_the_help_modal_is_open() {
        let mut app = app_with_loaded(vec![("cap", sample_diff("cap"))]);
        app.set_max_h_scroll(10);
        let focus_before = app.focus();
        let h_scroll_before = app.h_scroll();
        let tab_before = app.tab;
        let expanded_before = app.expanded.clone();

        app.handle_key(key(KeyCode::Char('?')));
        for code in [
            KeyCode::Char('h'),
            KeyCode::Char('l'),
            KeyCode::Left,
            KeyCode::Right,
            KeyCode::Char('['),
            KeyCode::Char(']'),
            KeyCode::Tab,
            KeyCode::Char('^'),
            KeyCode::Home,
            KeyCode::Char('$'),
            KeyCode::End,
            KeyCode::Enter,
            KeyCode::Char(' '),
        ] {
            app.handle_key(key(code));
        }

        assert_eq!(
            app.focus(),
            focus_before,
            "Tab must not move focus while the modal is open"
        );
        assert_eq!(app.h_scroll(), h_scroll_before);
        assert_eq!(app.tab, tab_before);
        assert_eq!(app.expanded, expanded_before);
    }

    #[test]
    fn closing_the_modal_leaves_pane_state_exactly_as_before_it_opened() {
        let mut app = app_with_loaded(vec![("cap", sample_diff("cap"))]);
        app.focus = Focus::Right;
        let expected_focus = app.focus();
        let expected_cursor = app.cursor;
        let expected_line_offset = app.line_offset;
        let expected_tab = app.tab;
        let expected_expanded = app.expanded.clone();

        app.handle_key(key(KeyCode::Char('?')));
        app.set_help_max_line_offset(20);
        app.handle_key(key(KeyCode::Char('j')));
        app.handle_key(key(KeyCode::Char('?')));

        assert_eq!(app.focus(), expected_focus);
        assert_eq!(app.cursor, expected_cursor);
        assert_eq!(app.line_offset, expected_line_offset);
        assert_eq!(app.tab, expected_tab);
        assert_eq!(app.expanded, expected_expanded);
    }
}
