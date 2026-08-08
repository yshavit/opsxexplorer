## Why

The left pane's archived section is sorted alphabetically by directory name (date prefix included), which happens to read as chronological ascending only because directory names begin with `YYYY-MM-DD`. Archived changes accumulate over time, so the entries a user most likely wants — the most recently archived ones — end up at the bottom of a growing list, below oldest-first history nobody needs to scroll past.

## What Changes

- Archived changes are sorted **descending** (most recent first) instead of ascending.
- The primary sort key becomes the change's `YYYY-MM-DD` date prefix (descending), not the raw directory-name string.
- When two archived changes share the same date, the tiebreaker is the timestamp of the commit that first introduced that change's directory in git history (descending — newer commit first). A change with no resolvable introducing commit (e.g. uncommitted, or no git repo) sorts as if it were newer than any change with a resolvable commit.
- If date and commit-timestamp tiebreakers are both equal (or both absent), the directory name is used as a final tiebreaker, this time ascending. This lets us present the most recent changes at the top, while still preserving an intuitive lexicographical order.
- Active changes are unaffected — they keep their existing alphabetical-ascending order.

## Capabilities

### Modified Capabilities
- `tui-changelist`: the "Archived changes sorted alphabetically, displayed with date" requirement changes from ascending alphabetical order to a descending order keyed on date, then first-introducing-commit timestamp, then directory name.

## Impact

- `src/changes/discovery.rs` / `src/changes/mod.rs`: archived changes need to be sorted after discovery (currently only implicitly ordered by the filesystem listing).
- `src/changes/history.rs`: needs a new, infallible lookup for the timestamp of the commit that first introduced a change's directory (distinct from `resolve_archive_base`, which returns the *parent* of that commit and errors when none exists — behavior not suited to a sort tiebreaker).
- `src/changes/change.rs`: `archive_date()` already extracts the date prefix; no change needed there.
- No change to active-change ordering, row rendering/flattening, or the TUI's collapse/expand behavior.
