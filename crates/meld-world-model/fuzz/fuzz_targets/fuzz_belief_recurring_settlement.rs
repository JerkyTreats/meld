#![no_main]

use std::collections::BTreeMap;
use std::sync::Arc;

use libfuzzer_sys::fuzz_target;
use meld_world_model::belief::{
    AssessmentLease, BeliefConfigLoader, BeliefEvidenceNormalizer, BeliefRuntime, BeliefStore,
    BranchScope, EvidenceConsumerCursor, EvidenceIngestionReceipt,
    EvidenceIngestionReceiptDisposition, EvidenceIngestionReceiptIdentity,
    EvidenceIngestionReceiptWriteDisposition, EvidenceValue, LeaseStatus, PromotedEvidenceRecord,
};
use meld_world_model::events::{DomainObjectRef, EventRecordRef, LedgerCursor};
use meld_world_model::world_state::graph::store::TraversalStore;
use meld_world_model::PerspectiveKey;

const CONFIG_JSON: &str = r#"{
  "family_id":"fuzz_family",
  "dimension_id":"fuzz_dimension",
  "predicate_id":"confidence",
  "evidence_policy_id":"fuzz_policy",
  "evidence_schemas":[{"schema_id":"signal","required":false,"role":"Support","reliability":1.0,"precision":1.0}],
  "source_mappings":[{"mapping_id":"signal_mapping","source_kind":"signal","evidence_schema_id":"signal","subject_from":"record.subject","value_field":"value","factor_id":"signal"}],
  "comparator":{"engine_id":"weighted_bayesian","engine_version":"1","factors":[{"factor_id":"signal","evidence_schema_id":"signal","weight":1.0,"polarity":"Supports"}],"missing_evidence_uncertainty":0.9},
  "default_prior":0.5,
  "planner_projection":{"confidence_field":"confidence","threshold":0.5,"posterior_meaning":"probability"},
  "config_version":"1"
}"#;

type RevisionSignature = Vec<(String, Vec<String>, u64, u64)>;

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }
    match data[0] % 4 {
        0 => exercise_page_equivalence(data),
        1 => exercise_receipt_authority(data),
        2 => exercise_active_lease_conflict(),
        _ => exercise_indeterminate_acquire_reopen(),
    }
});

fn exercise_page_equivalence(data: &[u8]) {
    let count = usize::from(data[0] % 6) + 1;
    let page_limit = usize::from(data.get(1).copied().unwrap_or(0) % 4) + 1;
    let reopen_after = usize::from(data.get(2).copied().unwrap_or(0)) % count;
    let mut sequences = Vec::with_capacity(count);
    for index in 0..count {
        let byte = data.get(index + 3).copied().unwrap_or(index as u8);
        sequences.push(u64::from(byte) + 1);
    }

    let narrow = settle(&sequences, page_limit, Some(reopen_after));
    let wide = settle(&sequences, 1024, None);
    assert_eq!(narrow, wide);
}

