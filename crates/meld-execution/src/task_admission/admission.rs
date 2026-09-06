//! Durable admission of complete Agent-authorized Tasks.

use crate::capability::{CapabilityCatalog, InputCardinality};
use crate::task_network::{command, contracts::stable_id, state::NetworkState};
use meld_lang::{validate, AuthorityDecision, Bindings, Composition, EdgeKind, StepKind, Term};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Complete Task body accepted by Execution without semantic reconstruction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionTask {
    /// Exact frozen inputs available to the selected Task steps.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub initial_inputs: Vec<meld_lang::TaskInput>,
    /// Stable Agent-authored Task identity.
    pub task_id: String,
    /// Sole semantic action body accepted by Execution.
    pub composition: Composition,
    /// Ground values frozen with the Task body.
    pub bindings: Bindings,
    /// Exact installed contract content identities required by the Task.
    pub capability_contract_ids: Vec<String>,
    /// Governed outcome contract expected by the producer.
    pub expected_outcome_contract_id: String,
    /// Capability actions authorized by the producer fence.
    pub authority_requirements: Vec<String>,
    /// Producer idempotency identity for this Task.
    pub idempotency_key: String,
}

/// Exact producer lineage retained at the Execution boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskAdmissionLineage {
    /// Agent that granted Task authority.
    pub agent_id: String,
    /// Goal to which the Task remains attributed.
    pub goal_id: String,
    /// Immutable Plan revision that contains the Task.
    pub plan_revision_id: String,
    /// Product identity inside the Plan.
    pub product_id: String,
    /// Fresh Agent product authorization identity.
    pub authorization_id: String,
    /// Frozen reasoning context identity.
    pub context_id: String,
    /// Authority scope selected by the Agent.
    pub authority_scope_id: String,
    /// Exact authority policy content identity.
    pub authority_policy_content_hash: String,
    /// Exact Agent-evaluated action grant retained for independent enforcement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authority_decision: Option<AuthorityDecision>,
    /// Activation generation that fences realization.
    pub activation_generation: String,
    /// Exact admission epoch, absent only in legacy records.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub admission_epoch: Option<String>,
}

/// One immutable offer to the Execution Goal Set.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskAdmissionRequest {
    /// Complete producer attribution.
    pub lineage: TaskAdmissionLineage,
    /// Complete executable product.
    pub task: ExecutionTask,
    /// Consumer replay identity, required to equal the Task identity.
    pub idempotency_key: String,
}

/// Consumer-owned admission decision over one exact Task authorization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskAdmissionDecision {
    /// The exact Task passed consumer validation.
    Admitted,
    /// The Task failed one or more consumer-owned checks.
    Rejected {
        /// Deterministic rejection grounds.
        grounds: Vec<String>,
    },
    /// The offer cited a generation other than the live fence.
    StaleFence {
        /// Generation carried by the authorization.
        expected_generation: String,
        /// Generation observed by the consumer.
        observed_generation: String,
    },
}

/// Durable decision retained in the existing Task Network journal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct TaskAdmissionRecord {
    /// Deterministic consumer decision identity.
    pub admission_id: String,
    /// Exact offered Task and lineage.
    pub request: TaskAdmissionRequest,
    /// Durable consumer decision.
    pub decision: TaskAdmissionDecision,
    /// Task Network journal revision that recorded the decision.
    pub recorded_revision: u64,
}

impl TaskAdmissionRecord {
    /// Derive the consumer identity for one exact offered Task.
    pub fn admission_id_for(request: &TaskAdmissionRequest) -> String {
        stable_id(
            "execution-task-admission",
            &(
                &request.lineage.authorization_id,
                &request.task.task_id,
                &request.idempotency_key,
            ),
        )
    }

    /// Build a revision-free proposal for the single-writer command boundary.
    pub(crate) fn proposed(request: TaskAdmissionRequest, decision: TaskAdmissionDecision) -> Self {
        let admission_id = Self::admission_id_for(&request);
        Self {
            admission_id,
            request,
            decision,
            recorded_revision: 0,
        }
    }
}

/// Admission write accepted only after the canonical facade has decided it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValidatedTaskAdmissionWrite {
    pub(crate) command_id: String,
    pub(crate) network_id: String,
    pub(crate) base_revision: u64,
    pub(crate) base_state_hash: String,
    pub(crate) admission: TaskAdmissionRecord,
}

