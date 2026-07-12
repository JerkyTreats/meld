//! Recoverable migration from frozen legacy event trees into an authority.
//!
//! Owner: event compatibility migration.
//! Inputs: one externally locked legacy ledger path, one distinct target
//! authority, and a product binding generation.
//! Outputs: durable source identity, source-to-target sequence mappings, a
//! verified target ledger, and a source cutover marker.
//! Does not own: product binding files, process locks, branch identity,
//! workspace path policy, or legacy session compatibility storage.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sled::transaction::{ConflictableTransactionError, TransactionError, Transactional};

use crate::error::EventAuthorityError;
use crate::events::authority::EventAuthority;
use crate::events::store::{META_KEY_AUTHORITY_CUTOVER, META_KEY_LEDGER_IDENTITY};
use crate::events::{EventEnvelope, EventRecord, LedgerIdentity};

const TREE_LEGACY_EVENTS: &str = "obs_events";
const TREE_EVENTS: &str = "obs_spine_events";
const TREE_SESSION_INDEX: &str = "obs_session_event_index";
const TREE_META: &str = "obs_spine_meta";
const TREE_RECORD_INDEX: &str = "obs_spine_record_index";
const TREE_MIGRATION_MAPPINGS: &str = "obs_authority_migration_map";
const META_KEY_GLOBAL: &[u8] = b"global";
const META_KEY_RETAINED_FROM: &[u8] = b"retained_from";
const EVENT_KEY_PAD: usize = 20;
const CUTOVER_SCHEMA_VERSION: u32 = 1;
const MAX_MIGRATION_BATCH_SIZE: usize = 10_000;
const LEGACY_MISSING_TIMESTAMP: &str = "1970-01-01T00:00:00.000Z";
const SOURCE_OPEN_RETRY_ATTEMPTS: usize = 100;
const SOURCE_OPEN_RETRY_DELAY: std::time::Duration = std::time::Duration::from_millis(10);

/// Options for one recoverable migration run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LegacyEventMigrationOptions {
    /// Product binding generation written into the source cutover marker.
    pub generation: u64,
    /// Number of source rows flushed per recovery checkpoint.
    pub batch_size: usize,
}

impl LegacyEventMigrationOptions {
    /// Creates options using a 256-record durability checkpoint.
    pub const fn new(generation: u64) -> Self {
        Self {
            generation,
            batch_size: 256,
        }
    }
}

/// Durable evidence mapping one frozen source sequence to the target ledger.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LegacyEventMigrationMapping {
    /// Frozen legacy ledger identity.
    pub source_ledger_id: LedgerIdentity,
    /// Normalized source sequence. Legacy per-session rows follow the greatest
    /// canonical source sequence in deterministic key order.
    pub source_seq: u64,
    /// Product authority ledger identity.
    pub target_ledger_id: LedgerIdentity,
    /// Sequence assigned by the target authority store.
    pub target_seq: u64,
    /// BLAKE3 hash of the normalized canonical source envelope.
    pub canonical_envelope_hash: String,
    /// Whether this mapping inserted a target row rather than reusing an
    /// exactly equal record-ID collision.
    pub inserted: bool,
}

/// Durable marker that makes a migrated legacy event ledger read-only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LegacyEventCutoverMarker {
    /// Marker wire schema.
    pub schema_version: u32,
    /// Frozen source ledger identity.
    pub source_ledger_id: LedgerIdentity,
    /// Product authority receiving the legacy history.
    pub target_ledger_id: LedgerIdentity,
    /// Product binding generation that completed the copy.
    pub generation: u64,
    /// Number of normalized source records verified at cutover.
    pub source_record_count: u64,
    /// BLAKE3 hash over the ordered normalized source snapshot.
    pub source_snapshot_hash: String,
}

/// Progress and verification evidence for a migration invocation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LegacyEventMigrationReport {
    /// Frozen source identity.
    pub source_ledger_id: LedgerIdentity,
    /// Target authority identity.
    pub target_ledger_id: LedgerIdentity,
    /// Binding generation being prepared.
    pub generation: u64,
    /// Number of normalized source records.
    pub source_record_count: u64,
    /// Number of source mappings durably verified so far.
    pub mapped_record_count: u64,
    /// Number of those mappings that inserted a target record.
    pub inserted_target_count: u64,
    /// Durable target tip after this invocation.
    pub target_tip_seq: u64,
    /// True only after full verification and the source cutover marker flush.
    pub complete: bool,
}

/// Strictly validated snapshot of frozen legacy event trees.
pub struct LegacyEventMigrationSource {
    canonical_path: PathBuf,
    db: sled::Db,
    ledger_id: LedgerIdentity,
    retained_from: u64,
    records: Vec<SourceRecord>,
    snapshot_hash: String,
}

#[derive(Clone)]
struct SourceRecord {
    source_seq: u64,
    envelope: EventEnvelope,
    envelope_hash: String,
}

#[derive(Clone)]
struct PlannedRecord {
    source: SourceRecord,
    translated: EventEnvelope,
    target_seq: u64,
    inserted: bool,
    already_mapped: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SequenceMeta {
    next_seq: u64,
}

impl LegacyEventMigrationSource {
    /// Opens and strictly validates both frozen event trees before establishing
    /// and flushing the source ledger identity.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, EventAuthorityError> {
        let canonical_path = canonicalize_existing(path.as_ref(), "legacy event source")?;
        let db = open_source_db(&canonical_path)?;
        let retained_from = read_retained_from(&db)?;
        let records = validate_source_rows(&db, retained_from)?;
        let ledger_id = establish_source_identity(&db)?;
        validate_source_contracts(&records, ledger_id)?;
        let snapshot_hash = source_snapshot_hash(&records);

