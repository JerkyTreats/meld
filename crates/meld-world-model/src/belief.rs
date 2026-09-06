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

pub mod assessment;
pub mod comparator;
pub mod config;
pub mod contracts;
pub mod evidence;
pub mod evidence_ingestion;
pub mod genesis;
pub mod ingestion;
pub mod outcome;
pub mod outcome_registry;
pub mod query;
pub mod registry;
pub mod registry_store;
pub mod runtime;
pub mod selection;
pub mod store;
mod subscription;

pub use assessment::{
    BeliefAssessmentActor, BeliefAssessmentIssue, BeliefAssessmentReport, BeliefAssessmentRequest,
};
pub use comparator::{BayesianComparator, ComparatorInput, ComparatorOutput};
pub use config::{BeliefConfigLoader, ConfigSnapshot};
pub use contracts::*;
pub use evidence::BeliefEvidenceNormalizer;
pub use evidence_ingestion::{
    EvidenceEventReplaySource, EvidenceIngestionActor, EvidenceIngestionIssue,
    EvidenceIngestionReport, EvidenceIngestionRequest,
};
pub use genesis::{
    UnobservedScopeDeclaration, EPISTEMIC_GENESIS_STREAM_ID, UNOBSERVED_SCOPE_EVENT_TYPE,
};
pub use ingestion::{
    ingest_promoted_evidence, PromotedEvidenceIngestionRequest, PromotedEvidenceIngestionResult,
};
pub use outcome::interpretation::{
    ConfiguredOutcomeMapping, ConfiguredOutcomeMappingSet, OutcomeContentRule, OutcomeFieldRule,
    OutcomeMappingConfig, OutcomeMappingSetConfig, OutcomeSubjectBinding, OutcomeValueSource,
};
pub use outcome::mapping::{
    promoted_evidence_identity, promoted_evidence_identity_for_revision, OutcomeEvidenceMapping,
    OutcomeMappingDisposition, OutcomeMappingInput, EVIDENCE_CONSUMER_ID,
};
pub use outcome_registry::{OutcomeMappingRegistryStore, OutcomeMappingRevision};
pub use query::BeliefQuery;
pub use registry::{
    BeliefFamilyRegistry, BeliefFamilyRevision, TheoryInstallDisposition, TheoryRevisionRef,
};
pub use registry_store::BeliefFamilyRegistryStore;
pub use runtime::{BeliefRuntime, RuntimeAssessmentResult};
pub use selection::{
    configured_belief_key, BeliefSubjectBinding, BeliefWorkItem, BeliefWorkKind,
    BeliefWorkSelection, BeliefWorkSelector,
};
pub use store::BeliefStore;
pub use subscription::{
    BeliefSubscriptionAcceptanceProof, BeliefSubscriptionAuthority, BeliefSubscriptionSource,
};
