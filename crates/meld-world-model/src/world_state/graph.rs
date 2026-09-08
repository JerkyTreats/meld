//! Canonical Graph projection of owner-published knowledge.
//!
//! Graph admits intact owner publications from one Event authority and provides
//! bounded traversal over explicit owner revision cuts. Owners define meaning;
//! Graph neither interprets foreign event payloads nor authors semantic facts.

pub mod admission;
pub mod contracts;
mod cursor;
pub mod events;
pub mod ports;
pub mod query;
pub mod reducer;
pub mod runtime;
pub mod store;
#[cfg(any(test, feature = "test-support"))]
pub mod test_support;
pub mod visibility;

pub use contracts::{PerspectiveKey, TraversalDirection};
pub use ports::{GraphConsumerCursorReporter, GraphEventReplaySource};
pub use query::TraversalQuery;

pub(crate) mod source_replay;