        if let Some(marker) = read_marker_from_db(&db)? {
            if marker.source_ledger_id != ledger_id {
                return Err(conflict(format!(
                    "source cutover marker identity {} disagrees with persisted identity {ledger_id}",
                    marker.source_ledger_id
                )));
            }
            if marker.source_record_count != records.len() as u64
                || marker.source_snapshot_hash != snapshot_hash
            {
                return Err(conflict(
                    "legacy event rows changed after the cutover marker was written",
                ));
            }
        }

        Ok(Self {
            canonical_path,
            db,
            ledger_id,
            retained_from,
            records,
            snapshot_hash,
        })
    }

    /// Returns the canonical source storage path used for equality fencing.
    pub fn canonical_path(&self) -> &Path {
        &self.canonical_path
    }

    /// Returns the persisted source identity.
    pub fn ledger_identity(&self) -> LedgerIdentity {
        self.ledger_id
    }

    /// Returns the source retention boundary applied to canonical rows.
    pub fn retained_from(&self) -> u64 {
        self.retained_from
    }

    /// Reads the flushed source marker, when cutover is complete.
    pub fn cutover_marker(&self) -> Result<Option<LegacyEventCutoverMarker>, EventAuthorityError> {
        read_marker_from_db(&self.db)
    }

    /// Migrates at most one durability batch. Repeated calls resume from
    /// mapping evidence and finish by flushing a source cutover marker.
    pub fn migrate_batch_into(
        &self,
        target_path: impl AsRef<Path>,
        target: &EventAuthority,
        generation: u64,
        batch_size: usize,
    ) -> Result<LegacyEventMigrationReport, EventAuthorityError> {
        validate_batch_size(batch_size)?;
        let target_path = canonicalize_existing(target_path.as_ref(), "event authority target")?;
        if target_path == self.canonical_path {
            return Err(conflict(format!(
                "legacy source and target resolve to the same path: {}",
                target_path.display()
            )));
        }

        let target_id = target.ledger_identity();
        if target_id == self.ledger_id {
            return Err(conflict(format!(
                "distinct source and target paths carry the same ledger identity {target_id}"
            )));
        }
        if let Some(marker) = read_marker_from_db(&self.db)? {
            validate_marker(
                &marker,
                self.ledger_id,
                target_id,
                generation,
                self.records.len() as u64,
                &self.snapshot_hash,
            )?;
        }

        let target_db = target.migration_db();
        let plan = preflight_migration(self, &target_db, target_id)?;
        for record in plan
            .iter()
            .filter(|record| !record.already_mapped)
            .take(batch_size)
        {
            apply_record_mapping(&target_db, self.ledger_id, target_id, record)?;
        }
        target_db.flush().map_err(persistence)?;
        let target_tip = target_tip(&target_db)?;
        target.acknowledge_migration_tip(target_tip);

        let mut report = verify_migration(self, &target_db, target_id, generation, false)?;
        if report.mapped_record_count == report.source_record_count {
            let marker = LegacyEventCutoverMarker {
                schema_version: CUTOVER_SCHEMA_VERSION,
                source_ledger_id: self.ledger_id,
                target_ledger_id: target_id,
                generation,
                source_record_count: self.records.len() as u64,
                source_snapshot_hash: self.snapshot_hash.clone(),
            };
            write_source_marker(&self.db, &marker)?;
            report.complete = true;
        }
        Ok(report)
    }

    /// Runs recoverable batches until every source record is verified and the
    /// source cutover marker is durable.
    pub fn migrate_all_into(
        &self,
        target_path: impl AsRef<Path>,
        target: &EventAuthority,
        options: LegacyEventMigrationOptions,
    ) -> Result<LegacyEventMigrationReport, EventAuthorityError> {
        validate_batch_size(options.batch_size)?;
        loop {
            let report = self.migrate_batch_into(
                target_path.as_ref(),
                target,
                options.generation,
                options.batch_size,
            )?;
            if report.complete {
                return Ok(report);
            }
        }
    }
}

fn validate_batch_size(batch_size: usize) -> Result<(), EventAuthorityError> {
    if (1..=MAX_MIGRATION_BATCH_SIZE).contains(&batch_size) {
        Ok(())
    } else {
        Err(EventAuthorityError::invalid_request(format!(
            "migration batch_size must be in 1..={MAX_MIGRATION_BATCH_SIZE}, got {batch_size}"
        )))
    }
}

fn validate_source_contracts(
    records: &[SourceRecord],
    source_id: LedgerIdentity,
) -> Result<(), EventAuthorityError> {
    let sequences = records
        .iter()
        .map(|record| record.source_seq)
        .collect::<BTreeSet<_>>();
    let mut record_ids = BTreeMap::<&str, &EventEnvelope>::new();
    for record in records {
        if let Some(record_id) = record.envelope.record_id.as_deref() {
            if record_ids.insert(record_id, &record.envelope).is_some() {
                return Err(conflict(format!(
                    "source record_id {record_id:?} occurs at multiple source sequences"
                )));
            }
        }
        for reference in &record.envelope.provenance.source_records {
            if reference.ledger_id != source_id {
                return Err(conflict(format!(
                    "source sequence {} provenance names foreign ledger {} instead of {source_id}",
                    record.source_seq, reference.ledger_id
                )));
            }
            if !sequences.contains(&reference.seq) {
                return Err(conflict(format!(
                    "source sequence {} provenance names missing source sequence {}",
                    record.source_seq, reference.seq
                )));
            }
        }
    }
    Ok(())
}

