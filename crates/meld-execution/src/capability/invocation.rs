//! Capability invocation registry and runtime execution contracts.

use crate::capability::catalog::CapabilityCatalog;
use crate::capability::contracts::{BoundCapabilityInstance, CapabilityTypeContract};
use crate::capability::runtime::{CapabilityInvocationPayload, CapabilityRuntimeInit};
use crate::error::ExecutionInvariantError;
use crate::execution::ExecutionEventContext;
use crate::task::ArtifactRecord;
use async_trait::async_trait;
use std::collections::BTreeMap;
use std::sync::Arc;

/// Successful capability invocation output for one task-owned attempt.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityInvocationResult {
    /// Artifacts emitted by this invocation attempt.
    pub emitted_artifacts: Vec<ArtifactRecord>,
}

/// Domain-owned capability runtime implementation.
#[async_trait]
pub trait CapabilityInvoker: Send + Sync {
    /// Error type returned by this invoker.
    type Error;
    /// Execution API boundary supplied to this invoker.
    type ExecutionApi: ?Sized;

    /// Published contract used by task compilation.
    fn contract(&self) -> CapabilityTypeContract;

    /// Builds the structured runtime init package for one bound capability instance.
    fn runtime_init(
        &self,
        instance: &BoundCapabilityInstance,
    ) -> Result<CapabilityRuntimeInit, ExecutionInvariantError> {
        let contract = self.contract();
        if instance.capability_type_id != contract.capability_type_id
            || instance.capability_version != contract.capability_version
        {
            return Err(ExecutionInvariantError::ConfigError(format!(
                "Capability invoker '{}' cannot initialize instance '{}' of '{}' version '{}'",
                contract.capability_type_id,
                instance.capability_instance_id,
                instance.capability_type_id,
                instance.capability_version
            )));
        }

        Ok(CapabilityRuntimeInit {
            capability_instance_id: instance.capability_instance_id.clone(),
            capability_type_id: instance.capability_type_id.clone(),
            capability_version: instance.capability_version,
            scope_ref: instance.scope_ref.clone(),
            scope_kind: instance.scope_kind.clone(),
            binding_values: instance.binding_values.clone(),
            input_contract: contract.input_contract,
            output_contract: contract.output_contract,
            effect_contract: contract.effect_contract,
            execution_contract: contract.execution_contract,
        })
    }

    /// Reconstruct completed output from durable owner evidence only.
    /// This does not authorize effects. Unsupported or absent evidence returns `None`.
    async fn recover(
        &self,
        _events: Option<&meld_events::EventReplayCapability>,
        _runtime_init: &CapabilityRuntimeInit,
        _payload: &CapabilityInvocationPayload,
        _event_context: Option<&ExecutionEventContext>,
    ) -> Result<Option<CapabilityInvocationResult>, Self::Error> {
        Ok(None)
    }

    /// Invokes the domain runtime for one structured payload.
    async fn invoke(
        &self,
        api: &Self::ExecutionApi,
        runtime_init: &CapabilityRuntimeInit,
        payload: &CapabilityInvocationPayload,
        event_context: Option<&ExecutionEventContext>,
    ) -> Result<CapabilityInvocationResult, Self::Error>;
}

/// In-memory runtime registry for published capability invokers.
pub struct CapabilityExecutorRegistry<E, A: ?Sized> {
    invokers: BTreeMap<(String, u32), Arc<dyn CapabilityInvoker<Error = E, ExecutionApi = A>>>,
}

impl<E, A: ?Sized> Clone for CapabilityExecutorRegistry<E, A> {
    fn clone(&self) -> Self {
        Self {
            invokers: self.invokers.clone(),
        }
    }
}

impl<E, A: ?Sized> Default for CapabilityExecutorRegistry<E, A> {
    fn default() -> Self {
        Self {
            invokers: BTreeMap::new(),
        }
    }
}

