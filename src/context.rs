//! Context domain: frame model, query, mutation, and bounded generation mechanisms.
//! Owns context behavior; CLI, agent adapter, and workspace watch consume via explicit contracts.

pub mod events;
pub mod frame;
pub(crate) mod frame_metadata_keys;
pub mod generation;
pub mod head;
pub mod query;
pub mod summary;
pub mod tooling;
pub mod types;

pub use frame::{Basis, Frame, FrameMerkleSet, FrameStorage};
pub use head::CurrentFrameHeadRead;
pub use types::{CompactResult, RestoreResult, TombstoneResult};

pub mod publication;

pub mod service;
pub use service::ContextService;
