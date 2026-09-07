use super::{contracts::*, operation::Operation};
use crate::{
    capability::*,
    error::ApiError,
    execution::{ExecutionEventContext, ExecutionRuntimeContext},
    task::{ArtifactProducerRef, ArtifactRecord},
};
use async_trait::async_trait;
use std::{collections::BTreeSet, path::PathBuf, sync::Arc};

pub const APPLY: &str = "code_change.apply_existing_files";
pub const CHANGE_SET: &str = "code_change_set";
pub const RECEIPT: &str = "code_change_receipt";

pub fn contract() -> CapabilityTypeContract {
    CapabilityTypeContract {
        capability_type_id: APPLY.into(),
        capability_version: 1,
        owning_domain: "code-change".into(),
        scope_contract: ScopeContract {
            scope_kind: "workspace_change".into(),
            scope_ref_kind: "subject_id".into(),
            allow_fan_out: false,
        },
        binding_contract: vec![],
        input_contract: vec![InputSlotSpec {
            slot_id: CHANGE_SET.into(),
            accepted_artifact_type_ids: vec![CHANGE_SET.into()],
            schema_versions: ArtifactSchemaVersionRange { min: 1, max: 1 },
            required: true,
            cardinality: InputCardinality::One,
        }],
        output_contract: vec![OutputSlotSpec {
            slot_id: RECEIPT.into(),
            artifact_type_id: RECEIPT.into(),
            schema_version: 1,
            guaranteed: true,
        }],
        effect_contract: vec![
            EffectSpec {
                effect_id: "read_declared_code_basis".into(),
                kind: EffectKind::Read,
                target: "workspace_tree".into(),
                exclusive: false,
            },
            EffectSpec {
                effect_id: "materialize_declared_code_change".into(),
                kind: EffectKind::Write,
                target: "workspace_tree".into(),
                exclusive: true,
            },
            EffectSpec {
                effect_id: "retain_code_change_evidence".into(),
                kind: EffectKind::Emit,
                target: RECEIPT.into(),
                exclusive: false,
            },
        ],
        execution_contract: ExecutionContract {
            execution_class: ExecutionClass::Inline,
            completion_semantics: "durable_exact_materialization_receipt_or_unresolved_intent"
                .into(),
            retry_class: "same_intent_and_unconflicted_source_basis".into(),
            cancellation_supported: false,
        },
    }
}

#[derive(Default)]
pub struct CodeChangeCapabilityContributor;
impl ProductCapabilityContributor for CodeChangeCapabilityContributor {
    fn owner_domain(&self) -> &str {
        "code-change"
    }
    fn published_contracts(&self) -> Vec<meld_execution::capability::CapabilityContractRevision> {
        [contract(), super::acquisition::contract()]
            .into_iter()
            .map(
                |contract| meld_execution::capability::CapabilityContractRevision {
                    content_identity: contract.content_identity(),
                    contract,
                    installed_at_seq: 0,
                },
            )
            .collect()
    }
    fn implementation_offers(&self) -> Vec<CapabilityImplementationOffer> {
        vec![
            CapabilityImplementationOffer {
                contract_ref: self.published_contracts()[0].revision_ref(),
                implementation_ref: "code-change.directory-handles.v1".into(),
                required_binding_ids: BTreeSet::from(["workspace".into(), "subject".into()]),
                execution_class: ExecutionClass::Inline,
                factory: Arc::new(Factory),
            },
            super::acquisition::offer(),
        ]
    }
}

