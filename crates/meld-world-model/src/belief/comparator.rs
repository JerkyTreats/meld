//! Comparator contracts and the first generic Bayesian engine.
//!
//! Comparators assess already-normalized evidence. They do not ingest graph
//! state, own runtime family content, or publish planner views directly.
//!
//! # Example
//!
//! ```rust,no_run
//! use meld_world_model::belief::{BayesianComparator, ComparatorInput};
//!
//! # let config = unimplemented!();
//! # let evidence = Vec::new();
//! let input = ComparatorInput {
//!     config,
//!     config_snapshot_hash: "config-hash".to_string(),
//!     prior_revision: None,
//!     evidence,
//!     source_cursor_start: 1,
//!     source_cursor_end: 1,
//! };
//!
//! let output = BayesianComparator::assess(input);
//! # let _ = output;
//! ```

use std::collections::BTreeSet;

use crate::belief::config::stable_hash_hex;
use crate::belief::contracts::{
    BeliefFamilyConfig, BeliefProvenanceSummary, BeliefRevision, BeliefStatus, ContradictionReason,
    ContradictionState, EvidenceItem, EvidencePolarity, EvidenceValue, FreshnessState,
    HydrationRefs, ObservationOpportunity, ObservationReason, PlannerProjectionSummary,
    PosteriorSummary,
};
use crate::error::StorageError;

/// Complete comparator input frozen for deterministic replay.
pub struct ComparatorInput {
    /// Runtime family configuration used by this comparator run.
    pub config: BeliefFamilyConfig,
    /// Snapshot hash stored on the resulting revision.
    pub config_snapshot_hash: String,
    /// Prior revision used as the prior source when available.
    pub prior_revision: Option<BeliefRevision>,
    /// Evidence window assessed by this run.
    pub evidence: Vec<EvidenceItem>,
    /// First source sequence included in the run.
    pub source_cursor_start: u64,
    /// Last source sequence included in the run.
    pub source_cursor_end: u64,
}

/// Proposed revision plus hydration handles for view projection.
pub struct ComparatorOutput {
    /// Append-only revision proposed by the comparator.
    pub revision: BeliefRevision,
    /// Handles copied into the planner-safe view.
    pub view_hydration: HydrationRefs,
}

/// Generic weighted Bayesian comparator.
///
/// Factor names, weights, evidence schema bindings, polarity, prior, and
/// planner projection are all read from runtime configuration.
pub struct BayesianComparator;

