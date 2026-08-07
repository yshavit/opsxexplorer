use std::path::Path;

use git2::{Oid, Repository};

use super::error::FsError;
use super::normalize_relative;
use super::{DirEntry, EntryKind};

const MODE_LINK: i32 = 0o120000;
const MODE_TREE: i32 = 0o040000;

pub struct GitTreeFs<'repo> {
    repo: &'repo Repository,
    oid: Oid,
}

impl<'repo> GitTreeFs<'repo> {
    pub(crate) fn new(repo: &'repo Repository, oid: Oid) -> Self {
        GitTreeFs { repo, oid }
    }

    fn tree(&self) -> Result<git2::Tree<'repo>, FsError> {
        let commit = self.repo.find_commit(self.oid)?;
        Ok(commit.tree()?)
    }

    pub fn read(&self, path: &Path) -> Result<Vec<u8>, FsError> {
        let rel = normalize_relative(path)?;
        let tree = self.tree()?;
        let entry = tree
            .get_path(&rel)
            .map_err(|_| FsError::NotFound(rel.clone()))?;

        if entry.filemode() == MODE_LINK {
            return Err(FsError::Unsupported {
                path: rel,
                reason: "symlink",
            });
        }

        let object = entry.to_object(self.repo)?;
        let blob = object
            .as_blob()
            .ok_or_else(|| FsError::NotADirectory(rel.clone()))?;
        Ok(blob.content().to_vec())
    }

    pub fn list_dir(&self, path: &Path) -> Result<Vec<DirEntry>, FsError> {
        let rel = normalize_relative(path)?;
        let tree = self.tree()?;

        let subtree = if rel.as_os_str().is_empty() {
            tree
        } else {
            let entry = tree
                .get_path(&rel)
                .map_err(|_| FsError::NotFound(rel.clone()))?;
            let object = entry.to_object(self.repo)?;
            object
                .into_tree()
                .map_err(|_| FsError::NotADirectory(rel.clone()))?
        };

        let mut entries = Vec::new();
        for entry in subtree.iter() {
            let name = entry.name().unwrap_or_default().to_string();
            let kind = if entry.filemode() == MODE_TREE {
                EntryKind::Dir
            } else {
                EntryKind::File
            };
            entries.push(DirEntry { name, kind });
        }
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(entries)
    }

    pub fn exists(&self, path: &Path) -> bool {
        let Ok(rel) = normalize_relative(path) else {
            return false;
        };
        let Ok(tree) = self.tree() else {
            return false;
        };
        if rel.as_os_str().is_empty() {
            true
        } else {
            tree.get_path(&rel).is_ok()
        }
    }
}
