//! Capability contract and bound instance records.

use crate::error::ApiError;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;

/// Supported schema version range for an artifact slot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactSchemaVersionRange {
    pub min: u32,
    pub max: u32,
}

impl ArtifactSchemaVersionRange {
    /// Returns true when the given schema version is accepted by the slot.
    pub fn accepts(&self, version: u32) -> bool {
        version >= self.min && version <= self.max
    }
}

/// Published scope information for one capability type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScopeContract {
    pub scope_kind: String,
    pub scope_ref_kind: String,
    pub allow_fan_out: bool,
}

/// Non-artifact binding source kinds accepted by a capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BindingValueKind {
    Literal,
    ConfigRef,
    PolicyRef,
    AgentRef,
    ProviderRef,
}

/// Binding specification published by a capability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BindingSpec {
    pub binding_id: String,
    pub value_kind: BindingValueKind,
    pub required: bool,
    pub affects_deterministic_identity: bool,
}

/// Cardinality rule for one input slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InputCardinality {
    One,
    Many,
}

/// Input slot specification published by a capability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputSlotSpec {
    pub slot_id: String,
    pub accepted_artifact_type_ids: Vec<String>,
    pub schema_versions: ArtifactSchemaVersionRange,
    pub required: bool,
    pub cardinality: InputCardinality,
}

/// Output slot specification published by a capability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputSlotSpec {
    pub slot_id: String,
    pub artifact_type_id: String,
    pub schema_version: u32,
    pub guaranteed: bool,
}

/// Effect kinds that may require ordering without an artifact handoff.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EffectKind {
    Read,
    Write,
    Append,
    Emit,
    Acquire,
}

/// Effect specification published by a capability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectSpec {
    pub effect_id: String,
    pub kind: EffectKind,
    pub target: String,
    pub exclusive: bool,
}

/// Execution class for the published capability contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionClass {
    Inline,
    Queued,
    SessionScoped,
}

/// Execution-facing behavior published by a capability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionContract {
    pub execution_class: ExecutionClass,
    pub completion_semantics: String,
    pub retry_class: String,
    pub cancellation_supported: bool,
}

/// Published cross-domain capability contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityTypeContract {
    pub capability_type_id: String,
    pub capability_version: u32,
    pub owning_domain: String,
    pub scope_contract: ScopeContract,
    pub binding_contract: Vec<BindingSpec>,
    pub input_contract: Vec<InputSlotSpec>,
    pub output_contract: Vec<OutputSlotSpec>,
    pub effect_contract: Vec<EffectSpec>,
    pub execution_contract: ExecutionContract,
}

impl CapabilityTypeContract {
    /// Validates the published contract before registration.
    pub fn validate(&self) -> Result<(), ApiError> {
        require_non_empty("capability_type_id", &self.capability_type_id)?;
        require_non_empty("owning_domain", &self.owning_domain)?;
        if self.capability_version == 0 {
            return Err(ApiError::ConfigError(
                "Capability contract version must be greater than zero".to_string(),
            ));
        }
        require_non_empty("scope_kind", &self.scope_contract.scope_kind)?;
        require_non_empty("scope_ref_kind", &self.scope_contract.scope_ref_kind)?;
        require_non_empty(
            "completion_semantics",
            &self.execution_contract.completion_semantics,
        )?;
        require_non_empty("retry_class", &self.execution_contract.retry_class)?;

        ensure_unique_ids(
            "binding",
            self.binding_contract
                .iter()
                .map(|binding| binding.binding_id.as_str()),
        )?;
        ensure_unique_ids(
            "input slot",
            self.input_contract.iter().map(|slot| slot.slot_id.as_str()),
        )?;
        ensure_unique_ids(
            "output slot",
            self.output_contract
                .iter()
                .map(|slot| slot.slot_id.as_str()),
        )?;
        ensure_unique_ids(
            "effect",
            self.effect_contract
                .iter()
                .map(|effect| effect.effect_id.as_str()),
        )?;

        for binding in &self.binding_contract {
            require_non_empty("binding_id", &binding.binding_id)?;
        }
        for slot in &self.input_contract {
            require_non_empty("input slot id", &slot.slot_id)?;
            if slot.accepted_artifact_type_ids.is_empty() {
                return Err(ApiError::ConfigError(format!(
                    "Capability '{}' input slot '{}' must accept at least one artifact type",
                    self.capability_type_id, slot.slot_id
                )));
            }
            if slot.schema_versions.min == 0 || slot.schema_versions.max == 0 {
                return Err(ApiError::ConfigError(format!(
                    "Capability '{}' input slot '{}' schema versions must be greater than zero",
                    self.capability_type_id, slot.slot_id
                )));
            }
            if slot.schema_versions.min > slot.schema_versions.max {
                return Err(ApiError::ConfigError(format!(
                    "Capability '{}' input slot '{}' has invalid schema version range",
                    self.capability_type_id, slot.slot_id
                )));
            }
            for artifact_type_id in &slot.accepted_artifact_type_ids {
                require_non_empty("accepted artifact type id", artifact_type_id)?;
            }
        }
        for slot in &self.output_contract {
            require_non_empty("output slot id", &slot.slot_id)?;
            require_non_empty("output artifact type id", &slot.artifact_type_id)?;
            if slot.schema_version == 0 {
                return Err(ApiError::ConfigError(format!(
                    "Capability '{}' output slot '{}' schema version must be greater than zero",
                    self.capability_type_id, slot.slot_id
                )));
            }
        }
        for effect in &self.effect_contract {
            require_non_empty("effect id", &effect.effect_id)?;
            require_non_empty("effect target", &effect.target)?;
        }

        Ok(())
    }
}