/// Command-store surface needed by the admission facade.
pub trait TaskAdmissionStore {
    /// Return the current reduced Task Network state.
    fn network_state(&self) -> &NetworkState;
    /// Submit one serialized admission command.
    fn submit_validated_admission(
        &mut self,
        request: ValidatedTaskAdmissionWrite,
    ) -> Result<command::Response, String>;
}

impl TaskAdmissionStore for crate::task_network::InMemoryTaskNetworkStore {
    fn network_state(&self) -> &NetworkState {
        self.state()
    }

    fn submit_validated_admission(
        &mut self,
        request: ValidatedTaskAdmissionWrite,
    ) -> Result<command::Response, String> {
        Ok(self.submit_validated_task_admission(request))
    }
}

impl TaskAdmissionStore for crate::task_network::SledTaskNetworkStore {
    fn network_state(&self) -> &NetworkState {
        self.state()
    }

    fn submit_validated_admission(
        &mut self,
        request: ValidatedTaskAdmissionWrite,
    ) -> Result<command::Response, String> {
        self.submit_validated_task_admission(request)
            .map_err(|error| error.to_string())
    }
}

/// Execution Goal Set facade for durable Task decisions.
pub struct TaskAdmissionApi<'a, S> {
    store: &'a mut S,
    catalog: &'a CapabilityCatalog,
    live_generation: &'a str,
    live_admission_epoch: Option<&'a str>,
    live_authority_policy_content_hash: &'a str,
}

impl<'a, S> TaskAdmissionApi<'a, S>
where
    S: TaskAdmissionStore,
{
    /// Bind the facade to one network, exact catalog, and live fence.
    pub fn new(
        store: &'a mut S,
        catalog: &'a CapabilityCatalog,
        live_generation: &'a str,
        live_authority_policy_content_hash: &'a str,
    ) -> Self {
        Self {
            store,
            catalog,
            live_generation,
            live_admission_epoch: None,
            live_authority_policy_content_hash,
        }
    }

    /// Bind the exact open admission epoch observed for this offer.
    pub fn with_admission_epoch(mut self, epoch: Option<&'a str>) -> Self {
        self.live_admission_epoch = epoch;
        self
    }

    /// Decide and durably record one exact Task offer before lowering.
    pub fn admit(&mut self, request: TaskAdmissionRequest) -> Result<TaskAdmissionRecord, String> {
        let decision = self.decide(&request);
        let proposal = TaskAdmissionRecord::proposed(request, decision);
        if let Some(existing) = self
            .store
            .network_state()
            .admissions
            .get(&proposal.admission_id)
        {
            return if existing.request == proposal.request {
                Ok(existing.clone())
            } else {
                Err("durable Task admission identity contains a different offer".to_string())
            };
        }
        let state = self.store.network_state();
        let command_id = format!("task-admission::{}", proposal.admission_id);
        let response = self
            .store
            .submit_validated_admission(ValidatedTaskAdmissionWrite {
                command_id,
                network_id: state.network_id.clone(),
                base_revision: state.revision,
                base_state_hash: state.state_hash.clone(),
                admission: proposal.clone(),
            })?;
        match response {
            command::Response::Accepted { .. } | command::Response::Duplicate { .. } => self
                .store
                .network_state()
                .admissions
                .get(&proposal.admission_id)
                .cloned()
                .ok_or_else(|| "accepted Task admission has no durable record".to_string()),
            command::Response::Rejected(rejection) => Err(format!(
                "Task admission command was rejected: {rejection:?}"
            )),
        }
    }

    fn decide(&self, request: &TaskAdmissionRequest) -> TaskAdmissionDecision {
        if request.lineage.activation_generation != self.live_generation
            || request.lineage.admission_epoch.as_deref() != self.live_admission_epoch
            || request.lineage.authority_policy_content_hash
                != self.live_authority_policy_content_hash
        {
            return TaskAdmissionDecision::StaleFence {
                expected_generation: request.lineage.activation_generation.clone(),
                observed_generation: self.live_generation.to_string(),
            };
        }
        let grounds = validation_grounds(request, self.catalog);
        if grounds.is_empty() {
            TaskAdmissionDecision::Admitted
        } else {
            TaskAdmissionDecision::Rejected { grounds }
        }
    }
}