fn preflight_migration(
    source: &LegacyEventMigrationSource,
    target_db: &sled::Db,
    target_id: LedgerIdentity,
) -> Result<Vec<PlannedRecord>, EventAuthorityError> {
    validate_target_rows(target_db, target_id)?;
    let mappings = existing_tree(target_db, TREE_MIGRATION_MAPPINGS)?;
    if let Some(mappings) = &mappings {
        let source_sequences = source
            .records
            .iter()
            .map(|record| record.source_seq)
            .collect::<BTreeSet<_>>();
        for row in mappings.iter() {
            let (key, raw) = row.map_err(persistence)?;
            let mapping = decode_mapping(&raw, &key)?;
            if mapping.source_ledger_id == source.ledger_id
                && !source_sequences.contains(&mapping.source_seq)
            {
                return Err(conflict(format!(
                    "migration mapping names absent source sequence {}",
                    mapping.source_seq
                )));
            }
        }
    }
    let events = target_db.open_tree(TREE_EVENTS).map_err(persistence)?;
    let mut target_envelopes = BTreeMap::<u64, EventEnvelope>::new();
    let mut record_ids = BTreeMap::<String, u64>::new();
    for row in events.iter() {
        let (key, value) = row.map_err(persistence)?;
        let seq = decode_canonical_key(&key, TREE_EVENTS)?;
        let record = decode_source_record(&value, TREE_EVENTS, &key)?;
        target_envelopes.insert(seq, record.envelope.clone());
        if let Some(record_id) = record.record_id.as_ref() {
            record_ids.insert(record_id.clone(), seq);
        }
    }

    let mut next_seq = target_next_seq(target_db)?;
    let mut saw_unmapped = false;
    let mut raw_plan = Vec::<(SourceRecord, u64, bool, bool)>::new();
    for record in &source.records {
        let key = mapping_key(source.ledger_id, record.source_seq);
        let existing = mapping_get(mappings.as_ref(), &key)?;
        if let Some(mapping) = existing {
            if saw_unmapped {
                return Err(conflict(format!(
                    "migration mappings are not a durable source prefix at sequence {}",
                    record.source_seq
                )));
            }
            validate_mapping_contract(&mapping, source, target_id, record)?;
            if let Some(record_id) = record.envelope.record_id.as_ref() {
                if let Some(indexed_seq) = record_ids.get(record_id) {
                    if *indexed_seq != mapping.target_seq {
                        return Err(conflict(format!(
                            "source record_id {record_id:?} maps to {}, but target index names {indexed_seq}",
                            mapping.target_seq
                        )));
                    }
                }
            }
            raw_plan.push((record.clone(), mapping.target_seq, mapping.inserted, true));
            continue;
        }

        saw_unmapped = true;
        let collision = record
            .envelope
            .record_id
            .as_ref()
            .and_then(|record_id| record_ids.get(record_id).copied());
        let (target_seq, inserted) = match collision {
            Some(target_seq) => (target_seq, false),
            None => {
                if next_seq == u64::MAX {
                    return Err(conflict("target event sequence allocation overflowed u64"));
                }
                let target_seq = next_seq;
                next_seq += 1;
                if let Some(record_id) = record.envelope.record_id.clone() {
                    record_ids.insert(record_id, target_seq);
                }
                (target_seq, true)
            }
        };
        raw_plan.push((record.clone(), target_seq, inserted, false));
    }

    if let Some(pair) = raw_plan.windows(2).find(|pair| pair[0].1 >= pair[1].1) {
        return Err(conflict(format!(
            "source-to-target mapping is not strictly monotonic: source {} maps to {}, then source {} maps to {}",
            pair[0].0.source_seq, pair[0].1, pair[1].0.source_seq, pair[1].1
        )));
    }

    let sequence_plan = raw_plan
        .iter()
        .map(|(record, target_seq, _, _)| (record.source_seq, *target_seq))
        .collect::<BTreeMap<_, _>>();
    let mut planned_envelopes = target_envelopes;
    let mut plan = Vec::with_capacity(raw_plan.len());
    for (record, target_seq, inserted, already_mapped) in raw_plan {
        let translated = translate_source_provenance(
            &record.envelope,
            source.ledger_id,
            target_id,
            &sequence_plan,
        )?;
        if already_mapped && !planned_envelopes.contains_key(&target_seq) {
            return Err(conflict(format!(
                "inserted migration mapping for source sequence {} names missing target sequence {target_seq}",
                record.source_seq
            )));
        }
        if let Some(existing) = planned_envelopes.get(&target_seq) {
            if existing != &translated {
                return Err(conflict(format!(
                    "source sequence {} collides with divergent target sequence {target_seq}",
                    record.source_seq
                )));
            }
        } else if !inserted {
            return Err(conflict(format!(
                "source sequence {} reuses missing target sequence {target_seq}",
                record.source_seq
            )));
        } else {
            planned_envelopes.insert(target_seq, translated.clone());
        }
        plan.push(PlannedRecord {
            source: record,
            translated,
            target_seq,
            inserted,
            already_mapped,
        });
    }
    Ok(plan)
}

