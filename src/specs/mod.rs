mod error;
mod load;
mod model;
pub(crate) mod parse;

pub use error::SpecError;
pub use load::{capabilities, load};
pub use model::{DeltaEntry, DeltaOp, Requirement, SpecPair};
