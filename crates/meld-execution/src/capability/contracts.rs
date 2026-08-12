//! Capability contract and bound instance records.

use crate::error::ApiError;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;

/// Supported schema version range for an artifact slot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactSchemaVersionRange {
    /// Lowest accepted schema version in this inclusive range.
    pub min: u32,
    /// Highest accepted schema version in this inclusive range.
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
    /// Domain scope category accepted by this capability contract.
    pub scope_kind: String,
    /// Identifier kind used to address the capability scope.
    pub scope_ref_kind: String,
    /// True when one capability binding may expand across multiple scoped targets.
    pub allow_fan_out: bool,
}

/// Non-artifact binding source kinds accepted by a capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BindingValueKind {
    /// Inline literal binding value.
    Literal,
    /// Reference to configuration owned outside this contract.
    ConfigRef,
    /// Reference to policy owned outside this contract.
    PolicyRef,
    /// Reference to an agent profile or identity.
    AgentRef,
    /// Reference to a configured provider binding.
    ProviderRef,
}

/// Binding specification published by a capability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BindingSpec {
    /// Stable binding identifier within the capability contract.
    pub binding_id: String,
    /// Kind of value accepted for this binding.
    pub value_kind: BindingValueKind,
    /// True when callers must provide this contract element.
    pub required: bool,
    /// True when this binding participates in deterministic instance identity.
    pub affects_deterministic_identity: bool,
}

/// Cardinality rule for one input slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InputCardinality {
    /// Exactly one artifact is accepted.
    One,
    /// Multiple artifacts are accepted.
    Many,
}

/// Input slot specification published by a capability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputSlotSpec {
    /// Stable slot identifier within the owning contract.
    pub slot_id: String,
    /// Artifact type identifiers accepted by this input slot.
    pub accepted_artifact_type_ids: Vec<String>,
    /// Accepted schema version range for artifacts supplied to this slot.
    pub schema_versions: ArtifactSchemaVersionRange,
    /// True when callers must provide this contract element.
    pub required: bool,
    /// Number of artifacts accepted by this input slot.
    pub cardinality: InputCardinality,
}

/// Output slot specification published by a capability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputSlotSpec {
    /// Stable slot identifier within the owning contract.
    pub slot_id: String,
    /// Artifact type identifier used for contract validation and routing.
    pub artifact_type_id: String,
    /// Schema version for the serialized contract or artifact shape.
    pub schema_version: u32,
    /// True when successful execution is expected to produce this output slot.
    pub guaranteed: bool,
}

/// Effect kinds that may require ordering without an artifact handoff.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EffectKind {
    /// Read access to the declared target.
    Read,
    /// Write access to the declared target.
    Write,
    /// Append access to the declared target.
    Append,
    /// Emission of an event or external side effect.
    Emit,
    /// Acquisition of an execution resource or lease.
    Acquire,
}

/// Effect specification published by a capability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectSpec {
    /// Stable effect identifier within the capability contract.
    pub effect_id: String,
    /// Contract kind used by the owning runtime.
    pub kind: EffectKind,
    /// Domain target affected by this contract entry.
    pub target: String,
    /// True when this effect requires exclusive execution over its target.
    pub exclusive: bool,
}

/// Execution class for the published capability contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionClass {
    /// Runs inline with the caller.
    Inline,
    /// Runs through a queue-backed executor.
    Queued,
    /// Runs within a session-scoped execution context.
    SessionScoped,
}

/// Execution-facing behavior published by a capability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionContract {
    /// Runtime scheduling class for this capability.
    pub execution_class: ExecutionClass,
    /// Named completion semantics expected from this capability.
    pub completion_semantics: String,
    /// Retry policy class used by execution adapters.
    pub retry_class: String,
    /// True when the capability supports runtime cancellation.
    pub cancellation_supported: bool,
}

/// Published cross-domain capability contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityTypeContract {
    /// Stable capability type identifier published by the owning domain.
    pub capability_type_id: String,
    /// Version of the published capability contract.
    pub capability_version: u32,
    /// Domain responsible for publishing and maintaining this contract.
    pub owning_domain: String,
    /// Scope contract used to validate bound capability instances.
    pub scope_contract: ScopeContract,
    /// Binding contract entries accepted by this capability.
    pub binding_contract: Vec<BindingSpec>,
    /// Input slot contracts accepted by this capability.
    pub input_contract: Vec<InputSlotSpec>,
    /// Output slot contracts produced by this capability.
    pub output_contract: Vec<OutputSlotSpec>,
    /// Side effect contracts declared by this capability.
    pub effect_contract: Vec<EffectSpec>,
    /// Execution behavior declared by this capability.
    pub execution_contract: ExecutionContract,
}