fn validate_source_rows(
    db: &sled::Db,
    retained_from: u64,
) -> Result<Vec<SourceRecord>, EventAuthorityError> {
    let canonical = db.open_tree(TREE_EVENTS).map_err(persistence)?;
    let legacy = db.open_tree(TREE_LEGACY_EVENTS).map_err(persistence)?;
    let mut records = Vec::new();
    let mut greatest_source_seq = 0_u64;

    for row in canonical.iter() {
        let (key, value) = row.map_err(persistence)?;
        let seq = decode_canonical_key(&key, TREE_EVENTS)?;
        let record = decode_source_record(&value, TREE_EVENTS, &key)?;
        if record.seq != seq {
            return Err(conflict(format!(
                "{TREE_EVENTS} key sequence {seq} disagrees with record sequence {}",
                record.seq
            )));
        }
        greatest_source_seq = greatest_source_seq.max(seq);
        if seq >= retained_from {
            records.push(source_record(seq, record.envelope)?);
        }
    }

    for row in legacy.iter() {
        let (key, value) = row.map_err(persistence)?;
        let (session, legacy_seq) = decode_legacy_key(&key)?;
        let record = decode_source_record(&value, TREE_LEGACY_EVENTS, &key)?;
        if record.seq != legacy_seq {
            return Err(conflict(format!(
                "{TREE_LEGACY_EVENTS} key sequence {legacy_seq} disagrees with record sequence {}",
                record.seq
            )));
        }
        if record.session != session {
            return Err(conflict(format!(
                "{TREE_LEGACY_EVENTS} key session {session:?} disagrees with record session {:?}",
                record.session
            )));
        }
        greatest_source_seq = greatest_source_seq
            .checked_add(1)
            .ok_or_else(|| conflict("legacy source sequence normalization overflowed u64"))?;
        records.push(source_record(greatest_source_seq, record.envelope)?);
    }

    records.sort_by_key(|record| record.source_seq);
    Ok(records)
}

fn source_record(
    source_seq: u64,
    envelope: EventEnvelope,
) -> Result<SourceRecord, EventAuthorityError> {
    let envelope_hash = canonical_envelope_hash(&envelope)?;
    Ok(SourceRecord {
        source_seq,
        envelope,
        envelope_hash,
    })
}

fn validate_target_rows(
    db: &sled::Db,
    target_id: LedgerIdentity,
) -> Result<(), EventAuthorityError> {
    let events = db.open_tree(TREE_EVENTS).map_err(persistence)?;
    let record_index = db.open_tree(TREE_RECORD_INDEX).map_err(persistence)?;
    let mut record_ids = BTreeMap::<String, u64>::new();
    for row in events.iter() {
        let (key, value) = row.map_err(persistence)?;
        let seq = decode_canonical_key(&key, TREE_EVENTS)?;
        let record = decode_source_record(&value, TREE_EVENTS, &key)?;
        if record.seq != seq {
            return Err(conflict(format!(
                "target {TREE_EVENTS} key sequence {seq} disagrees with record sequence {}",
                record.seq
            )));
        }
        if let Some(record_id) = &record.record_id {
            if let Some(previous) = record_ids.insert(record_id.clone(), seq) {
                return Err(conflict(format!(
                    "target record_id {record_id:?} occurs at both sequence {previous} and {seq}"
                )));
            }
            let raw = record_index
                .get(record_id.as_bytes())
                .map_err(persistence)?
                .ok_or_else(|| {
                    conflict(format!(
                        "target record_id {record_id:?} has no durable index entry"
                    ))
                })?;
            if decode_seq(&raw, "target record index")? != seq {
                return Err(conflict(format!(
                    "target record_id {record_id:?} index does not name sequence {seq}"
                )));
            }
        }
    }
    for row in record_index.iter() {
        let (record_id, raw_seq) = row.map_err(persistence)?;
        let record_id = std::str::from_utf8(&record_id)
            .map_err(|_| conflict("target record index contains a non-UTF-8 record_id"))?;
        let seq = decode_seq(&raw_seq, "target record index")?;
        let raw_record = events
            .get(encode_record_key(seq).as_bytes())
            .map_err(persistence)?
            .ok_or_else(|| {
                conflict(format!(
                    "target record index {record_id:?} names missing sequence {seq}"
                ))
            })?;
        let record =
            decode_source_record(&raw_record, TREE_EVENTS, encode_record_key(seq).as_bytes())?;
        if record.record_id.as_deref() != Some(record_id) {
            return Err(conflict(format!(
                "target record index {record_id:?} names sequence {seq} with a different record_id"
            )));
        }
    }
    if let Some(mappings) = existing_tree(db, TREE_MIGRATION_MAPPINGS)? {
        for row in mappings.iter() {
            let (key, value) = row.map_err(persistence)?;
            let mapping = decode_mapping(&value, &key)?;
            if key.as_ref() != mapping_key(mapping.source_ledger_id, mapping.source_seq) {
                return Err(conflict(
                    "migration mapping key disagrees with its identity-bearing payload",
                ));
            }
            if mapping.target_ledger_id != target_id {
                return Err(conflict(format!(
                    "migration mapping for source sequence {} names target {}, expected {target_id}",
                    mapping.source_seq, mapping.target_ledger_id
                )));
            }
        }
    }
    Ok(())
}

