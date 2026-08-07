use std::path::Path;

use crate::vfs::{EntryKind, Fs, FsError};

use super::Change;

const CHANGES_DIR: &str = "openspec/changes";
const ARCHIVE_DIR: &str = "openspec/changes/archive";

pub fn discover_active(fs: &Fs) -> Result<Vec<Change>, FsError> {
    let entries = fs.list_dir(Path::new(CHANGES_DIR))?;
    Ok(entries
        .into_iter()
        .filter(|e| e.kind == EntryKind::Dir && e.name != "archive")
        .map(|e| Change(e.name))
        .collect())
}

pub fn discover_archived(fs: &Fs) -> Result<Vec<Change>, FsError> {
    if !fs.exists(Path::new(ARCHIVE_DIR)) {
        return Ok(Vec::new());
    }
    let entries = fs.list_dir(Path::new(ARCHIVE_DIR))?;
    Ok(entries
        .into_iter()
        .filter(|e| e.kind == EntryKind::Dir)
        .map(|e| Change(format!("archive/{}", e.name)))
        .collect())
}
