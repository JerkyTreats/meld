//! Documentation stewardship domain.
//!
//! The domain publishes atomic capability implementations and claim-policy
//! revisions. Installed Strategy, planning, task-network, and dispatch code
//! consumes their exact contracts without knowing documentation semantics.

pub mod capability;
pub mod claim_validation;
/// Regression fixture for the retired hand-composed docs image.
#[cfg(test)]
pub mod pds;
