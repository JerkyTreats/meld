//! Evidence normalization and assignment.
//!
//! Normalization is the boundary between graph records and belief-readable
//! evidence. It preserves graph provenance while using runtime mappings to
//! avoid family-specific code paths.
//!
//! # Example
//!
//! ```rust
//! use meld_world_model::belief::{BeliefConfigLoader, BeliefEvidenceNormalizer, BranchScope};
//! use meld_world_model::PerspectiveKey;
//!
//! # let json = r#"{
//! #   "family_id": "docs_freshness",
//! #   "dimension_id": "docs_freshness",
//! #   "predicate_id": "confidence",
//! #   "evidence_policy_id": "default_policy",
//! #   "evidence_schemas": [
//! #     {"schema_id": "graph_anchor_signal", "required": true, "role": "Support", "reliability": 1.0, "precision": 1.0}
//! #   ],
//! #   "source_mappings": [
//! #     {"mapping_id": "anchor_to_signal", "source_kind": "graph_anchor", "evidence_schema_id": "graph_anchor_signal", "subject_from": "anchor.subject", "value_field": "ended", "factor_id": "freshness_signal"}
//! #   ],
//! #   "comparator": {
//! #     "engine_id": "weighted_bayesian",
//! #     "engine_version": "1",
//! #     "factors": [
//! #       {"factor_id": "freshness_signal", "evidence_schema_id": "graph_anchor_signal", "weight": 1.0, "polarity": "Supports"}
//! #     ],
//! #     "missing_evidence_uncertainty": 0.9
//! #   },
//! #   "default_prior": 0.8,
//! #   "planner_projection": {"confidence_field": "confidence", "threshold": 0.7, "posterior_meaning": "stale_probability"},
//! #   "config_version": "1"
//! # }"#;
//! let snapshot = BeliefConfigLoader::load_json(json).unwrap();
//! let normalizer = BeliefEvidenceNormalizer::new(
//!     snapshot.config,
//!     PerspectiveKey::new("default", "default").unwrap(),
//!     BranchScope::main(),
//! );
//! # let _ = normalizer;
//! ```

use crate::belief::config::stable_hash_hex;
use crate::belief::contracts::{
    BeliefFamilyConfig, BeliefKey, BeliefProvenanceSummary, BranchScope, EvidenceAssignment,
    EvidenceItem, EvidenceRejection, EvidenceRole, EvidenceSourceMapping, EvidenceValue,
    PromotedEvidenceRecord,
};
use crate::error::StorageError;
use crate::events::DomainObjectRef;
use crate::world_state::graph::PerspectiveKey;

/// Converts graph-facing records into configured evidence items.
pub struct BeliefEvidenceNormalizer {
    config: BeliefFamilyConfig,
    perspective: PerspectiveKey,
    branch_scope: BranchScope,
}

impl BeliefEvidenceNormalizer {
    /// Create a normalizer for one runtime family, perspective, and branch.
    pub fn new(
        config: BeliefFamilyConfig,
        perspective: PerspectiveKey,
        branch_scope: BranchScope,
    ) -> Self {
        Self {
            config,
            perspective,
            branch_scope,
        }
    }

