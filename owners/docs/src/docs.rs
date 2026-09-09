//! Documentation stewardship domain.
//!
//! The domain publishes atomic capability implementations and claim-policy
//! revisions. Installed Strategy, planning, task-network, and dispatch code
//! consumes their exact contracts without knowing documentation semantics.

pub mod capability;
pub mod claim_observation;
pub mod claim_validation;
pub mod contribution;
pub mod correspondence;
pub mod input_basis;
pub mod judgment;
pub mod observation;
pub mod observation_store;
pub mod owner;
/// Regression fixture for the retired hand-composed docs image.
#[cfg(test)]
pub mod pds;
pub mod publication;
pub(crate) mod publication_return;
pub(crate) mod runtime;
pub mod scope;
pub mod semantics;
pub mod source_claims;
pub mod theory;
