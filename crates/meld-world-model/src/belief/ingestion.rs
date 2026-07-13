//! Generic promoted evidence ingestion.
//!
//! Ingestion accepts promoted records from integration boundaries, normalizes
//! them through runtime configuration, assigns new evidence idempotently, and
//! reassesses only newly dirtied belief keys.
//!
//! # Example
//!
//! ```rust,no_run
//! use std::collections::BTreeMap;
//! use std::sync::Arc;
//!
//! use meld_world_model::belief::{
//!     ingest_promoted_evidence, BeliefConfigLoader, BeliefRuntime, BeliefStore,
//!     BranchScope, EvidenceValue, PromotedEvidenceIngestionRequest,
//!     PromotedEvidenceRecord,
//! };
//! use meld_world_model::events::DomainObjectRef;
//! use meld_world_model::world_state::graph::store::TraversalStore;
//! use meld_world_model::PerspectiveKey;
//!
//! let json = r#"{
//!   "family_id": "docs_freshness",
//!   "dimension_id": "docs_freshness",
//!   "predicate_id": "confidence",
//!   "evidence_policy_id": "default_policy",
//!   "evidence_schemas": [
//!     {
//!       "schema_id": "content_written_signal",
//!       "required": false,
//!       "role": "Support",
//!       "reliability": 1.0,
//!       "precision": 1.0
//!     }
//!   ],
//!   "source_mappings": [
//!     {
//!       "mapping_id": "content_written_to_signal",
//!       "source_kind": "content_written",
//!       "evidence_schema_id": "content_written_signal",
//!       "subject_from": "record.subject",
//!       "value_field": "stale_probability",
//!       "factor_id": "content_written_signal"
//!     }
//!   ],
//!   "comparator": {
//!     "engine_id": "weighted_bayesian",
//!     "engine_version": "1",
//!     "factors": [
//!       {
//!         "factor_id": "content_written_signal",
//!         "evidence_schema_id": "content_written_signal",
//!         "weight": 1.0,
//!         "polarity": "Supports"
//!       }
//!     ],
//!     "missing_evidence_uncertainty": 0.9
//!   },
//!   "default_prior": 0.8,
//!   "planner_projection": {
//!     "confidence_field": "confidence",
//!     "threshold": 0.7,
//!     "posterior_meaning": "stale_probability"
//!   },
//!   "config_version": "1"
//! }"#;
//!
//! let db = sled::Config::new().temporary(true).open().unwrap();
//! let belief = Arc::new(BeliefStore::new(db.clone()).unwrap());
//! let graph = Arc::new(TraversalStore::new(db).unwrap());
//! let config = BeliefConfigLoader::load_json(json).unwrap();
//! let perspective = PerspectiveKey::new("default", "default").unwrap();
//! let branch_scope = BranchScope::main();
//! let runtime = BeliefRuntime::new(
//!     belief.clone(),
//!     graph,
//!     config.clone(),
//!     perspective.clone(),
//!     branch_scope.clone(),
//! );
//!
//! let subject = DomainObjectRef::new("workspace_fs", "node", "node-a").unwrap();
//! let mut fields = BTreeMap::new();
//! fields.insert("stale_probability".to_string(), EvidenceValue::Scalar(0.0));
//! let record = PromotedEvidenceRecord {
//!     source_kind: "content_written".to_string(),
//!     source_id: "event-spine::2".to_string(),
//!     subject,
//!     source_fact_ids: vec!["event-spine::2".to_string()],
//!     graph_anchor_ids: Vec::new(),
//!     objects: Vec::new(),
//!     relations: Vec::new(),
//!     source_cursor_start: 2,
//!     source_cursor_end: 2,
//!     reference_time: None,
//!     transaction_seq: 2,
//!     content_hash: None,
//!     fields,
//! };
//!
//! let result = ingest_promoted_evidence(
//!     belief.as_ref(),
//!     &runtime,
//!     PromotedEvidenceIngestionRequest {
//!         record,
//!         config,
//!         perspective,
//!         branch_scope,
//!         owner_id: "worker-a",
//!     },
//! ).unwrap();
//!
//! assert_eq!(result.new_assignment_count, 1);
//! ```

