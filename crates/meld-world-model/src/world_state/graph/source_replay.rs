//! Bounded historical replay for exact owner routes installed after Graph advanced.

use meld_events::{LedgerCursor, ReplayRequest};

use super::ports::{GraphConsumerCursorReporter, GraphEventReplaySource};
use super::reducer::TraversalReducer;
use super::runtime::{
    validate_replay_page, GraphCatchUpReport, GraphWorkerIssue, OwnerEventReplayTick,
    GRAPH_ACTOR_ID,
};
use super::store::TraversalStore;
use crate::error::StorageError;

pub(crate) fn replay_pending_source(
    store: &TraversalStore,
    replay: &dyn GraphEventReplaySource,
    reporter: &dyn GraphConsumerCursorReporter,
    target: LedgerCursor,
    max_items: usize,
) -> Result<Option<GraphCatchUpReport>, StorageError> {
    let Some(state) = store
        .owner_event_replay_states(target.ledger_id)?
        .into_iter()
        .find(|state| !state.covered)
    else {
        return Ok(None);
    };
    let route = store
        .owner_event_routes()?
        .into_iter()
        .find(|route| {
            route
                .source_ref()
                .is_ok_and(|source| source == state.source)
        })
        .ok_or_else(|| {
            StorageError::InvalidPath("pending Graph owner route is not installed".into())
        })?;
    let before = state.position;
    let mut report = GraphCatchUpReport {
        source_replay: Some(OwnerEventReplayTick {
            source: state.source.clone(),
            before,
            after: before,
            target,
        }),
        actor_id: GRAPH_ACTOR_ID.into(),
        input_event_seq: target.after_seq,
        output_event_seq: target.after_seq,
        events_attempted: 0,
        traversal_events_applied: 0,
        derived_events_appended: 0,
        retryable_errors: Vec::new(),
        fatal_errors: Vec::new(),
        budget_exhausted: true,
        waiting_on: Vec::new(),
    };
    reporter
        .report_owner_source_cursor(&state.source, before)
        .map_err(|error| StorageError::Unavailable(error.to_string()))?;
    if before.after_seq < target.after_seq {
        let page = match replay.replay(ReplayRequest {
            cursor: before,
            limit: max_items,
        }) {
            Ok(page) => page,
            Err(meld_events::error::EventAuthorityError::RetentionGap {
                retained_from, ..
            }) => {
                report.fatal_errors.push(GraphWorkerIssue { item_id: Some(state.source.consumer_id()), code: "owner_source_retention_gap".into(), message: format!("owner route replay at {} cannot cover retained history starting at {retained_from}", before.after_seq) });
                report.budget_exhausted = false;
                return Ok(Some(report));
            }
            Err(error) => {
                report.retryable_errors.push(GraphWorkerIssue {
                    item_id: Some(state.source.consumer_id()),
                    code: "owner_source_replay_unavailable".into(),
                    message: error.to_string(),
                });
                report.budget_exhausted = false;
                return Ok(Some(report));
            }
        };
        if page.ledger_id != target.ledger_id || page.next_cursor.ledger_id != target.ledger_id {
            return Err(StorageError::InvalidPath(
                "owner source replay returned a foreign ledger".into(),
            ));
        }
        validate_replay_page(before.after_seq, max_items, &page)?;
        if page.coverage.retained_from > before.after_seq.saturating_add(1) {
            report.fatal_errors.push(GraphWorkerIssue {
                item_id: Some(state.source.consumer_id()),
                code: "owner_source_retention_gap".into(),
                message: format!(
                    "owner route replay at {} cannot cover retained history starting at {}",
                    before.after_seq, page.coverage.retained_from
                ),
            });
            report.budget_exhausted = false;
            return Ok(Some(report));
        }
        let mut cursor = before;
        for event in page
            .records
            .into_iter()
            .take_while(|event| event.seq <= target.after_seq)
        {
            report.events_attempted += 1;
            let next = event.seq;
            if event.domain_id == route.owner_id && event.event_type == route.event_type {
                let reduced = TraversalReducer::replay_records(
                    store,
                    target.ledger_id,
                    cursor.after_seq,
                    std::iter::once(event),
                )?;
                report.traversal_events_applied += reduced.applied_events;
                if !reduced.emitted_envelopes.is_empty() {
                    return Err(StorageError::InvalidPath(
                        "owner publication replay unexpectedly authored derived Events".into(),
                    ));
                }
            }
            cursor = store.advance_owner_event_replay(&state.source, cursor, next)?;
            report
                .source_replay
                .as_mut()
                .expect("source replay tick")
                .after = cursor;
        }
        if cursor == before {
            report.fatal_errors.push(GraphWorkerIssue {
                item_id: Some(state.source.consumer_id()),
                code: "owner_source_history_missing".into(),
                message: "replay did not supply the history preceding the canonical Graph position"
                    .into(),
            });
            report.budget_exhausted = false;
            return Ok(Some(report));
        }
    }
    let after = report
        .source_replay
        .as_ref()
        .expect("source replay tick")
        .after;
    reporter
        .report_owner_source_cursor(&state.source, after)
        .map_err(|error| StorageError::Unavailable(error.to_string()))?;
    if after == target {
        store.complete_owner_event_replay(&state.source, after)?;
    }
    Ok(Some(report))
}
