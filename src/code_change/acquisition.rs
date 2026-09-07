//! Read a declared proposal once per admitted invocation and retain its intact product.

use super::{capability::CHANGE_SET, contracts::*};
use crate::{
    capability::*,
    error::ApiError,
    execution::{ExecutionEventContext, ExecutionRuntimeContext},
    task::{ArtifactProducerRef, ArtifactRecord},
};
use async_trait::async_trait;
use meld_events::{AppendMode, EventEnvelope, EventReplayCapability};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, io::Read, path::PathBuf, sync::Arc};

pub const READ: &str = "code_change.read_proposal";
pub const SOURCE: &str = "code-change.proposal";
pub const EVENT: &str = "code_change.proposal_acquired.v1";
/// The serialized document is bounded independently of the accepted replacement bytes.
pub const MAX_DOCUMENT_BYTES: u64 = 32 * 1024 * 1024;

pub fn contract() -> CapabilityTypeContract {
    CapabilityTypeContract {
        capability_type_id: READ.into(),
        capability_version: 1,
        owning_domain: "code-change".into(),
        scope_contract: ScopeContract {
            scope_kind: "workspace_change".into(),
            scope_ref_kind: "subject_id".into(),
            allow_fan_out: false,
        },
        binding_contract: vec![],
        input_contract: vec![],
        output_contract: vec![OutputSlotSpec {
            slot_id: CHANGE_SET.into(),
            artifact_type_id: CHANGE_SET.into(),
            schema_version: 1,
            guaranteed: true,
        }],
        effect_contract: vec![
            EffectSpec {
                effect_id: "read_declared_code_proposal".into(),
                kind: EffectKind::Read,
                target: "declared_code_proposal".into(),
                exclusive: false,
            },
            EffectSpec {
                effect_id: "retain_acquired_code_proposal".into(),
                kind: EffectKind::Emit,
                target: CHANGE_SET.into(),
                exclusive: false,
            },
        ],
        execution_contract: ExecutionContract {
            execution_class: ExecutionClass::Inline,
            completion_semantics: "durable_exact_proposal_or_failure".into(),
            retry_class: "exact_acquisition_receipt_or_source_read".into(),
            cancellation_supported: false,
        },
    }
}

pub(super) fn offer() -> CapabilityImplementationOffer {
    let contract = contract();
    CapabilityImplementationOffer {
        contract_ref: meld_execution::capability::CapabilityContractRevision {
            content_identity: contract.content_identity(),
            contract,
            installed_at_seq: 0,
        }
        .revision_ref(),
        implementation_ref: "code-change.declared-proposal.v1".into(),
        required_binding_ids: BTreeSet::from([SOURCE.into(), "subject".into()]),
        execution_class: ExecutionClass::Inline,
        factory: Arc::new(Factory),
    }
}

struct Factory;
impl CapabilityInvokerFactory for Factory {
    fn prepare(
        &self,
        _: &CapabilityFactoryRequest<'_>,
        bindings: &OwnerBindingView,
    ) -> Result<PreparedCapabilityInvoker, CapabilityContributionDiagnostic> {
        let diagnostic = |message| {
            CapabilityContributionDiagnostic::new("code_proposal_binding_invalid", message)
        };
        let source = PathBuf::from(
            bindings
                .get(SOURCE)
                .ok_or_else(|| diagnostic("declared proposal source absent"))?,
        );
        let subject = bindings
            .get("subject")
            .filter(|value| !value.is_empty())
            .ok_or_else(|| diagnostic("proposal subject absent"))?
            .into();
        if !source.is_absolute() {
            return Err(diagnostic("proposal source must be absolute"));
        }
        // Preparation binds a source address. Only an admitted read touches its content.
        Ok(Arc::new(ProposalReader {
            source,
            subject,
            gate: Default::default(),
        }))
    }
}

