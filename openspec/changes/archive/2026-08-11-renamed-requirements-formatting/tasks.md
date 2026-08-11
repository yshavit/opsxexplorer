## 1. Diff model

- [x] 1.1 Change `Operation::Renamed` in `src/diff/model.rs` to carry a `title: Piece` instead of `from: String`.
- [x] 1.2 In `src/diff/mod.rs`, compute the title `Piece` at rename-construction time via `compare::changed_or_unchanged(from, to)` and store it on `Operation::Renamed`.
- [x] 1.3 Update the existing unit test(s) in `src/diff/mod.rs` that assert on `Operation::Renamed { from }` to assert on the new `title` field instead.

## 2. Rendering

- [x] 2.1 In `src/tui/layout.rs`, replace the `Operation::Renamed` arm of `content_spans` (the dimmed-former-name / `→` / modification-yellow spans) with `piece_spans(&title)`, following `"REQ "` the same way every other op does.
- [x] 2.2 Remove now-dead code this replaces (the `from.clone()` dim span and the `" → "` modified-style span), and confirm `modified_style()` is still used elsewhere (gutter marker, group heading) so it isn't left orphaned.

## 3. Tests

- [x] 3.1 Add/update a layout test asserting a renamed requirement with similar names renders as one inline passage with interleaved deletion/insertion runs (same styling a changed intro's runs get).
- [x] 3.2 Add/update a layout test asserting a renamed requirement with dissimilar names renders as the former name (deletion-styled) then the new name (insertion-styled), each on its own line.
- [x] 3.3 Add/update a layout test asserting a long rename (in either form) wraps to the pane width instead of collapsing or truncating, matching the existing "a long requirement name" wrap behavior.
- [x] 3.4 Confirm the gutter marker (`»`) and group heading for a renamed requirement are unchanged by running/inspecting the existing tests that cover them.

## 4. Stacked-replacement indentation

- [x] 4.1 In `src/tui/layout.rs`, give the wholesale-replacement rename its own line-building path (`renamed_replacement_lines`) so every line after the first — the new name's line, and any continuation line from either name wrapping on its own — is indented to align beneath the first line's own text (past the marker, disclosure triangle and `REQ `), not just beneath the gutter.
- [x] 4.2 Add layout tests covering: the new name's line aligns under the former name's text; every line still aligns when a name is long enough to wrap onto more than one line of its own.
- [x] 4.3 Update the `tui-specdiff` delta spec's "dissimilar names render as stacked before-and-after text" requirement to state the alignment explicitly.

## 5. Spec sync

- [x] 5.1 Run `openspec validate --change renamed-requirements-formatting --strict` and fix any issues.
