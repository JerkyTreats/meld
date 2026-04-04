//! Capability registration and lookup.

use crate::capability::contracts::CapabilityTypeContract;
use crate::error::ApiError;
use std::collections::BTreeMap;

/// In-memory registry of published capability contracts.
#[derive(Debug, Default, Clone)]
pub struct CapabilityCatalog {
    contracts: BTreeMap<(String, u32), CapabilityTypeContract>,
}

impl CapabilityCatalog {
    /// Creates an empty capability catalog.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a validated capability contract.
    pub fn register(&mut self, contract: CapabilityTypeContract) -> Result<(), ApiError> {
        contract.validate()?;
        let key = (
            contract.capability_type_id.clone(),
            contract.capability_version,
        );
        if self.contracts.contains_key(&key) {
            return Err(ApiError::ConfigError(format!(
                "Capability catalog already contains '{}' version '{}'",
                key.0, key.1
            )));
        }
        self.contracts.insert(key, contract);
        Ok(())
    }

    /// Returns one contract by exact type id and version.
    pub fn get(
        &self,
        capability_type_id: &str,
        capability_version: u32,
    ) -> Option<&CapabilityTypeContract> {
        self.contracts
            .get(&(capability_type_id.to_string(), capability_version))
    }

    /// Returns true when the contract exists in the catalog.
    pub fn contains(&self, capability_type_id: &str, capability_version: u32) -> bool {
        self.get(capability_type_id, capability_version).is_some()
    }

    /// Iterates over all registered contracts in deterministic order.
    pub fn iter(&self) -> impl Iterator<Item = &CapabilityTypeContract> {
        self.contracts.values()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::contracts::{
        ArtifactSchemaVersionRange, BindingSpec, BindingValueKind, CapabilityTypeContract,
        EffectKind, EffectSpec, ExecutionClass, ExecutionContract, InputCardinality, InputSlotSpec,
        OutputSlotSpec, ScopeContract,
    };

    fn contract(capability_type_id: &str, version: u32) -> CapabilityTypeContract {
        CapabilityTypeContract {
            capability_type_id: capability_type_id.to_string(),
            capability_version: version,
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

    #[test]
    fn register_and_get_contract() {
        let mut catalog = CapabilityCatalog::new();
        let contract = contract("provider_execute_chat", 1);

        catalog.register(contract.clone()).unwrap();

        assert!(catalog.contains("provider_execute_chat", 1));
        assert_eq!(catalog.get("provider_execute_chat", 1), Some(&contract));
    }

    #[test]
    fn register_rejects_duplicate_identity() {
        let mut catalog = CapabilityCatalog::new();
        catalog
            .register(contract("provider_execute_chat", 1))
            .unwrap();

        let error = catalog
            .register(contract("provider_execute_chat", 1))
            .unwrap_err();

        assert!(matches!(error, ApiError::ConfigError(_)));
        assert!(error.to_string().contains("already contains"));
    }
}
