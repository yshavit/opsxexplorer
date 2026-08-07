use git2::{Oid, Repository};

use super::error::FsError;

/// Resolved once at construction, so it stays pinned even if the revspec it came from later moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GitRef {
    pub oid: Oid,
}

impl GitRef {
    pub fn resolve(repo: &Repository, revspec: &str) -> Result<GitRef, FsError> {
        let object = repo.revparse_single(revspec)?;
        let commit = object.peel_to_commit()?;
        Ok(GitRef { oid: commit.id() })
    }
}
