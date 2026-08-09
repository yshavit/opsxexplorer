## Why

opsxexplorer has no on-screen reference for its keybindings — `Ctrl+d`/`Ctrl+u`, `[`/`]`, `^`/`$`, and the pane-specific meanings of `h`/`l` are discoverable only by reading the source. A `?`-triggered help overlay makes the full keybinding set discoverable from inside the running application.

## What Changes

- New global key `?` toggles a help modal that lists every keybinding, grouped into three sections separated by a blank line: **Global** (keys bound identically in both panes, e.g. `q`, `Tab`, `j`/`k`, `Enter`/`Space`), **Left pane** (`h`/`l` scroll, `^`/`$` jump), and **Right pane** (`h`/`l` expand/collapse, `[`/`]` tabs).
- `Esc` also closes the modal; `q` still quits regardless of whether the modal is open.
- The modal's content is always shown in full — there is nothing to expand or collapse. While the modal is open, `j`/`k`/`↓`/`↑` scroll its content by one line and `Ctrl+d`/`Ctrl+u` scroll by half a page, reaching the very first and very last line. `h`/`l`/arrow keys, `[`, `]`, `Tab`, `^`, `$`, `Enter`, and `Space` have no effect while the modal is open.
- The modal's popup is sized to its content, capped to the space available in the current frame, and scrolls vertically (with a scrollbar) when its content doesn't fit — so it never renders larger than needed, but also never overflows a short terminal window. Its width is fixed regardless of scroll position, and its content is padded on the left and right edges, wide enough that its widest line is never clipped by the border.
- While the modal is open, both panes render without color — borders, diff markers, word-level highlighting, everything — leaving a neutral, uniform background treatment, while the modal's own popup renders with a border treatment that makes it unmistakably distinct (see design.md for the current specific styling).
- The right pane's vertical scrollbar (previously always shown, even when there was nothing to scroll) now hides entirely when its content fits — a small, incidental convention change made alongside the modal's own scrollbar, since both share the same rendering helper.

## Capabilities

### New Capabilities
- `tui-help`: defines the `?` help modal — its content and grouping, in-modal scrolling and dismissal, and sizing/scrolling behavior.

### Modified Capabilities
- `tui-specdiff`: the right pane's vertical scrollbar is now hidden when there's nothing to scroll, rather than shown in a "nothing to scroll" state.

## Impact

- `src/tui/app.rs`: new modal state (open/closed, scroll offset) and key handling, gating the existing left/right pane dispatch while the modal is open.
- `src/tui/mod.rs`: rendering the modal as a centered, capped-height overlay with its own scrollbar, recognizing `?`/`Esc` in the event loop alongside the existing `q` handling, and hiding the (shared) vertical scrollbar helper's output when there's nothing to scroll.
- No other changes to `tui`, `tui-changelist`, or `tui-specdiff` behavior when the modal is closed.