struct ProposalReader {
    source: PathBuf,
    subject: String,
    gate: tokio::sync::Mutex<()>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AcquiredProposal {
    binding: serde_json::Value,
    proposal: CodeChangeSet,
}

impl ProposalReader {
    fn binding(
        &self,
        runtime: &CapabilityRuntimeInit,
        payload: &CapabilityInvocationPayload,
        context: Option<&ExecutionEventContext>,
    ) -> Result<serde_json::Value, ApiError> {
        let expected = contract();
        if runtime.capability_type_id != READ
            || runtime.capability_version != 1
            || runtime.scope_ref != self.subject
            || !runtime.binding_values.is_empty()
            || runtime.scope_kind != expected.scope_contract.scope_kind
            || runtime.input_contract != expected.input_contract
            || runtime.output_contract != expected.output_contract
            || runtime.effect_contract != expected.effect_contract
            || runtime.execution_contract != expected.execution_contract
        {
            return Err(invalid(
                "proposal read differs from its exact selected contract",
            ));
        }
        payload.validate_against(runtime)?;
        let context =
            context.ok_or_else(|| invalid("proposal read requires admitted effect context"))?;
        let authority = context
            .effect_authority
            .as_ref()
            .ok_or_else(|| invalid("proposal read requires separate effect authority"))?;
        if context.session_id.is_empty()
            || authority.subject.object_id != self.subject
            || authority.issuer_ref.is_empty()
            || authority.principal_id.is_empty()
            || authority.fence_ref.is_empty()
            || payload
                .upstream_lineage
                .as_ref()
                .is_none_or(|lineage| lineage.task_id.is_empty() || lineage.task_run_id.is_empty())
        {
            return Err(invalid(
                "proposal read requires exact subject and complete admitted lineage",
            ));
        }
        authority.subject.validate().map_err(invalid)?;
        Ok(
            serde_json::json!({"source": self.source, "runtime": runtime, "payload": payload,
            "session": context.session_id, "authority": {"issuer": authority.issuer_ref, "principal": authority.principal_id, "subject": authority.subject, "fence": authority.fence_ref}}),
        )
    }

