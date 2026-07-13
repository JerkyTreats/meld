//! Source-format-neutral product activation contracts for execution.
//!
//! Root assembly may load external configuration and domain-owned authored
//! assets, then pass this typed package into the pure validation boundary.
//! Execution activation validation never opens or mutates semantic stores.

mod binding;
mod contracts;
mod registry;
mod validation;

pub use binding::{
    bind_execution_activation, ExecutionActivationAssetDigests, ExecutionActivationAssets,
    ExecutionActivationBindingError, ExecutionRuntimeAssets,
};
pub use contracts::{
    CapabilityContractRef, ExecutionActivationInput, ExecutionActivationSelection,
    ExecutionActivationValidationContext, ExecutionActivationValidationReceipt,
    ExecutionForcePolicy, ExecutionTargetKind, ExecutionTargetSelector, MethodTaskPackageBinding,
    PublicationMapping, RequiredArtifactContract,
};
pub use registry::{bind_builtin_execution_activation, BuiltInExecutionActivationRegistry};
pub use validation::{validate_execution_activation, ExecutionActivationValidationError};
