use std::fmt;
use std::path::PathBuf;

use crate::vfs::FsError;

/// Where a [`SpecError::Structure`] occurred, structurally rather than by
/// line number: `md_elem` drops source positions during parsing, so a byte
/// or line offset is not available. See `2026-08-08-spec-model/design.md`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Location {
    /// The `## <OP> Requirements` (or `## Requirements`) section the failure occurred under.
    pub section: String,
    /// The requirement the failure occurred under, if known.
    pub requirement: Option<String>,
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.requirement {
            Some(requirement) => write!(f, "under `{}`, requirement {requirement:?}", self.section),
            None => write!(f, "under `{}`", self.section),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StructureErrorKind {
    /// A scenario heading appeared before any requirement heading.
    ScenarioBeforeRequirement,
    /// A `## <OP> Requirements` section whose operation is not one of the four recognised ones.
    UnrecognisedOperationSection,
    /// A `- FROM:` bullet in a RENAMED section with no matching `- TO:`.
    RenameMissingTarget,
}

impl fmt::Display for StructureErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StructureErrorKind::ScenarioBeforeRequirement => {
                write!(f, "scenario heading appears before any requirement heading")
            }
            StructureErrorKind::UnrecognisedOperationSection => {
                write!(f, "unrecognised requirements section")
            }
            StructureErrorKind::RenameMissingTarget => {
                write!(f, "rename is missing its `- TO:` target")
            }
        }
    }
}

#[derive(Debug)]
pub enum SpecError {
    Structure {
        path: PathBuf,
        at: Location,
        kind: StructureErrorKind,
    },
    Markdown {
        path: PathBuf,
        source: Box<mdq::md_elem::InvalidMd>,
    },
    MissingSpecDocument {
        capability: String,
    },
    MissingBaseSpec {
        capability: String,
        requirement: String,
    },
    Fs(FsError),
}

impl fmt::Display for SpecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SpecError::Structure { path, at, kind } => {
                write!(f, "{}: {kind} ({at})", path.display())
            }
            SpecError::Markdown { path, source } => {
                write!(f, "{}: malformed markdown: {source:?}", path.display())
            }
            SpecError::MissingSpecDocument { capability } => {
                write!(f, "capability {capability:?} has no spec.md")
            }
            SpecError::MissingBaseSpec {
                capability,
                requirement,
            } => write!(
                f,
                "capability {capability:?} has no spec of record at the diff base, \
                 but requirement {requirement:?} needs one"
            ),
            SpecError::Fs(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for SpecError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SpecError::Fs(e) => Some(e),
            SpecError::Structure { .. }
            | SpecError::Markdown { .. }
            | SpecError::MissingSpecDocument { .. }
            | SpecError::MissingBaseSpec { .. } => None,
        }
    }
}

impl From<FsError> for SpecError {
    fn from(e: FsError) -> Self {
        SpecError::Fs(e)
    }
}
