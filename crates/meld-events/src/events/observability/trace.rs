//! Causal trace computation over object references, relations, and
//! stored fact provenance.
//!
//! The walk is purely structural: it matches the object references,
//! relation endpoints, and `spine::{seq}` provenance strings that records
//! carry, and never interprets `event_type` semantics, which producers own.

use std::collections::{BTreeMap, HashMap, HashSet};

use serde_json::Value;

use crate::error::StorageError;
use crate::events::observability::{
    EventTraceReport, LedgerObservability, TraceHop, TraceLink, TraceSubject,
};
use crate::events::{DomainObjectRef, EventRecord};

/// Upper bound on events one trace scans, anchored at the ledger tip.
///
/// Trace is a diagnostic command, so its cost model is one bounded ledger
/// scan; ledgers whose retained history exceeds the bound get a truncated
/// chain rather than an unbounded read. The window covers the newest
/// records because "why did this just happen" is the question trace
/// exists for; a reference index that avoids the scan entirely is future
/// work.
const TRACE_SCAN_LIMIT: usize = 100_000;

/// Depth bound for the provenance walk over event data payloads.
///
/// Provenance references sit near the top of producer payloads (record
/// structs a level or two deep, plus their id arrays), so a shallow bound
/// finds them while keeping the walk cheap on large or pathological
/// payloads.
const PROVENANCE_WALK_DEPTH: usize = 6;

pub(super) fn compute(
    backing: &LedgerObservability,
    subject: TraceSubject,
) -> Result<EventTraceReport, StorageError> {
    let store = backing.store();
    // The window is the last TRACE_SCAN_LIMIT events: the cursor anchors at
    // tip minus the bound, clamped to the first readable position so a
    // raised retention boundary can never trip the store's gap check.
    let retained_from = store.retained_lower_boundary()?;
    let after_seq = store
        .tip_seq()?
        .saturating_sub(TRACE_SCAN_LIMIT as u64)
        .max(retained_from.saturating_sub(1));
    let records = store.read_all_events_after_limit(after_seq, TRACE_SCAN_LIMIT)?;

    let mut links: BTreeMap<u64, TraceLink> = BTreeMap::new();
    match &subject {
        TraceSubject::Object(object_ref) => {
            collect_object_links(&records, object_ref, &mut links);
        }
        TraceSubject::Stream {
            domain_id,
            stream_id,
        } => {
            for record in &records {
                if record.domain_id == *domain_id && record.stream_id == *stream_id {
                    upsert_link(&mut links, record.seq, TraceLink::Stream);
                }
            }
        }
        TraceSubject::Record { seq } => {
            collect_record_links(&records, *seq, &mut links);
        }
    }

    let by_seq: HashMap<u64, &EventRecord> =
        records.iter().map(|record| (record.seq, record)).collect();
    let hops = links
        .into_iter()
        .map(|(seq, link)| {
            let record = by_seq[&seq];
            TraceHop {
                seq,
                recorded_at: record.recorded_at.clone(),
                domain_id: record.domain_id.clone(),
                stream_id: record.stream_id.clone(),
                event_type: record.event_type.clone(),
                session_id: record.session.clone(),
                link,
            }
        })
        .collect();

    Ok(EventTraceReport { subject, hops })
}

/// Links every record that references the subject object, then one
/// provenance generation over the directly linked records.
///
/// The provenance pass seeds only from direct object and relation matches
/// and runs once: derived records that reference a directly linked record
/// join the chain, but records referencing those derived records do not.
/// One generation answers "what did this object cause next" without the
/// transitive closure pulling in unrelated downstream history; deeper
/// chains are traced by re-running on a hop.
fn collect_object_links(
    records: &[EventRecord],
    object_ref: &DomainObjectRef,
    links: &mut BTreeMap<u64, TraceLink>,
) {
    for record in records {
        if record.objects.contains(object_ref) {
            upsert_link(links, record.seq, TraceLink::ObjectRef);
        }
        let relation = record
            .relations
            .iter()
            .find(|relation| relation.src == *object_ref || relation.dst == *object_ref);
        if let Some(relation) = relation {
            upsert_link(
                links,
                record.seq,
                TraceLink::Relation {
                    relation_type: relation.relation_type.clone(),
                },
            );
        }
    }

    let provenance_targets: HashSet<String> =
        links.keys().map(|seq| format!("spine::{seq}")).collect();
    for record in records {
        if let Some(fact_id) = find_provenance_ref(&record.data, &provenance_targets) {
            upsert_link(links, record.seq, TraceLink::SourceFact { fact_id });
        }
    }
}

/// Links the subject record itself, records that share any of its object
/// references, and records whose payload provenance names its sequence.
fn collect_record_links(records: &[EventRecord], seq: u64, links: &mut BTreeMap<u64, TraceLink>) {
    let subject_objects: &[DomainObjectRef] = records
        .iter()
        .find(|record| record.seq == seq)
        .map(|record| record.objects.as_slice())
        .unwrap_or(&[]);
    let provenance_targets: HashSet<String> = std::iter::once(format!("spine::{seq}")).collect();

    for record in records {
        if record
            .objects
            .iter()
            .any(|object| subject_objects.contains(object))
        {
            upsert_link(links, record.seq, TraceLink::ObjectRef);
        }
        if let Some(fact_id) = find_provenance_ref(&record.data, &provenance_targets) {
            upsert_link(links, record.seq, TraceLink::SourceFact { fact_id });
        }
        if record.seq == seq {
            upsert_link(links, record.seq, TraceLink::Subject);
        }
    }
}

/// Records one hop per sequence, keeping the strongest link when several
/// inclusion rules match the same record.
fn upsert_link(links: &mut BTreeMap<u64, TraceLink>, seq: u64, link: TraceLink) {
    match links.get(&seq) {
        Some(existing) if link_strength(existing) >= link_strength(&link) => {}
        _ => {
            links.insert(seq, link);
        }
    }
}

/// Link strength for deduplication, most specific reference first:
/// Subject > SourceFact > Relation > ObjectRef > Stream. Being the subject
/// beats referencing it; stored provenance beats a typed edge; a typed edge
/// beats a bare object mention; sharing a stream is the weakest tie.
fn link_strength(link: &TraceLink) -> u8 {
    match link {
        TraceLink::Subject => 4,
        TraceLink::SourceFact { .. } => 3,
        TraceLink::Relation { .. } => 2,
        TraceLink::ObjectRef => 1,
        TraceLink::Stream => 0,
    }
}

/// Finds the first string in a payload equal to one of the provenance
/// targets, walking arrays and objects down to the depth bound.
///
/// This is the generic provenance convention: any string value equal to
/// `spine::{seq}` counts as a stored reference to that ledger record,
/// wherever the producer nested it. Key names and payload meaning stay
/// producer-owned.
fn find_provenance_ref(value: &Value, targets: &HashSet<String>) -> Option<String> {
    fn walk(value: &Value, targets: &HashSet<String>, depth: usize) -> Option<String> {
        if depth == 0 {
            return None;
        }
        match value {
            Value::String(text) if targets.contains(text) => Some(text.clone()),
            Value::Array(items) => items.iter().find_map(|item| walk(item, targets, depth - 1)),
            Value::Object(map) => map.values().find_map(|item| walk(item, targets, depth - 1)),
            _ => None,
        }
    }

    if targets.is_empty() {
        return None;
    }
    walk(value, targets, PROVENANCE_WALK_DEPTH)
}
