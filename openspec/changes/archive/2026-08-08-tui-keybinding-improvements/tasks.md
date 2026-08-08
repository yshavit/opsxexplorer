## 1. Quit key

- [x] 1.1 In `src/tui/mod.rs`'s event loop, change the quit check from `Char('q')` + `CONTROL` modifier to `Char('q')` with no modifier
- [x] 1.2 Update/add tests covering `q` quits and Ctrl+Q no longer does

## 2. Enter/Space focus-shift in the left pane

- [x] 2.1 In `App::handle_left_key` (`src/tui/app.rs`), change the `Enter | Char(' ')` arm to branch on the row under the cursor: `Row::ArchivedHeader` keeps calling `toggle_archived_at_cursor`; `Row::Active`/`Row::Archived` additionally sets `self.focus = Focus::Right`
- [x] 2.2 Add/update tests: Enter on an active row moves focus right, Space on an archived row moves focus right, Enter/Space on the archived header leaves focus on the left pane and still toggles the section

## 3. Half-page cursor movement

- [x] 3.1 Add viewport-row-count state to `App` for the left pane and right pane (mirroring the existing `max_h_scroll`/`max_line_offset` pattern), with setters the renderer calls each frame
- [x] 3.2 Wire those setters into `render_left_pane`/`render_right_pane` in `src/tui/mod.rs`, using the same inner-height computation already used for layout
- [x] 3.3 In `handle_left_key`, add `Ctrl+d`/`Ctrl+u` branches calling `move_selection` with a delta of roughly half the left pane's viewport rows (positive/negative)
- [x] 3.4 In `handle_right_key`, add `Ctrl+d`/`Ctrl+u` branches calling `move_cursor` with a delta of roughly half the right pane's viewport rows (positive/negative)
- [x] 3.5 Add/update tests: half-page down/up in each pane, and clamping at the first/last selectable row when fewer than half a page remains

## 4. Spec sync

- [x] 4.1 Run `openspec validate --change tui-keybinding-improvements --strict` and fix any reported issues