fn apply_record_mapping(
    db: &sled::Db,
    source_id: LedgerIdentity,
    target_id: LedgerIdentity,
    planned: &PlannedRecord,
) -> Result<LegacyEventMigrationMapping, EventAuthorityError> {
    let meta = db.open_tree(TREE_META).map_err(persistence)?;
    let events = db.open_tree(TREE_EVENTS).map_err(persistence)?;
    let session_index = db.open_tree(TREE_SESSION_INDEX).map_err(persistence)?;
    let record_index = db.open_tree(TREE_RECORD_INDEX).map_err(persistence)?;
    let mappings = db.open_tree(TREE_MIGRATION_MAPPINGS).map_err(persistence)?;
    let mapping_key = mapping_key(source_id, planned.source.source_seq);

    (&meta, &events, &session_index, &record_index, &mappings)
        .transaction(|(meta, events, session_index, record_index, mappings)| {
            if let Some(raw) = mappings.get(&mapping_key)? {
                let mapping: LegacyEventMigrationMapping =
                    serde_json::from_slice(&raw).map_err(transaction_data)?;
                if mapping.target_seq != planned.target_seq
                    || mapping.inserted != planned.inserted
                    || mapping.canonical_envelope_hash != planned.source.envelope_hash
                {
                    return Err(transaction_conflict(format!(
                        "migration plan conflicts with existing mapping for source sequence {}",
                        planned.source.source_seq
                    )));
                }
                return Ok(mapping);
            }

            let collision_seq = match planned.translated.record_id.as_deref() {
                Some(record_id) => record_index
                    .get(record_id.as_bytes())?
                    .map(|raw| decode_seq_transaction(&raw, "target record index"))
                    .transpose()?,
                None => None,
            };

            let (target_seq, inserted) = if let Some(target_seq) = collision_seq {
                if target_seq != planned.target_seq || planned.inserted {
                    return Err(transaction_conflict(format!(
                        "target record index changed after migration preflight for source sequence {}",
                        planned.source.source_seq
                    )));
                }
                let key = encode_record_key(target_seq);
                let raw = events.get(key.as_bytes())?.ok_or_else(|| {
                    transaction_conflict(format!(
                        "target record-ID index names missing sequence {target_seq}"
                    ))
                })?;
                let existing: EventRecord =
                    serde_json::from_slice(&raw).map_err(transaction_data)?;
                let existing = normalize_migration_record(existing);
                if existing.envelope != planned.translated {
                    return Err(transaction_conflict(format!(
                        "target record_id {:?} collides with a divergent envelope",
                        planned.translated.record_id
                    )));
                }
                (target_seq, false)
            } else {
                if !planned.inserted {
                    return Err(transaction_conflict(format!(
                        "planned target collision disappeared for source sequence {}",
                        planned.source.source_seq
                    )));
                }
                let mut sequence = match meta.get(META_KEY_GLOBAL)? {
                    Some(raw) => {
                        serde_json::from_slice::<SequenceMeta>(&raw).map_err(transaction_data)?
                    }
                    None => SequenceMeta { next_seq: 1 },
                };
                let target_seq = sequence.next_seq;
                if target_seq != planned.target_seq {
                    return Err(transaction_conflict(format!(
                        "target sequence changed after migration preflight: expected {}, got {target_seq}",
                        planned.target_seq
                    )));
                }
                if target_seq == u64::MAX {
                    return Err(transaction_conflict(
                        "target event sequence allocation overflowed u64",
                    ));
                }
                sequence.next_seq += 1;
                let record =
                    EventRecord::from_envelope(planned.translated.clone(), target_seq);
                events.insert(
                    encode_record_key(target_seq).as_bytes(),
                    serde_json::to_vec(&record).map_err(transaction_data)?,
                )?;
                session_index.insert(
                    encode_session_key(&record.session, target_seq).as_bytes(),
                    &[],
                )?;
                if let Some(record_id) = &record.record_id {
                    record_index.insert(record_id.as_bytes(), &target_seq.to_be_bytes())?;
                }
                meta.insert(
                    META_KEY_GLOBAL,
                    serde_json::to_vec(&sequence).map_err(transaction_data)?,
                )?;
                (target_seq, true)
            };

                let mapping = LegacyEventMigrationMapping {
                    source_ledger_id: source_id,
                    source_seq: planned.source.source_seq,
                    target_ledger_id: target_id,
                    target_seq,
                    canonical_envelope_hash: planned.source.envelope_hash.clone(),
                inserted,
            };
            mappings.insert(
                mapping_key.clone(),
                serde_json::to_vec(&mapping).map_err(transaction_data)?,
            )?;
            Ok(mapping)
        })
        .map_err(transaction_error)
}