impl<E, A: ?Sized> CapabilityExecutorRegistry<E, A> {
    /// Creates an empty capability executor registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers one capability invoker and publishes its contract into the catalog.
    pub fn register<I>(
        &mut self,
        catalog: &mut CapabilityCatalog,
        invoker: I,
    ) -> Result<(), ExecutionInvariantError>
    where
        I: CapabilityInvoker<Error = E, ExecutionApi = A> + 'static,
    {
        let contract = invoker.contract();
        let key = (
            contract.capability_type_id.clone(),
            contract.capability_version,
        );
        if self.invokers.contains_key(&key) {
            return Err(ExecutionInvariantError::ConfigError(format!(
                "Capability executor registry already contains '{}' version '{}'",
                key.0, key.1
            )));
        }
        catalog.register(contract)?;
        self.invokers.insert(key, Arc::new(invoker));
        Ok(())
    }

    /// Registers an already type-erased domain invoker.
    pub fn register_arc(
        &mut self,
        catalog: &mut CapabilityCatalog,
        invoker: Arc<dyn CapabilityInvoker<Error = E, ExecutionApi = A>>,
    ) -> Result<(), ExecutionInvariantError> {
        let contract = invoker.contract();
        let key = (
            contract.capability_type_id.clone(),
            contract.capability_version,
        );
        if self.invokers.contains_key(&key) {
            return Err(ExecutionInvariantError::ConfigError(format!(
                "Capability executor registry already contains '{}' version '{}'",
                key.0, key.1
            )));
        }
        catalog.register(contract)?;
        self.invokers.insert(key, invoker);
        Ok(())
    }

    /// Returns one registered invoker by type id and version.
    pub fn get(
        &self,
        capability_type_id: &str,
        capability_version: u32,
    ) -> Option<&Arc<dyn CapabilityInvoker<Error = E, ExecutionApi = A>>> {
        self.invokers
            .get(&(capability_type_id.to_string(), capability_version))
    }