/// Compile-time chosen binding value for one capability instance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoundBindingValue {
    pub binding_id: String,
    pub value: Value,
}

/// Wiring source for one bound input slot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BoundInputWiringSource {
    TaskInitSlot {
        init_slot_id: String,
        artifact_type_id: String,
        schema_version: u32,
    },
    UpstreamOutput {
        capability_instance_id: String,
        output_slot_id: String,
        artifact_type_id: String,
        schema_version: u32,
    },
}

impl BoundInputWiringSource {
    fn artifact_type_id(&self) -> &str {
        match self {
            Self::TaskInitSlot {
                artifact_type_id, ..
            } => artifact_type_id,
            Self::UpstreamOutput {
                artifact_type_id, ..
            } => artifact_type_id,
        }
    }

    fn schema_version(&self) -> u32 {
        match self {
            Self::TaskInitSlot { schema_version, .. } => *schema_version,
            Self::UpstreamOutput { schema_version, .. } => *schema_version,
        }
    }
}

/// Bound source list for one input slot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoundInputWiring {
    pub slot_id: String,
    pub sources: Vec<BoundInputWiringSource>,
}

/// Compile-time bound capability instance projection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoundCapabilityInstance {
    pub capability_instance_id: String,
    pub capability_type_id: String,
    pub capability_version: u32,
    pub scope_ref: String,
    pub scope_kind: String,
    pub binding_values: Vec<BoundBindingValue>,
    pub input_wiring: Vec<BoundInputWiring>,
}

impl BoundCapabilityInstance {
    /// Validates one bound instance against the published contract.
    pub fn validate_against(&self, contract: &CapabilityTypeContract) -> Result<(), ApiError> {
        require_non_empty("capability_instance_id", &self.capability_instance_id)?;
        require_non_empty("scope_ref", &self.scope_ref)?;
        require_non_empty("scope_kind", &self.scope_kind)?;

        if self.capability_type_id != contract.capability_type_id {
            return Err(ApiError::ConfigError(format!(
                "Capability instance '{}' type '{}' does not match contract '{}'",
                self.capability_instance_id, self.capability_type_id, contract.capability_type_id
            )));
        }
        if self.capability_version != contract.capability_version {
            return Err(ApiError::ConfigError(format!(
                "Capability instance '{}' version '{}' does not match contract '{}'",
                self.capability_instance_id, self.capability_version, contract.capability_version
            )));
        }
        if self.scope_kind != contract.scope_contract.scope_kind {
            return Err(ApiError::ConfigError(format!(
                "Capability instance '{}' scope kind '{}' does not match contract '{}'",
                self.capability_instance_id, self.scope_kind, contract.scope_contract.scope_kind
            )));
        }

        ensure_unique_ids(
            "bound binding",
            self.binding_values
                .iter()
                .map(|binding| binding.binding_id.as_str()),
        )?;
        ensure_unique_ids(
            "bound input slot",
            self.input_wiring
                .iter()
                .map(|wiring| wiring.slot_id.as_str()),
        )?;

        let binding_ids: HashSet<&str> = contract
            .binding_contract
            .iter()
            .map(|binding| binding.binding_id.as_str())
            .collect();
        for binding in &self.binding_values {
            require_non_empty("binding_id", &binding.binding_id)?;
            if !binding_ids.contains(binding.binding_id.as_str()) {
                return Err(ApiError::ConfigError(format!(
                    "Capability instance '{}' binds unknown binding '{}'",
                    self.capability_instance_id, binding.binding_id
                )));
            }
        }
        for binding in &contract.binding_contract {
            if binding.required
                && !self
                    .binding_values
                    .iter()
                    .any(|value| value.binding_id == binding.binding_id)
            {
                return Err(ApiError::ConfigError(format!(
                    "Capability instance '{}' is missing required binding '{}'",
                    self.capability_instance_id, binding.binding_id
                )));
            }
        }

        let input_slots: HashSet<&str> = contract
            .input_contract
            .iter()
            .map(|slot| slot.slot_id.as_str())
            .collect();
        for wiring in &self.input_wiring {
            require_non_empty("input slot id", &wiring.slot_id)?;
            if !input_slots.contains(wiring.slot_id.as_str()) {
                return Err(ApiError::ConfigError(format!(
                    "Capability instance '{}' wires unknown input slot '{}'",
                    self.capability_instance_id, wiring.slot_id
                )));
            }
            if wiring.sources.is_empty() {
                return Err(ApiError::ConfigError(format!(
                    "Capability instance '{}' input slot '{}' has no sources",
                    self.capability_instance_id, wiring.slot_id
                )));
            }
            let slot = contract
                .input_contract
                .iter()
                .find(|slot| slot.slot_id == wiring.slot_id)
                .expect("validated slot presence above");
            if slot.cardinality == InputCardinality::One && wiring.sources.len() > 1 {
                return Err(ApiError::ConfigError(format!(
                    "Capability instance '{}' input slot '{}' accepts one source but {} were bound",
                    self.capability_instance_id,
                    wiring.slot_id,
                    wiring.sources.len()
                )));
            }
            for source in &wiring.sources {
                if !slot
                    .accepted_artifact_type_ids
                    .iter()
                    .any(|artifact_type_id| artifact_type_id == source.artifact_type_id())
                {
                    return Err(ApiError::ConfigError(format!(
                        "Capability instance '{}' input slot '{}' rejects artifact type '{}'",
                        self.capability_instance_id,
                        wiring.slot_id,
                        source.artifact_type_id()
                    )));
                }
                if !slot.schema_versions.accepts(source.schema_version()) {
                    return Err(ApiError::ConfigError(format!(
                        "Capability instance '{}' input slot '{}' rejects schema version '{}'",
                        self.capability_instance_id,
                        wiring.slot_id,
                        source.schema_version()
                    )));
                }
            }
        }

        for slot in &contract.input_contract {
            if slot.required
                && !self
                    .input_wiring
                    .iter()
                    .any(|wiring| wiring.slot_id == slot.slot_id)
            {
                return Err(ApiError::ConfigError(format!(
                    "Capability instance '{}' is missing required input slot '{}'",
                    self.capability_instance_id, slot.slot_id
                )));
            }
        }

        Ok(())
    }
}