fn verify_migration(
    source: &LegacyEventMigrationSource,
    target_db: &sled::Db,
    target_id: LedgerIdentity,
    generation: u64,
    complete: bool,
) -> Result<LegacyEventMigrationReport, EventAuthorityError> {
    let plan = preflight_migration(source, target_db, target_id)?;
    let mappings = existing_tree(target_db, TREE_MIGRATION_MAPPINGS)?;
    let events = target_db.open_tree(TREE_EVENTS).map_err(persistence)?;
    let mut mapped = 0_u64;
    let mut inserted = 0_u64;

    for planned in &plan {
        if !planned.already_mapped {
            continue;
        }
        let key = mapping_key(source.ledger_id, planned.source.source_seq);
        let mapping = mapping_get(mappings.as_ref(), &key)?.ok_or_else(|| {
            conflict(format!(
                "preflight lost mapping for source sequence {}",
                planned.source.source_seq
            ))
        })?;
        validate_mapping_contract(&mapping, source, target_id, &planned.source)?;
        let target_raw = events
            .get(encode_record_key(mapping.target_seq).as_bytes())
            .map_err(persistence)?
            .ok_or_else(|| {
                conflict(format!(
                    "migration mapping names missing target sequence {}",
                    mapping.target_seq
                ))
            })?;
        let target_record = decode_source_record(
            &target_raw,
            TREE_EVENTS,
            encode_record_key(mapping.target_seq).as_bytes(),
        )?;
        if target_record.seq != mapping.target_seq || target_record.envelope != planned.translated {
            return Err(conflict(format!(
                "target sequence {} does not preserve source sequence {} envelope semantics",
                mapping.target_seq, planned.source.source_seq
            )));
        }
        mapped += 1;
        inserted += u64::from(mapping.inserted);
    }

    Ok(LegacyEventMigrationReport {
        source_ledger_id: source.ledger_id,
        target_ledger_id: target_id,
        generation,
        source_record_count: source.records.len() as u64,
        mapped_record_count: mapped,
        inserted_target_count: inserted,
        target_tip_seq: target_tip(target_db)?,
        complete,
    })
}

fn validate_mapping_contract(
    mapping: &LegacyEventMigrationMapping,
    source: &LegacyEventMigrationSource,
    target_id: LedgerIdentity,
    record: &SourceRecord,
) -> Result<(), EventAuthorityError> {
    if mapping.source_ledger_id != source.ledger_id
        || mapping.source_seq != record.source_seq
        || mapping.target_ledger_id != target_id
        || mapping.canonical_envelope_hash != record.envelope_hash
        || mapping.target_seq == 0
        || (!mapping.inserted && record.envelope.record_id.is_none())
    {
        return Err(conflict(format!(
            "durable migration mapping conflicts at source sequence {}",
            record.source_seq
        )));
    }
    Ok(())
}

fn translate_source_provenance(
    envelope: &EventEnvelope,
    source_id: LedgerIdentity,
    target_id: LedgerIdentity,
    sequence_plan: &BTreeMap<u64, u64>,
) -> Result<EventEnvelope, EventAuthorityError> {
    let mut translated = envelope.clone();
    for reference in &mut translated.provenance.source_records {
        if reference.ledger_id != source_id {
            return Err(conflict(format!(
                "source provenance names unrelated ledger {}",
                reference.ledger_id
            )));
        }
        let target_seq = sequence_plan.get(&reference.seq).copied().ok_or_else(|| {
            conflict(format!(
                "source provenance sequence {} is not planned",
                reference.seq
            ))
        })?;
        reference.ledger_id = target_id;
        reference.seq = target_seq;
    }
    Ok(translated)
}

fn write_source_marker(
    db: &sled::Db,
    marker: &LegacyEventCutoverMarker,
) -> Result<(), EventAuthorityError> {
    let meta = db.open_tree(TREE_META).map_err(persistence)?;
    let bytes = serde_json::to_vec(marker).map_err(data_failure)?;
    match meta
        .compare_and_swap(
            META_KEY_AUTHORITY_CUTOVER,
            None as Option<&[u8]>,
            Some(bytes.as_slice()),
        )
        .map_err(persistence)?
    {
        Ok(()) => {}
        Err(existing) => {
            let raw = existing.current.ok_or_else(|| {
                conflict("source cutover marker compare-and-swap lost without a winner")
            })?;
            let existing: LegacyEventCutoverMarker =
                serde_json::from_slice(&raw).map_err(data_failure)?;
            if existing != *marker {
                return Err(conflict(
                    "source already carries a different authority cutover marker",
                ));
            }
        }
    }
    db.flush().map_err(persistence)?;
    Ok(())
}

fn validate_marker(
    marker: &LegacyEventCutoverMarker,
    source_id: LedgerIdentity,
    target_id: LedgerIdentity,
    generation: u64,
    source_count: u64,
    snapshot_hash: &str,
) -> Result<(), EventAuthorityError> {
    if marker.schema_version != CUTOVER_SCHEMA_VERSION
        || marker.source_ledger_id != source_id
        || marker.target_ledger_id != target_id
        || marker.generation != generation
        || marker.source_record_count != source_count
        || marker.source_snapshot_hash != snapshot_hash
    {
        return Err(conflict(
            "source cutover marker conflicts with the requested migration",
        ));
    }
    Ok(())
}

fn establish_source_identity(db: &sled::Db) -> Result<LedgerIdentity, EventAuthorityError> {
    let meta = db.open_tree(TREE_META).map_err(persistence)?;
    if let Some(raw) = meta.get(META_KEY_LEDGER_IDENTITY).map_err(persistence)? {
        return LedgerIdentity::decode(&raw).map_err(|error| {
            EventAuthorityError::CorruptPersistedIdentity {
                message: format!("legacy source ledger_identity is not a UUID: {error}"),
            }
        });
    }
    let candidate = LedgerIdentity::new();
    let winner = match meta
        .compare_and_swap(
            META_KEY_LEDGER_IDENTITY,
            None as Option<&[u8]>,
            Some(candidate.encode().as_slice()),
        )
        .map_err(persistence)?
    {
        Ok(()) => candidate.encode().to_vec(),
        Err(conflict) => conflict.current.map(|raw| raw.to_vec()).ok_or_else(|| {
            EventAuthorityError::Internal {
                message: "source identity compare-and-swap lost without a winner".to_string(),
            }
        })?,
    };
    db.flush().map_err(persistence)?;
    LedgerIdentity::decode(&winner).map_err(|error| EventAuthorityError::CorruptPersistedIdentity {
        message: format!("legacy source ledger_identity is not a UUID: {error}"),
    })
}

