use git2::Oid;

/// Resolved once at construction, so it stays pinned even if the revspec it came from later moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GitRef {
    pub oid: Oid,
}
