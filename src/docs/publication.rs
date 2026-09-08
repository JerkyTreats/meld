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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocsObservationRevision {
    pub revision_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub work_input_basis: Option<super::input_basis::DocsWorkInputBasis>,
    pub predecessor: Option<String>,
    pub sequence: u64,
    pub subject: DomainObjectRef,
    pub scope: OwnerPublicationScope,
    pub evidence: DocsEvidenceBundle,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claim_report: Option<super::claim_observation::ObservedDocsClaimReport>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_claims: Option<super::source_claims::DocsSourceClaimReport>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correspondence: Option<super::correspondence::DocsCorrespondenceReport>,
}

impl DocsObservationRevision {
    pub(crate) fn new(
        predecessor: Option<String>,
        sequence: u64,
        subject: DomainObjectRef,
        scope: OwnerPublicationScope,
        evidence: DocsEvidenceBundle,
        work_input_basis: Option<super::input_basis::DocsWorkInputBasis>,
    ) -> Result<Self, String> {
        subject.validate().map_err(|e| e.to_string())?;
        scope.validate().map_err(|e| e.to_string())?;
        if sequence == 0 || evidence.observation.is_none() {
            return Err("Docs publication requires an observed source and nonzero sequence".into());
        }
        let mut bytes = serde_json::to_vec(&(
            OBSERVATION_SCHEMA,
            &predecessor,
            sequence,
            &subject,
            &scope,
            &evidence,
        ))
        .map_err(|e| e.to_string())?;
        if let Some(basis) = &work_input_basis {
            if basis.basis_id.trim().is_empty() || basis.policy_identity.trim().is_empty() {
                return Err("Docs work-input basis is incomplete".into());
            }
            bytes = serde_json::to_vec(&("docs-work-input-v1", bytes, basis))
                .map_err(|error| error.to_string())?;
        }
        Ok(Self {
            revision_id: format!("docs-revision::{}", blake3::hash(&bytes).to_hex()),
            work_input_basis,
            predecessor,
            sequence,
            subject,
            scope,
            evidence,
            claim_report: None,
            source_claims: None,
            correspondence: None,
        })
    }

    pub(crate) fn with_source_claims(
        mut self,
        report: super::source_claims::DocsSourceClaimReport,
    ) -> Result<Self, String> {
        if !report.complete || self.claim_report.is_some() {
            return Err(
                "Docs source claims require complete extraction before README judgment attachment"
                    .into(),
            );
        }
        report
            .validate_capture(&self.evidence)
            .map_err(|error| error.to_string())?;
        let seed = serde_json::to_vec(&(
            &self.revision_id,
            super::source_claims::SOURCE_CLAIM_CONTRACT,
            &report,
        ))
        .map_err(|error| error.to_string())?;
        self.revision_id = format!("docs-revision::{}", blake3::hash(&seed).to_hex());
        self.source_claims = Some(report);
        Ok(self)
    }

    pub(crate) fn with_claim_report(
        mut self,
        report: super::claim_observation::ObservedDocsClaimReport,
    ) -> Result<Self, String> {
        if !report.complete
            || !report
                .matches_capture(&self.evidence)
                .map_err(|error| error.to_string())?
        {
            return Err("Docs claim report names another captured observation".into());
        }
        let seed =
            serde_json::to_vec(&(&self.revision_id, &report)).map_err(|error| error.to_string())?;
        self.revision_id = format!("docs-revision::{}", blake3::hash(&seed).to_hex());
        self.claim_report = Some(report);
        Ok(self)
    }

    pub(crate) fn with_correspondence(
        mut self,
        report: super::correspondence::DocsCorrespondenceReport,
    ) -> Result<Self, String> {
        if !report.complete
            || self
                .claim_report
                .as_ref()
                .is_none_or(|claims| claims.policy_identity != report.policy_identity)
        {
            return Err(
                "Docs correspondence requires completed source and README judgments".into(),
            );
        }
        report
            .validate_capture(
                &self.evidence,
                self.source_claims
                    .as_ref()
                    .ok_or("Docs source claims absent")?,
            )
            .map_err(|error| error.to_string())?;
        let seed =
            serde_json::to_vec(&(&self.revision_id, &report)).map_err(|error| error.to_string())?;
        self.revision_id = format!("docs-revision::{}", blake3::hash(&seed).to_hex());
        self.correspondence = Some(report);
        Ok(self)
    }

