//! Standing epistemic Curation under exact Agent authority and Traversal input.
//!
//! Curation owns operation admission, terminal results, semantic authorship,
//! and publication recovery. It consumes Traversal through a public port and
//! publishes through Events. It never writes Graph, Belief, or Agent state.

mod actor;
mod contracts;
mod query;
mod selection;
mod store;
#[cfg(test)]
mod tests;
mod theory;

pub use actor::*;
pub use contracts::*;
pub use query::*;
pub use selection::*;
pub use store::*;
pub use theory::*;