impl BayesianComparator {
    /// Assess one evidence window and return a proposed revision.
    pub fn assess(input: ComparatorInput) -> Result<ComparatorOutput, StorageError> {
        if input.config.comparator.engine_id != "weighted_bayesian" {
            return missing_assessment(input);
        }
        let Some(first) = input.evidence.first() else {
            return missing_observation(input);
        };
        let key = first.candidate_key.clone();
        let prior = input
            .prior_revision
            .as_ref()
            .map(|revision| revision.posterior.probability)
            .unwrap_or(input.config.default_prior);

        let mut weighted = 0.0;
        let mut total_weight = 0.0;
        let mut supporting = Vec::new();
        let mut contradicted = Vec::new();
        let evidence_schemas: BTreeSet<String> = input
            .evidence
            .iter()
            .map(|item| item.evidence_schema_id.clone())
            .collect();
        let mut missing_required = Vec::new();
        for schema in &input.config.evidence_schemas {
            if schema.required && !evidence_schemas.contains(&schema.schema_id) {
                missing_required.push(schema.schema_id.clone());
            }
        }

        // Factor iteration follows configuration order so replay output stays
        // deterministic for the same config snapshot.
        for factor in &input.config.comparator.factors {
            for item in input
                .evidence
                .iter()
                .filter(|item| item.evidence_schema_id == factor.evidence_schema_id)
            {
                let EvidenceValue::Scalar(raw) = item.typed_value else {
                    continue;
                };
                let normalized = raw.clamp(0.0, 1.0);
                let contribution = match factor.polarity {
                    EvidencePolarity::Supports => normalized,
                    EvidencePolarity::Contradicts => 1.0 - normalized,
                };
                weighted += contribution * factor.weight * item.reliability * item.precision;
                total_weight += factor.weight * item.reliability * item.precision;
                match factor.polarity {
                    EvidencePolarity::Supports => supporting.push(item.evidence_id.clone()),
                    EvidencePolarity::Contradicts => contradicted.push(item.evidence_id.clone()),
                }
            }
        }

        let evidence_probability = if total_weight > 0.0 {
            weighted / total_weight
        } else {
            prior
        };
        let posterior = ((prior + evidence_probability) / 2.0).clamp(0.0, 1.0);
        let confidence = (1.0 - posterior).clamp(0.0, 1.0);
        let uncertainty = if missing_required.is_empty() {
            (1.0 - total_weight.min(1.0)).clamp(0.0, 1.0)
        } else {
            input.config.comparator.missing_evidence_uncertainty
        };
        let precision = input
            .evidence
            .iter()
            .map(|item| item.precision)
            .fold(0.0_f64, f64::max);
        let status = if !missing_required.is_empty() {
            BeliefStatus::NeedsObservation
        } else {
            BeliefStatus::Settled
        };
        let observation = missing_required
            .first()
            .map(|schema_id| ObservationOpportunity {
                opportunity_id: format!(
                    "observation-{}",
                    stable_hash_hex(format!("{}::{schema_id}", key.index_key()).as_bytes())
                ),
                belief_key: key.clone(),
                target_evidence_schema_id: schema_id.clone(),
                reason: ObservationReason::MissingRequiredEvidence,
                detail: "required evidence is missing".to_string(),
                source_revision_id: None,
                open: true,
            });

        let mut provenance = merge_provenance(&input.evidence);
        if let Some(prior_revision) = &input.prior_revision {
            provenance
                .revision_ids
                .push(prior_revision.revision_id.clone());
        }
        let evidence_ids: Vec<String> = input
            .evidence
            .iter()
            .map(|item| item.evidence_id.clone())
            .collect();
        provenance.evidence_ids = evidence_ids.clone();
        // The revision id commits to the key, config, evidence ids, and source
        // window. Prior revisions remain linked by field rather than folded
        // into the id.
        let revision_seed = serde_json::to_vec(&(
            &key,
            &input.config_snapshot_hash,
            &evidence_ids,
            input.source_cursor_start,
            input.source_cursor_end,
        ))
        .map_err(to_storage_data)?;
        let revision_id = format!("revision-{}", stable_hash_hex(&revision_seed));
        let mut contradiction_reasons = Vec::new();
        if !contradicted.is_empty() {
            contradiction_reasons.push(ContradictionReason::Counterevidence);
        }
        if total_weight < 1.0 {
            contradiction_reasons.push(ContradictionReason::WeakCoverage);
        }
        let contradiction = ContradictionState {
            contradicted: !contradicted.is_empty(),
            reasons: contradiction_reasons,
            supporting_evidence_ids: supporting.clone(),
            contradicted_evidence_ids: contradicted.clone(),
        };
        let revision = BeliefRevision {
            revision_id: revision_id.clone(),
            belief_key: key,
            prior_revision_id: input.prior_revision.map(|revision| revision.revision_id),
            comparator_engine_id: input.config.comparator.engine_id,
            comparator_engine_version: input.config.comparator.engine_version,
            config_snapshot_hash: input.config_snapshot_hash,
            evidence_ids: evidence_ids.clone(),
            supporting_evidence_ids: supporting,
            contradicted_evidence_ids: contradicted,
            source_cursor_start: input.source_cursor_start,
            source_cursor_end: input.source_cursor_end,
            posterior: PosteriorSummary {
                probability: posterior,
                meaning: input.config.planner_projection.posterior_meaning.clone(),
            },
            planner_projection: PlannerProjectionSummary {
                confidence_field: input.config.planner_projection.confidence_field,
                confidence,
                threshold: input.config.planner_projection.threshold,
            },
            confidence,
            uncertainty,
            precision,
            freshness: FreshnessState {
                stale: false,
                reasons: Vec::new(),
                high_water_seq: input.source_cursor_end,
            },
            contradiction,
            status,
            observation,
            provenance: provenance.clone(),
        };
        Ok(ComparatorOutput {
            view_hydration: HydrationRefs {
                evidence_ids,
                source_fact_ids: provenance.source_fact_ids,
                graph_anchor_ids: provenance.graph_anchor_ids,
                revision_id: Some(revision_id),
            },
            revision,
        })
    }
}