impl CapabilityTypeContract {
    /// Returns the content identity used to bind semantic authorization to
    /// this exact executable contract.
    pub fn content_identity(&self) -> String {
        let encoded = serde_json::to_vec(self)
            .expect("Capability contract identity serialization is infallible");
        format!("capability-contract-{}", blake3::hash(&encoded).to_hex())
    }

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
    /// Stable binding identifier within the capability contract.
    pub binding_id: String,
    /// Structured value carried by this contract boundary.
    pub value: Value,
}

/// Wiring source for one bound input slot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BoundInputWiringSource {
    /// Artifact supplied by task initialization.
    TaskInitSlot {
        /// Task initialization slot that supplies this artifact.
        init_slot_id: String,
        /// Artifact type required from the initialization slot.
        artifact_type_id: String,
        /// Schema version required from the initialization slot artifact.
        schema_version: u32,
    },
    /// Artifact supplied by an upstream capability output.
    UpstreamOutput {
        /// Producing capability instance in the compiled task graph.
        capability_instance_id: String,
        /// Output slot on the producing capability instance.
        output_slot_id: String,
        /// Artifact type required from the upstream output.
        artifact_type_id: String,
        /// Schema version required from the upstream artifact.
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
    /// Stable slot identifier within the owning contract.
    pub slot_id: String,
    /// Ordered wiring sources accepted by this input slot.
    pub sources: Vec<BoundInputWiringSource>,
}

