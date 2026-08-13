//! Pure Strategy construction over immutable world-model inputs.

mod contracts;
mod registry;
mod search;
mod verification;

pub use contracts::*;
pub use registry::*;
pub use search::search;
pub use verification::verify_candidate;

#[cfg(test)]
mod tests;
