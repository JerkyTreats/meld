//! Outcome-to-evidence mapping contract owned by the world model.
//!
//! Owner: world model. Execution owns canonical publication outcomes;
//! interpreting them as epistemic evidence is world-model theory. The mapper
//! receives the intact canonical event record — never a caller-flattened
//! projection — and decides applicability itself, so no root or execution
//! code chooses evidence probability, source kind, or subject binding.
//!
//! This module carries no implementation. The evidence-ingestion workstream
//! supplies the mapper selected by id from the stewardship expression.

use meld_events::EventRecord;
use serde::Serialize;

use crate::belief::contracts::PromotedEvidenceRecord;

/// Durable consumer identity for world-model evidence ingestion.
///
/// World model owns this name; events owns the durable cursor recorded
/// under it.
pub const EVIDENCE_CONSUMER_ID: &str = "world_model.evidence.ingestion";

/// Intact mapping input for one replayed ledger record.
#[derive(Debug, Clone, PartialEq)]
pub struct OutcomeMappingInput {
    /// Canonical record exactly as replayed from the ledger.
    pub record: EventRecord,
    /// Installed mapping identity selected by the stewardship expression.
    pub mapping_id: String,
}

/// Mapping decision for one replayed record.
#[derive(Debug, Clone, PartialEq)]
pub enum OutcomeMappingDisposition {
    /// The record maps to promoted evidence that must be durable before
    /// the consumer cursor advances.
    Applicable {
        /// Frozen deterministic evidence identity from
        /// [`promoted_evidence_identity`]. Durable evidence storage and
        /// reopen-replay deduplication key on this value.
        evidence_id: String,
        /// Promoted evidence produced by the mapping.
        record: Box<PromotedEvidenceRecord>,
    },
    /// The record is understood and carries no evidence; the cursor may
    /// advance past it.
    NotApplicable {
        /// Why the record produced no evidence.
        reason: String,
    },
    /// The record matched the mapping but its content is unusable. The
    /// rejection is recorded durably; the cursor may then advance.
    Invalid {
        /// Why the mapped content was rejected.
        reason: String,
    },
}

/// World-model-owned interpretation of canonical publication outcomes.
pub trait OutcomeEvidenceMapping {
    /// Map one intact canonical record to an evidence disposition.
    fn map_outcome(&self, input: &OutcomeMappingInput) -> OutcomeMappingDisposition;
}

/// Frozen deterministic identity for outcome-mapped promoted evidence.
///
/// Derived from the canonical publication record identity and the mapping
/// identity alone, so replaying the same publication through the same
/// installed mapping can never create distinct evidence. Fields hash as a
/// named-field structure so no delimiter inside either identity can make
/// two distinct inputs collide.
pub fn promoted_evidence_identity(publication_record_id: &str, mapping_id: &str) -> String {
    #[derive(Serialize)]
    struct Identity<'a> {
        publication_record_id: &'a str,
        mapping_id: &'a str,
    }
    let bytes = serde_json::to_vec(&Identity {
        publication_record_id,
        mapping_id,
    })
    .expect("evidence identity serialization cannot fail");
    format!("evidence-{}", blake3::hash(&bytes).to_hex())
}