    /// Builds one runtime init package from a bound capability instance.
    pub fn runtime_init_for(
        &self,
        instance: &BoundCapabilityInstance,
    ) -> Result<CapabilityRuntimeInit, ExecutionInvariantError> {
        let invoker = self
            .get(&instance.capability_type_id, instance.capability_version)
            .ok_or_else(|| {
                ExecutionInvariantError::ConfigError(format!(
                    "Capability executor registry is missing '{}' version '{}'",
                    instance.capability_type_id, instance.capability_version
                ))
            })?;
        invoker.runtime_init(instance)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::contracts::{
        ArtifactSchemaVersionRange, BindingSpec, BindingValueKind, BoundBindingValue,
        BoundInputWiring, BoundInputWiringSource, EffectKind, EffectSpec, ExecutionClass,
        ExecutionContract, InputCardinality, InputSlotSpec, OutputSlotSpec, ScopeContract,
    };
    use serde_json::{json, Value};

    #[derive(Debug, Clone)]
    struct FakeInvoker {
        contract: CapabilityTypeContract,
    }

    #[async_trait]
    impl CapabilityInvoker for FakeInvoker {
        type Error = String;
        type ExecutionApi = ();

        fn contract(&self) -> CapabilityTypeContract {
            self.contract.clone()
        }

        async fn invoke(
            &self,
            _api: &Self::ExecutionApi,
            _runtime_init: &CapabilityRuntimeInit,
            _payload: &CapabilityInvocationPayload,
            _event_context: Option<&ExecutionEventContext>,
        ) -> Result<CapabilityInvocationResult, Self::Error> {
            Ok(CapabilityInvocationResult::default())
        }
    }

    fn contract() -> CapabilityTypeContract {
        CapabilityTypeContract {
            capability_type_id: "provider_execute_chat".to_string(),
            capability_version: 1,
            owning_domain: "provider".to_string(),
            scope_contract: ScopeContract {
                scope_kind: "node".to_string(),
                scope_ref_kind: "node_id".to_string(),
                allow_fan_out: false,
            },
            binding_contract: vec![BindingSpec {
                binding_id: "provider".to_string(),
                value_kind: BindingValueKind::ProviderRef,
                required: true,
                affects_deterministic_identity: true,
            }],
            input_contract: vec![InputSlotSpec {
                slot_id: "provider_request".to_string(),
                accepted_artifact_type_ids: vec!["provider_execute_request".to_string()],
                schema_versions: ArtifactSchemaVersionRange { min: 1, max: 1 },
                required: true,
                cardinality: InputCardinality::One,
            }],
            output_contract: vec![OutputSlotSpec {
                slot_id: "provider_result".to_string(),
                artifact_type_id: "provider_execute_result".to_string(),
                schema_version: 1,
                guaranteed: true,
            }],
            effect_contract: vec![EffectSpec {
                effect_id: "provider_transport".to_string(),
                kind: EffectKind::Emit,
                target: "provider_service".to_string(),
                exclusive: false,
            }],
            execution_contract: ExecutionContract {
                execution_class: ExecutionClass::Queued,
                completion_semantics: "result_or_failure".to_string(),
                retry_class: "provider_io".to_string(),
                cancellation_supported: true,
            },
        }
    }

    fn instance() -> BoundCapabilityInstance {
        BoundCapabilityInstance {
            capability_instance_id: "capinst_provider_execute_chat".to_string(),
            capability_type_id: "provider_execute_chat".to_string(),
            capability_version: 1,
            scope_ref: "node_a".to_string(),
            scope_kind: "node".to_string(),
            binding_values: vec![BoundBindingValue {
                binding_id: "provider".to_string(),
                value: json!("local"),
            }],
            input_wiring: vec![BoundInputWiring {
                slot_id: "provider_request".to_string(),
                sources: vec![BoundInputWiringSource::TaskInitSlot {
                    init_slot_id: "request".to_string(),
                    artifact_type_id: "provider_execute_request".to_string(),
                    schema_version: 1,
                }],
            }],
        }
    }

    #[test]
    fn registry_registers_invoker_and_publishes_catalog_contract() {
        let mut catalog = CapabilityCatalog::new();
        let mut registry = CapabilityExecutorRegistry::<String, ()>::new();

        registry
            .register(
                &mut catalog,
                FakeInvoker {
                    contract: contract(),
                },
            )
            .unwrap();

        assert!(registry.get("provider_execute_chat", 1).is_some());
        assert!(catalog.get("provider_execute_chat", 1).is_some());
    }

    #[test]
    fn registry_rejects_duplicate_invoker_identity() {
        let mut catalog = CapabilityCatalog::new();
        let mut registry = CapabilityExecutorRegistry::<String, ()>::new();
        registry
            .register(
                &mut catalog,
                FakeInvoker {
                    contract: contract(),
                },
            )
            .unwrap();

        let error = registry
            .register(
                &mut catalog,
                FakeInvoker {
                    contract: contract(),
                },
            )
            .unwrap_err();

        assert!(error.to_string().contains("already contains"));
    }

    #[test]
    fn registry_reports_missing_invoker_for_runtime_init() {
        let registry = CapabilityExecutorRegistry::<String, ()>::new();

        let error = registry.runtime_init_for(&instance()).unwrap_err();

        assert!(error.to_string().contains("is missing"));
    }

    #[test]
    fn runtime_init_for_preserves_bound_instance_fields() {
        let mut catalog = CapabilityCatalog::new();
        let mut registry = CapabilityExecutorRegistry::<String, ()>::new();
        registry
            .register(
                &mut catalog,
                FakeInvoker {
                    contract: contract(),
                },
            )
            .unwrap();

        let runtime_init = registry.runtime_init_for(&instance()).unwrap();

        assert_eq!(
            runtime_init.capability_instance_id,
            "capinst_provider_execute_chat"
        );
        assert_eq!(runtime_init.scope_ref, "node_a");
        assert_eq!(
            runtime_init.binding_values,
            vec![BoundBindingValue {
                binding_id: "provider".to_string(),
                value: Value::String("local".to_string()),
            }]
        );
        assert_eq!(runtime_init.input_contract[0].slot_id, "provider_request");
        assert_eq!(runtime_init.output_contract[0].slot_id, "provider_result");
        assert_eq!(
            runtime_init.effect_contract[0].effect_id,
            "provider_transport"
        );
    }

    #[test]
    fn runtime_init_rejects_instance_version_mismatch() {
        let invoker = FakeInvoker {
            contract: contract(),
        };
        let mut instance = instance();
        instance.capability_version = 2;

        let error = invoker.runtime_init(&instance).unwrap_err();

        assert!(error.to_string().contains("cannot initialize"));
    }
}
