//! Dependency-security actions carry distinct owner products through Execution.

use async_trait::async_trait;
use serde::Serialize;
use std::path::PathBuf;

use super::contracts::*;
use super::inventory::cargo::CargoInventoryLimits;
use super::policy::DependencySecurityPolicyV1;
use crate::capability::*;
use crate::error::ApiError;
use crate::execution::{ExecutionEventContext, ExecutionRuntimeContext};
use crate::task::{ArtifactProducerRef, ArtifactRecord};

pub use super::contribution::DependencySecurityCapabilityContributor;

pub const OBSERVE_INVENTORY: &str = "dependency_security.observe_inventory";
pub const ACQUIRE_ADVISORIES: &str = "dependency_security.acquire_advisories";
pub const ASSESS: &str = "dependency_security.assess";
pub const VERIFY: &str = "dependency_security.verify";
pub const INVENTORY: &str = "dependency_security_inventory";
pub const ADVISORIES: &str = "dependency_security_advisories";
pub const ASSESSMENT: &str = "dependency_security_assessment";
pub const VERIFICATION: &str = "dependency_security_verification";
const VERSION: u32 = 1;

pub fn published_contracts() -> Vec<CapabilityTypeContract> {
    [OBSERVE_INVENTORY, ACQUIRE_ADVISORIES, ASSESS, VERIFY]
        .into_iter()
        .map(contract)
        .collect()
}

pub(crate) fn contract(id: &str) -> CapabilityTypeContract {
    let (inputs, output): (&[&str], _) = match id {
        OBSERVE_INVENTORY => (&[], INVENTORY),
        ACQUIRE_ADVISORIES => (&[], ADVISORIES),
        ASSESS => (&[INVENTORY, ADVISORIES], ASSESSMENT),
        VERIFY => (&[INVENTORY, ADVISORIES, ASSESSMENT], VERIFICATION),
        _ => unreachable!("unknown dependency-security Capability"),
    };
    let mut effects = vec![EffectSpec {
        effect_id: format!("{id}.emit"),
        kind: EffectKind::Emit,
        target: output.into(),
        exclusive: false,
    }];
    if matches!(id, OBSERVE_INVENTORY | ACQUIRE_ADVISORIES) {
        effects.push(EffectSpec {
            effect_id: format!("{id}.read"),
            kind: EffectKind::Read,
            target: if id == OBSERVE_INVENTORY {
                "cargo_dependency_graph"
            } else {
                "advisory_source"
            }
            .into(),
            exclusive: false,
        });
    }
    CapabilityTypeContract {
        capability_type_id: id.into(),
        capability_version: VERSION,
        owning_domain: "dependency-security".into(),
        scope_contract: ScopeContract {
            scope_kind: "cargo_dependency_graph".into(),
            scope_ref_kind: "subject_id".into(),
            allow_fan_out: false,
        },
        binding_contract: vec![],
        input_contract: inputs
            .iter()
            .map(|id| InputSlotSpec {
                slot_id: (*id).into(),
                accepted_artifact_type_ids: vec![(*id).into()],
                schema_versions: ArtifactSchemaVersionRange { min: 1, max: 1 },
                required: true,
                cardinality: InputCardinality::One,
            })
            .collect(),
        output_contract: vec![OutputSlotSpec {
            slot_id: output.into(),
            artifact_type_id: output.into(),
            schema_version: VERSION,
            guaranteed: true,
        }],
        effect_contract: effects,
        execution_contract: ExecutionContract {
            execution_class: ExecutionClass::Inline,
            completion_semantics: "owner_validated_product_or_failure".into(),
            retry_class: "source_reacquisition_or_exact_input_replay".into(),
            cancellation_supported: false,
        },
    }
}

pub(crate) struct SecurityCapability {
    pub id: String,
    pub subject: DependencySecuritySubjectV1,
    pub policy: DependencySecurityPolicyV1,
    pub workspace: Option<PathBuf>,
    pub cargo: Option<PathBuf>,
    pub advisories: Option<PathBuf>,
    pub limits: CargoInventoryLimits,
}

#[async_trait]
impl meld_execution::capability::CapabilityInvoker for SecurityCapability {
    type Error = ApiError;
    type ExecutionApi = dyn ExecutionRuntimeContext;
    fn contract(&self) -> CapabilityTypeContract {
        contract(&self.id)
    }

