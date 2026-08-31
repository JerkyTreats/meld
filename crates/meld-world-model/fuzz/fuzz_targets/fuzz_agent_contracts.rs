#![no_main]

use libfuzzer_sys::fuzz_target;
use meld_world_model::agent::{
    AgentActivationRecord, AgentConsumerReceipt, AgentCurationRuleConfig,
    AgentMilestoneAcceptance, AgentPlanJudgment, AgentProductAuthorization, AgentProductProgress,
    AgentReconciliationGoal, AgentRecord, AgentSubscriptionRecord,
};
use meld_world_model::StrategyPlan;

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
    if let Ok(rule) = serde_json::from_slice::<AgentCurationRuleConfig>(data) {
        let _ = rule.validate();
        let _ = rule.target_condition_key();
    }
    if let Ok(record) = serde_json::from_slice::<AgentReconciliationGoal>(data) {
        let _ = serde_json::to_vec(&record);
    }
    if let Ok(record) = serde_json::from_slice::<StrategyPlan>(data) {
        let _ = serde_json::to_vec(&record);
    }
    if let Ok(record) = serde_json::from_slice::<AgentPlanJudgment>(data) {
        let _ = serde_json::to_vec(&record);
    }
    if let Ok(record) = serde_json::from_slice::<AgentProductAuthorization>(data) {
        let _ = serde_json::to_vec(&record);
    }
    if let Ok(record) = serde_json::from_slice::<AgentProductProgress>(data) {
        let _ = serde_json::to_vec(&record);
    }
    if let Ok(record) = serde_json::from_slice::<AgentConsumerReceipt>(data) {
        let _ = serde_json::to_vec(&record);
    }
    if let Ok(record) = serde_json::from_slice::<AgentMilestoneAcceptance>(data) {
        let _ = serde_json::to_vec(&record);
    }
});
