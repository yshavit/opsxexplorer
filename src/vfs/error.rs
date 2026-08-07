use std::fmt;
use std::path::PathBuf;

#[derive(Debug)]
pub enum FsError {
    NotFound(PathBuf),
    NotADirectory(PathBuf),
    EscapesRoot(PathBuf),
    Unsupported { path: PathBuf, reason: &'static str },
    NotAGitRepo,
    BareRepositoryUnsupported,
    Io(std::io::Error),
    Git(git2::Error),
}

impl fmt::Display for FsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FsError::NotFound(p) => write!(f, "not found: {}", p.display()),
            FsError::NotADirectory(p) => write!(f, "not a directory: {}", p.display()),
            FsError::EscapesRoot(p) => write!(f, "path escapes root: {}", p.display()),
            FsError::Unsupported { path, reason } => {
                write!(f, "unsupported entry at {}: {reason}", path.display())
            }
            FsError::NotAGitRepo => write!(f, "not a git repository"),
            FsError::BareRepositoryUnsupported => {
                write!(f, "bare repositories are not supported")
            }
            FsError::Io(e) => write!(f, "{e}"),
            FsError::Git(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for FsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FsError::Io(e) => Some(e),
            FsError::Git(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for FsError {
    fn from(e: std::io::Error) -> Self {
        FsError::Io(e)
    }
}

impl From<git2::Error> for FsError {
    fn from(e: git2::Error) -> Self {
        FsError::Git(e)
    }
}
