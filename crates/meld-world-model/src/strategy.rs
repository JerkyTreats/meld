//! Pure Strategy construction over immutable world-model inputs.

mod contracts;
mod registry;
mod search;
mod verification;

pub use contracts::*;
pub use registry::*;
pub use search::{search, search_successor};
pub use verification::{verify_plan, verify_successor_plan};

#[cfg(test)]
mod tests;
