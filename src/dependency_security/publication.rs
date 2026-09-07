//! Security-owned durable products and their structural Graph publications.

use std::collections::BTreeMap;

use meld_events::{
    AppendMode, DomainObjectRef, EventAppendCapability, EventEnvelope, EventReplayCapability,
};
use meld_world_model::world_state::graph::{
    admission::GraphOwnerEventRoute, contracts::*, events::owner_publication_envelope,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{capability::*, contracts::*};
use crate::{
    capability::{CapabilityInvocationPayload, CapabilityInvocationResult, CapabilityRuntimeInit},
    error::ApiError,
    execution::ExecutionEventContext,
};

pub const OWNER: &str = "dependency-security";
pub const EVENT: &str = "dependency_security.product_published.v1";
pub const SCHEMA: &str = "dependency_security.product_publication.v1";
pub(super) const RECEIPT_EVENT: &str = "dependency_security.invocation_return.v1";

pub fn graph_route() -> GraphOwnerEventRoute {
    GraphOwnerEventRoute {
        complete_event_source: true,
        route_id: "dependency-security-products".into(),
        owner_id: OWNER.into(),
        event_type: EVENT.into(),
        enumeration_rule_revision: SCHEMA.into(),
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Receipt {
    pub(super) binding: Value,
    pub(super) artifacts: Vec<crate::task::ArtifactRecord>,
}

impl Receipt {
    pub(super) fn result(&self) -> CapabilityInvocationResult {
        CapabilityInvocationResult {
            emitted_artifacts: self.artifacts.clone(),
        }
    }
}

pub(crate) struct Publication<'a> {
    capability: &'a SecurityCapability,
    runtime: &'a CapabilityRuntimeInit,
    payload: &'a CapabilityInvocationPayload,
    context: &'a ExecutionEventContext,
    binding: Value,
}

impl<'a> Publication<'a> {
    pub fn new(
        capability: &'a SecurityCapability,
        runtime: &'a CapabilityRuntimeInit,
        payload: &'a CapabilityInvocationPayload,
        context: Option<&'a ExecutionEventContext>,
    ) -> Result<Self, ApiError> {
        capability.validate_invocation(runtime, payload)?;
        let context = context
            .ok_or_else(|| invalid("Security publication requires admitted effect context"))?;
        let authority = context
            .effect_authority
            .as_ref()
            .ok_or_else(|| invalid("Security publication requires admitted effect authority"))?;
        if authority.subject != capability.subject.subject
            || authority.issuer_ref.trim().is_empty()
            || authority.principal_id.trim().is_empty()
            || authority.fence_ref.trim().is_empty()
            || context.session_id.trim().is_empty()
        {
            return Err(invalid("Security publication differs from its assigned subject or has incomplete authority"));
        }
        let binding = serde_json::json!({
            "runtime": runtime, "payload": payload, "subject": capability.subject,
            "policy": capability.policy, "workspace": capability.workspace,
            "cargo": capability.cargo, "advisories": capability.advisories, "limits": capability.limits,
            "issuer": authority.issuer_ref, "principal": authority.principal_id,
            "fence": authority.fence_ref, "session": context.session_id,
        });
        Ok(Self {
            capability,
            runtime,
            payload,
            context,
            binding,
        })
    }

    fn record_id(&self, events: &EventReplayCapability) -> Result<String, ApiError> {
        Ok(format!(
            "security-return::{}",
            content_hash(&(events.ledger_identity(), &self.binding)).map_err(invalid)?
        ))
    }

    fn receipt_envelope(
        &self,
        events: &EventReplayCapability,
        receipt: &Receipt,
    ) -> Result<EventEnvelope, ApiError> {
        Ok(EventEnvelope::with_now_domain(
            &self.context.session_id,
            OWNER,
            &self.capability.subject.subject.object_id,
            RECEIPT_EVENT,
            None,
            serde_json::to_value(receipt).map_err(invalid)?,
        )
        .with_record_id(self.record_id(events)?))
    }

    fn retained(&self, events: &EventReplayCapability) -> Result<Option<Receipt>, ApiError> {
        let Some(record) = events
            .committed_record(&self.record_id(events)?)
            .map_err(storage)?
        else {
            return Ok(None);
        };
        let receipt: Receipt = serde_json::from_value(record.data.clone()).map_err(invalid)?;
        if receipt.binding != self.binding {
            return Err(invalid(
                "Security receipt names different invocation inputs",
            ));
        }
        self.capability
            .validate_result(self.runtime, self.payload, &receipt.result())?;
        events
            .prove_existing(&self.receipt_envelope(events, &receipt)?)
            .map_err(storage)?
            .ok_or_else(|| invalid("Security receipt is not durably proven"))?;
        Ok(Some(receipt))
    }

    fn publication_envelope(
        &self,
        events: &EventReplayCapability,
        receipt: &Receipt,
    ) -> Result<EventEnvelope, ApiError> {
        let operation = product_publication(
            &self.capability.subject,
            &self.record_id(events)?,
            &receipt.artifacts,
        )?;
        let mut envelope =
            owner_publication_envelope(&self.context.session_id, &operation).map_err(invalid)?;
        envelope.event_type = EVENT.into();
        Ok(envelope)
    }

    /// Recovery reports completion only after both the product and Graph source are durable.
    pub fn recover(
        &self,
        events: &EventReplayCapability,
    ) -> Result<Option<CapabilityInvocationResult>, ApiError> {
        let Some(receipt) = self.retained(events)? else {
            return Ok(None);
        };
        if events
            .prove_existing(&self.publication_envelope(events, &receipt)?)
            .map_err(storage)?
            .is_none()
        {
            return Ok(None);
        }
        if !super::condition::is_published(self.capability, events)? {
            return Ok(None);
        }
        Ok(Some(receipt.result()))
    }

    /// A retained owner product is reused even when its Graph publication was interrupted.
    pub fn resume(
        &self,
        events: &EventAppendCapability,
    ) -> Result<Option<CapabilityInvocationResult>, ApiError> {
        let replay = events.replay_capability();
        let Some(receipt) = self.retained(&replay)? else {
            return Ok(None);
        };
        let source = self.publication_envelope(&replay, &receipt)?;
        let condition = super::condition::publication(self.capability, &replay)?;
        events
            .append_durable_batch(
                vec![source.clone(), condition.clone()],
                AppendMode::Idempotent,
            )
            .map_err(storage)?;
        for envelope in [source, condition] {
            replay
                .prove_existing(&envelope)
                .map_err(storage)?
                .ok_or_else(|| invalid("Security publication batch is not durably proven"))?;
        }
        Ok(Some(receipt.result()))
    }

    pub fn publish(
        &self,
        events: &EventAppendCapability,
        result: CapabilityInvocationResult,
    ) -> Result<CapabilityInvocationResult, ApiError> {
        self.capability
            .validate_result(self.runtime, self.payload, &result)?;
        let replay = events.replay_capability();
        let receipt = Receipt {
            binding: self.binding.clone(),
            artifacts: result.emitted_artifacts,
        };
        events
            .append_durable_proven(
                self.receipt_envelope(&replay, &receipt)?,
                AppendMode::Idempotent,
            )
            .map_err(storage)?;
        self.resume(events)?
            .ok_or_else(|| invalid("Security receipt disappeared after durable append"))
    }
}

pub(super) fn product_publication(
    subject: &DependencySecuritySubjectV1,
    receipt_id: &str,
    artifacts: &[crate::task::ArtifactRecord],
) -> Result<OwnerPublicationOperation, ApiError> {
    let [artifact] = artifacts else {
        return Err(invalid(
            "Security publication requires one complete owner product",
        ));
    };
    publish_product(
        subject,
        receipt_id,
        &artifact.artifact_type_id,
        &artifact.content,
        content_hash(&(receipt_id, artifacts)).map_err(invalid)?,
    )
}

pub(super) fn observed_source_publication(
    subject: &DependencySecuritySubjectV1,
    receipt_id: &str,
    kind: &str,
    product: Value,
) -> Result<OwnerPublicationOperation, ApiError> {
    // Retained advisory publications keep their original typed serialization identity.
    let revision = match kind {
        ADVISORIES => content_hash(&(
            receipt_id,
            serde_json::from_value::<AdvisoryKnowledgeSnapshotV1>(product.clone())
                .map_err(invalid)?,
        )),
        INVENTORY => content_hash(&(
            receipt_id,
            serde_json::from_value::<DependencyInventorySnapshotV1>(product.clone())
                .map_err(invalid)?,
        )),
        _ => return Err(invalid("observation cannot publish a non-source product")),
    }
    .map_err(invalid)?;
    publish_product(subject, receipt_id, kind, &product, revision)
}

fn publish_product(
    subject: &DependencySecuritySubjectV1,
    receipt_id: &str,
    kind: &str,
    product: &Value,
    revision_id: String,
) -> Result<OwnerPublicationOperation, ApiError> {
    let product_id = match kind {
        INVENTORY | ADVISORIES => product["snapshot_id"].as_str(),
        ASSESSMENT => product["assessment_id"].as_str(),
        VERIFICATION => product["verification_id"].as_str(),
        _ => None,
    }
    .ok_or_else(|| invalid("Security product identity is absent"))?;
    let scope = OwnerPublicationScope {
        scope_id: format!(
            "security-scope::{}",
            content_hash(&(&subject.subject, kind)).map_err(invalid)?
        ),
        branch_id: None,
        perspective_id: None,
        valid_at: None,
    };
    let hydration = HydrationReference {
        owner_id: OWNER.into(),
        product_kind: kind.into(),
        product_id: product_id.into(),
        revision_id: revision_id.clone(),
        role: "security_owner_product".into(),
    };
    let mut objects = Vec::new();
    let mut relations = Vec::new();
    let mut object = |kind: &str, id: &str, body: Value| -> Result<DomainObjectRef, ApiError> {
        let reference = DomainObjectRef::new(OWNER, kind, id).map_err(invalid)?;
        objects.push(OwnerObjectPublication {
            publication_id: format!("{revision_id}::{}", reference.index_key()),
            object_ref: reference.clone(),
            state: OwnerPublicationState::Observed,
            source_product_ref: receipt_id.into(),
            hydration: hydration.clone(),
            provenance_refs: vec![receipt_id.into()],
            qualifications: BTreeMap::from([("product".into(), body.to_string())]),
        });
        Ok(reference)
    };
    let root = object(kind, product_id, product.clone())?;
    let mut relate = |relation: &str, src: DomainObjectRef, dst: DomainObjectRef| {
        relations.push(OwnerRelationOccurrence {
            occurrence_id: format!(
                "{revision_id}::{relation}::{}::{}",
                src.index_key(),
                dst.index_key()
            ),
            relation_type: relation.into(),
            src,
            dst,
            source_product_ref: receipt_id.into(),
            hydration: hydration.clone(),
            qualifications: BTreeMap::new(),
            provenance_refs: vec![receipt_id.into()],
        });
    };
    relate(
        "security_product_subject",
        root.clone(),
        subject.subject.clone(),
    );
    match kind {
        INVENTORY => {
            let inventory: DependencyInventorySnapshotV1 =
                serde_json::from_value(product.clone()).map_err(invalid)?;
            for component in inventory.components {
                let reference = object(
                    "dependency_component",
                    &component.component_id,
                    serde_json::to_value(&component).map_err(invalid)?,
                )?;
                relate("security_inventory_component", root.clone(), reference);
            }
        }
        ASSESSMENT => {
            let assessment: DependencySecurityAssessmentV1 =
                serde_json::from_value(product.clone()).map_err(invalid)?;
            for finding in assessment.findings {
                let reference = object(
                    "security_finding",
                    &finding.finding_id,
                    serde_json::to_value(&finding).map_err(invalid)?,
                )?;
                relate(
                    "security_assessment_finding",
                    root.clone(),
                    reference.clone(),
                );
                relate(
                    "security_finding_component",
                    reference,
                    DomainObjectRef::new(OWNER, "dependency_component", &finding.component_id)
                        .map_err(invalid)?,
                );
            }
        }
        _ => {}
    }
    for (field, target_kind, relation) in [
        (
            "inventory_snapshot_ref",
            INVENTORY,
            "security_uses_inventory",
        ),
        (
            "advisory_snapshot_ref",
            ADVISORIES,
            "security_uses_advisories",
        ),
        ("assessment_ref", ASSESSMENT, "security_verifies_assessment"),
    ] {
        if let Some(id) = product[field].as_str().filter(|id| !id.is_empty()) {
            relate(
                relation,
                root.clone(),
                DomainObjectRef::new(OWNER, target_kind, id).map_err(invalid)?,
            );
        }
    }
    let mut included_ids: Vec<_> = objects
        .iter()
        .map(|object| object.publication_id.clone())
        .collect();
    included_ids.extend(
        relations
            .iter()
            .map(|relation| relation.occurrence_id.clone()),
    );
    OwnerPublicationOperation::reconstruct(
        SCHEMA,
        OwnerPublicationBatch {
            owner_id: OWNER.into(),
            revision_id,
            scope: scope.clone(),
            objects,
            relations,
            // Completeness enumerates this product, not the advisory coverage it reports.
            completeness: OwnerCompletenessReceipt {
                receipt_id: format!("{receipt_id}::enumeration"),
                scope,
                included_ids,
                exclusions: vec![],
                failures: vec![],
                status: OwnerCompletenessStatus::Complete,
            },
        },
    )
    .map_err(invalid)
}

fn invalid(error: impl ToString) -> ApiError {
    ApiError::ConfigError(error.to_string())
}
fn storage(error: impl ToString) -> ApiError {
    ApiError::StorageError(crate::error::StorageError::EventAuthorityUnavailable(
        error.to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use meld_events::{EventAuthority, EventAuthorityOpenOptions};

    #[test]
    fn interrupted_publication_reopens_exact_receipt_without_source_reacquisition() {
        let root = tempfile::tempdir().unwrap();
        let subject = DependencySecuritySubjectV1 {
            subject: DomainObjectRef::new("workspace_fs", "node", "repo").unwrap(),
            ecosystem: PackageEcosystem::Cargo,
            inventory_scope: InventoryScopeV1 {
                manifest_ref: DomainObjectRef::new(OWNER, "manifest_source", "repo-manifest")
                    .unwrap(),
                lockfile_ref: DomainObjectRef::new(OWNER, "lockfile_source", "repo-lockfile")
                    .unwrap(),
                include_transitive: true,
            },
        };
        let source_path = root.path().join("advisories.json");
        let capability = SecurityCapability {
            publication_gate: Default::default(),
            id: ACQUIRE_ADVISORIES.into(),
            subject: subject.clone(),
            policy: serde_json::from_str(include_str!(
                "../../theory/dependency_security/policy.cargo_fixture.json"
            ))
            .unwrap(),
            workspace: None,
            cargo: None,
            advisories: Some(source_path.clone()),
            limits: Default::default(),
        };
        let contract = contract(ACQUIRE_ADVISORIES);
        let runtime = CapabilityRuntimeInit {
            capability_instance_id: "acquire".into(),
            capability_type_id: ACQUIRE_ADVISORIES.into(),
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
            invocation_id: "acquisition".into(),
            capability_instance_id: "acquire".into(),
            supplied_inputs: vec![],
            upstream_lineage: None,
            execution_context: Default::default(),
        };
        let context = ExecutionEventContext {
            session_id: "security-recovery".into(),
            effect_authority: Some(meld_execution::ExecutionEffectAuthority {
                request_ref: None,
                issuer_ref: "agent".into(),
                principal_id: "owner".into(),
                subject: subject.subject,
                fence_ref: "fence".into(),
            }),
        };
        let product = AdvisoryKnowledgeSnapshotV1::canonical(
            "fixture".into(),
            "captured-revision".into(),
            PackageEcosystem::Cargo,
            vec![],
            vec![],
            vec![],
            AdvisoryCompleteness::CompleteForDeclaredCoverage,
            10,
        )
        .unwrap();
        std::fs::write(
            &source_path,
            serde_json::to_vec(&super::super::advisory::AdvisorySourceDocumentV1::from(
                product.clone(),
            ))
            .unwrap(),
        )
        .unwrap();
        let offered = result(
            &runtime,
            &payload,
            ADVISORIES,
            serde_json::to_value(product).unwrap(),
        );
        let publication =
            Publication::new(&capability, &runtime, &payload, Some(&context)).unwrap();
        let receipt = Receipt {
            binding: publication.binding.clone(),
            artifacts: offered.emitted_artifacts.clone(),
        };
        {
            let events = EventAuthority::open(
                sled::open(root.path().join("events")).unwrap(),
                EventAuthorityOpenOptions::default(),
            )
            .unwrap();
            let replay = events.replay_capability();
            assert!(publication.recover(&replay).unwrap().is_none());
            events
                .append_capability()
                .append_durable_proven(
                    publication.receipt_envelope(&replay, &receipt).unwrap(),
                    AppendMode::Idempotent,
                )
                .unwrap();
            assert!(
                publication.recover(&replay).unwrap().is_none(),
                "a receipt alone does not prove source publication"
            );
        }
        std::fs::write(&source_path, "changed and no longer valid JSON").unwrap();
        let events = EventAuthority::open(
            sled::open(root.path().join("events")).unwrap(),
            EventAuthorityOpenOptions::default(),
        )
        .unwrap();
        let resumed = publication
            .resume(&events.append_capability())
            .unwrap()
            .unwrap();
        assert_eq!(resumed.emitted_artifacts, offered.emitted_artifacts);
        let position = events.watermark_capability().snapshot().unwrap();
        assert_eq!(
            publication
                .recover(&events.replay_capability())
                .unwrap()
                .unwrap()
                .emitted_artifacts,
            offered.emitted_artifacts
        );
        assert_eq!(
            publication
                .resume(&events.append_capability())
                .unwrap()
                .unwrap()
                .emitted_artifacts,
            offered.emitted_artifacts
        );
        assert_eq!(events.watermark_capability().snapshot().unwrap(), position);
        let foreign = EventAuthority::open(
            sled::Config::new().temporary(true).open().unwrap(),
            EventAuthorityOpenOptions::default(),
        )
        .unwrap();
        assert!(publication
            .recover(&foreign.replay_capability())
            .unwrap()
            .is_none());
        let mut changed = payload.clone();
        changed.invocation_id = "another-acquisition".into();
        assert!(
            Publication::new(&capability, &runtime, &changed, Some(&context))
                .unwrap()
                .recover(&events.replay_capability())
                .unwrap()
                .is_none()
        );
        let mut foreign_context = context.clone();
        foreign_context
            .effect_authority
            .as_mut()
            .unwrap()
            .subject
            .object_id = "another-repo".into();
        assert!(Publication::new(&capability, &runtime, &payload, Some(&foreign_context)).is_err());
        let invalid = Receipt {
            binding: receipt.binding,
            artifacts: {
                let mut artifacts = receipt.artifacts;
                artifacts[0].content["source_revision"] = "tampered".into();
                artifacts
            },
        };
        foreign
            .append_capability()
            .append_durable_proven(
                publication
                    .receipt_envelope(&foreign.replay_capability(), &invalid)
                    .unwrap(),
                AppendMode::Idempotent,
            )
            .unwrap();
        assert!(publication.recover(&foreign.replay_capability()).is_err());
    }
}