fn read_marker_from_db(
    db: &sled::Db,
) -> Result<Option<LegacyEventCutoverMarker>, EventAuthorityError> {
    let meta = db.open_tree(TREE_META).map_err(persistence)?;
    meta.get(META_KEY_AUTHORITY_CUTOVER)
        .map_err(persistence)?
        .map(|raw| serde_json::from_slice(&raw).map_err(data_failure))
        .transpose()
}

fn read_retained_from(db: &sled::Db) -> Result<u64, EventAuthorityError> {
    let meta = db.open_tree(TREE_META).map_err(persistence)?;
    match meta.get(META_KEY_RETAINED_FROM).map_err(persistence)? {
        Some(raw) => {
            let retained_from = decode_seq(&raw, "source retained_from")?;
            if retained_from == 0 {
                Err(conflict("source retained_from must be at least one"))
            } else {
                Ok(retained_from)
            }
        }
        None => Ok(1),
    }
}

fn target_tip(db: &sled::Db) -> Result<u64, EventAuthorityError> {
    let events = db.open_tree(TREE_EVENTS).map_err(persistence)?;
    match events.last().map_err(persistence)? {
        Some((key, _)) => decode_canonical_key(&key, TREE_EVENTS),
        None => Ok(0),
    }
}

fn target_next_seq(db: &sled::Db) -> Result<u64, EventAuthorityError> {
    let meta = db.open_tree(TREE_META).map_err(persistence)?;
    match meta.get(META_KEY_GLOBAL).map_err(persistence)? {
        Some(raw) => serde_json::from_slice::<SequenceMeta>(&raw)
            .map(|meta| meta.next_seq)
            .map_err(data_failure),
        None => target_tip(db)?
            .checked_add(1)
            .ok_or_else(|| conflict("target event sequence allocation overflowed u64")),
    }
}

fn existing_tree(db: &sled::Db, name: &str) -> Result<Option<sled::Tree>, EventAuthorityError> {
    if db
        .tree_names()
        .iter()
        .any(|candidate| candidate.as_ref() == name.as_bytes())
    {
        db.open_tree(name).map(Some).map_err(persistence)
    } else {
        Ok(None)
    }
}

fn mapping_get(
    mappings: Option<&sled::Tree>,
    key: &[u8],
) -> Result<Option<LegacyEventMigrationMapping>, EventAuthorityError> {
    let Some(mappings) = mappings else {
        return Ok(None);
    };
    mappings
        .get(key)
        .map_err(persistence)?
        .map(|raw| decode_mapping(&raw, key))
        .transpose()
}

fn decode_source_record(
    raw: &[u8],
    tree: &str,
    key: &[u8],
) -> Result<EventRecord, EventAuthorityError> {
    serde_json::from_slice::<EventRecord>(raw)
        .map(normalize_migration_record)
        .map_err(|error| {
            conflict(format!(
                "malformed row in {tree} at key {:?}: {error}",
                String::from_utf8_lossy(key)
            ))
        })
}

fn normalize_migration_record(mut record: EventRecord) -> EventRecord {
    if record.envelope.ts.is_empty() && record.envelope.recorded_at.is_empty() {
        // Producer `ts` is preserved byte-for-byte. Spine-era rows permitted
        // both timestamps to be empty, so migration supplies only the newer
        // storage timestamp using a frozen value rather than wall-clock time.
        record.envelope.recorded_at = LEGACY_MISSING_TIMESTAMP.to_string();
    } else if record.envelope.recorded_at.is_empty() {
        record.envelope.recorded_at = record.envelope.ts.clone();
    }
    if record.envelope.domain_id.is_empty() {
        record.envelope.domain_id = "telemetry".to_string();
    }
    if record.envelope.stream_id.is_empty() {
        record.envelope.stream_id = record.envelope.session.clone();
    }
    record
}

fn decode_canonical_key(key: &[u8], tree: &str) -> Result<u64, EventAuthorityError> {
    if key.len() != EVENT_KEY_PAD || !key.iter().all(u8::is_ascii_digit) {
        return Err(conflict(format!(
            "{tree} key is not a {EVENT_KEY_PAD}-digit canonical sequence: {:?}",
            String::from_utf8_lossy(key)
        )));
    }
    let seq = std::str::from_utf8(key)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .ok_or_else(|| conflict(format!("{tree} sequence key is invalid")))?;
    if seq == 0 {
        Err(conflict(format!("{tree} sequence zero is invalid")))
    } else {
        Ok(seq)
    }
}