    /// Normalize one promoted graph-readable outcome record.
    ///
    /// The mapping is entirely configuration driven. Unsupported shapes return
    /// durable rejection data instead of trying a family-specific fallback.
    pub fn normalize_promoted(
        &self,
        promoted: &PromotedEvidenceRecord,
    ) -> Result<Vec<EvidenceItem>, EvidenceRejection> {
        let mappings: Vec<&EvidenceSourceMapping> = self
            .config
            .source_mappings
            .iter()
            .filter(|mapping| mapping.source_kind == promoted.source_kind)
            .collect();
        if mappings.is_empty() {
            return Err(self.promoted_rejection(promoted, "missing promoted source mapping"));
        }

        let mut out = Vec::new();
        for mapping in mappings {
            let Some(schema) = self
                .config
                .evidence_schemas
                .iter()
                .find(|schema| schema.schema_id == mapping.evidence_schema_id)
            else {
                return Err(self.promoted_rejection(promoted, "mapping refers to missing schema"));
            };
            let Some(subject) = subject_from_promoted(promoted, &mapping.subject_from) else {
                return Err(self.promoted_rejection(promoted, "missing promoted subject binding"));
            };
            let Some(value) = promoted.fields.get(&mapping.value_field).cloned() else {
                return Err(self.promoted_rejection(promoted, "missing promoted value field"));
            };
            let key = BeliefKey {
                subject,
                dimension_id: self.config.dimension_id.clone(),
                predicate_id: self.config.predicate_id.clone(),
                perspective: self.perspective.clone(),
                branch_scope: self.branch_scope.clone(),
                evidence_policy_id: self.config.evidence_policy_id.clone(),
            };
            let provenance_summary = BeliefProvenanceSummary {
                evidence_ids: Vec::new(),
                source_fact_ids: promoted.source_fact_ids.clone(),
                graph_anchor_ids: promoted.graph_anchor_ids.clone(),
                objects: promoted.objects.clone(),
                relations: promoted.relations.clone(),
                revision_ids: Vec::new(),
            };
            let seed = serde_json::to_vec(&(
                &promoted.source_kind,
                &promoted.source_id,
                &mapping.mapping_id,
                &schema.schema_id,
                key.index_key(),
                promoted.source_cursor_start,
                promoted.source_cursor_end,
                &promoted.content_hash,
                &self.config.config_version,
            ))
            .map_err(|_| self.promoted_rejection(promoted, "promoted evidence seed failed"))?;
            let evidence_id = match &promoted.publication_record_id {
                Some(record_id) => {
                    let publication_seed =
                        serde_json::to_vec(&(&seed, record_id)).map_err(|_| {
                            self.promoted_rejection(promoted, "publication evidence seed failed")
                        })?;
                    format!(
                        "publication-evidence-{}",
                        stable_hash_hex(&publication_seed)
                    )
                }
                None => format!("evidence-{}", stable_hash_hex(&seed)),
            };
            out.push(EvidenceItem {
                publication_record_id: promoted.publication_record_id.clone(),
                evidence_id,
                candidate_key: key,
                source_fact_ids: promoted.source_fact_ids.clone(),
                graph_anchor_ids: promoted.graph_anchor_ids.clone(),
                source_cursor_start: promoted.source_cursor_start,
                source_cursor_end: promoted.source_cursor_end,
                role: schema.role.clone(),
                evidence_schema_id: schema.schema_id.clone(),
                typed_value: value,
                reliability: schema.reliability,
                precision: schema.precision,
                reference_time: promoted.reference_time.clone(),
                transaction_seq: promoted.transaction_seq,
                content_hash: promoted.content_hash.clone(),
                outcome_mapping_revision: promoted.outcome_mapping_revision.clone(),
                provenance: provenance_summary,
            });
        }
        Ok(out)
    }

    /// Attach normalized evidence to its candidate belief key.
    pub fn assign(&self, evidence: &EvidenceItem) -> Result<EvidenceAssignment, StorageError> {
        evidence.candidate_key.validate()?;
        Ok(EvidenceAssignment {
            assignment_id: format!(
                "assignment-{}",
                stable_hash_hex(
                    format!(
                        "{}::{}",
                        evidence.evidence_id,
                        evidence.candidate_key.index_key()
                    )
                    .as_bytes()
                )
            ),
            evidence_id: evidence.evidence_id.clone(),
            belief_key: evidence.candidate_key.clone(),
            role: evidence.role.clone(),
            source_cursor_start: evidence.source_cursor_start,
            source_cursor_end: evidence.source_cursor_end,
        })
    }

    fn promoted_rejection(
        &self,
        promoted: &PromotedEvidenceRecord,
        reason: &str,
    ) -> EvidenceRejection {
        EvidenceRejection {
            rejection_id: format!(
                "rejection-{}",
                stable_hash_hex(format!("{}::{reason}", promoted.source_id).as_bytes())
            ),
            source_id: promoted.source_id.clone(),
            reason: reason.to_string(),
            source_cursor_start: promoted.source_cursor_start,
            source_cursor_end: promoted.source_cursor_end,
            outcome_mapping_revision: promoted.outcome_mapping_revision.clone().map(Box::new),
        }
    }
}

fn subject_from_promoted(
    promoted: &PromotedEvidenceRecord,
    binding: &str,
) -> Option<DomainObjectRef> {
    match binding {
        "record.subject" => Some(promoted.subject.clone()),
        raw if raw.starts_with("field:") => promoted
            .fields
            .get(raw.trim_start_matches("field:"))
            .and_then(|value| match value {
                EvidenceValue::Map(parts) => DomainObjectRef::new(
                    parts.get("domain_id")?.clone(),
                    parts.get("object_kind")?.clone(),
                    parts.get("object_id")?.clone(),
                )
                .ok(),
                _ => None,
            }),
        _ => None,
    }
}

fn _role_is_assignable(role: &EvidenceRole) -> bool {
    matches!(
        role,
        EvidenceRole::Support | EvidenceRole::Contradiction | EvidenceRole::Supersession
    )
}
