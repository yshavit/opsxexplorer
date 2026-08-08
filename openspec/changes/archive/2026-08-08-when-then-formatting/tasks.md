## 1. Styling primitive

- [x] 1.1 Add `when_then_style() -> Style` in `src/tui/layout.rs`, alongside `added_style`/`removed_style`/`modified_style`, returning `Style::new().add_modifier(Modifier::BOLD)` (italic was tried first; bold read better in the terminal).

## 2. Rewrite scenario-body bullets

- [x] 2.1 Add a function in `src/tui/layout.rs` that takes a row's `Vec<Span<'static>>` and rewrites any bullet opening `- **WHEN**` or `- **THEN**` (matched on the flattened character+style sequence, so a keyword split across multiple spans by word-diff highlighting is still caught) into spans with the `**` dropped and the keyword styled via `when_then_style()`, leaving surrounding text's original style untouched.
- [x] 2.2 Call it from `content_spans`'s `DiffRow::Body` arm, applied to the output of `piece_spans(piece)`.

## 3. Tests

- [x] 3.1 Unit test: a plain (`Unchanged`/`Added`/`Deleted`) scenario body with `- **WHEN** x\n- **THEN** y` renders `WHEN` and `THEN` bold with no `**` in the output spans.
- [x] 3.2 Unit test: a `Changed` scenario body (word-diff `Run`s) still renders correct insertion/deletion styling around a `**WHEN**`/`**THEN**` bullet, and the keyword itself is still de-asterisked and bold.
- [x] 3.3 Unit test: `**bold**` text elsewhere in a requirement/scenario body (not a leading WHEN/THEN bullet) is left rendered as literal `**bold**`, unchanged.
- [x] 3.4 Run `cargo test` for the workspace and confirm no regressions in existing `layout.rs` / `wrap.rs` tests.

## 4. Manual verification

- [x] 4.1 Run the TUI against this repo's own specs (e.g. `tui-specdiff`, which already has WHEN/THEN-heavy scenarios) and visually confirm the bullets read as intended, then adjust `when_then_style()` if italic doesn't look good. Confirmed by running the TUI: italic didn't read well, switched to bold.