use crate::belief::{
    BeliefEvidenceNormalizer, BeliefKey, BeliefRuntime, BeliefStore, BranchScope, ConfigSnapshot,
    EvidenceConsumerCursor, EvidenceIngestionReceipt, EvidenceIngestionReceiptWriteDisposition,
    PromotedEvidenceRecord, RuntimeAssessmentResult,
};
use crate::error::StorageError;
use crate::world_state::graph::PerspectiveKey;

/// Request to ingest one promoted evidence record through a belief family.
#[derive(Debug, Clone)]
pub struct PromotedEvidenceIngestionRequest<'a> {
    /// Generic promoted evidence candidate.
    pub record: PromotedEvidenceRecord,
    /// Runtime family configuration used for normalization.
    pub config: ConfigSnapshot,
    /// Perspective for candidate belief keys.
    pub perspective: PerspectiveKey,
    /// Branch scope for candidate belief keys.
    pub branch_scope: BranchScope,
    /// Worker identity recorded on assessment leases.
    pub owner_id: &'a str,
}

/// Result of one promoted evidence ingestion attempt.
#[derive(Debug, Clone, PartialEq)]
pub struct PromotedEvidenceIngestionResult {
    /// Count of evidence items produced by normalization.
    pub normalized_evidence_count: usize,
    /// Count of assignment edges newly inserted.
    pub new_assignment_count: usize,
    /// Dirty-key reassessments committed by this ingestion.
    pub committed: Vec<RuntimeAssessmentResult>,
    /// True when the promoted record was rejected by configuration.
    pub rejected: bool,
}

/// Normalize, assign, and reassess one promoted evidence record.
pub fn ingest_promoted_evidence(
    store: &BeliefStore,
    runtime: &BeliefRuntime,
    request: PromotedEvidenceIngestionRequest<'_>,
) -> Result<PromotedEvidenceIngestionResult, StorageError> {
    let normalizer = BeliefEvidenceNormalizer::new(
        request.config.config,
        request.perspective,
        request.branch_scope,
    );
    let evidence = match normalizer.normalize_promoted(&request.record) {
        Ok(evidence) => evidence,
        Err(rejection) => {
            store.put_rejection(&rejection)?;
            return Ok(PromotedEvidenceIngestionResult {
                normalized_evidence_count: 0,
                new_assignment_count: 0,
                committed: Vec::new(),
                rejected: true,
            });
        }
    };

    let mut affected_keys = Vec::new();
    let mut new_assignment_count = 0;
    for item in &evidence {
        store.put_evidence_once(item)?;
        let assignment = normalizer.assign(item)?;
        if store.put_assignment_once(&assignment)? {
            new_assignment_count += 1;
            affected_keys.push(item.candidate_key.clone());
        }
    }

    sort_dedup_keys(&mut affected_keys);
    let mut committed = Vec::new();
    for key in affected_keys {
        if let Some(result) = runtime.assess_dirty_key(&key, request.owner_id)? {
            committed.push(result);
        }
    }

    Ok(PromotedEvidenceIngestionResult {
        normalized_evidence_count: evidence.len(),
        new_assignment_count,
        committed,
        rejected: false,
    })
}

/// Durably bind one evidence receipt to its owner-scoped cursor advancement.
pub fn persist_evidence_receipt_and_advance(
    store: &BeliefStore,
    expected: Option<&EvidenceConsumerCursor>,
    receipt: &EvidenceIngestionReceipt,
    next: &EvidenceConsumerCursor,
) -> Result<EvidenceIngestionReceiptWriteDisposition, StorageError> {
    store.record_evidence_receipt_and_advance(expected, receipt, next)
}

fn sort_dedup_keys(keys: &mut Vec<BeliefKey>) {
    keys.sort_by_key(|key| key.index_key());
    keys.dedup_by_key(|key| key.index_key());
}
