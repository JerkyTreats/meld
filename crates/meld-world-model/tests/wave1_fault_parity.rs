use meld_world_model::belief::{
    BeliefAuthorityMigrationIdentity, BeliefAuthorityMigrationProgress, BeliefStore, BranchScope,
    EvidenceConsumerCursor, EvidenceIngestionReceipt, EvidenceIngestionReceiptDisposition,
    EvidenceIngestionReceiptIdentity, EvidenceIngestionReceiptWriteDisposition,
    LegacyBeliefCompatibilityPosture,
};
use meld_world_model::events::{EventRecordRef, LedgerCursor};
use meld_world_model::PerspectiveKey;

fn migration_identity() -> BeliefAuthorityMigrationIdentity {
    BeliefAuthorityMigrationIdentity::try_new(
        "migration-wave1",
        "legacy-belief",
        "product-belief",
        1,
    )
    .unwrap()
}

fn cursor(ledger_id: meld_world_model::events::LedgerIdentity, seq: u64) -> EvidenceConsumerCursor {
    EvidenceConsumerCursor {
        consumer_id: "belief.evidence.wave1".to_string(),
        ledger_cursor: LedgerCursor {
            ledger_id,
            after_seq: seq,
        },
        family_config_hash: "family-wave1".to_string(),
        source_mapping_hash: "mapping-wave1".to_string(),
        perspective: PerspectiveKey::new("agent", "default").unwrap(),
        branch_scope: BranchScope::main(),
    }
}

fn receipt(cursor: &EvidenceConsumerCursor) -> EvidenceIngestionReceipt {
    EvidenceIngestionReceipt {
        identity: EvidenceIngestionReceiptIdentity {
            consumer_id: cursor.consumer_id.clone(),
            source_record: EventRecordRef {
                ledger_id: cursor.ledger_cursor.ledger_id,
                seq: cursor.ledger_cursor.after_seq,
            },
            family_config_hash: cursor.family_config_hash.clone(),
            source_mapping_hash: cursor.source_mapping_hash.clone(),
            perspective: cursor.perspective.clone(),
            branch_scope: cursor.branch_scope.clone(),
        },
        disposition: EvidenceIngestionReceiptDisposition::Irrelevant,
        evidence_ids: Vec::new(),
    }
}

#[test]
fn migration_parity_preserves_gapless_receipts_and_fences_the_source() {
    let legacy_temp = tempfile::tempdir().unwrap();
    let product_temp = tempfile::tempdir().unwrap();
    let legacy_path = legacy_temp.path().join("belief");
    let product_path = product_temp.path().join("belief");
    let legacy_db = sled::open(&legacy_path).unwrap();
    let product_db = sled::open(&product_path).unwrap();
    let ledger_id = "00000000-0000-0000-0000-000000000123".parse().unwrap();
    let first = cursor(ledger_id, 1);
    let first_receipt = receipt(&first);
    let gap = cursor(ledger_id, 3);
    let gap_receipt = receipt(&gap);
    let second = cursor(ledger_id, 2);
    let second_receipt = receipt(&second);

    let legacy = BeliefStore::new(legacy_db.clone()).unwrap();
    legacy.put_runtime_meta("legacy-seq", "1").unwrap();
    assert_eq!(
        legacy
            .record_evidence_receipt_and_advance(None, &first_receipt, &first)
            .unwrap(),
        EvidenceIngestionReceiptWriteDisposition::Inserted
    );
    assert!(legacy
        .record_evidence_receipt_and_advance(Some(&first), &gap_receipt, &gap)
        .is_err());
    assert_eq!(
        legacy
            .record_evidence_receipt_and_advance(None, &first_receipt, &first)
            .unwrap(),
        EvidenceIngestionReceiptWriteDisposition::ExactReplay
    );

    let product = BeliefStore::new(product_db.clone()).unwrap();
    assert_eq!(
        product
            .migrate_legacy_authority(&legacy, migration_identity())
            .unwrap(),
        LegacyBeliefCompatibilityPosture::ProductAuthoritative
    );
    let marker = product.authority_migration_marker().unwrap().unwrap();
    let parity = match marker.progress() {
        BeliefAuthorityMigrationProgress::Cutover { parity } => parity,
        other => panic!("expected cutover marker, got {other:?}"),
    };
    assert_eq!(parity.source(), parity.target());
    assert_eq!(product.authority_snapshot().unwrap(), *parity.target());
    assert!(legacy.put_runtime_meta("legacy-seq", "late").is_err());
    assert!(product
        .evidence_ingestion_receipt(&gap_receipt)
        .unwrap()
        .is_none());
    assert_eq!(
        product
            .record_evidence_receipt_and_advance(None, &first_receipt, &first)
            .unwrap(),
        EvidenceIngestionReceiptWriteDisposition::ExactReplay
    );
    assert_eq!(
        product
            .record_evidence_receipt_and_advance(Some(&first), &second_receipt, &second)
            .unwrap(),
        EvidenceIngestionReceiptWriteDisposition::Inserted
    );
    drop(legacy);
    drop(product);

    for _ in 0..4 {
        let legacy = BeliefStore::new(legacy_db.clone()).unwrap();
        assert!(legacy.put_runtime_meta("legacy-seq", "late").is_err());
        drop(legacy);
        let product = BeliefStore::new(product_db.clone()).unwrap();
        assert_eq!(
            product
                .record_evidence_receipt_and_advance(None, &second_receipt, &second)
                .unwrap(),
            EvidenceIngestionReceiptWriteDisposition::ExactReplay
        );
        assert_eq!(
            product.evidence_consumer_cursor(&second).unwrap(),
            Some(second.clone())
        );
        assert!(product
            .record_evidence_receipt_and_advance(Some(&first), &gap_receipt, &gap)
            .is_err());
    }
}
