//! Belief domain for world-model inference.
//!
//! Belief consumes graph state and runtime family configuration, then produces
//! append-only revisions and planner-safe views. Family content stays outside
//! core Rust code as configuration data.
//!
//! # Example
//!
//! ```rust
//! use meld_world_model::belief::BeliefConfigLoader;
//!
//! let json = r#"{
//!   "family_id": "docs_freshness",
//!   "dimension_id": "docs_freshness",
//!   "predicate_id": "confidence",
//!   "evidence_policy_id": "default_policy",
//!   "evidence_schemas": [
//!     {
//!       "schema_id": "graph_anchor_signal",
//!       "required": true,
//!       "role": "Support",
//!       "reliability": 1.0,
//!       "precision": 1.0
//!     }
//!   ],
//!   "source_mappings": [
//!     {
//!       "mapping_id": "anchor_to_signal",
//!       "source_kind": "graph_anchor",
//!       "evidence_schema_id": "graph_anchor_signal",
//!       "subject_from": "anchor.subject",
//!       "value_field": "ended",
//!       "factor_id": "freshness_signal"
//!     }
//!   ],
//!   "comparator": {
//!     "engine_id": "weighted_bayesian",
//!     "engine_version": "1",
//!     "factors": [
//!       {
//!         "factor_id": "freshness_signal",
//!         "evidence_schema_id": "graph_anchor_signal",
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
//! let snapshot = BeliefConfigLoader::load_json(json).unwrap();
//! assert_eq!(snapshot.config.family_id, "docs_freshness");
//! ```

mod activation;
pub mod comparator;
pub mod config;
pub mod contracts;
pub mod evidence;
pub mod ingestion;
pub mod query;
pub mod runtime;
pub mod store;

pub(crate) use activation::{BeliefActivation, BeliefActivationError};
pub use comparator::{BayesianComparator, ComparatorInput, ComparatorOutput};
pub use config::{BeliefConfigLoader, ConfigSnapshot};
pub use contracts::{
    AssessmentLease, BeliefAuthorityMigrationIdentity, BeliefAuthorityMigrationMarker,
    BeliefAuthorityMigrationProgress, BeliefAuthorityParityReceipt, BeliefAuthoritySnapshot,
    BeliefCommitRecoveryDisposition, BeliefFamilyConfig, BeliefKey, BeliefProvenanceSummary,
    BeliefRevision, BeliefStatus, BeliefView, BranchScope, ComparatorConfig,
    ComparatorFactorConfig, ContradictionReason, ContradictionState, DirtyKeyState, DirtyReason,
    EvidenceAssignment, EvidenceConsumerCursor, EvidenceIngestionReceipt,
    EvidenceIngestionReceiptDisposition, EvidenceIngestionReceiptIdentity,
    EvidenceIngestionReceiptWriteDisposition, EvidenceItem, EvidencePolarity, EvidencePolicyId,
    EvidenceRejection, EvidenceRole, EvidenceSchemaConfig, EvidenceSourceMapping, EvidenceValue,
    FreshnessReason, FreshnessState, HydrationRefs, LeaseStatus, LegacyBeliefCompatibilityPosture,
    ObservationOpportunity, ObservationReason, PlannerProjectionConfig, PlannerProjectionSummary,
    PosteriorSummary, PromotedEvidenceRecord, BELIEF_AUTHORITY_MIGRATION_SCHEMA_VERSION,
};
pub use evidence::BeliefEvidenceNormalizer;
pub use ingestion::{
    ingest_promoted_evidence, PromotedEvidenceIngestionRequest, PromotedEvidenceIngestionResult,
};
pub use query::BeliefQuery;
pub use runtime::{BeliefRuntime, RuntimeAssessmentResult};
pub use store::BeliefStore;