fn exercise_receipt_authority(data: &[u8]) {
    let temp = tempfile::tempdir().expect("temporary receipt directory");
    let path = temp.path().join("belief");
    let store = BeliefStore::new(sled::open(&path).expect("open receipt store"))
        .expect("construct receipt store");
    let ledger_id = "00000000-0000-0000-0000-000000000123"
        .parse()
        .expect("valid ledger identity");
    let receipt_count = u64::from(data.get(3).copied().unwrap_or(0) % 4) + 1;
    let receipt_pair = |sequence| {
        let cursor = EvidenceConsumerCursor {
            consumer_id: "fuzz-consumer".to_string(),
            ledger_cursor: LedgerCursor {
                ledger_id,
                after_seq: sequence,
            },
            family_config_hash: "fuzz-family".to_string(),
            source_mapping_hash: "fuzz-mapping".to_string(),
            perspective: PerspectiveKey::new("agent", "fuzz").expect("valid perspective"),
            branch_scope: BranchScope::main(),
        };
        let receipt = EvidenceIngestionReceipt {
            identity: EvidenceIngestionReceiptIdentity {
                consumer_id: cursor.consumer_id.clone(),
                source_record: EventRecordRef {
                    ledger_id,
                    seq: sequence,
                },
                family_config_hash: cursor.family_config_hash.clone(),
                source_mapping_hash: cursor.source_mapping_hash.clone(),
                perspective: cursor.perspective.clone(),
                branch_scope: cursor.branch_scope.clone(),
            },
            disposition: EvidenceIngestionReceiptDisposition::Irrelevant,
            evidence_ids: Vec::new(),
        };
        (cursor, receipt)
    };
    let (cursor, receipt) = receipt_pair(1);
    if data.get(1).copied().unwrap_or(0) & 1 == 1 {
        store.fail_flush_after_for_test(1);
        assert!(store
            .record_evidence_receipt_and_advance(None, &receipt, &cursor)
            .is_err());
    } else {
        assert_eq!(
            store
                .record_evidence_receipt_and_advance(None, &receipt, &cursor)
                .expect("insert receipt"),
            EvidenceIngestionReceiptWriteDisposition::Inserted
        );
    }

    let mut forged_next = cursor.clone();
    forged_next.ledger_cursor.after_seq = u64::from(data.get(2).copied().unwrap_or(1)) + 2;
    assert!(store
        .record_evidence_receipt_and_advance(None, &receipt, &forged_next)
        .is_err());
    assert_eq!(
        store
            .assessment_source_high_water_for_test()
            .expect("read source authority"),
        1
    );
    assert_eq!(
        store
            .record_evidence_receipt_and_advance(None, &receipt, &cursor)
            .expect("exact receipt retry"),
        EvidenceIngestionReceiptWriteDisposition::ExactReplay
    );
    let mut final_cursor = cursor.clone();
    for sequence in 2..=receipt_count {
        let (next, next_receipt) = receipt_pair(sequence);
        store
            .record_evidence_receipt_and_advance(Some(&final_cursor), &next_receipt, &next)
            .expect("advance contiguous receipt history");
        final_cursor = next;
    }
    store.flush().expect("flush exact receipt retry");
    drop(store);

    if receipt_count >= 3 && data.get(4).copied().unwrap_or(0) & 1 == 1 {
        let db = reopen_sled(&path);
        let receipts = db
            .open_tree("belief_evidence_ingestion_receipts")
            .expect("open receipt authority tree");
        let (_, middle) = receipt_pair(2);
        receipts
            .remove(serde_json::to_vec(&middle.identity).expect("encode middle receipt identity"))
            .expect("remove middle receipt");
        db.flush().expect("flush middle receipt loss");
        drop(receipts);
        drop(db);
        assert!(BeliefStore::new(reopen_sled(&path)).is_err());
        return;
    }

    let reopened = BeliefStore::new(reopen_sled(&path)).expect("reopen receipt store");
    assert_eq!(
        reopened
            .evidence_consumer_cursor(&final_cursor)
            .expect("read receipt cursor"),
        Some(final_cursor)
    );
    assert_eq!(
        reopened
            .assessment_source_high_water_for_test()
            .expect("read reopened source authority"),
        receipt_count
    );
}

fn exercise_active_lease_conflict() {
    let temp = tempfile::tempdir().expect("temporary active lease directory");
    let path = temp.path().join("belief");
    let config = BeliefConfigLoader::load_json(CONFIG_JSON).expect("valid fuzz config");
    let perspective = PerspectiveKey::new("agent", "fuzz").expect("valid perspective");
    let branch_scope = BranchScope::main();
    let db = sled::open(&path).expect("open active lease store");
    let store = Arc::new(BeliefStore::new(db.clone()).expect("construct active lease store"));
    let traversal = Arc::new(TraversalStore::new(db).expect("open active lease traversal"));
    let runtime = BeliefRuntime::new(
        Arc::clone(&store),
        traversal,
        config.clone(),
        perspective.clone(),
        branch_scope.clone(),
    );
    let item = normalized_item(&config, &perspective, &branch_scope, "active", 1);
    let normalizer =
        BeliefEvidenceNormalizer::new(config.config.clone(), perspective, branch_scope);
    store
        .put_evidence_once(&item)
        .expect("persist active evidence");
    store
        .put_assignment_once(&normalizer.assign(&item).expect("assign active evidence"))
        .expect("persist active assignment");
    assert_eq!(
        store
            .assessment_source_high_water_for_test()
            .expect("read unacknowledged source authority"),
        0
    );
    let dirty = store
        .dirty_state(&item.candidate_key)
        .expect("read active dirty state")
        .expect("active dirty state");
    let blocking = AssessmentLease {
        lease_id: "fuzz-active-blocker".to_string(),
        belief_key: item.candidate_key.clone(),
        epoch: dirty.mutation_generation,
        owner_id: "blocking-owner".to_string(),
        input_cursor_start: 1,
        input_cursor_end: 1,
        assignment_cursor_start: None,
        assignment_cursor_end: None,
        assignment_window_complete: true,
        started_at_seq: 0,
        expires_at_seq: 2,
        comparator_engine_id: config.config.comparator.engine_id.clone(),
        config_snapshot_hash: config.hash.clone(),
        status: LeaseStatus::Queued,
    };
    store
        .acquire_lease(blocking)
        .expect("acquire active blocker");

    assert!(runtime
        .assess_dirty_key(&item.candidate_key, "settlement-owner")
        .is_err());
    assert_eq!(
        store
            .assessment_source_high_water_for_test()
            .expect("read blocked source authority"),
        0
    );
    assert!(store
        .current_revision(&item.candidate_key)
        .expect("read blocked revision")
        .is_none());
    assert!(runtime
        .assess_dirty_key(&item.candidate_key, "settlement-owner")
        .expect("settle after expiry")
        .is_some());
    assert_eq!(
        store
            .get_lease("fuzz-active-blocker")
            .expect("read blocking lease")
            .expect("blocking lease remains")
            .status,
        LeaseStatus::Abandoned
    );
    store.flush().expect("flush active lease settlement");
    drop(runtime);
    drop(store);
    let reopened = BeliefStore::new(reopen_sled(&path)).expect("reopen active lease store");
    assert!(reopened
        .current_revision(&item.candidate_key)
        .expect("read reopened active revision")
        .is_some());
}

