//! Documentation stewardship domain.
//!
//! The domain publishes atomic capabilities and the docs freshness PDS
//! vocabulary. Generic Strategy, planning, task-network, and dispatch code
//! consumes those contracts without knowing documentation semantics.

pub mod capability;
pub mod claim_validation;
pub mod pds;
