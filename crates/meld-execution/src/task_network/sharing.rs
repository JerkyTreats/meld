//! Execution-owned compatibility and attribution for shared operational work.

use serde::{Deserialize, Serialize};

use super::{
    contracts::{stable_hash, stable_id},
    state::{NetworkState, TaskInitSource, TaskNode, TaskStatus},
};
use crate::capability::CapabilityTypeContract;

pub use crate::capability::EXACT_INPUT_SHARING_V1;

/// Durable compatibility decision. The intact Capability contract is checked
/// against both admitted Tasks, so replay never substitutes a current catalog head.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SharedActionDecision {
    /// Identity of the exact compatibility decision.
    pub decision_id: String,
    /// Existing pending operational node receiving another admission.
    pub shared_node_id: String,
    /// Owner contract selected by both admitted Tasks.
    pub capability_contract: CapabilityTypeContract,
}

impl SharedActionDecision {
    pub(crate) fn decide(
        state: &NetworkState,
        candidate: &TaskNode,
        primary: &TaskNode,
        contract: &CapabilityTypeContract,
    ) -> Result<Self, String> {
        contract.validate().map_err(|error| error.to_string())?;
        if contract.execution_contract.completion_semantics != EXACT_INPUT_SHARING_V1
            || contract.effect_contract.iter().any(|effect| {
                effect.kind != crate::capability::EffectKind::Emit || effect.exclusive
            })
            || candidate.init_sources.is_empty()
            || candidate
                .init_sources
                .iter()
                .any(|source| !matches!(source, TaskInitSource::StaticSeed(_)))
            || candidate.init_sources != primary.init_sources
            || candidate.lifecycle_epoch != primary.lifecycle_epoch
            || state.statuses.get(&primary.task_instance_id) != Some(&TaskStatus::Pending)
            || state
                .edges
                .iter()
                .any(|edge| edge.to == primary.task_instance_id)
        {
            return Err(
                "shared work requires a pending exact-input artifact-only invocation".into(),
            );
        }
        let left = primary
            .lineage
            .admission
            .as_ref()
            .ok_or("shared work requires primary admission")?;
        let right = candidate
            .lineage
            .admission
            .as_ref()
            .ok_or("shared work requires contributing admission")?;
        if left.admission_id == right.admission_id
            || left.agent_id != right.agent_id
            || left.authority_scope_id != right.authority_scope_id
            || left.authority_policy_content_hash != right.authority_policy_content_hash
            || left.activation_generation != right.activation_generation
            || left.admission_epoch != right.admission_epoch
            || primary.lineage.authority_decision != candidate.lineage.authority_decision
            || primary.lineage.authority_decision.is_none()
        {
            return Err(
                "shared work requires independent admissions under compatible authority".into(),
            );
        }
        super::state::validate_task_admission_attribution(state, primary)?;
        super::state::validate_task_admission_attribution_for_lowering(state, candidate)?;
        let original = &state.admissions[&left.admission_id].request.task;
        let incoming = &state.admissions[&right.admission_id].request.task;
        let identity = contract.content_identity();
        if !original.capability_contract_ids.contains(&identity)
            || !incoming.capability_contract_ids.contains(&identity)
            || original.execution_subject != incoming.execution_subject
            || original.expected_outcome_contract_id != incoming.expected_outcome_contract_id
            || primary.compiled_task.task_version != candidate.compiled_task.task_version
            || primary.compiled_task.init_slots != candidate.compiled_task.init_slots
            || !primary.compiled_task.dependency_edges.is_empty()
            || !candidate.compiled_task.dependency_edges.is_empty()
        {
            return Err(
                "shared work differs in admitted contract, subject, or result shape".into(),
            );
        }
        let [left_capability] = primary.compiled_task.capability_instances.as_slice() else {
            return Err("shared operational node must contain one Capability".into());
        };
        let [right_capability] = candidate.compiled_task.capability_instances.as_slice() else {
            return Err("shared operational node must contain one Capability".into());
        };
        let mut comparable = right_capability.clone();
        comparable.capability_instance_id = left_capability.capability_instance_id.clone();
        if &comparable != left_capability
            || comparable.capability_type_id != contract.capability_type_id
            || comparable.capability_version != contract.capability_version
        {
            return Err("shared work differs in executable binding or input wiring".into());
        }
        Ok(Self {
            decision_id: stable_id(
                "execution-work-compatibility-v1",
                &(
                    state.network_id.as_str(),
                    stable_hash(primary),
                    stable_hash(candidate),
                    &identity,
                ),
            ),
            shared_node_id: primary.task_instance_id.clone(),
            capability_contract: contract.clone(),
        })
    }

