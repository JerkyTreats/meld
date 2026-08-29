#![no_main]

use libfuzzer_sys::fuzz_target;
use meld_world_model::{
    CurationAcceptanceRecord, CurationOperation, CurationPublicationKind,
    CurationPublicationReceipt, CurationResult, CurationStore, StandingCurationRule,
    StandingCurationRuleRevision,
};

fuzz_target!(|data: &[u8]| {
    if let Ok(rule) = serde_json::from_slice::<StandingCurationRule>(data) {
        let _ = rule.validate();
    }
    if let Ok(revision) = serde_json::from_slice::<StandingCurationRuleRevision>(data) {
        let _ = revision.validate();
    }
    if let Ok(operation) = serde_json::from_slice::<CurationOperation>(data) {
        let _ = operation.validate();
    }
    if let Ok((operation, result)) =
        serde_json::from_slice::<(CurationOperation, CurationResult)>(data)
    {
        let _ = result.validate(&operation);
    }

    let Ok((rule, operation, acceptance, result, receipts)) = serde_json::from_slice::<(
        StandingCurationRule,
        CurationOperation,
        CurationAcceptanceRecord,
        CurationResult,
        Vec<CurationPublicationReceipt>,
    )>(data)
    else {
        return;
    };

    let temp = tempfile::tempdir().expect("tempdir");
    let store_path = temp.path().join("curation.sled");
    {
        let store = CurationStore::new(sled::open(&store_path).expect("sled")).expect("store");
        let _ = store.install_rule(rule, 1);
        if store.put_operation(&operation).is_ok() && store.put_acceptance(&acceptance).is_ok() {
            let _ = store.put_result(&result);
            for receipt in &receipts {
                let _ = store.put_publication_receipt(receipt);
            }
        }
        let _ = store.flush();
    }

    let reopened =
        CurationStore::new(sled::open(&store_path).expect("reopen sled")).expect("reopen store");
    let _ = reopened.operation_for_selection(&operation.selection_id);
    let _ = reopened.acceptance(&acceptance.acceptance_id);
    let _ = reopened.result_for_operation(&operation.operation_id);
    for receipt in receipts {
        let _ = reopened.publication_receipt(&receipt.result_id, receipt.kind);
        if receipt.kind == CurationPublicationKind::Terminal {
            let _ = reopened.result_for_operation(&operation.operation_id);
        }
    }
});
