//! Shared capability contracts and registration.
//!
//! This domain defines the task-facing capability model.
//! Owning domains publish typed contracts through this surface so task code can
//! bind, validate, and invoke capabilities without reaching into domain internals.

pub mod catalog;
pub mod contracts;
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
pub use meld_execution::capability::{CapabilityInvocationResult, CapabilityInvoker};
pub use runtime::{
    ArtifactValueRef, CapabilityExecutionContext, CapabilityInvocationPayload,
    CapabilityRuntimeInit, InputValueSource, SuppliedInputValue, SuppliedValueRef, UpstreamLineage,
};

pub type CapabilityExecutorRegistry =
    meld_execution::capability::CapabilityExecutorRegistry<ApiError, dyn ExecutionRuntimeContext>;