    pub(crate) fn validate(
        &self,
        state: &NetworkState,
        candidate: &TaskNode,
    ) -> Result<(), String> {
        let primary = state
            .tasks
            .get(&self.shared_node_id)
            .ok_or("shared operational node is absent")?;
        if &Self::decide(state, candidate, primary, &self.capability_contract)? != self {
            return Err("shared compatibility decision does not match its exact inputs".into());
        }
        Ok(())
    }
}

pub(crate) fn decision_ids_for_node(state: &NetworkState, node_id: &str) -> Vec<String> {
    let mut ids: Vec<_> = state
        .shared_steps
        .values()
        .filter_map(|step| {
            let decision = step.sharing.as_ref()?;
            (decision.shared_node_id == node_id).then(|| decision.decision_id.clone())
        })
        .collect();
    ids.sort();
    ids
}

/// Separate Execution discharge evidence for one independently authorized Task.
/// A shared outcome never replaces the admission's own authority or attribution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdmissionDischargeAccount {
    /// Stable identity of this admission's discharge proof.
    pub account_id: String,
    /// Exact producer and consumer attribution being discharged.
    pub admission: super::state::TaskAdmissionAttribution,
    /// Native terminal outcome for the complete admitted region.
    pub outcome_id: String,
    /// Compatibility decisions contributing shared work to this admission.
    pub shared_action_decision_ids: Vec<String>,
}

/// Read a completed region's admission-specific account from canonical owner state.
pub fn admission_discharge_account(
    state: &NetworkState,
    admission_id: &str,
) -> Option<AdmissionDischargeAccount> {
    let record = state.admissions.get(admission_id)?;
    if record.decision != crate::task_admission::TaskAdmissionDecision::Admitted {
        return None;
    }
    let outcome_id = super::state::admission_region_terminal_outcome_id(state, admission_id)?;
    let mut decisions = Vec::new();
    for step in state.shared_steps.values() {
        let attribution = step.task_node.lineage.admission.as_ref()?;
        let decision = step.sharing.as_ref()?;
        let primary = state.tasks.get(&decision.shared_node_id)?;
        let primary_admission = primary.lineage.admission.as_ref()?;
        if attribution.admission_id != admission_id
            && primary_admission.admission_id != admission_id
        {
            continue;
        }
        let status = state.statuses.get(&decision.shared_node_id)?;
        let shared_outcome_id = match status {
            TaskStatus::Succeeded { outcome_id } | TaskStatus::Failed { outcome_id, .. } => {
                outcome_id
            }
            _ => return None,
        };
        let outcome = state.outcomes.get(shared_outcome_id)?;
        let claim = state.claims.get(&outcome.claim_id)?;
        if claim.task_instance_id != decision.shared_node_id
            || !claim
                .shared_action_decision_ids
                .contains(&decision.decision_id)
        {
            return None;
        }
        decisions.push(decision.decision_id.clone());
    }
    decisions.sort();
    let attribution = super::state::TaskAdmissionAttribution::from_record(record);
    Some(AdmissionDischargeAccount {
        account_id: stable_id(
            "execution-admission-discharge-v1",
            &(
                state.network_id.as_str(),
                &attribution,
                outcome_id,
                &decisions,
            ),
        ),
        admission: attribution,
        outcome_id: outcome_id.to_string(),
        shared_action_decision_ids: decisions,
    })
}

/// Accounts first established by the current transition travel with its publication,
/// even when a previously failed shared producer supplies the terminal outcome.
pub(crate) fn unpublished_shared_discharge_accounts(
    state: &NetworkState,
) -> Vec<AdmissionDischargeAccount> {
    let published_accounts: std::collections::BTreeSet<_> = state
        .publications
        .values()
        .flat_map(|publication| &publication.shared_discharge_accounts)
        .map(|account| account.account_id.as_str())
        .collect();
    state
        .admissions
        .keys()
        .filter_map(|id| admission_discharge_account(state, id))
        .filter(|account| {
            !account.shared_action_decision_ids.is_empty()
                && !published_accounts.contains(account.account_id.as_str())
        })
        .collect()
}