    pub fn publication(&self) -> Result<OwnerPublicationOperation, String> {
        let mut expected = Self::new(
            self.predecessor.clone(),
            self.sequence,
            self.subject.clone(),
            self.scope.clone(),
            self.evidence.clone(),
            self.work_input_basis.clone(),
        )?;
        if let Some(report) = &self.source_claims {
            expected = expected.with_source_claims(report.clone())?;
        }
        if let Some(report) = &self.claim_report {
            expected = expected.with_claim_report(report.clone())?;
        }
        if let Some(report) = &self.correspondence {
            expected = expected.with_correspondence(report.clone())?;
        }
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
        let mut semantic_qualifications = BTreeMap::new();
        if let Some(report) = &self.source_claims {
            let set = add(
                "source_claim_set",
                &report.report_id,
                serde_json::to_value(report).map_err(|error| error.to_string())?,
            )?;
            relate("docs_claims_source_scope", set.clone(), root.clone());
            for file in &report.files {
                let file_key = format!("{}::{}", report.report_id, file.path);
                let file_claims = add(
                    "source_file_claims",
                    &file_key,
                    serde_json::to_value(file).map_err(|error| error.to_string())?,
                )?;
                relate(
                    "docs_source_claim_file_in_set",
                    file_claims.clone(),
                    set.clone(),
                );
                let source_key = format!("{}::{}", self.scope.scope_id, file.path);
                let source = DomainObjectRef::new(OWNER_ID, "source_observation", &source_key)
                    .map_err(|error| error.to_string())?;
                for claim in &file.claims {
                    let object = add(
                        "source_claim",
                        &claim.claim_id,
                        serde_json::to_value(claim).map_err(|error| error.to_string())?,
                    )?;
                    relate("docs_claim_from_source", object.clone(), source.clone());
                    relate(
                        "docs_source_claim_in_file",
                        object.clone(),
                        file_claims.clone(),
                    );
                    semantic_qualifications.insert(
                        object,
                        BTreeMap::from([
                            ("source_path".to_string(), file.path.clone()),
                            (
                                "source_directory".to_string(),
                                file.path
                                    .rsplit_once('/')
                                    .map_or(".", |(parent, _)| parent)
                                    .to_string(),
                            ),
                            ("source_hash".to_string(), file.content_hash.clone()),
                            ("claim_policy".to_string(), report.policy_identity.clone()),
                            (
                                "extraction_contract".to_string(),
                                report.contract_revision.clone(),
                            ),
                        ]),
                    );
                }
            }
        }
        if let Some(report) = &self.claim_report {
            let scope_judgment = add(
                "observed_claim_assessment",
                &report.report_id,
                serde_json::to_value(report).map_err(|error| error.to_string())?,
            )?;
            relate(
                "docs_judges_observed_claims",
                scope_judgment.clone(),
                root.clone(),
            );
            for judgment in &report.readmes {
                let readme_key = format!("{}::{}", self.scope.scope_id, judgment.path);
                let key = format!("{}::{}", report.report_id, judgment.path);
                let readme_judgment = add(
                    "readme_claim_assessment",
                    &key,
                    serde_json::to_value(judgment).map_err(|error| error.to_string())?,
                )?;
                relate(
                    "docs_readme_judgment_in_scope",
                    readme_judgment.clone(),
                    scope_judgment.clone(),
                );
                relate(
                    "docs_judges_readme",
                    readme_judgment.clone(),
                    DomainObjectRef::new(OWNER_ID, "readme_observation", &readme_key)
                        .map_err(|error| error.to_string())?,
                );
                if let super::claim_observation::ObservedClaimDisposition::Assessed {
                    report: readme,
                } = &judgment.disposition
                {
                    semantic_qualifications.insert(
                        readme_judgment.clone(),
                        BTreeMap::from([
                            (
                                "assertions_supported".to_string(),
                                readme.accepted.to_string(),
                            ),
                            ("claim_policy".to_string(), report.policy_identity.clone()),
                        ]),
                    );
                    if report.acceptance_evaluator.is_some() {
                        semantic_qualifications
                            .entry(readme_judgment.clone())
                            .or_default()
                            .extend(BTreeMap::from([
                                ("accepted_by_policy".into(), readme.accepted.to_string()),
                                (
                                    "assertions_supported".into(),
                                    readme
                                        .assessments
                                        .iter()
                                        .all(|assessment| {
                                            assessment.verdict
                                                == super::claim_validation::ClaimVerdict::Supported
                                        })
                                        .to_string(),
                                ),
                            ]));
                    }
                    for assessment in &readme.assessments {
                        let key = format!("{key}::{}", assessment.claim.claim_id);
                        let claim_judgment = add(
                            "claim_assessment",
                            &key,
                            serde_json::to_value(assessment).map_err(|error| error.to_string())?,
                        )?;
                        let claim_key = format!(
                            "{readme_key}::{}::{}",
                            readme.content_hash, assessment.claim.claim_id
                        );
                        relate(
                            "docs_judges_claim",
                            claim_judgment.clone(),
                            DomainObjectRef::new(OWNER_ID, "observed_claim", &claim_key)
                                .map_err(|error| error.to_string())?,
                        );
                        relate(
                            "docs_claim_judgment_in_readme",
                            claim_judgment.clone(),
                            readme_judgment.clone(),
                        );
                        let verdict = match assessment.verdict {
                            super::claim_validation::ClaimVerdict::Supported => "supported",
                            super::claim_validation::ClaimVerdict::Unsupported => "unsupported",
                            super::claim_validation::ClaimVerdict::Contradicted => "contradicted",
                        };
                        semantic_qualifications.insert(
                            claim_judgment,
                            BTreeMap::from([
                                ("verdict".to_string(), verdict.to_string()),
                                ("claim_policy".to_string(), report.policy_identity.clone()),
                            ]),
                        );
                    }
                }
            }
        }
        if let Some(report) = &self.correspondence {
            let set = add(
                "claim_correspondence_set",
                &report.report_id,
                serde_json::to_value(report).map_err(|error| error.to_string())?,
            )?;
            relate("docs_compares_source_to_readme", set.clone(), root.clone());
            for readme in &report.readmes {
                let readme_key = format!("{}::{}", self.scope.scope_id, readme.path);
                for claim in &readme.claims {
                    let key = format!(
                        "{}::{}::{}",
                        report.report_id, readme.path, claim.source_claim_id
                    );
                    let object = add(
                        "source_readme_correspondence",
                        &key,
                        serde_json::to_value(claim).map_err(|error| error.to_string())?,
                    )?;
                    relate("docs_correspondence_in_set", object.clone(), set.clone());
                    relate(
                        "docs_correspondence_source",
                        object.clone(),
                        DomainObjectRef::new(OWNER_ID, "source_claim", &claim.source_claim_id)
                            .map_err(|error| error.to_string())?,
                    );
                    relate(
                        "docs_correspondence_readme",
                        object.clone(),
                        DomainObjectRef::new(OWNER_ID, "readme_observation", &readme_key)
                            .map_err(|error| error.to_string())?,
                    );
                    for observed_claim in &claim.readme_claim_ids {
                        let observed_key = format!(
                            "{readme_key}::{}::{observed_claim}",
                            readme
                                .content_hash
                                .as_ref()
                                .ok_or("matched README content hash absent")?
                        );
                        relate(
                            "docs_correspondence_match",
                            object.clone(),
                            DomainObjectRef::new(OWNER_ID, "observed_claim", observed_key)
                                .map_err(|error| error.to_string())?,
                        );
                    }
                    semantic_qualifications.insert(
                        object,
                        BTreeMap::from([
                            ("readme_path".into(), readme.path.clone()),
                            (
                                "correspondence".into(),
                                if claim.readme_claim_ids.is_empty() {
                                    "missing"
                                } else {
                                    "represented"
                                }
                                .into(),
                            ),
                            ("claim_policy".into(), report.policy_identity.clone()),
                            (
                                "comparison_contract".into(),
                                report.contract_revision.clone(),
                            ),
                        ]),
                    );
                }
            }
        }
        if self.correspondence.as_ref().is_some_and(|report| {
            report.contract_revision == super::correspondence::CORRESPONDENCE_CONTRACT
        }) {
            semantic_qualifications
                .entry(root.clone())
                .or_default()
                .insert("comparison_complete".into(), "true".into());
            for readme in &observed.readmes {
                let object = DomainObjectRef::new(
                    OWNER_ID,
                    "readme_observation",
                    format!("{}::{}", self.scope.scope_id, readme.path),
                )
                .map_err(|error| error.to_string())?;
                let supported = self.claim_report.as_ref().and_then(|report| report.readmes.iter().find(|judgment| judgment.path == readme.path))
                    .is_some_and(|judgment| matches!(&judgment.disposition, super::claim_observation::ObservedClaimDisposition::Assessed { report } if report.accepted));
                semantic_qualifications
                    .entry(object)
                    .or_default()
                    .extend(BTreeMap::from([
                        (
                            "content_available".into(),
                            matches!(readme.state, ObservedReadmeState::Present { .. }).to_string(),
                        ),
                        ("assertions_supported".into(), supported.to_string()),
                    ]));
            }
            if let Some(claims) = self
                .claim_report
                .as_ref()
                .filter(|claims| claims.acceptance_evaluator.is_some())
            {
                for readme in &observed.readmes {
                    let judgment = claims
                        .readmes
                        .iter()
                        .find(|judgment| judgment.path == readme.path)
                        .and_then(|judgment| match &judgment.disposition {
                            super::claim_observation::ObservedClaimDisposition::Assessed {
                                report,
                            } => Some(report),
                            _ => None,
                        });
                    let object = DomainObjectRef::new(
                        OWNER_ID,
                        "readme_observation",
                        format!("{}::{}", self.scope.scope_id, readme.path),
                    )
                    .map_err(|error| error.to_string())?;
                    semantic_qualifications
                        .entry(object)
                        .or_default()
                        .extend(BTreeMap::from([
                            (
                                "accepted_by_policy".into(),
                                judgment.is_some_and(|report| report.accepted).to_string(),
                            ),
                            (
                                "assertions_supported".into(),
                                judgment
                                    .is_some_and(|report| {
                                        report.assessments.iter().all(|assessment| {
                                            assessment.verdict
                                                == super::claim_validation::ClaimVerdict::Supported
                                        })
                                    })
                                    .to_string(),
                            ),
                        ]));
                }
            }
            let report = self
                .correspondence
                .as_ref()
                .expect("current correspondence");
            for readme in &report.readmes {
                let assessments = self
                    .claim_report
                    .as_ref()
                    .and_then(|report| {
                        report
                            .readmes
                            .iter()
                            .find(|judgment| judgment.path == readme.path)
                    })
                    .and_then(|judgment| match &judgment.disposition {
                        super::claim_observation::ObservedClaimDisposition::Assessed { report } => {
                            Some(&report.assessments)
                        }
                        _ => None,
                    });
                for claim in &readme.claims {
                    let supported = !claim.readme_claim_ids.is_empty()
                        && claim.readme_claim_ids.iter().all(|id| {
                            assessments.is_some_and(|assessments| {
                                assessments.iter().any(|assessment| {
                                    assessment.claim.claim_id == *id
                                        && assessment.verdict
                                            == super::claim_validation::ClaimVerdict::Supported
                                })
                            })
                        });
                    let object = DomainObjectRef::new(
                        OWNER_ID,
                        "source_readme_correspondence",
                        format!(
                            "{}::{}::{}",
                            report.report_id, readme.path, claim.source_claim_id
                        ),
                    )
                    .map_err(|error| error.to_string())?;
                    semantic_qualifications
                        .entry(object)
                        .or_default()
                        .insert("matches_supported".into(), supported.to_string());
                }
            }
        }
        for object in &mut objects {
            if let Some(qualifications) = semantic_qualifications.remove(&object.object_ref) {
                object.qualifications.extend(qualifications);
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
                work_input_basis_id: self
                    .work_input_basis
                    .as_ref()
                    .map(|basis| basis.basis_id.clone()),
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

/// Docs owns the address and scope of the comparison source consumed by Curation.
pub fn curation_source(
    binding: &meld_world_model::curation::CurationRuleBinding,
) -> Result<meld_world_model::curation::CurationSourceBinding, String> {
    binding
        .subject
        .validate()
        .map_err(|error| error.to_string())?;
    binding
        .scope
        .validate()
        .map_err(|error| error.to_string())?;
    Ok(meld_world_model::curation::CurationSourceBinding {
        // Docs proves a captured scope, not exhaustive absence across an Event route.
        event_source: None,
        scope: binding.scope.clone(),
        roots: vec![
            DomainObjectRef::new(OWNER_ID, "scope_observation", &binding.scope.scope_id)
                .map_err(|error| error.to_string())?,
        ],
    })
}
