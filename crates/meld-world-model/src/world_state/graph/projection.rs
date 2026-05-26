//! In-memory projections for graph traversal reduction.
//!
//! These types track the state discovered during one replay pass. Durable state
//! is written through `TraversalStore`; query callers should not depend on these
//! reducer internals.

use std::collections::{BTreeMap, BTreeSet};

use crate::world_state::graph::contracts::AnchorSelectionRecord;

/// Current anchors observed during reducer replay.
#[derive(Debug, Clone, Default)]
pub struct CurrentAnchorProjection {
    /// Current anchor records keyed by anchor reference.
    pub current_anchor_by_ref: BTreeMap<String, AnchorSelectionRecord>,
    /// Current anchor records keyed by subject and perspective.
    pub current_anchor_by_subject_perspective: BTreeMap<String, AnchorSelectionRecord>,
    /// Highest event sequence applied to this projection.
    pub last_applied_seq: u64,
}

impl CurrentAnchorProjection {
    /// Select an anchor as current for its reference and subject perspective.
    pub fn select(&mut self, record: AnchorSelectionRecord) {
        let anchor_key = record.anchor_ref.index_key();
        let subject_key = format!(
            "{}::{}",
            record.subject.index_key(),
            record.perspective.index_key()
        );
        self.last_applied_seq = self.last_applied_seq.max(record.selected_at_seq);
        self.current_anchor_by_subject_perspective
            .insert(subject_key, record.clone());
        self.current_anchor_by_ref.insert(anchor_key, record);
    }

    /// End the current anchor for an anchor reference.
    pub fn end(&mut self, anchor_ref_key: &str, ended_at_seq: u64) {
        self.current_anchor_by_ref.remove(anchor_ref_key);
        self.current_anchor_by_subject_perspective
            .retain(|_, record| record.anchor_ref.index_key() != anchor_ref_key);
        self.last_applied_seq = self.last_applied_seq.max(ended_at_seq);
    }
}

/// Anchor provenance and supersession edges observed during replay.
#[derive(Debug, Clone, Default)]
pub struct AnchorLineageProjection {
    /// Superseding anchor ids keyed by original anchor id.
    pub lineage_by_anchor_id: BTreeMap<String, Vec<String>>,
    /// Source fact ids keyed by anchor id.
    pub source_fact_ids_by_anchor_id: BTreeMap<String, BTreeSet<String>>,
    /// Highest event sequence applied to this projection.
    pub last_applied_seq: u64,
}

impl AnchorLineageProjection {
    /// Attach a source fact id to an anchor.
    pub fn add_source_fact(&mut self, anchor_id: &str, source_fact_id: String, seq: u64) {
        self.source_fact_ids_by_anchor_id
            .entry(anchor_id.to_string())
            .or_default()
            .insert(source_fact_id);
        self.last_applied_seq = self.last_applied_seq.max(seq);
    }

    /// Attach a supersession edge to an anchor.
    pub fn add_supersession(&mut self, anchor_id: &str, superseded_by_anchor_id: String, seq: u64) {
        self.lineage_by_anchor_id
            .entry(anchor_id.to_string())
            .or_default()
            .push(superseded_by_anchor_id);
        self.last_applied_seq = self.last_applied_seq.max(seq);
    }
}