fn exercise_indeterminate_acquire_reopen() {
    let temp = tempfile::tempdir().expect("temporary indeterminate lease directory");
    let path = temp.path().join("belief");
    let config = BeliefConfigLoader::load_json(CONFIG_JSON).expect("valid fuzz config");
    let perspective = PerspectiveKey::new("agent", "fuzz").expect("valid perspective");
    let branch_scope = BranchScope::main();
    let item;
    let active_lease_id;
    {
        let db = sled::open(&path).expect("open indeterminate store");
        let store = Arc::new(BeliefStore::new(db.clone()).expect("construct indeterminate store"));
        let traversal = Arc::new(TraversalStore::new(db).expect("open indeterminate traversal"));
        let runtime = BeliefRuntime::new(
            Arc::clone(&store),
            traversal,
            config.clone(),
            perspective.clone(),
            branch_scope.clone(),
        );
        item = normalized_item(&config, &perspective, &branch_scope, "indeterminate", 1);
        let normalizer = BeliefEvidenceNormalizer::new(
            config.config.clone(),
            perspective.clone(),
            branch_scope.clone(),
        );
        store
            .put_evidence_once(&item)
            .expect("persist indeterminate evidence");
        store
            .put_assignment_once(
                &normalizer
                    .assign(&item)
                    .expect("assign indeterminate evidence"),
            )
            .expect("persist indeterminate assignment");
        store.fail_flush_after_for_test(2);
        assert!(runtime
            .assess_dirty_key(&item.candidate_key, "indeterminate-owner")
            .is_err());
        active_lease_id = store
            .dirty_state(&item.candidate_key)
            .expect("read indeterminate dirty state")
            .expect("indeterminate dirty state remains")
            .active_lease_id
            .expect("indeterminate active lease remains");
        store
            .flush()
            .expect("resolve indeterminate persisted branch");
    }

    let db = reopen_sled(&path);
    let store = Arc::new(BeliefStore::new(db.clone()).expect("reopen indeterminate store"));
    let traversal = Arc::new(TraversalStore::new(db).expect("reopen indeterminate traversal"));
    let runtime = BeliefRuntime::new(
        Arc::clone(&store),
        traversal,
        config,
        perspective,
        branch_scope,
    );
    assert!(runtime
        .assess_dirty_key(&item.candidate_key, "indeterminate-owner")
        .expect("resume indeterminate lease")
        .is_some());
    assert_eq!(
        store
            .get_lease(&active_lease_id)
            .expect("read resumed lease")
            .expect("resumed lease remains")
            .status,
        LeaseStatus::Completed
    );
    assert!(store
        .current_revision(&item.candidate_key)
        .expect("read resumed revision")
        .is_some());
}

fn normalized_item(
    config: &meld_world_model::belief::ConfigSnapshot,
    perspective: &PerspectiveKey,
    branch_scope: &BranchScope,
    source_id: &str,
    sequence: u64,
) -> meld_world_model::belief::EvidenceItem {
    let normalizer = BeliefEvidenceNormalizer::new(
        config.config.clone(),
        perspective.clone(),
        branch_scope.clone(),
    );
    let mut fields = BTreeMap::new();
    fields.insert("value".to_string(), EvidenceValue::Scalar(0.25));
    normalizer
        .normalize_promoted(&PromotedEvidenceRecord {
            source_kind: "signal".to_string(),
            source_id: source_id.to_string(),
            subject: DomainObjectRef::new("workspace_fs", "node", "subject")
                .expect("valid subject"),
            source_fact_ids: vec![format!("fact-{source_id}")],
            graph_anchor_ids: Vec::new(),
            objects: Vec::new(),
            relations: Vec::new(),
            source_cursor_start: sequence,
            source_cursor_end: sequence,
            reference_time: None,
            transaction_seq: sequence,
            content_hash: None,
            fields,
        })
        .expect("normalize fuzz item")
        .remove(0)
}