fn decode_legacy_key(key: &[u8]) -> Result<(String, u64), EventAuthorityError> {
    let key =
        std::str::from_utf8(key).map_err(|_| conflict("legacy event key is not valid UTF-8"))?;
    if key.len() <= EVENT_KEY_PAD || key.as_bytes()[key.len() - EVENT_KEY_PAD - 1] != b':' {
        return Err(conflict(format!(
            "legacy event key lacks a fixed-width sequence suffix: {key:?}"
        )));
    }
    let session = &key[..key.len() - EVENT_KEY_PAD - 1];
    let digits = &key[key.len() - EVENT_KEY_PAD..];
    if session.is_empty() || !digits.as_bytes().iter().all(u8::is_ascii_digit) {
        return Err(conflict(format!("legacy event key is malformed: {key:?}")));
    }
    let seq = digits
        .parse::<u64>()
        .map_err(|_| conflict(format!("legacy event key sequence is invalid: {key:?}")))?;
    if seq == 0 {
        return Err(conflict(format!(
            "legacy event key sequence zero is invalid: {key:?}"
        )));
    }
    Ok((session.to_string(), seq))
}

fn canonical_envelope_hash(envelope: &EventEnvelope) -> Result<String, EventAuthorityError> {
    let mut value = serde_json::to_value(envelope).map_err(data_failure)?;
    canonicalize_json(&mut value);
    let bytes = serde_json::to_vec(&value).map_err(data_failure)?;
    Ok(blake3::hash(&bytes).to_hex().to_string())
}

fn canonicalize_json(value: &mut Value) {
    match value {
        Value::Array(values) => values.iter_mut().for_each(canonicalize_json),
        Value::Object(values) => {
            for value in values.values_mut() {
                canonicalize_json(value);
            }
            let sorted = std::mem::take(values)
                .into_iter()
                .collect::<BTreeMap<_, _>>();
            values.extend(sorted);
        }
        _ => {}
    }
}

fn source_snapshot_hash(records: &[SourceRecord]) -> String {
    let mut hasher = blake3::Hasher::new();
    for record in records {
        hasher.update(&record.source_seq.to_be_bytes());
        hasher.update(record.envelope_hash.as_bytes());
    }
    hasher.finalize().to_hex().to_string()
}

fn mapping_key(source_id: LedgerIdentity, source_seq: u64) -> Vec<u8> {
    let mut key = Vec::with_capacity(24);
    key.extend_from_slice(&source_id.encode());
    key.extend_from_slice(&source_seq.to_be_bytes());
    key
}

fn decode_mapping(
    raw: &[u8],
    key: &[u8],
) -> Result<LegacyEventMigrationMapping, EventAuthorityError> {
    serde_json::from_slice(raw).map_err(|error| {
        conflict(format!(
            "malformed migration mapping at key {}: {error}",
            blake3::hash(key).to_hex()
        ))
    })
}

fn encode_record_key(seq: u64) -> String {
    format!("{seq:0EVENT_KEY_PAD$}")
}

fn encode_session_key(session: &str, seq: u64) -> String {
    format!("{session}:{seq:0EVENT_KEY_PAD$}")
}

fn decode_seq(raw: &[u8], label: &str) -> Result<u64, EventAuthorityError> {
    let bytes: [u8; 8] = raw
        .try_into()
        .map_err(|_| conflict(format!("{label} is not an eight-byte sequence")))?;
    Ok(u64::from_be_bytes(bytes))
}

fn decode_seq_transaction(
    raw: &[u8],
    label: &str,
) -> Result<u64, ConflictableTransactionError<EventAuthorityError>> {
    let bytes: [u8; 8] = raw
        .try_into()
        .map_err(|_| transaction_conflict(format!("{label} is not an eight-byte sequence")))?;
    Ok(u64::from_be_bytes(bytes))
}

fn canonicalize_existing(path: &Path, label: &str) -> Result<PathBuf, EventAuthorityError> {
    std::fs::canonicalize(path).map_err(|error| EventAuthorityError::Persistence {
        message: format!("cannot resolve {label} {}: {error}", path.display()),
    })
}

fn open_source_db(path: &Path) -> Result<sled::Db, EventAuthorityError> {
    for attempt in 0..SOURCE_OPEN_RETRY_ATTEMPTS {
        match sled::open(path) {
            Ok(db) => return Ok(db),
            Err(error)
                if error.to_string().contains("could not acquire lock")
                    && attempt + 1 < SOURCE_OPEN_RETRY_ATTEMPTS =>
            {
                std::thread::sleep(SOURCE_OPEN_RETRY_DELAY);
            }
            Err(error) => return Err(persistence(error)),
        }
    }
    unreachable!("bounded source database open loop must return")
}

fn persistence(error: sled::Error) -> EventAuthorityError {
    EventAuthorityError::Persistence {
        message: error.to_string(),
    }
}

fn data_failure(error: serde_json::Error) -> EventAuthorityError {
    EventAuthorityError::MigrationConflict {
        message: format!("migration serialization failed: {error}"),
    }
}

fn conflict(message: impl Into<String>) -> EventAuthorityError {
    EventAuthorityError::MigrationConflict {
        message: message.into(),
    }
}

fn transaction_data(error: serde_json::Error) -> ConflictableTransactionError<EventAuthorityError> {
    ConflictableTransactionError::Abort(data_failure(error))
}

fn transaction_conflict(
    message: impl Into<String>,
) -> ConflictableTransactionError<EventAuthorityError> {
    ConflictableTransactionError::Abort(conflict(message))
}

fn transaction_error(error: TransactionError<EventAuthorityError>) -> EventAuthorityError {
    match error {
        TransactionError::Abort(error) => error,
        TransactionError::Storage(error) => persistence(error),
    }
}
