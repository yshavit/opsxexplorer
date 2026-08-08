use std::fmt;

/// A delta entry naming a requirement absent from the spec of record: a
/// mistyped heading or a hand-done rename, distinct from
/// [`crate::specs::SpecError::MissingBaseSpec`], which is an absent spec of
/// record entirely.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffError {
    MissingBaseRequirement {
        capability: String,
        requirement: String,
    },
}

impl fmt::Display for DiffError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiffError::MissingBaseRequirement {
                capability,
                requirement,
            } => write!(
                f,
                "capability {capability:?} has no requirement named {requirement:?} \
                 in its spec of record"
            ),
        }
    }
}

impl std::error::Error for DiffError {}