struct Factory;
impl CapabilityInvokerFactory for Factory {
    fn prepare(
        &self,
        _: &CapabilityFactoryRequest<'_>,
        bindings: &OwnerBindingView,
    ) -> Result<PreparedCapabilityInvoker, CapabilityContributionDiagnostic> {
        let root = PathBuf::from(
            bindings
                .get("workspace")
                .ok_or_else(|| diagnostic("code change workspace is absent"))?,
        );
        let subject = bindings
            .get("subject")
            .filter(|s| !s.is_empty())
            .ok_or_else(|| diagnostic("code change subject is absent"))?
            .into();
        if !root.is_absolute() {
            return Err(diagnostic(
                "code change requires an absolute workspace binding",
            ));
        }
        Ok(Arc::new(CodeChangeCapability {
            root,
            subject,
            gate: Default::default(),
        }))
    }
}

struct CodeChangeCapability {
    root: PathBuf,
    subject: String,
    gate: tokio::sync::Mutex<()>,
}

impl CodeChangeCapability {
    fn operation(
        &self,
        runtime: &CapabilityRuntimeInit,
        payload: &CapabilityInvocationPayload,
        context: Option<&ExecutionEventContext>,
    ) -> Result<Operation<'_>, ApiError> {
        validate_runtime(runtime, &self.subject)?;
        payload.validate_against(runtime)?;
        if payload
            .upstream_lineage
            .as_ref()
            .is_none_or(|lineage| lineage.task_id.is_empty() || lineage.task_run_id.is_empty())
        {
            return Err(invalid(
                "code mutation requires complete Task and run lineage",
            ));
        }
        let input = payload
            .supplied_inputs
            .iter()
            .find(|input| input.slot_id == CHANGE_SET)
            .ok_or_else(|| invalid("code change input is absent"))?;
        let body = match &input.value {
            SuppliedValueRef::StructuredValue(value) => value,
            SuppliedValueRef::Artifact(value) => &value.content,
        };
        let change: CodeChangeSet = serde_json::from_value(body.clone()).map_err(invalid)?;
        change.validate().map_err(invalid)?;
        let context =
            context.ok_or_else(|| invalid("code mutation requires admitted effect context"))?;
        let authority = context
            .effect_authority
            .as_ref()
            .ok_or_else(|| invalid("code mutation requires separate effect authority"))?;
        if change.subject != authority.subject
            || change.subject.object_id != self.subject
            || authority.issuer_ref.is_empty()
            || authority.principal_id.is_empty()
            || authority.fence_ref.is_empty()
            || context.session_id.is_empty()
        {
            return Err(invalid(
                "code change does not match its admitted subject, principal or fence",
            ));
        }
        let input_origin = match &input.value {
            SuppliedValueRef::StructuredValue(_) => {
                serde_json::json!({"source": input.source, "kind": "structured"})
            }
            SuppliedValueRef::Artifact(artifact) => {
                serde_json::json!({"source": input.source, "kind": "artifact", "artifact_id": artifact.artifact_id, "artifact_type_id": artifact.artifact_type_id, "schema_version": artifact.schema_version})
            }
        };
        // Intent retains the intact change once; the payload hash binds its original transport envelope.
        let mut binding = serde_json::json!({"runtime": runtime, "payload_identity": hash(payload).map_err(invalid)?, "invocation_id": payload.invocation_id, "input_origin": input_origin, "lineage": payload.upstream_lineage, "execution_context": payload.execution_context, "workspace": self.root, "session": context.session_id, "authority": {"issuer": authority.issuer_ref, "principal": authority.principal_id, "subject": authority.subject, "fence": authority.fence_ref}});
        if let Some(request) = &authority.request_ref {
            if request.trim().is_empty() {
                return Err(invalid("code request identity is empty"));
            }
            binding["authority"]["request_ref"] = request.clone().into();
        }
        Ok(Operation {
            binding,
            change,
            root: &self.root,
        })
    }
}

