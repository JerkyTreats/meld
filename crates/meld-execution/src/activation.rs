//! Source-format-neutral product activation contracts for execution.
//!
//! Root assembly may load external configuration and domain-owned authored
//! assets, then pass this typed package into the pure validation boundary.
//! Execution activation validation never opens or mutates semantic stores.

mod contracts;
mod validation;

pub use contracts::{
    CapabilityContractRef, ExecutionActivationInput, ExecutionActivationSelection,
    ExecutionActivationValidationReceipt, ExecutionForcePolicy, ExecutionTargetKind,
    ExecutionTargetSelector, MethodTaskPackageBinding, PublicationMapping,
    RequiredArtifactContract,
};
pub use validation::{validate_execution_activation, ExecutionActivationValidationError};
