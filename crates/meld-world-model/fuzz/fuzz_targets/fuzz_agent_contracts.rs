#![no_main]

use libfuzzer_sys::fuzz_target;
use meld_world_model::activation::{
    AgentBootstrapReceipt, AgentCurationRuleRecord, DirectiveRecord,
    LegacyDirectiveMigrationReceipt,
};
use meld_world_model::agent::{
    AgentActivationRecord, AgentBootstrapProgress, AgentBootstrapReport, AgentCurationDecision,
    AgentCurationDedupeKey, AgentCurationRuleConfig, AgentRecord, AgentSubscriptionRecord,
};

fuzz_target!(|data: &[u8]| {
    if let Ok(record) = serde_json::from_slice::<AgentRecord>(data) {
        let _ = record.validate();
    }
    if let Ok(record) = serde_json::from_slice::<AgentSubscriptionRecord>(data) {
        let _ = record.validate();
    }
    if let Ok(record) = serde_json::from_slice::<AgentActivationRecord>(data) {
        let _ = record.validate();
    }
    if let Ok(decision) = serde_json::from_slice::<AgentCurationDecision>(data) {
        let _ = decision.validate();
    }
    if let Ok(key) = serde_json::from_slice::<AgentCurationDedupeKey>(data) {
        let _ = key.validate();
        let _ = key.index_key();
    }
    if let Ok(rule) = serde_json::from_slice::<AgentCurationRuleConfig>(data) {
        let _ = rule.validate();
        let _ = rule.target_condition_key();
    }
    let _ = serde_json::from_slice::<DirectiveRecord>(data);
    let _ = serde_json::from_slice::<AgentCurationRuleRecord>(data);
    let _ = serde_json::from_slice::<LegacyDirectiveMigrationReceipt>(data);
    let _ = serde_json::from_slice::<AgentBootstrapProgress>(data);
    let _ = serde_json::from_slice::<AgentBootstrapReceipt>(data);
    let _ = serde_json::from_slice::<AgentBootstrapReport>(data);
});
