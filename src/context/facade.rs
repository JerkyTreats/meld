//! Context facade: re-exports and optional single entrypoint for building context services.
//! Query and mutation contracts are available through this module.

pub use crate::context::frame::{Basis, Frame, FrameMerkleSet, FrameStorage};
pub use crate::context::head::CurrentFrameHeadRead;
pub use crate::context::query::{ContextView, ContextViewBuilder, NodeContext};
pub use crate::context::types::{CompactResult, RestoreResult, TombstoneResult};

/// Placeholder for a single entrypoint that builds a context service from dependencies.
/// Will be expanded when query and mutation are extracted and dependency injection is wired.
pub struct ContextFacade;