#[async_trait]
impl CapabilityInvoker for CodeChangeCapability {
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
        let operation = self.operation(runtime, payload, context)?;
        let events = api
            .durable_event_append()
            .ok_or_else(|| invalid("code change requires durable Event authority"))?;
        let operation = operation
            .retain_original_binding(&events.replay_capability())
            .map_err(invalid)?;
        let result = result(runtime, payload, operation.apply(&events).map_err(invalid)?)?;
        let authority = context
            .and_then(|context| context.effect_authority.as_ref())
            .ok_or_else(|| invalid("code account requires admitted authority"))?;
        events
            .append_durable_proven(
                super::publication::envelope(
                    &events.replay_capability(),
                    authority,
                    &result.emitted_artifacts[0],
                )
                .map_err(invalid)?,
                meld_events::AppendMode::Idempotent,
            )
            .map_err(invalid)?;
        Ok(result)
    }
    async fn recover(
        &self,
        events: Option<&meld_events::EventReplayCapability>,
        runtime: &CapabilityRuntimeInit,
        payload: &CapabilityInvocationPayload,
        context: Option<&ExecutionEventContext>,
    ) -> Result<Option<CapabilityInvocationResult>, ApiError> {
        let operation = self.operation(runtime, payload, context)?;
        let Some(events) = events else {
            return Ok(None);
        };
        let operation = operation.retain_original_binding(events).map_err(invalid)?;
        let result = operation
            .recover(events)
            .map_err(invalid)?
            .map(|receipt| result(runtime, payload, receipt))
            .transpose()?;
        let Some(result) = result else {
            return Ok(None);
        };
        let authority = context
            .and_then(|context| context.effect_authority.as_ref())
            .ok_or_else(|| invalid("code account requires admitted authority"))?;
        let envelope =
            super::publication::envelope(events, authority, &result.emitted_artifacts[0])
                .map_err(invalid)?;
        // A retained mutation with an interrupted publication resumes through invoke,
        // whose native operation recovers the receipt without touching source files.
        Ok(events
            .prove_existing(&envelope)
            .map_err(invalid)?
            .map(|_| result))
    }
}

pub(super) fn validate_runtime(
    runtime: &CapabilityRuntimeInit,
    subject: &str,
) -> Result<(), ApiError> {
    let expected = contract();
    if runtime.capability_type_id != APPLY
        || runtime.capability_version != 1
        || runtime.scope_ref != subject
        || !runtime.binding_values.is_empty()
        || runtime.scope_kind != expected.scope_contract.scope_kind
        || runtime.input_contract != expected.input_contract
        || runtime.output_contract != expected.output_contract
        || runtime.effect_contract != expected.effect_contract
        || runtime.execution_contract != expected.execution_contract
    {
        return Err(invalid(
            "code change invocation differs from its exact selected contract",
        ));
    }
    Ok(())
}

fn result(
    runtime: &CapabilityRuntimeInit,
    payload: &CapabilityInvocationPayload,
    receipt: CodeChangeReceipt,
) -> Result<CapabilityInvocationResult, ApiError> {
    Ok(CapabilityInvocationResult {
        emitted_artifacts: vec![materialization_artifact(
            runtime,
            &payload.invocation_id,
            payload
                .upstream_lineage
                .as_ref()
                .map(|lineage| lineage.task_id.as_str())
                .unwrap_or_default(),
            receipt,
        )?],
    })
}

pub(super) fn materialization_artifact(
    runtime: &CapabilityRuntimeInit,
    invocation_id: &str,
    task_id: &str,
    receipt: CodeChangeReceipt,
) -> Result<ArtifactRecord, ApiError> {
    Ok(ArtifactRecord {
        artifact_id: format!("{invocation_id}::{RECEIPT}"),
        artifact_type_id: RECEIPT.into(),
        schema_version: 1,
        content: serde_json::to_value(receipt).map_err(invalid)?,
        producer: ArtifactProducerRef {
            task_id: task_id.into(),
            capability_instance_id: runtime.capability_instance_id.clone(),
            invocation_id: Some(invocation_id.into()),
            output_slot_id: Some(RECEIPT.into()),
        },
    })
}

