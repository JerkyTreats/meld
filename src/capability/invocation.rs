//! Capability invocation registry and runtime execution contracts.

use crate::api::ContextApi;
use crate::capability::catalog::CapabilityCatalog;
use crate::capability::contracts::{BoundCapabilityInstance, CapabilityTypeContract};
use crate::capability::runtime::{CapabilityInvocationPayload, CapabilityRuntimeInit};
use crate::context::queue::QueueEventContext;
use crate::error::ApiError;
use crate::task::ArtifactRecord;
use async_trait::async_trait;
use std::collections::BTreeMap;
use std::sync::Arc;

/// Successful capability invocation output for one task-owned attempt.
#[derive(Debug, Clone, Default)]
pub struct CapabilityInvocationResult {
    pub emitted_artifacts: Vec<ArtifactRecord>,
}

/// Domain-owned capability runtime implementation.
#[async_trait]
pub trait CapabilityInvoker: Send + Sync {
    /// Published contract used by task compilation.
    fn contract(&self) -> CapabilityTypeContract;

    /// Builds the structured runtime init package for one bound capability instance.
    fn runtime_init(
        &self,
        instance: &BoundCapabilityInstance,
    ) -> Result<CapabilityRuntimeInit, ApiError> {
        let contract = self.contract();
        if instance.capability_type_id != contract.capability_type_id
            || instance.capability_version != contract.capability_version
        {
            return Err(ApiError::ConfigError(format!(
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

    /// Invokes the domain runtime for one structured payload.
    async fn invoke(
        &self,
        api: &ContextApi,
        runtime_init: &CapabilityRuntimeInit,
        payload: &CapabilityInvocationPayload,
        event_context: Option<&QueueEventContext>,
    ) -> Result<CapabilityInvocationResult, ApiError>;
}

/// In-memory runtime registry for published capability invokers.
#[derive(Default, Clone)]
pub struct CapabilityExecutorRegistry {
    invokers: BTreeMap<(String, u32), Arc<dyn CapabilityInvoker>>,
}

impl CapabilityExecutorRegistry {
    /// Creates an empty capability executor registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers one capability invoker and publishes its contract into the catalog.
    pub fn register<I>(
        &mut self,
        catalog: &mut CapabilityCatalog,
        invoker: I,
    ) -> Result<(), ApiError>
    where
        I: CapabilityInvoker + 'static,
    {
        let contract = invoker.contract();
        let key = (
            contract.capability_type_id.clone(),
            contract.capability_version,
        );
        if self.invokers.contains_key(&key) {
            return Err(ApiError::ConfigError(format!(
                "Capability executor registry already contains '{}' version '{}'",
                key.0, key.1
            )));
        }
        catalog.register(contract)?;
        self.invokers.insert(key, Arc::new(invoker));
        Ok(())
    }

    /// Returns one registered invoker by type id and version.
    pub fn get(
        &self,
        capability_type_id: &str,
        capability_version: u32,
    ) -> Option<&Arc<dyn CapabilityInvoker>> {
        self.invokers
            .get(&(capability_type_id.to_string(), capability_version))
    }

    /// Builds one runtime init package from a bound capability instance.
    pub fn runtime_init_for(
        &self,
        instance: &BoundCapabilityInstance,
    ) -> Result<CapabilityRuntimeInit, ApiError> {
        let invoker = self
            .get(&instance.capability_type_id, instance.capability_version)
            .ok_or_else(|| {
                ApiError::ConfigError(format!(
                    "Capability executor registry is missing '{}' version '{}'",
                    instance.capability_type_id, instance.capability_version
                ))
            })?;
        invoker.runtime_init(instance)
    }
}
