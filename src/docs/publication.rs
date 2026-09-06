//! Docs-owned publication of captured observations and addressable README assertions.

use super::capability::DocsEvidenceBundle;
use super::observation::ObservedReadmeState;
use meld_events::DomainObjectRef;
use meld_world_model::world_state::graph::admission::GraphOwnerEventRoute;
use meld_world_model::world_state::graph::contracts::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const OWNER_ID: &str = "docs";
pub const OBSERVATION_SCHEMA: &str = "docs.observation.v1";
pub const OBSERVATION_EVENT: &str = "docs.observation";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocsObservationRevision {
    pub revision_id: String,
    pub predecessor: Option<String>,
    pub sequence: u64,
    pub subject: DomainObjectRef,
    pub scope: OwnerPublicationScope,
    pub evidence: DocsEvidenceBundle,
}

impl DocsObservationRevision {
    pub(crate) fn new(
        predecessor: Option<String>,
        sequence: u64,
        subject: DomainObjectRef,
        scope: OwnerPublicationScope,
        evidence: DocsEvidenceBundle,
    ) -> Result<Self, String> {
        subject.validate().map_err(|e| e.to_string())?;
        scope.validate().map_err(|e| e.to_string())?;
        if sequence == 0 || evidence.observation.is_none() {
            return Err("Docs publication requires an observed source and nonzero sequence".into());
        }
        let bytes = serde_json::to_vec(&(
            OBSERVATION_SCHEMA,
            &predecessor,
            sequence,
            &subject,
            &scope,
            &evidence,
        ))
        .map_err(|e| e.to_string())?;
        Ok(Self {
            revision_id: format!("docs-revision::{}", blake3::hash(&bytes).to_hex()),
            predecessor,
            sequence,
            subject,
            scope,
            evidence,
        })
    }

    pub fn publication(&self) -> Result<OwnerPublicationOperation, String> {
        let expected = Self::new(
            self.predecessor.clone(),
            self.sequence,
            self.subject.clone(),
            self.scope.clone(),
            self.evidence.clone(),
        )?;
        if &expected != self {
            return Err("Docs observation revision identity is invalid".into());
        }
        let observed = self
            .evidence
            .observation
            .as_ref()
            .ok_or("Docs observation is absent")?;
        let hydration = HydrationReference {
            owner_id: OWNER_ID.into(),
            product_kind: "scope_observation".into(),
            product_id: self.scope.scope_id.clone(),
            revision_id: self.revision_id.clone(),
            role: "observed_docs_material".into(),
        };
        let mut objects = Vec::new();
        let mut relations = Vec::new();
        let mut add =
            |kind: &str, key: &str, data: serde_json::Value| -> Result<DomainObjectRef, String> {
                let object =
                    DomainObjectRef::new(OWNER_ID, kind, key).map_err(|e| e.to_string())?;
                objects.push(OwnerObjectPublication {
                    publication_id: format!("{}::{}", self.revision_id, object.index_key()),
                    object_ref: object.clone(),
                    state: OwnerPublicationState::Observed,
                    source_product_ref: self.revision_id.clone(),
                    hydration: hydration.clone(),
                    provenance_refs: vec![observed.revision_id.clone()],
                    qualifications: BTreeMap::from([("observation".into(), data.to_string())]),
                });
                Ok(object)
            };
        let root = add(
            "scope_observation",
            &self.scope.scope_id,
            serde_json::to_value(self).map_err(|e| e.to_string())?,
        )?;
        let mut relate = |relation: &str, src: DomainObjectRef, dst: DomainObjectRef| {
            relations.push(OwnerRelationOccurrence {
                occurrence_id: format!(
                    "{}::{relation}::{}::{}",
                    self.revision_id,
                    src.index_key(),
                    dst.index_key()
                ),
                relation_type: relation.into(),
                src,
                dst,
                source_product_ref: self.revision_id.clone(),
                hydration: hydration.clone(),
                qualifications: BTreeMap::new(),
                provenance_refs: vec![observed.revision_id.clone()],
            });
        };
        relate("docs_observes", root.clone(), self.subject.clone());
        for source in &observed.sources {
            let key = format!("{}::{}", self.scope.scope_id, source.path);
            let object = add(
                "source_observation",
                &key,
                serde_json::to_value(source).map_err(|e| e.to_string())?,
            )?;
            relate("docs_source_in_scope", object, root.clone());
        }
        let mut failures = observed
            .coverage_gaps
            .iter()
            .map(|gap| OwnerPublicationFailure {
                source_ref: gap.path.clone(),
                reason: gap.reason.clone(),
            })
            .collect::<Vec<_>>();
        for readme in &observed.readmes {
            let key = format!("{}::{}", self.scope.scope_id, readme.path);
            let object = add(
                "readme_observation",
                &key,
                serde_json::to_value(readme).map_err(|e| e.to_string())?,
            )?;
            relate("docs_readme_in_scope", object.clone(), root.clone());
            match &readme.state {
                ObservedReadmeState::Present {
                    claims,
                    content_hash,
                    ..
                } => {
                    for claim in claims {
                        let claim_key = format!("{key}::{content_hash}::{}", claim.claim_id);
                        let statement = add(
                            "observed_claim",
                            &claim_key,
                            serde_json::to_value(claim).map_err(|e| e.to_string())?,
                        )?;
                        relate("docs_claim_in_readme", statement, object.clone());
                    }
                }
                ObservedReadmeState::Unavailable { reason } => {
                    failures.push(OwnerPublicationFailure {
                        source_ref: readme.path.clone(),
                        reason: reason.clone(),
                    })
                }
                ObservedReadmeState::Missing => {}
            }
        }
        let included_ids = objects
            .iter()
            .map(|o| o.publication_id.clone())
            .chain(relations.iter().map(|r| r.occurrence_id.clone()))
            .collect();
        let complete = failures.is_empty();
        OwnerPublicationOperation::reconstruct(
            OBSERVATION_SCHEMA,
            OwnerPublicationBatch {
                owner_id: OWNER_ID.into(),
                revision_id: self.revision_id.clone(),
                scope: self.scope.clone(),
                objects,
                relations,
                completeness: OwnerCompletenessReceipt {
                    receipt_id: format!("{}::coverage", self.revision_id),
                    scope: self.scope.clone(),
                    included_ids,
                    exclusions: observed
                        .exclusions
                        .iter()
                        .map(|entry| OwnerPublicationExclusion {
                            source_ref: entry.path.clone(),
                            reason: entry.reason.clone(),
                        })
                        .collect(),
                    failures,
                    status: if complete {
                        OwnerCompletenessStatus::Complete
                    } else {
                        OwnerCompletenessStatus::Incomplete
                    },
                },
            },
        )
        .map_err(|e| e.to_string())
    }

    pub fn envelope(&self, session_id: &str) -> Result<meld_events::EventEnvelope, String> {
        let mut envelope =
            meld_world_model::world_state::graph::events::owner_publication_envelope(
                session_id,
                &self.publication()?,
            )
            .map_err(|e| e.to_string())?;
        envelope.event_type = OBSERVATION_EVENT.into();
        Ok(envelope)
    }
}

pub fn graph_route() -> GraphOwnerEventRoute {
    GraphOwnerEventRoute {
        route_id: "docs-owner-observation".into(),
        owner_id: OWNER_ID.into(),
        event_type: OBSERVATION_EVENT.into(),
        enumeration_rule_revision: OBSERVATION_SCHEMA.into(),
        complete_event_source: false,
    }
}
