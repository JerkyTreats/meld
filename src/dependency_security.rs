//! Bounded fixture-backed dependency-security truth domain.

pub mod adapter;
pub mod admission;
pub mod advisory;
pub mod assessment;
pub mod capability;
pub mod contracts;
pub mod events;
pub mod inventory;
pub mod policy;
pub mod theory;
pub mod verification;

pub use contracts::*;
