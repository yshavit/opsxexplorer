use std::fmt;

use crate::vfs::FsError;

use super::Change;

#[derive(Debug)]
pub enum ChangesError {
    Fs(FsError),
    Git(git2::Error),
    /// No commit reachable from HEAD introduces the change's directory, or that
    /// commit has no parent to use as a diff base.
    ArchiveHistoryNotFound(Change),
}

impl fmt::Display for ChangesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChangesError::Fs(e) => write!(f, "{e}"),
            ChangesError::Git(e) => write!(f, "{e}"),
            ChangesError::ArchiveHistoryNotFound(change) => {
                write!(f, "could not find a diff base commit for {}", change.0)
            }
        }
    }
}

impl std::error::Error for ChangesError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ChangesError::Fs(e) => Some(e),
            ChangesError::Git(e) => Some(e),
            ChangesError::ArchiveHistoryNotFound(_) => None,
        }
    }
}

impl From<FsError> for ChangesError {
    fn from(e: FsError) -> Self {
        ChangesError::Fs(e)
    }
}

impl From<git2::Error> for ChangesError {
    fn from(e: git2::Error) -> Self {
        ChangesError::Git(e)
    }
}
