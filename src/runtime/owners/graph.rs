//! Bound source reads delegated to the product's native Traversal authority.

use std::collections::BTreeMap;
use std::sync::Arc;

use meld_events::events::remote::{EventAuthorityContract, WatermarkRequest};
use meld_world_model::world_state::{
    graph::{contracts::*, runtime::GraphRuntime},
    query_runtime::WorldModelQueries,
};

use super::{
    encode_owner_result, OwnerCallbackPort, OwnerCallbackV1, OwnerDiagnosticV1, OwnerGraphReadV1,
    OwnerResult,
};

pub(super) struct OwnerGraphCallbacks {
    pub next: Arc<dyn OwnerCallbackPort>,
    pub graph: Arc<GraphRuntime>,
    pub events: Arc<dyn EventAuthorityContract>,
    pub ledger_id: meld_events::LedgerIdentity,
    pub inputs: BTreeMap<String, TraversalOwnerRequirement>,
}

impl OwnerCallbackPort for OwnerGraphCallbacks {
    fn call(&self, callback: OwnerCallbackV1) -> OwnerResult {
        let OwnerCallbackV1::GraphRead {
            input_id,
            traversal,
        } = callback
        else {
            return self.next.call(callback);
        };
        let input = self
            .inputs
            .get(&input_id)
            .ok_or_else(|| failure("observation input is not bound"))?;
        traversal.validate().map_err(failure)?;
        let current = self
            .events
            .watermark(WatermarkRequest {
                ledger_id: self.ledger_id,
            })
            .map_err(failure)?;
        let query = WorldModelQueries::new(self.graph.clone());
        let cut = query
            .cut(&TraversalCutRequest {
                owners: vec![input.clone()],
                scope: input.scope.clone(),
                currentness: OwnerCurrentnessPolicy::LatestComplete,
                event_position: meld_events::LedgerCursor {
                    ledger_id: current.ledger_id,
                    after_seq: current.committed_seq,
                },
            })
            .map_err(failure)?;
        let traversal = query.traverse(&cut, &traversal).map_err(failure)?;
        encode_owner_result(OwnerGraphReadV1 { cut, traversal })
    }
}

fn failure(error: impl ToString) -> OwnerDiagnosticV1 {
    OwnerDiagnosticV1::new("owner_graph_read_invalid", error)
}
