use std::path::{Path, PathBuf};

use crate::vfs::GitRef;

const CHANGES_DIR: &str = "openspec/changes";
const ARCHIVE_PREFIX: &str = "archive/";
const DATE_PREFIX_LEN: usize = "YYYY-MM-DD".len();

/// An OpenSpec change, identified by its path relative to `openspec/changes/`.
///
/// For an active change this is just the change name (e.g. `"change-modeling"`).
/// For an archived change it includes the `archive/` segment and the
/// `${date}-${change-name}` directory naming convention (e.g.
/// `"archive/2026-08-07-add-readonly-filesystem"`), so active/archived status,
/// the on-disk path, and (for archived changes) the display name and archive
/// date are all derivable from this one value.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Change(pub String);

impl Change {
    pub fn is_archived(&self) -> bool {
        self.0.starts_with(ARCHIVE_PREFIX)
    }

    /// Path relative to the repository root.
    pub fn relative_path(&self) -> PathBuf {
        Path::new(CHANGES_DIR).join(&self.0)
    }

    /// The change's name without the `archive/` and `${date}-` segments, if any.
    pub fn display_name(&self) -> &str {
        match self.archived_tail() {
            Some(tail) => strip_date_prefix(tail),
            None => &self.0,
        }
    }

    /// The `${date}` segment of an archived change's directory name, if present and well-formed.
    pub fn archive_date(&self) -> Option<&str> {
        let tail = self.archived_tail()?;
        let date = tail.get(..DATE_PREFIX_LEN)?;
        (tail.as_bytes().get(DATE_PREFIX_LEN) == Some(&b'-')).then_some(date)
    }

    fn archived_tail(&self) -> Option<&str> {
        self.0.strip_prefix(ARCHIVE_PREFIX)
    }
}

fn strip_date_prefix(tail: &str) -> &str {
    if tail.len() > DATE_PREFIX_LEN && tail.as_bytes().get(DATE_PREFIX_LEN) == Some(&b'-') {
        &tail[DATE_PREFIX_LEN + 1..]
    } else {
        tail
    }
}

/// How to resolve the state of the spec of record to diff a change's delta specs against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffBase {
    /// The live working tree — used for active changes.
    Current,
    /// A specific, previously-resolved commit — used for archived changes.
    At(GitRef),
}

/// A change paired with its resolved diff base, so the two travel together
/// without the holder needing to re-check whether the change is active or archived.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeView {
    pub change: Change,
    pub diff_base: DiffBase,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_change_display_name_and_date() {
        let change = Change("change-modeling".to_string());
        assert!(!change.is_archived());
        assert_eq!(change.display_name(), "change-modeling");
        assert_eq!(change.archive_date(), None);
        assert_eq!(
            change.relative_path(),
            Path::new("openspec/changes/change-modeling")
        );
    }

    #[test]
    fn archived_change_display_name_and_date() {
        let change = Change("archive/2026-08-07-add-readonly-filesystem".to_string());
        assert!(change.is_archived());
        assert_eq!(change.display_name(), "add-readonly-filesystem");
        assert_eq!(change.archive_date(), Some("2026-08-07"));
        assert_eq!(
            change.relative_path(),
            Path::new("openspec/changes/archive/2026-08-07-add-readonly-filesystem")
        );
    }

    #[test]
    fn archived_change_with_malformed_name_falls_back_gracefully() {
        let change = Change("archive/notadate123456".to_string());
        assert!(change.is_archived());
        assert_eq!(change.archive_date(), None);
        assert_eq!(change.display_name(), "notadate123456");
    }
}
