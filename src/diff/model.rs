use std::ops::Range;

use crate::diff::error::DiffError;
use crate::specs::UnrecognizedSection;

/// A span of text marked as unchanged, deleted or inserted. Offsets are byte
/// ranges into the two body strings `spec-model` supplied, never into the
/// source file (see `2026-08-08-spec-diff/design.md`).
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
    /// The two texts are too dissimilar for an inline diff to read better
    /// than the two texts in turn (see `2026-08-08-diff-legibility/design.md`).
    /// Carries both texts in full and no runs.
    Replaced {
        base: String,
        delta: String,
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
    Removed { note: String },
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
    /// The capability's purpose comparison, if there is anything to report:
    /// only ever `Added`, `Changed` or `Replaced` (see `diff()`).
    pub purpose: Option<Piece>,
    /// `##` sections the *delta's* parser did not recognise, carried through
    /// unchanged and in the same order.
    pub delta_unrecognized_sections: Vec<UnrecognizedSection>,
    /// `##` sections the *spec of record's* parser did not recognise, carried
    /// through the same way. Kept separate from the delta's rather than
    /// merged: the two come from different documents and mean different
    /// things to a reader, so the render layer needs to tell them apart
    /// without re-deriving the origin (see this change's design.md).
    pub base_unrecognized_sections: Vec<UnrecognizedSection>,
}