    fn acquire(&self) -> Result<CodeChangeSet, ApiError> {
        let fd = rustix::fs::open(
            &self.source,
            rustix::fs::OFlags::RDONLY
                | rustix::fs::OFlags::NOFOLLOW
                | rustix::fs::OFlags::NONBLOCK
                | rustix::fs::OFlags::CLOEXEC,
            rustix::fs::Mode::empty(),
        )
        .map_err(invalid)?;
        let file = std::fs::File::from(fd);
        if !file.metadata().map_err(invalid)?.is_file() {
            return Err(invalid("declared proposal is not a regular file"));
        }
        let mut bytes = Vec::new();
        file.take(MAX_DOCUMENT_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(invalid)?;
        if bytes.len() as u64 > MAX_DOCUMENT_BYTES {
            return Err(invalid("declared proposal exceeds its document bound"));
        }
        let proposal: CodeChangeSet = serde_json::from_slice(&bytes).map_err(invalid)?;
        proposal.validate().map_err(invalid)?;
        Ok(proposal)
    }
}

fn id(events: &EventReplayCapability, binding: &serde_json::Value) -> Result<String, ApiError> {
    Ok(format!(
        "code-proposal::{}",
        hash(&(events.ledger_identity(), binding)).map_err(invalid)?
    ))
}
fn envelope(
    events: &EventReplayCapability,
    acquired: &AcquiredProposal,
) -> Result<EventEnvelope, ApiError> {
    Ok(EventEnvelope::with_now_domain(
        "code-change",
        "code-change",
        &acquired.proposal.subject.object_id,
        EVENT,
        None,
        serde_json::to_value(acquired).map_err(invalid)?,
    )
    .with_record_id(id(events, &acquired.binding)?))
}
fn recover(
    events: &EventReplayCapability,
    binding: &serde_json::Value,
) -> Result<Option<CodeChangeSet>, ApiError> {
    let Some(record) = events
        .committed_record(&id(events, binding)?)
        .map_err(invalid)?
    else {
        return Ok(None);
    };
    let acquired: AcquiredProposal =
        serde_json::from_value(record.data.clone()).map_err(invalid)?;
    acquired.proposal.validate().map_err(invalid)?;
    if &acquired.binding != binding
        || serde_json::to_value(&acquired.proposal.subject).map_err(invalid)?
            != binding["authority"]["subject"]
        || events
            .prove_existing(&envelope(events, &acquired)?)
            .map_err(invalid)?
            .is_none()
    {
        return Err(invalid(
            "retained proposal differs from its exact admitted acquisition",
        ));
    }
    Ok(Some(acquired.proposal))
}
fn result(
    runtime: &CapabilityRuntimeInit,
    payload: &CapabilityInvocationPayload,
    proposal: CodeChangeSet,
) -> Result<CapabilityInvocationResult, ApiError> {
    Ok(CapabilityInvocationResult {
        emitted_artifacts: vec![ArtifactRecord {
            artifact_id: format!("{}::{CHANGE_SET}", payload.invocation_id),
            artifact_type_id: CHANGE_SET.into(),
            schema_version: 1,
            content: serde_json::to_value(proposal).map_err(invalid)?,
            producer: ArtifactProducerRef {
                task_id: payload
                    .upstream_lineage
                    .as_ref()
                    .map(|lineage| lineage.task_id.clone())
                    .unwrap_or_default(),
                capability_instance_id: runtime.capability_instance_id.clone(),
                invocation_id: Some(payload.invocation_id.clone()),
                output_slot_id: Some(CHANGE_SET.into()),
            },
        }],
    })
}
fn invalid(error: impl ToString) -> ApiError {
    ApiError::ConfigError(error.to_string())
}

#[async_trait]
impl CapabilityInvoker for ProposalReader {
    type Error = ApiError;
    type ExecutionApi = dyn ExecutionRuntimeContext;
    fn contract(&self) -> CapabilityTypeContract {
        contract()
    }
    async fn invoke(
        &self,
        api: &dyn ExecutionRuntimeContext,
        runtime: &CapabilityRuntimeInit,
        payload: &CapabilityInvocationPayload,
        context: Option<&ExecutionEventContext>,
    ) -> Result<CapabilityInvocationResult, ApiError> {
        let _owner = self.gate.lock().await;
        let binding = self.binding(runtime, payload, context)?;
        let events = api
            .durable_event_append()
            .ok_or_else(|| invalid("proposal acquisition requires durable Events"))?;
        let replay = events.replay_capability();
        if let Some(proposal) = recover(&replay, &binding)? {
            return result(runtime, payload, proposal);
        }
        let proposal = self.acquire()?;
        if serde_json::to_value(&proposal.subject).map_err(invalid)?
            != binding["authority"]["subject"]
        {
            return Err(invalid("proposal names a foreign admitted subject"));
        }
        let acquired = AcquiredProposal {
            binding: binding.clone(),
            proposal,
        };
        events
            .append_durable_proven(envelope(&replay, &acquired)?, AppendMode::Idempotent)
            .map_err(invalid)?;
        result(
            runtime,
            payload,
            recover(&replay, &binding)?
                .ok_or_else(|| invalid("proposal acquisition has no durable proof"))?,
        )
    }
    async fn recover(
        &self,
        events: Option<&EventReplayCapability>,
        runtime: &CapabilityRuntimeInit,
        payload: &CapabilityInvocationPayload,
        context: Option<&ExecutionEventContext>,
    ) -> Result<Option<CapabilityInvocationResult>, ApiError> {
        let binding = self.binding(runtime, payload, context)?;
        events
            .map(|events| recover(events, &binding))
            .transpose()?
            .flatten()
            .map(|proposal| result(runtime, payload, proposal))
            .transpose()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use meld_events::{DomainObjectRef, EventAuthority, EventAuthorityOpenOptions};

    fn proposal() -> CodeChangeSet {
        CodeChangeSet::new(
            DomainObjectRef::new("workspace_fs", "node", "repo").unwrap(),
            vec![],
            vec![FileReplacement {
                relative_path: "Cargo.toml".into(),
                expected_content_hash: blake3::hash(b"before").to_hex().to_string(),
                replacement: "after".into(),
            }],
        )
        .unwrap()
    }

    #[test]
    fn retained_proposal_survives_source_loss_and_refuses_foreign_binding_or_tampering() {
        let root = tempfile::tempdir().unwrap();
        let reader = ProposalReader {
            source: root.path().join("proposal.json"),
            subject: "repo".into(),
            gate: Default::default(),
        };
        let original = proposal();
        std::fs::write(&reader.source, serde_json::to_vec(&original).unwrap()).unwrap();
        let proposal = reader.acquire().unwrap();
        assert_eq!(proposal, original);
        let events = EventAuthority::open(
            sled::open(root.path().join("events")).unwrap(),
            EventAuthorityOpenOptions::default(),
        )
        .unwrap();
        let replay = events.replay_capability();
        let binding = serde_json::json!({ "source": reader.source, "authority": { "subject": proposal.subject, "fence": "original" } });
        let acquired = AcquiredProposal {
            binding: binding.clone(),
            proposal,
        };
        events
            .append_capability()
            .append_durable_proven(
                envelope(&replay, &acquired).unwrap(),
                AppendMode::Idempotent,
            )
            .unwrap();
        std::fs::remove_file(&reader.source).unwrap();
        drop(replay);
        drop(events);
        let events = EventAuthority::open(
            sled::open(root.path().join("events")).unwrap(),
            EventAuthorityOpenOptions::default(),
        )
        .unwrap();
        let replay = events.replay_capability();
        assert_eq!(recover(&replay, &binding).unwrap(), Some(original));
        let mut foreign = binding.clone();
        foreign["authority"]["fence"] = "successor".into();
        assert!(recover(&replay, &foreign).unwrap().is_none());
        let mut foreign = binding.clone();
        foreign["source"] = "another/proposal.json".into();
        assert!(recover(&replay, &foreign).unwrap().is_none());
        let mut tampered = acquired.clone();
        tampered.binding["authority"]["subject"]["domain_id"] = "foreign".into();
        events
            .append_capability()
            .append_durable_proven(
                envelope(&replay, &tampered).unwrap(),
                AppendMode::Idempotent,
            )
            .unwrap();
        assert!(recover(&replay, &tampered.binding).is_err());
        let mut tampered = acquired;
        tampered.binding["attempt"] = "forged-content".into();
        tampered.proposal.files[0].replacement = "different".into();
        events
            .append_capability()
            .append_durable_proven(
                envelope(&replay, &tampered).unwrap(),
                AppendMode::Idempotent,
            )
            .unwrap();
        assert!(recover(&replay, &tampered.binding).is_err());
    }

    #[test]
    fn proposal_source_refuses_nonregular_oversized_and_noncanonical_documents() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("proposal.json");
        let reader = ProposalReader {
            source: path.clone(),
            subject: "repo".into(),
            gate: Default::default(),
        };
        let mut tampered = proposal();
        tampered.files[0].relative_path = "../escape".into();
        std::fs::write(&path, serde_json::to_vec(&tampered).unwrap()).unwrap();
        assert!(reader.acquire().is_err());
        let file = std::fs::File::create(&path).unwrap();
        file.set_len(MAX_DOCUMENT_BYTES + 1).unwrap();
        assert!(reader.acquire().is_err());
        drop(file);
        std::fs::remove_file(&path).unwrap();
        let elsewhere = root.path().join("elsewhere");
        std::fs::write(&elsewhere, serde_json::to_vec(&proposal()).unwrap()).unwrap();
        std::os::unix::fs::symlink(elsewhere, &path).unwrap();
        assert!(reader.acquire().is_err());
        std::fs::remove_file(&path).unwrap();
        std::fs::create_dir(&path).unwrap();
        assert!(reader.acquire().is_err());
    }
}