fn settle(sequences: &[u64], page_limit: usize, reopen_after: Option<usize>) -> RevisionSignature {
    let temp = tempfile::tempdir().expect("temporary belief directory");
    let path = temp.path().join("belief");
    let config = BeliefConfigLoader::load_json(CONFIG_JSON).expect("valid fuzz config");
    let perspective = PerspectiveKey::new("agent", "fuzz").expect("valid perspective");
    let branch_scope = BranchScope::main();
    let normalizer = BeliefEvidenceNormalizer::new(
        config.config.clone(),
        perspective.clone(),
        branch_scope.clone(),
    );
    let db = sled::open(&path).expect("open belief database");
    let mut store = Arc::new(BeliefStore::new(db.clone()).expect("open belief store"));
    let mut traversal = Arc::new(TraversalStore::new(db).expect("open traversal store"));
    let mut runtime = BeliefRuntime::new(
        Arc::clone(&store),
        Arc::clone(&traversal),
        config.clone(),
        perspective.clone(),
        branch_scope.clone(),
    );
    let mut key = None;
    for (index, sequence) in sequences.iter().copied().enumerate() {
        let mut fields = BTreeMap::new();
        fields.insert(
            "value".to_string(),
            EvidenceValue::Scalar(f64::from(index as u32) / sequences.len() as f64),
        );
        let record = PromotedEvidenceRecord {
            source_kind: "signal".to_string(),
            source_id: format!("source-{index}"),
            subject: DomainObjectRef::new("workspace_fs", "node", "subject")
                .expect("valid subject"),
            source_fact_ids: vec![format!("fact-{index}")],
            graph_anchor_ids: Vec::new(),
            objects: Vec::new(),
            relations: Vec::new(),
            source_cursor_start: sequence,
            source_cursor_end: sequence,
            reference_time: None,
            transaction_seq: sequence,
            content_hash: None,
            fields,
        };
        let item = normalizer
            .normalize_promoted(&record)
            .expect("normalize fuzz record")
            .remove(0);
        key = Some(item.candidate_key.clone());
        store.put_evidence_once(&item).expect("persist evidence");
        store
            .put_assignment_once(&normalizer.assign(&item).expect("assign evidence"))
            .expect("persist assignment");
    }
    let key = key.expect("at least one assignment");

    for iteration in 0..sequences.len() + 2 {
        if store.dirty_state(&key).expect("read dirty state").is_none() {
            break;
        }
        runtime
            .assess_dirty_key_bounded(&key, "fuzz-owner", page_limit)
            .expect("settle bounded evidence");
        if reopen_after == Some(iteration) {
            store.flush().expect("flush before reopen");
            drop(runtime);
            drop(store);
            drop(traversal);
            let db = reopen_sled(&path);
            store = Arc::new(BeliefStore::new(db.clone()).expect("reopen belief store"));
            traversal = Arc::new(TraversalStore::new(db).expect("reopen traversal store"));
            runtime = BeliefRuntime::new(
                Arc::clone(&store),
                Arc::clone(&traversal),
                config.clone(),
                perspective.clone(),
                branch_scope.clone(),
            );
        }
    }
    assert!(store
        .dirty_state(&key)
        .expect("read final dirty state")
        .is_none());

    let mut revisions = Vec::new();
    let mut current = store.current_revision(&key).expect("read current revision");
    while let Some(revision) = current {
        let prior = revision.prior_revision_id.clone();
        revisions.push((
            revision.revision_id,
            revision.evidence_ids,
            revision.source_cursor_end,
            revision.posterior.probability.to_bits(),
        ));
        current = prior
            .as_deref()
            .map(|revision_id| {
                store
                    .get_revision(revision_id)
                    .expect("read prior revision")
            })
            .unwrap_or(None);
    }
    revisions.reverse();
    revisions
}

fn reopen_sled(path: &std::path::Path) -> sled::Db {
    for _ in 0..1000 {
        match sled::open(path) {
            Ok(db) => return db,
            Err(error) if error.to_string().contains("could not acquire lock") => {
                std::thread::yield_now();
            }
            Err(error) => panic!("reopen belief database: {error}"),
        }
    }
    panic!("reopen belief database after lock release");
}
