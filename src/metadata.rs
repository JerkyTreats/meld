//! Metadata domain contracts.
//!
//! Owns frame metadata types and frame write boundary checks.

pub mod frame_key_descriptor;
pub mod frame_key_registry;
pub mod frame_types;
pub mod frame_write_contract;
pub(crate) mod owned_frame_metadata_keys;
pub mod prompt_link_contract;

pub use frame_types::FrameMetadata;