fn require_non_empty(field_name: &str, value: &str) -> Result<(), ApiError> {
    if value.trim().is_empty() {
        return Err(ApiError::ConfigError(format!(
            "Capability {} must not be empty",
            field_name
        )));
    }
    Ok(())
}

fn ensure_unique_ids<'a>(label: &str, ids: impl Iterator<Item = &'a str>) -> Result<(), ApiError> {
    let mut seen = HashSet::new();
    for id in ids {
        if !seen.insert(id) {
            return Err(ApiError::ConfigError(format!(
                "Capability contract has duplicate {} '{}'",
                label, id
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn contract_validation_rejects_duplicate_slots() {
        let mut contract = contract();
        contract
            .input_contract
            .push(contract.input_contract[0].clone());

        let error = contract.validate().unwrap_err();

        assert!(matches!(error, ApiError::ConfigError(_)));
        assert!(error.to_string().contains("duplicate input slot"));
    }

    #[test]
    fn bound_instance_validation_rejects_unknown_binding() {
        let contract = contract();
        let instance = BoundCapabilityInstance {
            capability_instance_id: "capinst_provider_execute_chat".to_string(),
            capability_type_id: contract.capability_type_id.clone(),
            capability_version: contract.capability_version,
            scope_ref: "node_a".to_string(),
            scope_kind: contract.scope_contract.scope_kind.clone(),
            binding_values: vec![BoundBindingValue {
                binding_id: "missing".to_string(),
                value: Value::String("provider-a".to_string()),
            }],
            input_wiring: vec![BoundInputWiring {
                slot_id: "provider_request".to_string(),
                sources: vec![BoundInputWiringSource::TaskInitSlot {
                    init_slot_id: "request".to_string(),
                    artifact_type_id: "provider_execute_request".to_string(),
                    schema_version: 1,
                }],
            }],
        };

        let error = instance.validate_against(&contract).unwrap_err();

        assert!(matches!(error, ApiError::ConfigError(_)));
        assert!(error.to_string().contains("unknown binding"));
    }

    #[test]
    fn bound_instance_validation_rejects_incompatible_artifact_type() {
        let contract = contract();
        let instance = BoundCapabilityInstance {
            capability_instance_id: "capinst_provider_execute_chat".to_string(),
            capability_type_id: contract.capability_type_id.clone(),
            capability_version: contract.capability_version,
            scope_ref: "node_a".to_string(),
            scope_kind: contract.scope_contract.scope_kind.clone(),
            binding_values: vec![BoundBindingValue {
                binding_id: "provider".to_string(),
                value: Value::String("provider-a".to_string()),
            }],
            input_wiring: vec![BoundInputWiring {
                slot_id: "provider_request".to_string(),
                sources: vec![BoundInputWiringSource::TaskInitSlot {
                    init_slot_id: "request".to_string(),
                    artifact_type_id: "wrong_type".to_string(),
                    schema_version: 1,
                }],
            }],
        };

        let error = instance.validate_against(&contract).unwrap_err();

        assert!(matches!(error, ApiError::ConfigError(_)));
        assert!(error.to_string().contains("rejects artifact type"));
    }
}