fn diagnostic(message: impl Into<String>) -> CapabilityContributionDiagnostic {
    CapabilityContributionDiagnostic::new("code_change_binding_invalid", message)
}
fn invalid(error: impl ToString) -> ApiError {
    ApiError::ConfigError(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use meld_events::{DomainObjectRef, EventAuthority, EventAuthorityOpenOptions};
    use std::collections::BTreeMap;

    fn api(root: &std::path::Path) -> crate::api::ContextApi {
        crate::api::ContextApi::new(
            Arc::new(crate::store::SledNodeRecordStore::new(root.join("nodes")).unwrap()),
            Arc::new(crate::context::frame::FrameStorage::new(root.join("frames")).unwrap()),
            Arc::new(parking_lot::RwLock::new(crate::heads::HeadIndex::new())),
            Arc::new(
                crate::prompt_context::PromptContextArtifactStorage::new(root.join("prompts"))
                    .unwrap(),
            ),
            Arc::new(parking_lot::RwLock::new(crate::agent::AgentRegistry::new())),
            Arc::new(parking_lot::RwLock::new(
                crate::provider::ProviderRegistry::new(),
            )),
            Arc::new(crate::concurrency::NodeLockManager::new()),
        )
    }

    #[tokio::test]
    async fn exact_preparation_keeps_mutation_unselected_until_requested_and_preserves_fenced_returns(
    ) {
        let root = tempfile::tempdir().unwrap();
        let external = tempfile::tempdir().unwrap();
        let target = root.path().join("workspace");
        let inventory = crate::capability::product_capability_inventory().unwrap();
        let reference = inventory
            .contracts()
            .find(|revision| revision.contract.capability_type_id == APPLY)
            .unwrap()
            .revision_ref();
        let bindings = OwnerBindingView::new(BTreeMap::from([
            ("workspace".into(), target.display().to_string()),
            ("subject".into(), "repo".into()),
        ]));
        let request = ExactCapabilityActivationRequest {
            assignment_id: "code-assignment".into(),
            activation_id: "code-activation".into(),
            selected_contracts: vec![reference.clone()],
            selected_implementations: BTreeMap::from([(
                reference.clone(),
                inventory.unique_implementation_ref(&reference).unwrap(),
            )]),
            compatibility_policy_revision: "capability-compatibility.v1".into(),
        };
        let mut without = request.clone();
        without.selected_contracts.clear();
        without.selected_implementations.clear();
        let unselected = inventory.prepare(without, &bindings).unwrap();
        assert!(unselected.invokers.get(APPLY, 1).is_none());
        let prepared = inventory.prepare(request, &bindings).unwrap();
        assert!(
            !target.exists(),
            "preparation cannot read or create source files"
        );
        std::fs::create_dir(&target).unwrap();
        std::fs::write(target.join("Cargo.toml"), "old content").unwrap();
        let subject = DomainObjectRef::new("workspace_fs", "node", "repo").unwrap();
        let change = CodeChangeSet::new(
            subject.clone(),
            vec![],
            vec![FileReplacement {
                relative_path: "Cargo.toml".into(),
                expected_content_hash: blake3::hash(b"old content").to_hex().to_string(),
                replacement: "new content".into(),
            }],
        )
        .unwrap();
        let contract = contract();
        let runtime = CapabilityRuntimeInit {
            capability_instance_id: "apply".into(),
            capability_type_id: APPLY.into(),
            capability_version: 1,
            scope_ref: "repo".into(),
            scope_kind: contract.scope_contract.scope_kind,
            binding_values: vec![],
            input_contract: contract.input_contract,
            output_contract: contract.output_contract,
            effect_contract: contract.effect_contract,
            execution_contract: contract.execution_contract,
        };
        let payload = CapabilityInvocationPayload {
            invocation_id: "apply-exact-change".into(),
            capability_instance_id: "apply".into(),
            supplied_inputs: vec![SuppliedInputValue {
                slot_id: CHANGE_SET.into(),
                source: InputValueSource::InitPayload,
                value: SuppliedValueRef::StructuredValue(serde_json::to_value(&change).unwrap()),
            }],
            upstream_lineage: Some(UpstreamLineage {
                task_id: "code-task".into(),
                task_run_id: "code-task-run".into(),
                capability_path: vec!["apply".into()],
                ..Default::default()
            }),
            execution_context: Default::default(),
        };
        let context = ExecutionEventContext {
            session_id: "code-test".into(),
            effect_authority: Some(meld_execution::ExecutionEffectAuthority {
                request_ref: None,
                issuer_ref: "agent".into(),
                principal_id: "workspace-owner".into(),
                subject,
                fence_ref: "code-fence".into(),
            }),
        };
        let events = EventAuthority::open(
            sled::open(external.path().join("events")).unwrap(),
            EventAuthorityOpenOptions::default(),
        )
        .unwrap();
        let api = api(external.path());
        api.bind_event_append(events.append_capability()).unwrap();
        let invoker = prepared.invokers.get(APPLY, 1).unwrap();
        assert!(invoker
            .invoke(&api, &runtime, &payload, None)
            .await
            .is_err());
        let mut no_lineage = payload.clone();
        no_lineage.upstream_lineage = None;
        assert!(invoker
            .invoke(&api, &runtime, &no_lineage, Some(&context))
            .await
            .is_err());
        let mut no_write = runtime.clone();
        no_write.effect_contract.clear();
        assert!(invoker
            .invoke(&api, &no_write, &payload, Some(&context))
            .await
            .is_err());
        let mut foreign = context.clone();
        foreign.effect_authority.as_mut().unwrap().subject.domain_id = "foreign".into();
        assert!(invoker
            .invoke(&api, &runtime, &payload, Some(&foreign))
            .await
            .is_err());
        assert_eq!(events.watermark_capability().snapshot().unwrap().tip_seq, 0);
        assert_eq!(
            std::fs::read_to_string(target.join("Cargo.toml")).unwrap(),
            "old content"
        );
        let owner = CodeChangeCapability {
            root: target.clone(),
            subject: "repo".into(),
            gate: Default::default(),
        };
        owner
            .operation(&runtime, &payload, Some(&context))
            .unwrap()
            .apply(&events.append_capability())
            .unwrap();
        assert!(
            invoker
                .recover(
                    Some(&events.replay_capability()),
                    &runtime,
                    &payload,
                    Some(&context)
                )
                .await
                .unwrap()
                .is_none(),
            "materialization alone cannot stand in for its missing owner publication"
        );
        std::fs::write(
            target.join("Cargo.toml"),
            "edit after materialization before publication",
        )
        .unwrap();
        let result = invoker
            .invoke(&api, &runtime, &payload, Some(&context))
            .await
            .unwrap();
        assert_eq!(
            std::fs::read_to_string(target.join("Cargo.toml")).unwrap(),
            "edit after materialization before publication"
        );
        let published = super::super::publication::envelope(
            &events.replay_capability(),
            context.effect_authority.as_ref().unwrap(),
            &result.emitted_artifacts[0],
        )
        .unwrap();
        assert!(events
            .replay_capability()
            .prove_existing(&published)
            .unwrap()
            .is_some());
        let mut foreign_authority = context.effect_authority.clone().unwrap();
        foreign_authority.fence_ref = "foreign-epoch".into();
        assert!(super::super::publication::envelope(
            &events.replay_capability(),
            &foreign_authority,
            &result.emitted_artifacts[0]
        )
        .is_err());
        assert_eq!(result.emitted_artifacts[0].producer.task_id, "code-task");
        let evidence = crate::code_change::materialization_evidence(
            &events.replay_capability(),
            &result.emitted_artifacts[0],
        )
        .unwrap();
        assert_eq!(evidence.change, change);
        assert!(evidence.intent.seq < evidence.materialization.seq);
        let mut foreign_artifact = result.emitted_artifacts[0].clone();
        foreign_artifact.producer.invocation_id = Some("foreign-invocation".into());
        assert!(crate::code_change::materialization_evidence(
            &events.replay_capability(),
            &foreign_artifact
        )
        .is_err());
        let mut tampered = result.emitted_artifacts[0].clone();
        tampered.content["materialized_files"][0]["after_hash"] = "foreign-content".into();
        assert!(crate::code_change::materialization_evidence(
            &events.replay_capability(),
            &tampered
        )
        .is_err());
        assert_eq!(
            invoker
                .recover(
                    Some(&events.replay_capability()),
                    &runtime,
                    &payload,
                    Some(&context)
                )
                .await
                .unwrap()
                .unwrap()
                .emitted_artifacts,
            result.emitted_artifacts
        );
        let mut successor = context.clone();
        successor.effect_authority.as_mut().unwrap().fence_ref = "another-fence".into();
        assert!(invoker
            .recover(
                Some(&events.replay_capability()),
                &runtime,
                &payload,
                Some(&successor)
            )
            .await
            .unwrap()
            .is_none());
        assert!(
            invoker
                .invoke(&api, &runtime, &payload, Some(&successor))
                .await
                .is_err(),
            "another operation cannot borrow previously materialized bytes as its own completion"
        );
        std::fs::write(target.join("Cargo.toml"), "later edit").unwrap();
        assert_eq!(
            invoker
                .invoke(&api, &runtime, &payload, Some(&context))
                .await
                .unwrap()
                .emitted_artifacts,
            result.emitted_artifacts
        );
        assert_eq!(
            std::fs::read_to_string(target.join("Cargo.toml")).unwrap(),
            "later edit"
        );
        let mut attributed = context.clone();
        attributed.effect_authority.as_mut().unwrap().request_ref = Some("new-request".into());
        let count = events.watermark_capability().snapshot().unwrap().tip_seq;
        assert_eq!(
            invoker
                .invoke(&api, &runtime, &payload, Some(&attributed))
                .await
                .unwrap()
                .emitted_artifacts,
            result.emitted_artifacts
        );
        assert_eq!(
            events.watermark_capability().snapshot().unwrap().tip_seq,
            count,
            "legacy replay cannot create a newly attributed account"
        );
        let mut next_payload = payload.clone();
        next_payload.invocation_id = "second-materialization".into();
        let next_change = CodeChangeSet::new(
            context.effect_authority.as_ref().unwrap().subject.clone(),
            vec![],
            vec![FileReplacement {
                relative_path: "Cargo.toml".into(),
                expected_content_hash: blake3::hash(b"later edit").to_hex().to_string(),
                replacement: "second change".into(),
            }],
        )
        .unwrap();
        next_payload.supplied_inputs[0].value =
            SuppliedValueRef::StructuredValue(serde_json::to_value(next_change).unwrap());
        let next_result = invoker
            .invoke(&api, &runtime, &next_payload, Some(&attributed))
            .await
            .unwrap();
        let evidence = super::super::publication::envelope(
            &events.replay_capability(),
            attributed.effect_authority.as_ref().unwrap(),
            &next_result.emitted_artifacts[0],
        )
        .unwrap();
        let expected = super::super::publication::request_observation_source(
            "agent",
            &context.effect_authority.as_ref().unwrap().subject,
            "new-request",
        )
        .unwrap()
        .0;
        assert_eq!(
            evidence.data["batch"]["scope"],
            serde_json::to_value(expected).unwrap()
        );
        attributed.effect_authority.as_mut().unwrap().request_ref = Some("foreign-request".into());
        assert!(super::super::publication::envelope(
            &events.replay_capability(),
            attributed.effect_authority.as_ref().unwrap(),
            &next_result.emitted_artifacts[0]
        )
        .is_err());
    }
}
