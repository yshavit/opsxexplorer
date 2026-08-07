use git2::{Repository, Sort};

use crate::vfs::GitRef;

use super::{Change, ChangesError};

/// Resolves the diff base for an archived change: the commit immediately
/// before the earliest commit (reachable from HEAD) that introduced any file
/// under the change's directory.
///
/// Real history can touch that directory more than once (files added across
/// a few commits, later corrections); anchoring to the earliest such commit
/// sidesteps having to identify one single "the archiving commit".
pub fn resolve_archive_base(repo: &Repository, change: &Change) -> Result<GitRef, ChangesError> {
    let path = change.relative_path();

    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;
    revwalk.set_sorting(Sort::TIME | Sort::REVERSE)?;

    for oid in revwalk {
        let oid = oid?;
        let commit = repo.find_commit(oid)?;
        let tree = commit.tree()?;
        if tree.get_path(&path).is_ok() {
            return match commit.parent_id(0) {
                Ok(oid) => Ok(GitRef { oid }),
                Err(_) => Err(ChangesError::ArchiveHistoryNotFound(change.clone())),
            };
        }
    }

    Err(ChangesError::ArchiveHistoryNotFound(change.clone()))
}
