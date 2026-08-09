## 1. Keyword matching and rewrite

- [ ] 1.1 In `src/tui/layout.rs`, add `"AND"` to the keyword list `bullet_keyword` checks (alongside `"WHEN"`/`"THEN"`).
- [ ] 1.2 In `style_when_then`, replace the hardcoded `i += 10` with an advance computed from the actual matched pattern (`"- **" + keyword + "**"`.len(), or equivalent), so keywords of different lengths — `AND` at 3 letters vs. `WHEN`/`THEN` at 4 — are each skipped correctly instead of relying on them coincidentally sharing a length.

## 2. Tests

- [ ] 2.1 Extend the existing WHEN/THEN unit tests in `src/tui/layout.rs` (`when_then_bullets_lose_their_asterisks_and_gain_bold`, `when_then_styling_survives_word_level_diff_highlighting`) to also cover a `- **AND** ...` bullet: asterisks stripped, `AND` styled, and — critically — the text immediately following the bullet is intact (this is what would catch a regression of the old hardcoded-offset bug).
- [ ] 2.2 Run `cargo test` for the workspace and confirm no regressions.

## 3. Manual verification

- [ ] 3.1 Run the TUI against this repo's own specs and view the `change-model` capability's "uncommitted spec edit reflected" scenario (`openspec/specs/change-model/spec.md`), which already has a `- **AND**` bullet, and confirm it now renders styled with no literal `**`.