    async fn invoke(
        &self,
        _api: &dyn ExecutionRuntimeContext,
        runtime_init: &CapabilityRuntimeInit,
        payload: &CapabilityInvocationPayload,
        _event_context: Option<&ExecutionEventContext>,
    ) -> Result<CapabilityInvocationResult, ApiError> {
        let expected = self.contract();
        if runtime_init.capability_type_id != self.id
            || runtime_init.capability_version != VERSION
            || runtime_init.scope_ref != self.subject.subject.object_id
            || runtime_init.scope_kind != expected.scope_contract.scope_kind
            || runtime_init.input_contract != expected.input_contract
            || runtime_init.output_contract != expected.output_contract
            || runtime_init.effect_contract != expected.effect_contract
            || runtime_init.execution_contract != expected.execution_contract
        {
            return Err(invalid(
                "invocation differs from its prepared Security owner contract or subject",
            ));
        }
        payload.validate_against(runtime_init)?;
        let reference_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|error| invalid(error.to_string()))?
            .as_secs();
        let (artifact_type, content) = match self.id.as_str() {
            OBSERVE_INVENTORY => {
                let inventory = super::inventory::cargo::observe(
                    self.workspace
                        .as_deref()
                        .ok_or_else(|| invalid("inventory workspace is absent"))?,
                    self.cargo
                        .as_deref()
                        .ok_or_else(|| invalid("inventory executable is absent"))?,
                    self.subject.clone(),
                    reference_time,
                    &self.limits,
                )
                .await
                .map_err(invalid)?;
                (INVENTORY, encode(inventory)?)
            }
            ACQUIRE_ADVISORIES => {
                let bytes = super::inventory::cargo::read_bounded(
                    self.advisories
                        .as_deref()
                        .ok_or_else(|| invalid("advisory resource is absent"))?,
                    self.limits.maximum_bytes,
                )
                .map_err(invalid)?;
                let source: super::advisory::AdvisorySourceDocumentV1 =
                    serde_json::from_slice(&bytes).map_err(|error| invalid(error.to_string()))?;
                let advisory = source.admit().map_err(invalid)?;
                if advisory.source_id != self.policy.required_advisory_source_id
                    || advisory.covered_ecosystem != self.policy.ecosystem
                {
                    return Err(invalid(
                        "advisory source does not match the exact installed policy",
                    ));
                }
                (ADVISORIES, encode(advisory)?)
            }
            ASSESS => {
                let inventory = decode(payload, INVENTORY)?;
                let advisories = decode(payload, ADVISORIES)?;
                let assessment = super::assessment::assess(
                    &self.subject,
                    Some(&inventory),
                    Some(&advisories),
                    &self.policy,
                    reference_time,
                )
                .map_err(invalid)?;
                (ASSESSMENT, encode(assessment)?)
            }
            VERIFY => {
                let inventory: DependencyInventorySnapshotV1 = decode(payload, INVENTORY)?;
                let advisories = decode(payload, ADVISORIES)?;
                let assessment: DependencySecurityAssessmentV1 = decode(payload, ASSESSMENT)?;
                if assessment.subject != self.subject || inventory.subject != self.subject {
                    return Err(invalid(
                        "verification products name a foreign assigned subject",
                    ));
                }
                let verification =
                    super::verification::verify(&assessment, &inventory, &advisories, &self.policy)
                        .map_err(invalid)?;
                (VERIFICATION, encode(verification)?)
            }
            _ => return Err(invalid("unknown dependency-security action")),
        };
        Ok(CapabilityInvocationResult {
            emitted_artifacts: vec![ArtifactRecord {
                artifact_id: format!("{}::{artifact_type}", payload.invocation_id),
                artifact_type_id: artifact_type.into(),
                schema_version: VERSION,
                content,
                producer: ArtifactProducerRef {
                    task_id: payload
                        .upstream_lineage
                        .as_ref()
                        .map(|lineage| lineage.task_id.clone())
                        .unwrap_or_default(),
                    capability_instance_id: runtime_init.capability_instance_id.clone(),
                    invocation_id: Some(payload.invocation_id.clone()),
                    output_slot_id: Some(artifact_type.into()),
                },
            }],
        })
    }
}

fn decode<T: serde::de::DeserializeOwned>(
    payload: &CapabilityInvocationPayload,
    slot: &str,
) -> Result<T, ApiError> {
    let input = payload
        .supplied_inputs
        .iter()
        .find(|input| input.slot_id == slot)
        .ok_or_else(|| invalid(format!("missing Security input '{slot}'")))?;
    let value = match &input.value {
        SuppliedValueRef::Artifact(value) => value.content.clone(),
        SuppliedValueRef::StructuredValue(value) => value.clone(),
    };
    serde_json::from_value(value).map_err(|error| invalid(error.to_string()))
}

fn encode(value: impl Serialize) -> Result<serde_json::Value, ApiError> {
    serde_json::to_value(value).map_err(|error| invalid(error.to_string()))
}

fn invalid(message: impl Into<String>) -> ApiError {
    ApiError::ConfigError(format!("dependency-security: {}", message.into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn capabilities_are_read_and_emit_only() {
        for contract in published_contracts() {
            println!(
                "{} {}",
                contract.capability_type_id,
                contract.content_identity()
            );
            assert_eq!(contract.owning_domain, "dependency-security");
            assert!(contract
                .effect_contract
                .iter()
                .all(|effect| matches!(effect.kind, EffectKind::Read | EffectKind::Emit)));
        }
    }
}
