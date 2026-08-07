use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::error::FsError;
use super::normalize_relative;
use super::{DirEntry, EntryKind};

#[derive(Debug, Clone)]
pub struct DiskFs {
    root: PathBuf,
}

impl DiskFs {
    pub(crate) fn new(root: PathBuf) -> Self {
        DiskFs { root }
    }

    pub fn read(&self, path: &Path) -> Result<Vec<u8>, FsError> {
        let rel = normalize_relative(path)?;
        let full = self.root.join(&rel);
        fs::read(&full).map_err(|e| io_error(e, &rel))
    }

    pub fn list_dir(&self, path: &Path) -> Result<Vec<DirEntry>, FsError> {
        let rel = normalize_relative(path)?;
        let full = self.root.join(&rel);
        let read_dir = fs::read_dir(&full).map_err(|e| io_error(e, &rel))?;

        let mut entries = Vec::new();
        for entry in read_dir {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let kind = if fs::metadata(entry.path())?.is_dir() {
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
        match normalize_relative(path) {
            Ok(rel) => self.root.join(rel).exists(),
            Err(_) => false,
        }
    }
}

fn io_error(e: io::Error, path: &Path) -> FsError {
    if e.kind() == io::ErrorKind::NotFound {
        FsError::NotFound(path.to_path_buf())
    } else {
        FsError::Io(e)
    }
}
