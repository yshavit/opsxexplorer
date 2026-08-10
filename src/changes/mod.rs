mod change;
mod discovery;
mod error;
mod history;

pub use change::{Change, ChangeView, DiffBase};
pub use error::ChangesError;

/// Both views needed to diff a change: `live` always holds the change's own
/// delta specs (the live working tree), and `base` holds the spec of record
/// at the change's resolved diff base. `change` identifies which change this
/// pair belongs to, so a consumer can resolve `<change>/specs/` on `live`
/// without a second parameter — both `live` and `base` are full-repo-rooted
/// `Fs` views (required so `base` can reach `openspec/specs/<cap>/spec.md`,
/// outside the change directory), so neither view alone carries that path.
pub struct ChangeViews<'a> {
    pub live: Fs<'a>,
    pub base: Fs<'a>,
    pub change: Change,
}

use std::cmp::Ordering;
use std::collections::HashMap;
use std::path::Path;

use crate::vfs::{Fs, FsError, Workspace};

pub struct Changes {
    vfs: Workspace,
    /// The introducing-commit resolution for every archived change, computed
    /// once at discovery. `None` when discovery found no git repository;
    /// distinct from an archived change being absent from an inner map,
    /// which means that change's introducing commit could not be resolved.
    archived_history: Option<HashMap<Change, history::Resolution>>,
    pub active: Vec<Change>,
    pub archived: Vec<Change>,
}

impl Changes {
    pub fn discover(start: &Path) -> Result<Changes, FsError> {
        let vfs = Workspace::open(start)?;
        let current = vfs.current();
        let active = discovery::discover_active(&current)?;
        let mut archived = discovery::discover_archived(&current)?;
        let archived_history = vfs.repo().map(|r| history::resolve_all(r, &archived));
        archived.sort_by(|a, b| {
            let a_introduced_at = archived_history
                .as_ref()
                .and_then(|m| m.get(a))
                .map(|r| r.time);
            let b_introduced_at = archived_history
                .as_ref()
                .and_then(|m| m.get(b))
                .map(|r| r.time);
            b.archive_date()
                .cmp(&a.archive_date())
                .then_with(|| cmp_introduced_at(a_introduced_at, b_introduced_at))
                .then_with(|| a.0.cmp(&b.0))
        });
        Ok(Changes {
            vfs,
            archived_history,
            active,
            archived,
        })
    }

    pub fn resolve(&self, change: &Change) -> Result<ChangeView, ChangesError> {
        let diff_base = if change.is_archived() {
            let history = self.archived_history.as_ref().ok_or(FsError::NotAGitRepo)?;
            match history.get(change).and_then(|r| r.diff_base) {
                Some(base) => DiffBase::At(base),
                None => return Err(ChangesError::ArchiveHistoryNotFound(change.clone())),
            }
        } else {
            DiffBase::Current
        };
        Ok(ChangeView {
            change: change.clone(),
            diff_base,
        })
    }

    pub fn views<'a>(&'a self, view: &ChangeView) -> Result<ChangeViews<'a>, FsError> {
        Ok(ChangeViews {
            live: self.vfs.current(),
            base: self.resolve_base(view)?,
            change: view.change.clone(),
        })
    }

    fn resolve_base<'a>(&'a self, view: &ChangeView) -> Result<Fs<'a>, FsError> {
        match view.diff_base {
            DiffBase::Current => Ok(self.vfs.current()),
            DiffBase::At(r) => self.vfs.at(&r),
        }
    }
}

/// Orders introducing-commit timestamps descending (most recent first), with
/// `None` (no resolvable introducing commit) treated as newer than any
/// resolvable timestamp.
fn cmp_introduced_at(a: Option<i64>, b: Option<i64>) -> Ordering {
    match (a, b) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Less,
        (Some(_), None) => Ordering::Greater,
        (Some(x), Some(y)) => y.cmp(&x),
    }
}

