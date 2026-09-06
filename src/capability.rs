//! Shared capability contracts and registration.
//!
//! This domain defines the task-facing capability model.
//! Owning domains publish typed contracts through this surface so task code can
//! bind, validate, and invoke capabilities without reaching into domain internals.

pub mod catalog;
pub mod contracts;
pub mod contribution;
pub mod runtime;

use crate::error::ApiError;
use crate::execution::ExecutionRuntimeContext;

pub use catalog::CapabilityCatalog;
pub use contracts::{
    ArtifactSchemaVersionRange, BindingSpec, BindingValueKind, BoundBindingValue,
    BoundCapabilityInstance, BoundInputWiring, BoundInputWiringSource, CapabilityTypeContract,
    EffectKind, EffectSpec, ExecutionClass, ExecutionContract, InputCardinality, InputSlotSpec,
    OutputSlotSpec, ScopeContract,
};
pub use contribution::*;
pub use meld_execution::capability::{CapabilityInvocationResult, CapabilityInvoker};
pub use runtime::{
    ArtifactValueRef, CapabilityExecutionContext, CapabilityInvocationPayload,
    CapabilityRuntimeInit, InputValueSource, SuppliedInputValue, SuppliedValueRef, UpstreamLineage,
};

pub type CapabilityExecutorRegistry =
    meld_execution::capability::CapabilityExecutorRegistry<ApiError, dyn ExecutionRuntimeContext>;

/// Publish every capability contract implemented by this product binary.
///
/// This is implementation inventory, not stewardship expression dispatch.
/// Initialization selects exact entries through the installed Strategy
/// package and runtime activation requires matching invokers.
pub fn published_product_contracts() -> Vec<CapabilityTypeContract> {
    product_capability_inventory()
        .map(|inventory| {
            inventory
                .contracts()
                .map(|revision| revision.contract.clone())
                .collect()
        })
        .unwrap_or_default()
}

/// Build the deterministic compiled product capability inventory.
pub fn product_capability_inventory(
) -> Result<ProductCapabilityInventory, CapabilityContributionDiagnostic> {
    ProductCapabilityInventory::assemble(vec![
        std::sync::Arc::new(crate::nonce::capability::NonceCapabilityContributor),
        std::sync::Arc::new(crate::docs::contribution::DocsCapabilityContributor::shipped()),
        std::sync::Arc::new(
            crate::dependency_security::capability::DependencySecurityCapabilityContributor,
        ),
    ])
}