fn validation_grounds(request: &TaskAdmissionRequest, catalog: &CapabilityCatalog) -> Vec<String> {
    let mut grounds = Vec::new();
    let declared_contracts = request
        .task
        .capability_contract_ids
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let declared_actions = request
        .task
        .authority_requirements
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    if declared_contracts.len() != request.task.capability_contract_ids.len() {
        grounds.push("Task Capability contract identities contain duplicates".to_string());
    }
    if declared_actions.len() != request.task.authority_requirements.len() {
        grounds.push("Task authority requirements contain duplicates".to_string());
    }
    for value in [
        request.lineage.agent_id.as_str(),
        request.lineage.goal_id.as_str(),
        request.lineage.plan_revision_id.as_str(),
        request.lineage.product_id.as_str(),
        request.lineage.authorization_id.as_str(),
        request.lineage.context_id.as_str(),
        request.lineage.authority_scope_id.as_str(),
        request.task.task_id.as_str(),
        request.task.expected_outcome_contract_id.as_str(),
        request.task.idempotency_key.as_str(),
        request.idempotency_key.as_str(),
    ] {
        if value.trim().is_empty() {
            grounds.push("Task admission contains an empty required identity".to_string());
            break;
        }
    }
    if request.lineage.product_id != request.task.task_id {
        grounds.push("authorization product does not name the offered Task".to_string());
    }
    if request.idempotency_key != request.task.idempotency_key {
        grounds.push("admission and Task idempotency keys differ".to_string());
    }
    match &request.lineage.authority_decision {
        Some(decision) => {
            if let Err(error) = decision.validate() {
                grounds.push(format!("Task authority decision is invalid: {error}"));
            }
            if decision.policy_content_hash != request.lineage.authority_policy_content_hash {
                grounds.push(
                    "Task authority decision cites a different policy content identity".to_string(),
                );
            }
            let mut expected = request.task.authority_requirements.clone();
            expected.sort();
            expected.dedup();
            let mut authorized = decision.authorized_action_ids.clone();
            authorized.sort();
            authorized.dedup();
            if authorized != expected {
                grounds.push(
                    "Task authority decision does not exactly cover Task requirements".to_string(),
                );
            }
        }
        None if !request.lineage.authority_policy_content_hash.is_empty() => grounds
            .push("Task admission has no authority decision under the live policy".to_string()),
        None => {}
    }
    let validation = validate(&request.task.composition);
    if !validation.valid {
        grounds.push(format!(
            "Task composition is invalid: {:?}",
            validation.errors
        ));
    }
    let mut used_contracts = BTreeSet::new();
    let mut used_actions = BTreeSet::new();
    let mut resolved_contracts = std::collections::BTreeMap::new();
    for step in &request.task.composition.steps {
        let StepKind::Op(operator) = &step.kind else {
            grounds.push("Task contains a recursive Goal step".to_string());
            continue;
        };
        let Some(specific) = &operator.resolution.specific else {
            grounds.push(format!(
                "operator '{}' has no exact Capability reference",
                operator.operator_id
            ));
            continue;
        };
        let Some(contract) = catalog.get(&specific.capability_type_id, specific.capability_version)
        else {
            grounds.push(format!(
                "Capability '{}' version {} is not installed",
                specific.capability_type_id, specific.capability_version
            ));
            continue;
        };
        used_actions.insert(specific.capability_type_id.clone());
        used_contracts.insert(contract.content_identity());
        resolved_contracts.insert(step.step_id.as_str(), contract);
        for binding in &contract.binding_contract {
            match exact_contract_binding(&request.task.bindings, &binding.binding_id) {
                Ok(Some(_)) => {}
                Ok(None) if !binding.required => {}
                Ok(None) => grounds.push(format!(
                    "Capability '{}' required binding '{}' is absent from the Task",
                    specific.capability_type_id, binding.binding_id
                )),
                Err(error) => grounds.push(format!(
                    "Capability '{}' {error}",
                    specific.capability_type_id
                )),
            }
        }
    }
    if declared_actions != used_actions {
        grounds.push(
            "Task authority requirements do not exactly equal the used Capability actions"
                .to_string(),
        );
    }
    if declared_contracts != used_contracts {
        grounds.push(
            "Task Capability contract identities do not exactly equal the used installed contracts"
                .to_string(),
        );
    }
    for edge in &request.task.composition.edges {
        let EdgeKind::DataFlow {
            artifact_type: Term::ArtifactType(artifact_type_id),
        } = &edge.kind
        else {
            if matches!(edge.kind, EdgeKind::DataFlow { .. }) {
                grounds.push(format!(
                    "Task data flow edge '{} -> {}' has an ungrounded artifact type",
                    edge.from, edge.to
                ));
            }
            continue;
        };
        let Some(target) = resolved_contracts.get(edge.to.as_str()) else {
            continue;
        };
        let matching_inputs = target
            .input_contract
            .iter()
            .filter(|slot| {
                slot.accepted_artifact_type_ids
                    .iter()
                    .any(|accepted| accepted == artifact_type_id)
            })
            .collect::<Vec<_>>();
        let Some(input) = (matching_inputs.len() == 1).then(|| matching_inputs[0]) else {
            grounds.push(format!(
                "Task data flow artifact '{}' does not identify exactly one input slot on '{}'",
                artifact_type_id, edge.to
            ));
            continue;
        };
        let Some(source) = resolved_contracts.get(edge.from.as_str()) else {
            continue;
        };
        let matching_outputs = source
            .output_contract
            .iter()
            .filter(|output| output.artifact_type_id == *artifact_type_id)
            .collect::<Vec<_>>();
        if matching_outputs.len() != 1 {
            grounds.push(format!(
                "Task data flow source '{}' does not publish exactly one '{}' output",
                edge.from, artifact_type_id
            ));
        } else if !input
            .schema_versions
            .accepts(matching_outputs[0].schema_version)
        {
            grounds.push(format!(
                "Task data flow schema from '{}' is outside input slot '{}' range",
                edge.from, input.slot_id
            ));
        }
    }
    for input in &request.task.initial_inputs {
        if let Err(error) = input.validate() {
            grounds.push(error);
        }
        let slot = resolved_contracts
            .get(input.step_id.as_str())
            .and_then(|contract| {
                contract
                    .input_contract
                    .iter()
                    .find(|slot| slot.slot_id == input.slot_id)
            });
        match slot {
            Some(slot)
                if slot
                    .accepted_artifact_type_ids
                    .contains(&input.artifact_type_id)
                    && slot.schema_versions.accepts(input.schema_version) => {}
            _ => grounds
                .push("Task frozen input does not match an exact consumer slot and schema".into()),
        }
    }
    for (step_id, contract) in &resolved_contracts {
        for slot in &contract.input_contract {
            let matching_edges = request
                .task
                .composition
                .edges
                .iter()
                .filter(|edge| edge.to == **step_id)
                .filter(|edge| match &edge.kind {
                    EdgeKind::DataFlow {
                        artifact_type: Term::ArtifactType(artifact_type_id),
                    } => slot
                        .accepted_artifact_type_ids
                        .iter()
                        .any(|accepted| accepted == artifact_type_id),
                    _ => false,
                })
                .count();
            let matching_inputs = request
                .task
                .initial_inputs
                .iter()
                .filter(|input| input.step_id == **step_id && input.slot_id == slot.slot_id)
                .count();
            let matching_edges = matching_edges + matching_inputs;
            match slot.cardinality {
                InputCardinality::One if slot.required && matching_edges != 1 => {
                    grounds.push(format!(
                        "Capability step '{}' required input slot '{}' is not closed by exactly one Task input source",
                        step_id, slot.slot_id
                    ));
                }
                InputCardinality::One if !slot.required && matching_edges > 1 => {
                    grounds.push(format!(
                        "Capability step '{}' optional input slot '{}' accepts at most one Task input source",
                        step_id, slot.slot_id
                    ));
                }
                InputCardinality::Many => grounds.push(format!(
                    "Capability step '{}' input slot '{}' uses unsupported data flow cardinality",
                    step_id, slot.slot_id
                )),
                InputCardinality::One => {}
            }
        }
    }
    grounds.sort();
    grounds.dedup();
    grounds
}

pub(crate) fn exact_contract_binding<'a>(
    bindings: &'a Bindings,
    binding_id: &str,
) -> Result<Option<&'a Term>, String> {
    let canonical_id = binding_id.strip_prefix('?').unwrap_or(binding_id);
    let aliases = [canonical_id.to_string(), format!("?{canonical_id}")];
    let matches = aliases
        .iter()
        .filter_map(|candidate| {
            bindings
                .get(candidate)
                .map(|term| (candidate.as_str(), term))
        })
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [] => Ok(None),
        [(_, term)] => Ok(Some(*term)),
        _ => Err(format!(
            "binding '{}' is supplied through ambiguous aliases: {}",
            binding_id,
            matches
                .iter()
                .map(|(candidate, _)| *candidate)
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}