fn missing_assessment(input: ComparatorInput) -> Result<ComparatorOutput, StorageError> {
    let Some(first) = input.evidence.first() else {
        return missing_observation(input);
    };
    let key = first.candidate_key.clone();
    let revision_id = format!(
        "revision-{}",
        stable_hash_hex(format!("{}::missing-assessment", key.index_key()).as_bytes())
    );
    let observation = ObservationOpportunity {
        opportunity_id: format!(
            "observation-{}",
            stable_hash_hex(format!("{}::missing-comparator", key.index_key()).as_bytes())
        ),
        belief_key: key.clone(),
        target_evidence_schema_id: String::new(),
        reason: ObservationReason::MissingComparator,
        detail: "comparator engine is not available".to_string(),
        source_revision_id: None,
        open: true,
    };
    let revision = BeliefRevision {
        revision_id: revision_id.clone(),
        belief_key: key,
        prior_revision_id: input.prior_revision.map(|revision| revision.revision_id),
        comparator_engine_id: input.config.comparator.engine_id,
        comparator_engine_version: input.config.comparator.engine_version,
        config_snapshot_hash: input.config_snapshot_hash,
        evidence_ids: Vec::new(),
        supporting_evidence_ids: Vec::new(),
        contradicted_evidence_ids: Vec::new(),
        source_cursor_start: input.source_cursor_start,
        source_cursor_end: input.source_cursor_end,
        posterior: PosteriorSummary {
            probability: input.config.default_prior,
            meaning: input.config.planner_projection.posterior_meaning,
        },
        planner_projection: PlannerProjectionSummary {
            confidence_field: input.config.planner_projection.confidence_field,
            confidence: 0.0,
            threshold: input.config.planner_projection.threshold,
        },
        confidence: 0.0,
        uncertainty: 1.0,
        precision: 0.0,
        freshness: FreshnessState {
            stale: false,
            reasons: Vec::new(),
            high_water_seq: input.source_cursor_end,
        },
        contradiction: ContradictionState {
            contradicted: false,
            reasons: vec![ContradictionReason::MissingComparatorState],
            supporting_evidence_ids: Vec::new(),
            contradicted_evidence_ids: Vec::new(),
        },
        status: BeliefStatus::NeedsAssessment,
        observation: Some(observation),
        provenance: BeliefProvenanceSummary::empty(),
    };
    Ok(ComparatorOutput {
        view_hydration: HydrationRefs {
            evidence_ids: Vec::new(),
            source_fact_ids: Vec::new(),
            graph_anchor_ids: Vec::new(),
            revision_id: Some(revision_id),
        },
        revision,
    })
}

fn missing_observation(input: ComparatorInput) -> Result<ComparatorOutput, StorageError> {
    Err(StorageError::InvalidPath(format!(
        "no evidence available for family '{}'",
        input.config.family_id
    )))
}

fn merge_provenance(evidence: &[EvidenceItem]) -> BeliefProvenanceSummary {
    let mut summary = BeliefProvenanceSummary::empty();
    for item in evidence {
        for source_fact_id in &item.provenance.source_fact_ids {
            push_unique(&mut summary.source_fact_ids, source_fact_id.clone());
        }
        for anchor_id in &item.provenance.graph_anchor_ids {
            push_unique(&mut summary.graph_anchor_ids, anchor_id.clone());
        }
        for object in &item.provenance.objects {
            if !summary.objects.contains(object) {
                summary.objects.push(object.clone());
            }
        }
        for relation in &item.provenance.relations {
            if !summary.relations.contains(relation) {
                summary.relations.push(relation.clone());
            }
        }
    }
    summary
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.contains(&value) {
        values.push(value);
    }
}

fn to_storage_data(err: serde_json::Error) -> StorageError {
    StorageError::IoError(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        err.to_string(),
    ))
}
