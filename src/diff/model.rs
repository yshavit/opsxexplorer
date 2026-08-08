use std::ops::Range;

use crate::diff::error::DiffError;

/// A span of text marked as unchanged, deleted or inserted. Offsets are byte
/// ranges into the two body strings `spec-model` supplied, never into the
/// source file (see design.md).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Run {
    Equal {
        base: Range<usize>,
        delta: Range<usize>,
    },
    Delete {
        base: Range<usize>,
    },
    Insert {
        delta: Range<usize>,
    },
}

/// A requirement's intro, or one scenario's body, under any operation. One
/// enum for every position so a consumer has a single set of match arms to
/// style.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Piece {
    Unchanged {
        text: String,
    },
    Changed {
        base: String,
        delta: String,
        runs: Vec<Run>,
    },
    Added {
        delta: String,
    },
    Deleted {
        base: String,
    },
    Unmentioned {
        base: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioDiff {
    pub name: String,
    pub body: Piece,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operation {
    Added,
    Modified,
    Removed,
    Renamed { from: String },
}

/// `name` is always the display name: for a rename that is the new name,
/// with the former name carried on `Operation::Renamed`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequirementDiff {
    pub name: String,
    pub op: Operation,
    pub intro: Piece,
    pub scenarios: Vec<ScenarioDiff>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityDiff {
    pub capability: String,
    pub requirements: Vec<RequirementDiff>,
    pub errors: Vec<DiffError>,
}
