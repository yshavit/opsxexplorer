## Context

`src/changes::Changes::discover` already produces `active: Vec<Change>` and `archived: Vec<Change>`, both alphabetically sorted (both `DiskFs` and `GitTreeFs` sort `list_dir` entries by name, and `discovery::discover_active`/`discover_archived` map over that order without re-sorting). Archived directory names are `${date}-${change-name}`, so alphabetical order on the raw `Change(String)` is also chronological order; `Change::display_name()` strips the date prefix for display only. This design does not touch `src/changes` or `src/vfs` — it's a pure consumer.

`Cargo.toml` already has `ratatui`, `crossterm`, and `color-eyre`. No tree-widget crate is added; see specs/tui-changelist for the required behavior this implements.

## Goals / Non-Goals

**Goals:**
- A running terminal application with a real event loop, replacing the `println!` in `main.rs`.
- Left pane fully interactive per specs/tui-changelist.
- Right pane rendered, empty, per specs/tui.

**Non-Goals:**
- Any diff rendering or right-pane content.
- Pane-switching / multi-pane focus.
- Persisting cursor or expand/collapse state across runs.

## Decisions

### Flatten rows into a single `Vec<Row>`, render with `ratatui::widgets::List`

```rust
enum Row<'a> {
    Active(&'a Change),
    ArchivedHeader { expanded: bool },
    Archived(&'a Change),
    Placeholder { text: &'static str, indented: bool },
}
```

`Row` is rebuilt from `(&changes.active, &changes.archived, archived_expanded)` whenever that state changes (on toggle) — it's cheap (dozens of entries at most) so there's no need to cache it across frames. `List` has no concept of hierarchy; it just renders whatever `Vec<ListItem>` it's given and tracks a selected index via `ListState`. All tree-like behavior (indentation, the `▸`/`▾` marker, which rows exist at all) lives in how `Row` → `ListItem` conversion happens, not in the widget: indentation is a pure function of the `Row` value alone (`Archived` and `Placeholder { indented: true, .. }` indent, everything else doesn't), so the state→`Row` flattening step never has to think about rendering, and the `Row`→`ListItem` step never has to inspect placeholder text to guess which section it belongs to. This also gives scrolling for free: `List`'s `StatefulWidget` impl keeps the selected index in view by adjusting `ListState`'s offset during render.

Alternative considered: a dedicated tree-widget crate (e.g. `tui-tree-widget`). Rejected — the tree here is exactly one level deep with a single expandable node, so a real tree widget's generality (arbitrary nesting, per-node state trees) buys nothing and adds a dependency.

### Selection is a plain index into `Vec<Row>`

`ListState.selected: Option<usize>` indexes directly into the flattened `rows`. Up/down (and `k`/`j`) move the index by one, skipping over any `Row::Placeholder` in that direction — since a placeholder is only ever adjacent to a real row (it replaces an empty section, never sits between two populated ones), a single skip is sufficient; no loop-until-non-placeholder logic is needed. Enter/Space checks `rows[selected]`; if it's `ArchivedHeader`, flips `archived_expanded` and rebuilds `rows`. Collapsing always sets `selected` to the (rebuilt) index of `ArchivedHeader`, discarding whatever the previous index pointed at — this is simpler and more robust than trying to detect "was the previous selection a now-removed archived child," and it's what specs/tui-changelist requires unconditionally.

Alternative considered: separate `SectionState` tracking "which section + offset within it" instead of a flat index. Rejected — it duplicates what `ListState` already does, and every operation (move, render, clamp) would need a translation step back to a flat position anyway.

### Archived rows render as two styled spans: dimmed date + name

`Row::Archived` needs both `Change::archive_date()` and `Change::display_name()` to build its `ListItem` — not just `display_name()` alone. When `archive_date()` returns `Some`, the row's text is two `ratatui::text::Span`s: the date in a dimmed style, followed by the change name in normal style. When it returns `None` (a malformed archive directory name), the row falls back to `display_name()` alone with no date span — consistent with `Change`'s existing fallback behavior (see `archived_change_with_malformed_name_falls_back_gracefully` in `src/changes/change.rs`). The exact dim treatment (`Modifier::DIM` vs. an explicit muted `Color`) is left to implementation time, whichever renders more consistently across terminals.

### `archived_expanded` starts `false`

The proposal's "collapsible row that reveals archived changes when expanded" reads as collapsed-by-default; nothing in the conversation this was scoped from called for auto-expanding on launch, and defaulting to collapsed keeps the common case (a repo with more history than active work) from opening into a long list. Flagged here in case that reading is wrong — it's a one-line change (`archived_expanded: false` → `true`) if so.

### Module layout

Follows the shape `src/changes/` and `src/vfs/` already use — a `mod.rs` that orchestrates, plus focused submodules:

```
src/tui/
  mod.rs   — pub `run()` entrypoint; terminal lifecycle (raw mode/alt screen enter+restore); event loop; layout + rendering
  app.rs   — `App` struct and its key-handling logic
  row.rs   — the `Row` enum and the state→Row flattening function
```

Rendering (layout split, right-pane placeholder, `Row`→`ListItem` conversion, drawing the `List`) starts out living in `mod.rs` alongside the event loop; split into its own file only if it grows large enough to warrant it.

### App structure

A small `App` struct owns everything render/input touch:

```rust
struct App {
    changes: Changes,       // from src/changes
    archived_expanded: bool,
    list_state: ListState,
}
```

`App` has no `rows` field. A `Vec<Row<'a>>` borrows from `changes`, and a struct can't hold both an owned value and a reference into that same value as sibling fields — the classic self-referential-struct limitation (references don't get relocated if the struct itself moves, so the borrow checker won't let one field's lifetime point at another field of the same struct). Storing `rows` on `App` would run straight into that. Instead, rows are computed on demand via a method, `App::rows(&self) -> Vec<Row<'_>>`, called separately at render time and at key-handling time — never stored. Cheap enough to not need caching, per above.

Event loop: standard crossterm + ratatui pattern — enter raw mode / alternate screen, loop on `event::read()`, match `KeyCode` to actions, `terminal.draw(...)` each iteration, restore terminal on exit (including on panic, via a `color-eyre` panic hook or a drop guard — whichever is more idiomatic to wire up at implementation time).

## Risks / Trade-offs

- [Recomputing `rows` on every render, not just on toggle] → Negligible cost at this scale (active + archived changes are small lists); simpler than cache invalidation. Revisit only if change counts become large enough to matter, which is not expected for this tool's use case.
- [`archived_expanded` default may not match user intent] → See decision above; easy to flip, called out explicitly rather than silently guessed.

## Open Questions

None — the conversation that led to this change resolved the behavioral questions (row model, selection mechanics, empty states, focus, key bindings, row content, capability split) before this proposal was written; the only judgment call made without explicit confirmation is the `archived_expanded` default noted above.