#[cfg(test)]
mod test_support {
    use git2::{Oid, Repository, Signature};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    pub struct TempDir(PathBuf);

    impl TempDir {
        pub fn new(label: &str) -> Self {
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let id = COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "opsxexplorer-changes-test-{label}-{}-{id}",
                std::process::id()
            ));
            fs::create_dir_all(&path).unwrap();
            TempDir(path)
        }

        pub fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    pub fn write_file(root: &Path, rel: &str, content: &str) {
        let full = root.join(rel);
        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(full, content).unwrap();
    }

    pub fn stage_and_commit(repo: &Repository, message: &str, paths: &[&str]) -> Oid {
        let mut index = repo.index().unwrap();
        for path in paths {
            index.add_path(Path::new(path)).unwrap();
        }
        index.write().unwrap();
        let tree_oid = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();
        let sig = Signature::now("Test", "test@example.com").unwrap();
        let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
        let parents: Vec<&git2::Commit> = parent.iter().collect();
        repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)
            .unwrap()
    }

    /// Like `stage_and_commit`, but with an explicit commit timestamp, so
    /// tests that assert an ordering derived from commit time aren't at the
    /// mercy of two commits landing in the same wall-clock second.
    pub fn stage_and_commit_at(repo: &Repository, message: &str, paths: &[&str], time: i64) -> Oid {
        let mut index = repo.index().unwrap();
        for path in paths {
            index.add_path(Path::new(path)).unwrap();
        }
        index.write().unwrap();
        let tree_oid = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();
        let sig = Signature::new("Test", "test@example.com", &git2::Time::new(time, 0)).unwrap();
        let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
        let parents: Vec<&git2::Commit> = parent.iter().collect();
        repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)
            .unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::*;
    use super::*;
    use git2::Repository;
    use std::path::Path;

    #[test]
    fn discovers_active_and_archived_changes() {
        let dir = TempDir::new("discover");
        let repo = Repository::init(dir.path()).unwrap();
        write_file(dir.path(), "openspec/changes/add-thing/proposal.md", "x");
        write_file(
            dir.path(),
            "openspec/changes/archive/2026-01-01-old-thing/proposal.md",
            "y",
        );
        stage_and_commit(
            &repo,
            "initial",
            &[
                "openspec/changes/add-thing/proposal.md",
                "openspec/changes/archive/2026-01-01-old-thing/proposal.md",
            ],
        );

        let changes = Changes::discover(dir.path()).unwrap();

        assert_eq!(changes.active, vec![Change("add-thing".to_string())]);
        assert_eq!(
            changes.archived,
            vec![Change("archive/2026-01-01-old-thing".to_string())]
        );
    }

    #[test]
    fn active_change_diff_base_reflects_uncommitted_edit() {
        let dir = TempDir::new("active-live");
        let repo = Repository::init(dir.path()).unwrap();
        write_file(dir.path(), "openspec/changes/add-thing/proposal.md", "x");
        write_file(dir.path(), "openspec/specs/cap/spec.md", "v1");
        stage_and_commit(
            &repo,
            "initial",
            &[
                "openspec/changes/add-thing/proposal.md",
                "openspec/specs/cap/spec.md",
            ],
        );

        write_file(dir.path(), "openspec/specs/cap/spec.md", "v2-uncommitted");

        let changes = Changes::discover(dir.path()).unwrap();
        let change = changes.active[0].clone();
        let view = changes.resolve(&change).unwrap();
        assert_eq!(view.diff_base, DiffBase::Current);

        let views = changes.views(&view).unwrap();
        assert_eq!(
            views
                .base
                .read(Path::new("openspec/specs/cap/spec.md"))
                .unwrap(),
            b"v2-uncommitted"
        );
    }

    #[test]
    fn archived_change_diff_base_is_commit_before_it_was_introduced() {
        let dir = TempDir::new("archived-base");
        let repo = Repository::init(dir.path()).unwrap();

        write_file(dir.path(), "openspec/specs/cap/spec.md", "v1");
        stage_and_commit(&repo, "spec v1", &["openspec/specs/cap/spec.md"]);

        // The archiving commit both introduces the archived change and edits the spec of record.
        write_file(
            dir.path(),
            "openspec/changes/archive/2026-01-01-old-thing/proposal.md",
            "x",
        );
        write_file(dir.path(), "openspec/specs/cap/spec.md", "v2-archived-with");
        stage_and_commit(
            &repo,
            "archive old-thing",
            &[
                "openspec/changes/archive/2026-01-01-old-thing/proposal.md",
                "openspec/specs/cap/spec.md",
            ],
        );

        let changes = Changes::discover(dir.path()).unwrap();
        let change = changes.archived[0].clone();
        let view = changes.resolve(&change).unwrap();

        let views = changes.views(&view).unwrap();
        assert_eq!(
            views
                .base
                .read(Path::new("openspec/specs/cap/spec.md"))
                .unwrap(),
            b"v1"
        );
    }

    #[test]
    fn archived_change_diff_base_anchored_to_earliest_commit_touching_it() {
        let dir = TempDir::new("archived-multi-commit");
        let repo = Repository::init(dir.path()).unwrap();

        write_file(dir.path(), "openspec/specs/cap/spec.md", "v1");
        stage_and_commit(&repo, "spec v1", &["openspec/specs/cap/spec.md"]);

        // First commit introduces the archived directory.
        write_file(
            dir.path(),
            "openspec/changes/archive/2026-01-01-old-thing/proposal.md",
            "x",
        );
        stage_and_commit(
            &repo,
            "archive old-thing",
            &["openspec/changes/archive/2026-01-01-old-thing/proposal.md"],
        );

        // A later, separate commit further edits a file inside the same archived
        // directory and also edits the spec of record - this must not become the anchor.
        write_file(
            dir.path(),
            "openspec/changes/archive/2026-01-01-old-thing/proposal.md",
            "x-corrected",
        );
        write_file(
            dir.path(),
            "openspec/specs/cap/spec.md",
            "v2-after-correction",
        );
        stage_and_commit(
            &repo,
            "fix typo in archived proposal",
            &[
                "openspec/changes/archive/2026-01-01-old-thing/proposal.md",
                "openspec/specs/cap/spec.md",
            ],
        );

        let changes = Changes::discover(dir.path()).unwrap();
        let change = changes.archived[0].clone();
        let view = changes.resolve(&change).unwrap();

        let views = changes.views(&view).unwrap();
        assert_eq!(
            views
                .base
                .read(Path::new("openspec/specs/cap/spec.md"))
                .unwrap(),
            b"v1"
        );
    }

    #[test]
    fn archived_change_diff_base_unaffected_by_later_history() {
        let dir = TempDir::new("archived-later-history");
        let repo = Repository::init(dir.path()).unwrap();

        write_file(dir.path(), "openspec/specs/cap/spec.md", "v1");
        stage_and_commit(&repo, "spec v1", &["openspec/specs/cap/spec.md"]);

        write_file(
            dir.path(),
            "openspec/changes/archive/2026-01-01-old-thing/proposal.md",
            "x",
        );
        stage_and_commit(
            &repo,
            "archive old-thing",
            &["openspec/changes/archive/2026-01-01-old-thing/proposal.md"],
        );

        write_file(dir.path(), "openspec/specs/cap/spec.md", "v2-later");
        stage_and_commit(&repo, "later spec change", &["openspec/specs/cap/spec.md"]);

        let changes = Changes::discover(dir.path()).unwrap();
        let change = changes.archived[0].clone();
        let view = changes.resolve(&change).unwrap();

        let views = changes.views(&view).unwrap();
        assert_eq!(
            views
                .base
                .read(Path::new("openspec/specs/cap/spec.md"))
                .unwrap(),
            b"v1"
        );
    }

    #[test]
    fn change_view_travels_without_relookup_of_status() {
        let dir = TempDir::new("change-view-travels");
        let repo = Repository::init(dir.path()).unwrap();

        write_file(dir.path(), "openspec/specs/cap/spec.md", "v1");
        stage_and_commit(&repo, "spec v1", &["openspec/specs/cap/spec.md"]);
        write_file(
            dir.path(),
            "openspec/changes/archive/2026-01-01-old-thing/proposal.md",
            "x",
        );
        stage_and_commit(
            &repo,
            "archive old-thing",
            &["openspec/changes/archive/2026-01-01-old-thing/proposal.md"],
        );

        let changes = Changes::discover(dir.path()).unwrap();
        let change = changes.archived[0].clone();
        let view = changes.resolve(&change).unwrap();

        // Held across two simulated "frames" with no re-lookup of active/archived status.
        let held = view.clone();

        for v in [&view, &held] {
            let views = changes.views(v).unwrap();
            assert_eq!(
                views
                    .base
                    .read(Path::new("openspec/specs/cap/spec.md"))
                    .unwrap(),
                b"v1"
            );
        }
    }

    #[test]
    fn views_exposes_live_delta_and_base_spec_of_record_for_archived_change() {
        let dir = TempDir::new("views-archived");
        let repo = Repository::init(dir.path()).unwrap();

        write_file(dir.path(), "openspec/specs/cap/spec.md", "v1");
        stage_and_commit(&repo, "spec v1", &["openspec/specs/cap/spec.md"]);

        // The archiving commit introduces the change directory (with its own
        // delta spec) and edits the spec of record in the same commit.
        write_file(
            dir.path(),
            "openspec/changes/archive/2026-01-01-old-thing/proposal.md",
            "x",
        );
        write_file(
            dir.path(),
            "openspec/changes/archive/2026-01-01-old-thing/specs/cap/spec.md",
            "delta",
        );
        write_file(dir.path(), "openspec/specs/cap/spec.md", "v2-archived-with");
        stage_and_commit(
            &repo,
            "archive old-thing",
            &[
                "openspec/changes/archive/2026-01-01-old-thing/proposal.md",
                "openspec/changes/archive/2026-01-01-old-thing/specs/cap/spec.md",
                "openspec/specs/cap/spec.md",
            ],
        );

        let changes = Changes::discover(dir.path()).unwrap();
        let change = changes.archived[0].clone();
        let view = changes.resolve(&change).unwrap();

        let views = changes.views(&view).unwrap();

        // The change's own delta spec is readable through the live view...
        assert_eq!(
            views
                .live
                .read(Path::new(
                    "openspec/changes/archive/2026-01-01-old-thing/specs/cap/spec.md"
                ))
                .unwrap(),
            b"delta"
        );
        // ...but absent from the base view, whose diff base predates the change directory.
        assert!(matches!(
            views.base.read(Path::new(
                "openspec/changes/archive/2026-01-01-old-thing/specs/cap/spec.md"
            )),
            Err(FsError::NotFound(_))
        ));
        // The spec of record read through the base view is its state at the
        // resolved diff base, not its current state.
        assert_eq!(
            views
                .base
                .read(Path::new("openspec/specs/cap/spec.md"))
                .unwrap(),
            b"v1"
        );
    }

    #[test]
    fn views_are_both_live_for_active_change() {
        let dir = TempDir::new("views-active");
        let repo = Repository::init(dir.path()).unwrap();
        write_file(dir.path(), "openspec/changes/add-thing/proposal.md", "x");
        write_file(dir.path(), "openspec/specs/cap/spec.md", "v1");
        stage_and_commit(
            &repo,
            "initial",
            &[
                "openspec/changes/add-thing/proposal.md",
                "openspec/specs/cap/spec.md",
            ],
        );

        // Uncommitted edit to the spec of record.
        write_file(dir.path(), "openspec/specs/cap/spec.md", "v2-uncommitted");

        let changes = Changes::discover(dir.path()).unwrap();
        let change = changes.active[0].clone();
        let view = changes.resolve(&change).unwrap();
        let views = changes.views(&view).unwrap();

        assert_eq!(
            views
                .live
                .read(Path::new("openspec/specs/cap/spec.md"))
                .unwrap(),
            b"v2-uncommitted"
        );
        assert_eq!(
            views
                .base
                .read(Path::new("openspec/specs/cap/spec.md"))
                .unwrap(),
            b"v2-uncommitted"
        );
    }

    #[test]
    fn views_travels_without_relookup_of_status() {
        let dir = TempDir::new("views-travels");
        let repo = Repository::init(dir.path()).unwrap();

        write_file(dir.path(), "openspec/specs/cap/spec.md", "v1");
        stage_and_commit(&repo, "spec v1", &["openspec/specs/cap/spec.md"]);
        write_file(
            dir.path(),
            "openspec/changes/archive/2026-01-01-old-thing/proposal.md",
            "x",
        );
        stage_and_commit(
            &repo,
            "archive old-thing",
            &["openspec/changes/archive/2026-01-01-old-thing/proposal.md"],
        );

        let changes = Changes::discover(dir.path()).unwrap();
        let change = changes.archived[0].clone();
        let view = changes.resolve(&change).unwrap();

        // Held across two simulated "frames" with no re-lookup of active/archived status.
        let held = view.clone();

        for v in [&view, &held] {
            let views = changes.views(v).unwrap();
            assert_eq!(
                views
                    .base
                    .read(Path::new("openspec/specs/cap/spec.md"))
                    .unwrap(),
                b"v1"
            );
        }
    }

    #[test]
    fn discover_archived_orders_by_date_descending() {
        let dir = TempDir::new("discover-date-desc");
        let repo = Repository::init(dir.path()).unwrap();

        write_file(
            dir.path(),
            "openspec/changes/archive/2026-01-03-foo/proposal.md",
            "x",
        );
        write_file(
            dir.path(),
            "openspec/changes/archive/2026-06-19-bar/proposal.md",
            "y",
        );
        stage_and_commit(
            &repo,
            "initial",
            &[
                "openspec/changes/archive/2026-01-03-foo/proposal.md",
                "openspec/changes/archive/2026-06-19-bar/proposal.md",
            ],
        );

        let changes = Changes::discover(dir.path()).unwrap();

        assert_eq!(
            changes.archived,
            vec![
                Change("archive/2026-06-19-bar".to_string()),
                Change("archive/2026-01-03-foo".to_string()),
            ]
        );
    }

    #[test]
    fn discover_archived_same_date_tiebreak_by_introducing_commit_most_recent_first() {
        let dir = TempDir::new("discover-same-date-commit-tiebreak");
        let repo = Repository::init(dir.path()).unwrap();

        write_file(
            dir.path(),
            "openspec/changes/archive/2026-01-01-aaa/proposal.md",
            "x",
        );
        stage_and_commit_at(
            &repo,
            "archive aaa",
            &["openspec/changes/archive/2026-01-01-aaa/proposal.md"],
            1_000_000,
        );

        write_file(
            dir.path(),
            "openspec/changes/archive/2026-01-01-bbb/proposal.md",
            "y",
        );
        stage_and_commit_at(
            &repo,
            "archive bbb",
            &["openspec/changes/archive/2026-01-01-bbb/proposal.md"],
            1_000_100,
        );

        let changes = Changes::discover(dir.path()).unwrap();

        assert_eq!(
            changes.archived,
            vec![
                Change("archive/2026-01-01-bbb".to_string()),
                Change("archive/2026-01-01-aaa".to_string()),
            ]
        );
    }

    #[test]
    fn discover_archived_unresolvable_introducing_commit_sorts_first() {
        let dir = TempDir::new("discover-unresolvable-sorts-first");
        let repo = Repository::init(dir.path()).unwrap();

        write_file(
            dir.path(),
            "openspec/changes/archive/2026-01-01-committed/proposal.md",
            "x",
        );
        stage_and_commit(
            &repo,
            "archive committed",
            &["openspec/changes/archive/2026-01-01-committed/proposal.md"],
        );

        // Present in the working tree but never committed.
        write_file(
            dir.path(),
            "openspec/changes/archive/2026-01-01-uncommitted/proposal.md",
            "y",
        );

        let changes = Changes::discover(dir.path()).unwrap();

        assert_eq!(
            changes.archived,
            vec![
                Change("archive/2026-01-01-uncommitted".to_string()),
                Change("archive/2026-01-01-committed".to_string()),
            ]
        );
    }

    #[test]
    fn discover_archived_tied_on_date_and_introducing_commit_falls_back_to_dirname_ascending() {
        let dir = TempDir::new("discover-dirname-tiebreak");
        let repo = Repository::init(dir.path()).unwrap();

        // Both directories are introduced in the same commit, so both date
        // and introducing-commit timestamp are tied.
        write_file(
            dir.path(),
            "openspec/changes/archive/2026-01-01-bbb/proposal.md",
            "x",
        );
        write_file(
            dir.path(),
            "openspec/changes/archive/2026-01-01-aaa/proposal.md",
            "y",
        );
        stage_and_commit(
            &repo,
            "archive both",
            &[
                "openspec/changes/archive/2026-01-01-bbb/proposal.md",
                "openspec/changes/archive/2026-01-01-aaa/proposal.md",
            ],
        );

        let changes = Changes::discover(dir.path()).unwrap();

        assert_eq!(
            changes.archived,
            vec![
                Change("archive/2026-01-01-aaa".to_string()),
                Change("archive/2026-01-01-bbb".to_string()),
            ]
        );
    }

    #[test]
    fn discover_archived_keeps_change_with_unresolvable_introducing_commit() {
        let dir = TempDir::new("discover-unresolvable-not-dropped");
        Repository::init(dir.path()).unwrap();

        // Present in the working tree but never committed - no enclosing
        // commit history to resolve a timestamp from.
        write_file(
            dir.path(),
            "openspec/changes/archive/2026-01-01-uncommitted/proposal.md",
            "x",
        );

        let changes = Changes::discover(dir.path()).unwrap();

        assert!(
            changes
                .archived
                .contains(&Change("archive/2026-01-01-uncommitted".to_string()))
        );
    }

    #[test]
    fn archived_change_diff_base_survives_commits_made_after_discovery() {
        let dir = TempDir::new("snapshot-later-commits");
        let repo = Repository::init(dir.path()).unwrap();

        write_file(dir.path(), "openspec/specs/cap/spec.md", "v1");
        stage_and_commit(&repo, "spec v1", &["openspec/specs/cap/spec.md"]);
        write_file(
            dir.path(),
            "openspec/changes/archive/2026-01-01-old-thing/proposal.md",
            "x",
        );
        stage_and_commit(
            &repo,
            "archive old-thing",
            &["openspec/changes/archive/2026-01-01-old-thing/proposal.md"],
        );

        let changes = Changes::discover(dir.path()).unwrap();
        let change = changes.archived[0].clone();
        let before = changes.resolve(&change).unwrap();

        // Commits made after discovery - including one touching the spec of
        // record - must not move the already-resolved diff base.
        write_file(
            dir.path(),
            "openspec/specs/cap/spec.md",
            "v2-after-discovery",
        );
        stage_and_commit(
            &repo,
            "spec v2, after discovery",
            &["openspec/specs/cap/spec.md"],
        );

        let after = changes.resolve(&change).unwrap();
        assert_eq!(before.diff_base, after.diff_base);
    }

    #[test]
    fn archived_change_uncommitted_at_discovery_stays_unresolvable_once_committed() {
        let dir = TempDir::new("snapshot-uncommitted-then-committed");
        let repo = Repository::init(dir.path()).unwrap();

        write_file(dir.path(), "openspec/specs/cap/spec.md", "v1");
        stage_and_commit(&repo, "spec v1", &["openspec/specs/cap/spec.md"]);

        // Present in the working tree but not committed at discovery time.
        write_file(
            dir.path(),
            "openspec/changes/archive/2026-01-01-uncommitted/proposal.md",
            "x",
        );

        let changes = Changes::discover(dir.path()).unwrap();
        let change = Change("archive/2026-01-01-uncommitted".to_string());
        assert!(changes.archived.contains(&change));
        assert!(changes.resolve(&change).is_err());

        // Committing it mid-session must not retroactively resolve it: the
        // discovery result is a snapshot.
        stage_and_commit(
            &repo,
            "commit the previously-uncommitted change",
            &["openspec/changes/archive/2026-01-01-uncommitted/proposal.md"],
        );

        assert!(changes.archived.contains(&change));
        assert!(matches!(
            changes.resolve(&change),
            Err(ChangesError::ArchiveHistoryNotFound(_))
        ));
    }

    #[test]
    fn fresh_discover_resolves_against_newer_history() {
        let dir = TempDir::new("snapshot-fresh-discover");
        let repo = Repository::init(dir.path()).unwrap();

        write_file(dir.path(), "openspec/specs/cap/spec.md", "v1");
        stage_and_commit(&repo, "spec v1", &["openspec/specs/cap/spec.md"]);
        write_file(
            dir.path(),
            "openspec/changes/archive/2026-01-01-uncommitted/proposal.md",
            "x",
        );

        let stale = Changes::discover(dir.path()).unwrap();
        let change = Change("archive/2026-01-01-uncommitted".to_string());
        assert!(stale.resolve(&change).is_err());

        stage_and_commit(
            &repo,
            "commit the change",
            &["openspec/changes/archive/2026-01-01-uncommitted/proposal.md"],
        );

        let fresh = Changes::discover(dir.path()).unwrap();
        assert!(fresh.resolve(&change).is_ok());
    }

    #[test]
    fn archived_change_introduced_by_root_commit_sorts_by_it_and_errors_on_resolve() {
        let dir = TempDir::new("root-commit-introduction");
        let repo = Repository::init(dir.path()).unwrap();

        // The root commit both creates the repo and introduces the archived
        // change - there is no earlier commit to use as a diff base.
        write_file(
            dir.path(),
            "openspec/changes/archive/2026-01-01-root-thing/proposal.md",
            "x",
        );
        stage_and_commit_at(
            &repo,
            "root commit introduces change",
            &["openspec/changes/archive/2026-01-01-root-thing/proposal.md"],
            1_000_000,
        );

        write_file(
            dir.path(),
            "openspec/changes/archive/2026-01-01-later-thing/proposal.md",
            "y",
        );
        stage_and_commit_at(
            &repo,
            "archive later-thing",
            &["openspec/changes/archive/2026-01-01-later-thing/proposal.md"],
            1_000_100,
        );

        let changes = Changes::discover(dir.path()).unwrap();

        // Same archive date, so the tiebreak is introducing-commit
        // timestamp, most recent first: later-thing (root-thing has no
        // parent, so its diff base cannot be resolved).
        assert_eq!(
            changes.archived,
            vec![
                Change("archive/2026-01-01-later-thing".to_string()),
                Change("archive/2026-01-01-root-thing".to_string()),
            ]
        );

        let root_change = Change("archive/2026-01-01-root-thing".to_string());
        assert!(matches!(
            changes.resolve(&root_change),
            Err(ChangesError::ArchiveHistoryNotFound(_))
        ));
    }
}
