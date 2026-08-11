/// A single scenario within a requirement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scenario {
    pub name: String,
    pub body: String,
}

/// A requirement, as parsed from either a spec of record or a delta spec.
///
/// `intro` is a plain `String`, not `Option<String>`: a requirement with no
/// intro block and one with an empty intro block are indistinguishable in
/// the source (the only textual difference is a blank line, which carries
/// no authorial intent), so both parse to `""`. See
/// `2026-08-08-spec-model/design.md`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Requirement {
    pub name: String,
    pub intro: String,
    pub scenarios: Vec<Scenario>,
}

/// A `##` section the parser did not recognise, carried through with enough
/// of itself — title *and* rendered body — that a consumer can show what the
/// tool skipped rather than only that something was skipped. Produced
/// identically by both parsers; see
/// `2026-08-10-unrecognized-spec-sections/design.md` and
/// `2026-08-10-unrecognized-sections-in-base/design.md`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnrecognizedSection {
    pub title: String,
    pub body: String,
}

/// A capability's spec of record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spec {
    pub purpose: Option<String>,
    pub requirements: Vec<Requirement>,
    /// `##` sections the parser did not recognise, in document order.
    pub unrecognized_sections: Vec<UnrecognizedSection>,
}

/// The operation section a delta requirement was parsed from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeltaOp {
    Added,
    Modified,
    Removed,
}

/// A requirement tagged with the delta operation it was parsed from.
///
/// A `Removed` entry is a `DeltaEntry` like any other: its heading supplies
/// its name, and any body content following the heading — conventionally a
/// Reason and Migration explanation — is parsed into `intro` exactly as an
/// added or modified entry's body would be. Its scenarios and intro have no
/// bearing on the requirement's actual (deleted) content, which is instead
/// recovered from the base spec. See `2026-08-08-spec-model/design.md` and
/// `2026-08-10-spec-model-removals/design.md`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeltaEntry {
    pub op: DeltaOp,
    pub requirement: Requirement,
}

/// A RENAMED entry: a pairing of the name a requirement is renamed from to
/// the name it is renamed to. Kept as a separate list rather than a
/// `DeltaOp` variant, since it has no heading, body, or intro of its own —
/// folding it into `DeltaEntry` would leave those fields meaningless. See
/// `2026-08-08-spec-model/design.md`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rename {
    pub from: String,
    pub to: String,
}

/// A change's delta spec for one capability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Delta {
    pub purpose: Option<String>,
    pub entries: Vec<DeltaEntry>,
    pub renames: Vec<Rename>,
    /// `##` sections the parser did not recognise, in document order. See
    /// `2026-08-10-unrecognized-spec-sections/design.md`.
    pub unrecognized_sections: Vec<UnrecognizedSection>,
}

/// Both sides of a change-and-capability pair, as loaded by [`crate::specs::load`].
///
/// `base` is `None` when the delta introduces a capability with no spec of
/// record yet at the diff base (an all-ADDED delta). Every other case where
/// a base is required but absent is a [`crate::specs::SpecError::MissingBaseSpec`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecPair {
    pub delta: Delta,
    pub base: Option<Spec>,
}