/// Compile-time bound capability instance projection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoundCapabilityInstance {
    /// Deterministic capability instance identifier within a compiled task.
    pub capability_instance_id: String,
    /// Stable capability type identifier published by the owning domain.
    pub capability_type_id: String,
    /// Version of the published capability contract.
    pub capability_version: u32,
    /// Concrete scope reference bound to this capability instance.
    pub scope_ref: String,
    /// Domain scope category accepted by this capability contract.
    pub scope_kind: String,
    /// Resolved binding values for this capability instance.
    pub binding_values: Vec<BoundBindingValue>,
    /// Resolved input wiring for this capability instance.
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

    /// Mutation closure used by contract validation table cases.
    type ContractMutation = Box<dyn FnOnce(&mut CapabilityTypeContract)>;
    /// Mutation closure used by instance validation table cases.
    type InstanceMutation = Box<dyn FnOnce(&mut BoundCapabilityInstance)>;

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
    fn contract_validation_rejects_required_field_violations() {
        let cases: Vec<(&str, ContractMutation, &str)> = vec![
            (
                "empty capability type id",
                Box::new(|contract| contract.capability_type_id.clear()),
                "capability_type_id must not be empty",
            ),
            (
                "zero capability version",
                Box::new(|contract| contract.capability_version = 0),
                "version must be greater than zero",
            ),
            (
                "empty scope kind",
                Box::new(|contract| contract.scope_contract.scope_kind.clear()),
                "scope_kind must not be empty",
            ),
            (
                "empty scope ref kind",
                Box::new(|contract| contract.scope_contract.scope_ref_kind.clear()),
                "scope_ref_kind must not be empty",
            ),
            (
                "duplicate bindings",
                Box::new(|contract| {
                    contract
                        .binding_contract
                        .push(contract.binding_contract[0].clone())
                }),
                "duplicate binding",
            ),
            (
                "duplicate outputs",
                Box::new(|contract| {
                    contract
                        .output_contract
                        .push(contract.output_contract[0].clone())
                }),
                "duplicate output slot",
            ),
            (
                "duplicate effects",
                Box::new(|contract| {
                    contract
                        .effect_contract
                        .push(contract.effect_contract[0].clone())
                }),
                "duplicate effect",
            ),
            (
                "empty accepted artifact types",
                Box::new(|contract| {
                    contract.input_contract[0]
                        .accepted_artifact_type_ids
                        .clear()
                }),
                "must accept at least one artifact type",
            ),
            (
                "zero input schema version",
                Box::new(|contract| contract.input_contract[0].schema_versions.min = 0),
                "schema versions must be greater than zero",
            ),
            (
                "inverted input schema version range",
                Box::new(|contract| {
                    contract.input_contract[0].schema_versions.min = 2;
                    contract.input_contract[0].schema_versions.max = 1;
                }),
                "invalid schema version range",
            ),
            (
                "empty output artifact type",
                Box::new(|contract| contract.output_contract[0].artifact_type_id.clear()),
                "output artifact type id must not be empty",
            ),
            (
                "zero output schema version",
                Box::new(|contract| contract.output_contract[0].schema_version = 0),
                "schema version must be greater than zero",
            ),
            (
                "empty effect target",
                Box::new(|contract| contract.effect_contract[0].target.clear()),
                "effect target must not be empty",
            ),
        ];

        for (case_name, mutate, expected) in cases {
            let mut contract = contract();
            mutate(&mut contract);

            let error = match contract.validate() {
                Ok(()) => panic!("{case_name} should fail validation"),
                Err(error) => error,
            };

            assert!(
                error.to_string().contains(expected),
                "{case_name} expected error containing '{expected}', got '{error}'"
            );
        }
    }

    fn valid_instance(contract: &CapabilityTypeContract) -> BoundCapabilityInstance {
        BoundCapabilityInstance {
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
                    artifact_type_id: "provider_execute_request".to_string(),
                    schema_version: 1,
                }],
            }],
        }
    }

    #[test]
    fn bound_instance_validation_rejects_unknown_binding() {
        let contract = contract();
        let mut instance = valid_instance(&contract);
        instance.binding_values[0].binding_id = "missing".to_string();

        let error = instance.validate_against(&contract).unwrap_err();

        assert!(matches!(error, ApiError::ConfigError(_)));
        assert!(error.to_string().contains("unknown binding"));
    }

    #[test]
    fn bound_instance_validation_rejects_incompatible_artifact_type() {
        let contract = contract();
        let mut instance = valid_instance(&contract);
        let BoundInputWiringSource::TaskInitSlot {
            artifact_type_id, ..
        } = &mut instance.input_wiring[0].sources[0]
        else {
            unreachable!("fixture uses task init wiring");
        };
        *artifact_type_id = "wrong_type".to_string();

        let error = instance.validate_against(&contract).unwrap_err();

        assert!(matches!(error, ApiError::ConfigError(_)));
        assert!(error.to_string().contains("rejects artifact type"));
    }

    #[test]
    fn bound_instance_validation_rejects_binding_and_input_shape_violations() {
        let cases: Vec<(&str, InstanceMutation, &str)> = vec![
            (
                "scope kind mismatch",
                Box::new(|instance| instance.scope_kind = "workspace".to_string()),
                "scope kind",
            ),
            (
                "missing required binding",
                Box::new(|instance| instance.binding_values.clear()),
                "missing required binding",
            ),
            (
                "duplicate bound binding",
                Box::new(|instance| {
                    instance
                        .binding_values
                        .push(instance.binding_values[0].clone())
                }),
                "duplicate bound binding",
            ),
            (
                "missing required input",
                Box::new(|instance| instance.input_wiring.clear()),
                "missing required input slot",
            ),
            (
                "empty source list",
                Box::new(|instance| instance.input_wiring[0].sources.clear()),
                "has no sources",
            ),
            (
                "duplicate bound input",
                Box::new(|instance| instance.input_wiring.push(instance.input_wiring[0].clone())),
                "duplicate bound input slot",
            ),
            (
                "schema version mismatch",
                Box::new(|instance| {
                    let BoundInputWiringSource::TaskInitSlot { schema_version, .. } =
                        &mut instance.input_wiring[0].sources[0]
                    else {
                        unreachable!("fixture uses task init wiring");
                    };
                    *schema_version = 2;
                }),
                "rejects schema version",
            ),
            (
                "one cardinality with multiple sources",
                Box::new(|instance| {
                    let source = instance.input_wiring[0].sources[0].clone();
                    instance.input_wiring[0].sources.push(source);
                }),
                "accepts one source",
            ),
        ];

        for (case_name, mutate, expected) in cases {
            let contract = contract();
            let mut instance = valid_instance(&contract);
            mutate(&mut instance);

            let error = match instance.validate_against(&contract) {
                Ok(()) => panic!("{case_name} should fail validation"),
                Err(error) => error,
            };

            assert!(
                error.to_string().contains(expected),
                "{case_name} expected error containing '{expected}', got '{error}'"
            );
        }
    }
}
