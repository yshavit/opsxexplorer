use std::path::{Path, PathBuf};

use git2::Repository;

use super::Fs;
use super::disk::DiskFs;
use super::error::FsError;
use super::git::GitTreeFs;
use super::git_ref::GitRef;

pub struct Workspace {
    repo: Option<Repository>,
    root: PathBuf,
}

impl Workspace {
    pub fn open(start: &Path) -> Result<Workspace, FsError> {
        match Repository::discover(start) {
            Ok(repo) => {
                let root = repo
                    .workdir()
                    .map(Path::to_path_buf)
                    .ok_or(FsError::BareRepositoryUnsupported)?;
                Ok(Workspace {
                    repo: Some(repo),
                    root,
                })
            }
            Err(_) => {
                let root = start.canonicalize()?;
                Ok(Workspace { repo: None, root })
            }
        }
    }

    pub fn current(&self) -> Fs<'_> {
        Fs::Disk(DiskFs::new(self.root.clone()))
    }

    pub fn at(&self, r: &GitRef) -> Result<Fs<'_>, FsError> {
        let repo = self.repo.as_ref().ok_or(FsError::NotAGitRepo)?;
        Ok(Fs::Git(GitTreeFs::new(repo, r.oid)))
    }
}
