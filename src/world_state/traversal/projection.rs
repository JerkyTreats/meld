use std::collections::{BTreeMap, BTreeSet};

use crate::world_state::traversal::contracts::AnchorSelectionRecord;

#[derive(Debug, Clone, Default)]
pub struct CurrentAnchorProjection {
    pub current_anchor_by_ref: BTreeMap<String, AnchorSelectionRecord>,
    pub current_anchor_by_subject_perspective: BTreeMap<String, AnchorSelectionRecord>,
    pub last_applied_seq: u64,
}

impl CurrentAnchorProjection {
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

    pub fn end(&mut self, anchor_ref_key: &str, ended_at_seq: u64) {
        self.current_anchor_by_ref.remove(anchor_ref_key);
        self.current_anchor_by_subject_perspective
            .retain(|_, record| record.anchor_ref.index_key() != anchor_ref_key);
        self.last_applied_seq = self.last_applied_seq.max(ended_at_seq);
    }
}

#[derive(Debug, Clone, Default)]
pub struct AnchorLineageProjection {
    pub lineage_by_anchor_id: BTreeMap<String, Vec<String>>,
    pub source_fact_ids_by_anchor_id: BTreeMap<String, BTreeSet<String>>,
    pub last_applied_seq: u64,
}

impl AnchorLineageProjection {
    pub fn add_source_fact(&mut self, anchor_id: &str, source_fact_id: String, seq: u64) {
        self.source_fact_ids_by_anchor_id
            .entry(anchor_id.to_string())
            .or_default()
            .insert(source_fact_id);
        self.last_applied_seq = self.last_applied_seq.max(seq);
    }

    pub fn add_supersession(&mut self, anchor_id: &str, superseded_by_anchor_id: String, seq: u64) {
        self.lineage_by_anchor_id
            .entry(anchor_id.to_string())
            .or_default()
            .push(superseded_by_anchor_id);
        self.last_applied_seq = self.last_applied_seq.max(seq);
    }
}
