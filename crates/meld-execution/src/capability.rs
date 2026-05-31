//! Shared capability contracts and registration.
//!
//! # Example
//!
//! ```rust
//! use meld_execution::capability::{
//!     ArtifactSchemaVersionRange, BindingSpec, BindingValueKind, CapabilityCatalog,
//!     CapabilityTypeContract, EffectSpec, EffectKind, ExecutionClass, ExecutionContract,
//!     InputCardinality, InputSlotSpec, OutputSlotSpec, ScopeContract,
//! };
//!
//! let contract = CapabilityTypeContract {
//!     capability_type_id: "provider_execute_chat".to_string(),
//!     capability_version: 1,
//!     owning_domain: "provider".to_string(),
//!     scope_contract: ScopeContract {
//!         scope_kind: "node".to_string(),
//!         scope_ref_kind: "node_id".to_string(),
//!         allow_fan_out: false,
//!     },
//!     binding_contract: vec![BindingSpec {
//!         binding_id: "provider".to_string(),
//!         value_kind: BindingValueKind::ProviderRef,
//!         required: true,
//!         affects_deterministic_identity: true,
//!     }],
//!     input_contract: vec![InputSlotSpec {
//!         slot_id: "provider_request".to_string(),
//!         accepted_artifact_type_ids: vec!["provider_execute_request".to_string()],
//!         schema_versions: ArtifactSchemaVersionRange { min: 1, max: 1 },
//!         required: true,
//!         cardinality: InputCardinality::One,
//!     }],
//!     output_contract: vec![OutputSlotSpec {
//!         slot_id: "provider_result".to_string(),
//!         artifact_type_id: "provider_execute_result".to_string(),
//!         schema_version: 1,
//!         guaranteed: true,
//!     }],
//!     effect_contract: vec![EffectSpec {
//!         effect_id: "provider_transport".to_string(),
//!         kind: EffectKind::Emit,
//!         target: "provider_service".to_string(),
//!         exclusive: false,
//!     }],
//!     execution_contract: ExecutionContract {
//!         execution_class: ExecutionClass::Queued,
//!         completion_semantics: "result_or_failure".to_string(),
//!         retry_class: "provider_io".to_string(),
//!         cancellation_supported: true,
//!     },
//! };
//!
//! contract.validate().unwrap();
//!
//! let mut catalog = CapabilityCatalog::new();
//! catalog.register(contract).unwrap();
//! assert!(catalog.contains("provider_execute_chat", 1));
//! ```

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
