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
    // Compatibility reader for admission-time sharing written before the ready-work
    // command. New admission writes reject this shape; the retained journal proof
    // is covered by legacy_admission_sharing_reopens_and_freezes_current_claim.
    fn legacy_admission_decision(
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
        let decision = Self::compatible_contract(state, candidate, primary, contract)?;
        let primary_admission = primary
            .lineage
            .admission
            .as_ref()
            .ok_or("primary admission missing")?;
        let candidate_admission = candidate
            .lineage
            .admission
            .as_ref()
            .ok_or("contributing admission missing")?;
        if primary_admission.agent_id != candidate_admission.agent_id
            || primary.lineage.authority_decision != candidate.lineage.authority_decision
            || state.admissions[&primary_admission.admission_id]
                .request
                .task
                .expected_outcome_contract_id
                != state.admissions[&candidate_admission.admission_id]
                    .request
                    .task
                    .expected_outcome_contract_id
        {
            return Err(
                "historical admission sharing requires equal complete Task grants and endpoints"
                    .into(),
            );
        }
        Ok(decision)
    }

    fn compatible_contract(
        state: &NetworkState,
        candidate: &TaskNode,
        primary: &TaskNode,
        contract: &CapabilityTypeContract,
    ) -> Result<Self, String> {
        contract.validate().map_err(|error| error.to_string())?;
        if contract.execution_contract.completion_semantics != EXACT_INPUT_SHARING_V1
            || candidate.lifecycle_epoch != primary.lifecycle_epoch
        {
            return Err("Capability does not authorize compatible artifact sharing".into());
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
            || left.authority_scope_id != right.authority_scope_id
            || left.authority_policy_content_hash != right.authority_policy_content_hash
            || left.activation_generation != right.activation_generation
            || left.admission_epoch != right.admission_epoch
        {
            return Err(
                "shared work requires independent admissions under compatible authority".into(),
            );
        }
        // Agent identity is attribution, not an action permission. Compatibility
        // requires each Agent's grant to independently cover this action; complete
        // grants and terminal obligations remain attached to their own admissions.
        let primary_authority = primary
            .lineage
            .authority_decision
            .as_ref()
            .ok_or("shared work requires primary authority")?;
        let candidate_authority = candidate
            .lineage
            .authority_decision
            .as_ref()
            .ok_or("shared work requires contributing authority")?;
        primary_authority
            .validate()
            .map_err(|error| error.to_string())?;
        candidate_authority
            .validate()
            .map_err(|error| error.to_string())?;
        if primary_authority.policy_id != candidate_authority.policy_id
            || primary_authority.policy_content_hash != candidate_authority.policy_content_hash
            || primary_authority.principal_id != candidate_authority.principal_id
            || primary_authority.subject != candidate_authority.subject
            || [primary_authority, candidate_authority]
                .iter()
                .any(|authority| {
                    !authority
                        .requested_action_ids
                        .contains(&contract.capability_type_id)
                        || !authority
                            .authorized_action_ids
                            .contains(&contract.capability_type_id)
                })
        {
            return Err("each contributor must independently authorize the shared action under the same principal, policy and subject".into());
        }
        super::state::validate_task_admission_attribution(state, primary)?;
        super::state::validate_task_admission_attribution_for_lowering(state, candidate)?;
        let original = &state.admissions[&left.admission_id].request.task;
        let incoming = &state.admissions[&right.admission_id].request.task;
        let identity = contract.content_identity();
        if !original.capability_contract_ids.contains(&identity)
            || !incoming.capability_contract_ids.contains(&identity)
            || original.execution_subject != incoming.execution_subject
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
        if &Self::legacy_admission_decision(state, candidate, primary, &self.capability_contract)?
            != self
        {
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

/// Compatibility of two already lowered, ready nodes, including both exact input
/// materializations. The retained provenance explains why distinct producer runs
/// can supply equivalent inputs without discarding either admission's evidence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReadyWorkSharing {
    /// Logical node whose admitted step will use the shared operational node.
    pub contributor_node_id: String,
    /// Owner contract, shared node and compatibility identity.
    pub decision: SharedActionDecision,
    /// Exact selected primary input and source provenance.
    pub primary_inputs: super::initialization::MaterializedTaskInitialization,
    /// Exact selected contributing input and source provenance.
    pub contributor_inputs: super::initialization::MaterializedTaskInitialization,
}

impl ReadyWorkSharing {
    /// Propose sharing from exact ready owner state. Command acceptance and replay
    /// independently revalidate this proof before changing the operational graph.
    pub fn decide(
        state: &NetworkState,
        candidate: &TaskNode,
        primary: &TaskNode,
        contract: &CapabilityTypeContract,
    ) -> Result<Self, String> {
        let ready = super::readiness::compute_ready_set(state);
        if candidate.task_instance_id == primary.task_instance_id
            || !ready
                .task_instance_ids
                .contains(&candidate.task_instance_id)
            || !ready.task_instance_ids.contains(&primary.task_instance_id)
            || !decision_ids_for_node(state, &candidate.task_instance_id).is_empty()
        {
            return Err(
                "sharing requires two unclaimed ready nodes without migrating contributors".into(),
            );
        }
        super::state::validate_task_admission_attribution(state, candidate)?;
        let mut decision =
            SharedActionDecision::compatible_contract(state, candidate, primary, contract)?;
        let primary_inputs = super::initialization::materialize_task_initialization(
            state,
            &primary.task_instance_id,
        )
        .map_err(|error| format!("primary inputs are unavailable: {error:?}"))?;
        let contributor_inputs = super::initialization::materialize_task_initialization(
            state,
            &candidate.task_instance_id,
        )
        .map_err(|error| format!("contributing inputs are unavailable: {error:?}"))?;
        if primary_inputs.payload.init_artifacts.is_empty()
            || primary_inputs.payload.init_artifacts != contributor_inputs.payload.init_artifacts
        {
            return Err("materialized inputs differ in value, schema, type or slot".into());
        }
        decision.decision_id = stable_id(
            "execution-ready-work-compatibility-v1",
            &(&decision.decision_id, &primary_inputs, &contributor_inputs),
        );
        Ok(Self {
            contributor_node_id: candidate.task_instance_id.clone(),
            decision,
            primary_inputs,
            contributor_inputs,
        })
    }

    pub(crate) fn apply(&self, state: &mut NetworkState) -> Result<(), String> {
        let candidate = state
            .tasks
            .get(&self.contributor_node_id)
            .ok_or("contributing node is absent")?
            .clone();
        let primary = state
            .tasks
            .get(&self.decision.shared_node_id)
            .ok_or("shared node is absent")?;
        if Self::decide(
            state,
            &candidate,
            primary,
            &self.decision.capability_contract,
        )? != *self
        {
            return Err("ready-work decision differs from exact durable inputs".into());
        }
        for node in state.tasks.values().filter(|node| node.init_sources.iter().any(|source| {
            matches!(source, TaskInitSource::UpstreamArtifact(source) if source.upstream_task_instance_id == self.contributor_node_id)
        })) {
            super::state::validate_task_admission_attribution(state, node)?;
            if state.statuses.get(&node.task_instance_id) != Some(&TaskStatus::Pending) {
                return Err("shared dependent must remain unclaimed".into());
            }
        }
        let mut retained = super::mutation::Inject::new(
            candidate,
            state
                .edges
                .iter()
                .filter(|edge| edge.to == self.contributor_node_id)
                .cloned()
                .collect(),
        );
        retained.sharing = Some(self.decision.clone());
        state.tasks.remove(&self.contributor_node_id);
        state.statuses.remove(&self.contributor_node_id);
        state
            .shared_steps
            .insert(self.contributor_node_id.clone(), retained);
        for edge in &mut state.edges {
            if edge.from == self.contributor_node_id {
                edge.from = self.decision.shared_node_id.clone();
            }
            if edge.to == self.contributor_node_id {
                edge.to = self.decision.shared_node_id.clone();
                // Both materializations are retained in the decision. The shared
                // invocation reads its primary payload; the contributor's already
                // discharged dataflow remains an ordering prerequisite here.
                edge.kind = super::state::DependencyKind::Ordering;
            }
        }
        state.edges.sort();
        state.edges.dedup();
        for node in state.tasks.values_mut() {
            let mut remapped = false;
            for source in &mut node.init_sources {
                if let TaskInitSource::UpstreamArtifact(source) = source {
                    if source.upstream_task_instance_id == self.contributor_node_id {
                        source.upstream_task_instance_id = self.decision.shared_node_id.clone();
                        remapped = true;
                    }
                }
            }
            if remapped {
                let attribution = node
                    .lineage
                    .admission
                    .as_ref()
                    .ok_or("shared successor has no admission")?;
                let hashes = state
                    .admitted_region_node_hashes
                    .get_mut(&attribution.admission_id)
                    .ok_or("shared successor has no canonical region")?;
                hashes.insert(node.lineage.step_id.clone(), stable_hash(node));
            }
        }
        let graph = super::readiness::validate_active_graph(state);
        let inputs = super::initialization::validate_task_init_graph_sources(state);
        if !graph.is_empty() || !inputs.is_empty() {
            return Err(format!("shared graph is invalid: {graph:?}; {inputs:?}"));
        }
        Ok(())
    }
}
