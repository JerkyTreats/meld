use crate::error::StorageError;
use crate::telemetry::DomainObjectRef;
use crate::world_state::contracts::{ClaimKind, ClaimRecord, SettlementStatus};
use crate::world_state::graph::query::TraversalQuery;
use crate::world_state::graph::store::TraversalStore;

pub struct LegacyClaimAdapter<'a> {
    traversal: TraversalQuery<'a>,
}

impl<'a> LegacyClaimAdapter<'a> {
    pub fn new(store: &'a TraversalStore) -> Self {
        Self {
            traversal: TraversalQuery::new(store),
        }
    }

    pub fn current_claims_for_object(
        &self,
        subject: &DomainObjectRef,
    ) -> Result<Vec<ClaimRecord>, StorageError> {
        let mut claims = Vec::new();
        for anchor in self.traversal.current_anchors_for_subject(subject)? {
            if anchor.perspective.perspective_kind != "frame_type" {
                continue;
            }
            claims.push(ClaimRecord {
                claim_id: anchor.anchor_id.clone(),
                claim_kind: ClaimKind::GenerationSucceeded,
                subject: subject.clone(),
                status: SettlementStatus::Active,
                supporting_fact_ids: anchor.source_fact_ids.clone(),
                superseded_by: anchor.ended_by_anchor_id.clone(),
                created_by_fact_id: anchor.created_by_fact_id.clone(),
                created_at_seq: anchor.selected_at_seq,
                last_updated_seq: anchor.ended_at_seq.unwrap_or(anchor.selected_at_seq),
            });
        }
        Ok(claims)
    }
}
