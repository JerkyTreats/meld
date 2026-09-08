//! Pure Strategy construction over immutable world-model inputs.

mod contracts;
mod registry;
mod search;
mod verification;

pub use contracts::*;
pub use registry::*;
pub(crate) use search::interrupted_history_entry;
pub use search::{search, search_successor};
pub use verification::{verify_plan, verify_successor_plan};

#[cfg(test)]
pub(crate) mod tests;

mod history;

mod rule_history;

#[cfg(test)]
pub(crate) use history::tests::old_wire as historical_plan_fixture;
