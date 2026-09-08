//! Thin public adapter for the Context owner service.

pub use crate::context::query::view::{ContextView, ContextViewBuilder, NodeContext};
pub use crate::context::service::ContextService as ContextApi;
pub use crate::context::types::{CompactResult, RestoreResult, TombstoneResult};
