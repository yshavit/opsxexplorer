## Why

opsxexplorer can discover changes and read file contents through the vfs, but there is no way to look at any of it — `main.rs` is just a "Hello, world!". We need a terminal UI shell so a user can actually browse the changes a repo has, as a foundation the diff view will attach to later.

## What Changes

- Add a two-pane terminal UI: a left pane listing changes, a right pane reserved for the (not yet implemented) diff view.
- Right pane renders as an empty placeholder for now — no diff rendering in this change.
- Left pane shows active changes first (alphabetical), then a collapsible `archived` row; expanding it reveals archived changes (alphabetical, which is also chronological since archived directory names are date-prefixed).
- Selection is a single cursor over a flattened list of rows (active changes, the archived header, archived changes, and placeholder rows for empty sections) navigable with arrow keys and vim-style `j`/`k`.
- Enter/Space toggles the archived section when the cursor is on its header; collapsing always returns the cursor to the header, even if a child row was selected.
- Empty sections show placeholder text (`(no active changes)` / `(no archived changes)`) instead of being hidden; placeholder rows are not selectable and are skipped during navigation.
- On launch, the cursor starts on the first row and focus starts (and, for this change, stays) in the left pane.

## Capabilities

### New Capabilities
- `tui`: The application's overall terminal UI shell — the two-pane layout and the right pane's placeholder state.
- `tui-changelist`: The left pane's behavior — what rows it shows, how they're sorted and grouped, how the archived section expands/collapses, and how selection and keyboard navigation work over them.

### Modified Capabilities
(none)

## Impact

- `src/main.rs`: replace the placeholder `println!` with a real terminal event loop (crossterm + ratatui).
- New UI module(s) under `src/` for app state, the row model, and rendering — consumes `Changes` (`src/changes/mod.rs`) but doesn't change it.
- New dependencies: none required beyond what's already in `Cargo.toml` (`ratatui`, `crossterm`, `color-eyre`); no tree-widget crate needed since the left pane is a flattened list rendered with `ratatui::widgets::List`.
