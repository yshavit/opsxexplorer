mod change;
mod discovery;
mod error;
mod history;

pub use change::{Change, ChangeView, DiffBase};
pub use error::ChangesError;

use std::path::Path;

use git2::Repository;

use crate::vfs::{Fs, FsError, Workspace};

pub struct Changes {
    vfs: Workspace,
    repo: Option<Repository>,
    pub active: Vec<Change>,
    pub archived: Vec<Change>,
}

impl Changes {
    pub fn discover(start: &Path) -> Result<Changes, FsError> {
        let vfs = Workspace::open(start)?;
        let repo = Repository::discover(start).ok();
        let current = vfs.current();
        let active = discovery::discover_active(&current)?;
        let archived = discovery::discover_archived(&current)?;
        Ok(Changes {
            vfs,
            repo,
            active,
            archived,
        })
    }

    pub fn resolve(&self, change: &Change) -> Result<ChangeView, ChangesError> {
        let diff_base = if change.is_archived() {
            let repo = self.repo.as_ref().ok_or(FsError::NotAGitRepo)?;
            DiffBase::At(history::resolve_archive_base(repo, change)?)
        } else {
            DiffBase::Current
        };
        Ok(ChangeView {
            change: change.clone(),
            diff_base,
        })
    }

    pub fn open<'a>(&'a self, view: &ChangeView) -> Result<Fs<'a>, FsError> {
        match view.diff_base {
            DiffBase::Current => Ok(self.vfs.current()),
            DiffBase::At(r) => self.vfs.at(&r),
        }
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

        let fs = changes.open(&view).unwrap();
        assert_eq!(
            fs.read(Path::new("openspec/specs/cap/spec.md")).unwrap(),
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

        let fs = changes.open(&view).unwrap();
        assert_eq!(
            fs.read(Path::new("openspec/specs/cap/spec.md")).unwrap(),
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
        write_file(dir.path(), "openspec/specs/cap/spec.md", "v2-after-correction");
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

        let fs = changes.open(&view).unwrap();
        assert_eq!(
            fs.read(Path::new("openspec/specs/cap/spec.md")).unwrap(),
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

        let fs = changes.open(&view).unwrap();
        assert_eq!(
            fs.read(Path::new("openspec/specs/cap/spec.md")).unwrap(),
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
            let fs = changes.open(v).unwrap();
            assert_eq!(
                fs.read(Path::new("openspec/specs/cap/spec.md")).unwrap(),
                b"v1"
            );
        }
    }
}
