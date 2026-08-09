## 1. Modal content model

- [x] 1.1 Add a modal-state field to `App` (open/closed, scroll offset), separate from `focus` and the existing pane fields
- [x] 1.2 Write the static keybinding content table (key label + description) for the Global, Left pane, and Right pane sections per `specs/tui-help/spec.md`, and a `help::lines()` function producing the modal's fixed, always-fully-shown content (headings, entries, and a blank line between each section)

## 2. Key handling

- [x] 2.1 Wire `?` in `App::handle_key` to toggle the modal open/closed; opening resets scroll to the top
- [x] 2.2 Short-circuit `handle_key` to modal-specific handling whenever the modal is open, ahead of the existing `match self.focus` dispatch, so `handle_left_key`/`handle_right_key` are never reached while it's open
- [x] 2.3 Handle `Esc` to close the modal
- [x] 2.4 Handle `j`/`k`/`↓`/`↑` to scroll the modal's content by one line and `Ctrl+d`/`Ctrl+u` to scroll by half a page, clamped at the first and last line (reuse the plain-offset shape of the left pane's `h_scroll`/`max_h_scroll`, not the right pane's selectable-row cursor, so the very first line is always reachable)
- [x] 2.5 Confirm `h`/`l`/`←`/`→`, `[`, `]`, `Tab`, `^`/`Home`, `$`/`End`, `Enter`, and `Space` are no-ops while the modal is open
- [x] 2.6 Confirm `q` still exits regardless of modal state (already true via `event_loop`'s `is_quit_key` running before `App::handle_key`)

## 3. Rendering

- [x] 3.1 Compute the modal's content height and width once, from the fixed content plus its padding, so width never varies with scroll position; a single `H_PADDING` constant feeds both the width computation and the `Padding::horizontal` call so they can't drift apart (a mismatch here previously clipped the widest line's tail — see design.md, Decision 4)
- [x] 3.2 Render the popup as a `Clear` + bordered widget over the two-pane layout, sized to `min(content height, available frame height minus margin)` and `min(content width, available frame width minus margin)`
- [x] 3.3 Render each section heading as underlined, bold text, and each entry's key/description line (with each `/`-separated key segment styled distinctly from the separator itself), with a blank line between sections
- [x] 3.4 Pad the modal's content on the left and right edges, inside the border, wide enough that the widest line is never clipped
- [x] 3.5 Render a vertical scrollbar for the modal, hidden when content fits and shown only while it overflows (see 3.7a: this also changed the right pane's own scrollbar convention, since both share `render_right_scrollbar`)
- [x] 3.6 Write a desaturation pass that strips foreground/background color from a set of styled spans (preserving bold/underline/reversed) and apply it to both panes' `ListItem`s/`Line`s and border `Block`s whenever the modal is open; leave panes unaffected when it's closed
- [x] 3.7 Style the modal's popup border distinctly per design.md: light-blue background, dark-gray rounded border, no shadow
- [x] 3.7a Update `render_right_scrollbar` (shared by the right pane and the help modal) to hide the scrollbar entirely when there's nothing to scroll, and update `tui-specdiff`'s spec of record accordingly via a `MODIFIED Requirement` in this change's delta specs

## 4. Tests

- [x] 4.1 `?` opens the modal from closed and closes it from open; `Esc` also closes it; opening resets scroll to the top
- [x] 4.2 `j`/`k`/`↓`/`↑` scroll by one line, `Ctrl+d`/`Ctrl+u` scroll by half a page
- [x] 4.3 Scrolling clamps at the first and last line without needing a selectable row at either end
- [x] 4.4 `h`/`l`/arrows, `[`, `]`, `Tab`, `^`, `$`, `Enter`, `Space` have no effect on modal state or on the underlying pane's state while the modal is open
- [x] 4.5 Closing the modal leaves the underlying pane's focus, selection, scroll offset, tab, and collapse state exactly as they were before it opened
- [x] 4.6 Modal sizing: renders at content height/width when content fits the frame, clamps and scrolls when it doesn't
- [x] 4.7 The desaturation pass strips color from styled spans while preserving non-color modifiers, and is applied to both panes' content and borders only while the modal is open, reverting once it closes
- [x] 4.8 The modal's content (headings, entries, blank-line separators) has a fixed, state-independent shape
- [x] 4.9 The vertical scrollbar (shared helper) is hidden when there's nothing to scroll and shown when there is, for both the right pane and the help modal
- [x] 4.10 The widest content line renders in full end-to-end (via a real render pass), not clipped by the popup's border or padding — a regression test for the padding-undercounting bug found during review

## 5. Verification (manual, by a human)

- [x] 5.1 `cargo test`
- [x] 5.2 `cargo run`: manually verify the modal in both a tall and a short/resized terminal window, confirming fixed width, padding, blank lines between sections, no shadow, that scrolling reaches the very top and bottom of the content, and that both the modal's and the right pane's scrollbars are hidden when there's nothing to scroll
