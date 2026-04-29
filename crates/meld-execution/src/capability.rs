//! Shared capability contracts and registration.

pub mod catalog;
pub mod contracts;
pub mod invocation;
pub mod runtime;

pub use catalog::CapabilityCatalog;
pub use contracts::{
    ArtifactSchemaVersionRange, BindingSpec, BindingValueKind, BoundBindingValue,
    BoundCapabilityInstance, BoundInputWiring, BoundInputWiringSource, CapabilityTypeContract,
    EffectKind, EffectSpec, ExecutionClass, ExecutionContract, InputCardinality, InputSlotSpec,
    OutputSlotSpec, ScopeContract,
};
pub use invocation::{CapabilityExecutorRegistry, CapabilityInvocationResult, CapabilityInvoker};
pub use runtime::{
    ArtifactValueRef, CapabilityExecutionContext, CapabilityInvocationPayload,
    CapabilityRuntimeInit, InputValueSource, SuppliedInputValue, SuppliedValueRef, UpstreamLineage,
};
