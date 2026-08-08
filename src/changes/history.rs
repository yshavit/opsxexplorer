use std::path::Path;

use git2::{Oid, Repository, Sort};

use crate::vfs::GitRef;

use super::{Change, ChangesError};

/// The earliest commit (reachable from HEAD) that introduced any file under a
/// given path, along with its timestamp.
struct CommitInfo {
    oid: Oid,
    time: i64,
}

/// Finds the earliest commit (reachable from HEAD) whose tree contains `path`.
/// Returns `None` if the path is never found, or on any revwalk/git error.
fn first_commit(repo: &Repository, path: &Path) -> Option<CommitInfo> {
    let mut revwalk = repo.revwalk().ok()?;
    revwalk.push_head().ok()?;
    revwalk.set_sorting(Sort::TIME | Sort::REVERSE).ok()?;

    for oid in revwalk {
        let oid = oid.ok()?;
        let commit = repo.find_commit(oid).ok()?;
        let tree = commit.tree().ok()?;
        if tree.get_path(path).is_ok() {
            return Some(CommitInfo {
                oid,
                time: commit.time().seconds(),
            });
        }
    }

    None
}

/// Resolves the diff base for an archived change: the commit immediately
/// before the earliest commit (reachable from HEAD) that introduced any file
/// under the change's directory.
///
/// Real history can touch that directory more than once (files added across
/// a few commits, later corrections); anchoring to the earliest such commit
/// sidesteps having to identify one single "the archiving commit".
pub fn resolve_archive_base(repo: &Repository, change: &Change) -> Result<GitRef, ChangesError> {
    let path = change.relative_path();

    let commit_info = first_commit(repo, &path)
        .ok_or_else(|| ChangesError::ArchiveHistoryNotFound(change.clone()))?;

    match repo
        .find_commit(commit_info.oid)
        .ok()
        .and_then(|c| c.parent_id(0).ok())
    {
        Some(oid) => Ok(GitRef { oid }),
        None => Err(ChangesError::ArchiveHistoryNotFound(change.clone())),
    }
}

/// The timestamp of the commit that first introduced `change`'s directory in
/// git history. Returns `None` on any failure to resolve it (path never
/// committed, revwalk/git error) — infallible from the caller's perspective,
/// suited for use as a sort tiebreaker rather than an error condition.
pub fn first_commit_time(repo: &Repository, change: &Change) -> Option<i64> {
    first_commit(repo, &change.relative_path()).map(|c| c.time)
}

#[cfg(test)]
mod tests {
    use super::super::test_support::*;
    use super::*;
    use git2::Repository;

    #[test]
    fn first_commit_time_is_the_introducing_commits_own_timestamp() {
        let dir = TempDir::new("history-first-commit-time");
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
        let introducing_time = repo
            .head()
            .unwrap()
            .peel_to_commit()
            .unwrap()
            .time()
            .seconds();

        // A later commit also touches the same directory; it must not shift the timestamp.
        write_file(
            dir.path(),
            "openspec/changes/archive/2026-01-01-old-thing/proposal.md",
            "x-corrected",
        );
        stage_and_commit(
            &repo,
            "fix typo",
            &["openspec/changes/archive/2026-01-01-old-thing/proposal.md"],
        );

        let change = Change("archive/2026-01-01-old-thing".to_string());
        assert_eq!(first_commit_time(&repo, &change), Some(introducing_time));
    }

    #[test]
    fn first_commit_time_is_none_for_never_committed_change() {
        let dir = TempDir::new("history-first-commit-time-uncommitted");
        let repo = Repository::init(dir.path()).unwrap();

        write_file(dir.path(), "openspec/specs/cap/spec.md", "v1");
        stage_and_commit(&repo, "spec v1", &["openspec/specs/cap/spec.md"]);

        let change = Change("archive/2026-01-01-never-committed".to_string());
        assert_eq!(first_commit_time(&repo, &change), None);
    }

    #[test]
    fn first_commit_time_resolves_for_root_commit_introduction() {
        let dir = TempDir::new("history-first-commit-time-root");
        let repo = Repository::init(dir.path()).unwrap();

        write_file(
            dir.path(),
            "openspec/changes/archive/2026-01-01-old-thing/proposal.md",
            "x",
        );
        stage_and_commit(
            &repo,
            "root commit introduces change",
            &["openspec/changes/archive/2026-01-01-old-thing/proposal.md"],
        );
        let root_time = repo
            .head()
            .unwrap()
            .peel_to_commit()
            .unwrap()
            .time()
            .seconds();

        let change = Change("archive/2026-01-01-old-thing".to_string());
        assert_eq!(first_commit_time(&repo, &change), Some(root_time));

        // resolve_archive_base, by contrast, errors here: the root commit has no parent.
        assert!(resolve_archive_base(&repo, &change).is_err());
    }
}
