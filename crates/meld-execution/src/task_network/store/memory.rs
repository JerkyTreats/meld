//! In-memory task network command store and reducer.

use crate::task_network::{
    command,
    contracts::stable_hash,
    dispatch::{self, OutcomeStatus},
    initialization::{
        materialize_task_initialization, validate_task_init_graph_sources,
        validate_task_init_sources,
    },
    journal::JournalRecord,
    mutation::{self, CommitRecord, Rejection},
    outcome::{Publication, PublicationState},
    readiness::{compute_ready_set, validate_active_graph},
    state::{DependencyEdge, DependencyEdgeOrigin, DependencyKind, NetworkState, TaskStatus},
    store::{codec::decode_error, error::TaskNetworkStoreError, records::StoredJournalRecord},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// Deterministic in-memory task network command store.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InMemoryTaskNetworkStore {
    state: NetworkState,
    journal: Vec<JournalRecord>,
    pub(super) command_requests: BTreeMap<String, String>,
    pub(super) command_responses: BTreeMap<String, command::Response>,
}

impl InMemoryTaskNetworkStore {
    /// Creates a new empty task network store.
    pub fn new(network_id: impl Into<String>) -> Self {
        Self {
            state: NetworkState::empty(network_id),
            journal: Vec::new(),
            command_requests: BTreeMap::new(),
            command_responses: BTreeMap::new(),
        }
    }

    /// Returns the latest reduced state.
    pub fn state(&self) -> &NetworkState {
        &self.state
    }

    /// Returns the accepted journal records in revision order.
    pub fn journal(&self) -> &[JournalRecord] {
        &self.journal
    }

    /// Submits one command through the single writer reducer.
    pub fn submit(&mut self, request: command::Request) -> command::Response {
        let request_hash = command_request_hash(&request);
        let command_id = request.command_id.clone();
        if let Some(stored_hash) = self.command_requests.get(&command_id) {
            if stored_hash == &request_hash {
                return self
                    .command_responses
                    .get(&command_id)
                    .map(duplicate_or_replay)
                    .unwrap_or_else(|| {
                        command::Response::Rejected(Rejection::DuplicateCommand(command_id))
                    });
            }
            return command::Response::Rejected(Rejection::DuplicateCommand(command_id));
        }

        if request.network_id != self.state.network_id {
            return self.record_response(
                request.command_id,
                request_hash,
                command::Response::Rejected(Rejection::InvalidGraph(format!(
                    "command targeted network '{}' but store owns '{}'",
                    request.network_id, self.state.network_id
                ))),
            );
        }

        if request.base_revision != self.state.revision {
            return self.record_response(
                request.command_id,
                request_hash,
                command::Response::Rejected(Rejection::StaleBase {
                    expected: request.base_revision,
                    actual: self.state.revision,
                }),
            );
        }

        if request.base_state_hash != self.state.state_hash {
            return self.record_response(
                request.command_id,
                request_hash,
                command::Response::Rejected(Rejection::StateHashMismatch {
                    expected: request.base_state_hash,
                    actual: self.state.state_hash.clone(),
                }),
            );
        }

        if let Err(rejection) = self.validate_preconditions(&request.read_preconditions) {
            return self.record_response(
                request.command_id,
                request_hash,
                command::Response::Rejected(rejection),
            );
        }

        match request.command {
            command::Command::ApplyMutationSet(set) => {
                self.apply_mutation_set(request.command_id, request_hash, set)
            }
            command::Command::ClaimReadyTask(claim_request) => {
                self.claim_ready_task(request.command_id, request_hash, claim_request)
            }
            command::Command::RecordTaskOutcome(outcome) => {
                self.record_task_outcome(request.command_id, request_hash, outcome)
            }
            command::Command::MarkPublication(publication) => {
                self.mark_publication(request.command_id, request_hash, publication)
            }
        }
    }

    fn record_response(
        &mut self,
        command_id: String,
        request_hash: String,
        response: command::Response,
    ) -> command::Response {
        self.command_requests
            .insert(command_id.clone(), request_hash);
        self.command_responses.insert(command_id, response.clone());
        response
    }

    fn apply_mutation_set(
        &mut self,
        command_id: String,
        request_hash: String,
        set: mutation::Set,
    ) -> command::Response {
        if set.network_id != self.state.network_id {
            return self.record_response(
                command_id,
                request_hash,
                command::Response::Rejected(Rejection::InvalidGraph(format!(
                    "mutation set targeted network '{}' but store owns '{}'",
                    set.network_id, self.state.network_id
                ))),
            );
        }

        let previous_state_hash = self.state.state_hash.clone();
        let mut proposed = self.state.clone();
        for mutation in &set.mutations {
            match mutation {
                mutation::Mutation::Inject(inject) => {
                    let init_diagnostics = validate_task_init_sources(&inject.task_node);
                    if !init_diagnostics.is_empty() {
                        return self.record_response(
                            command_id,
                            request_hash,
                            command::Response::Rejected(Rejection::InvalidGraph(
                                init_diagnostics
                                    .into_iter()
                                    .map(|diagnostic| diagnostic.message)
                                    .collect::<Vec<_>>()
                                    .join("; "),
                            )),
                        );
                    }
                    let task_instance_id = inject.task_node.task_instance_id.clone();
                    if proposed.tasks.contains_key(&task_instance_id) {
                        return self.record_response(
                            command_id,
                            request_hash,
                            command::Response::Rejected(Rejection::FailedPrecondition(
                                mutation::ReadPrecondition::NodeAbsent(task_instance_id),
                            )),
                        );
                    }
                    proposed
                        .statuses
                        .insert(task_instance_id.clone(), TaskStatus::Pending);
                    proposed
                        .tasks
                        .insert(task_instance_id, inject.task_node.clone());
                    proposed.edges.extend(inject.incoming_edges.clone());
                }
            }
        }

        dedupe_edges(&mut proposed.edges);
        let graph_diagnostics = validate_active_graph(&proposed);
        if !graph_diagnostics.is_empty() {
            return self.record_response(
                command_id,
                request_hash,
                command::Response::Rejected(Rejection::InvalidGraph(
                    graph_diagnostics
                        .into_iter()
                        .map(|diagnostic| diagnostic.message)
                        .collect::<Vec<_>>()
                        .join("; "),
                )),
            );
        }

        let init_graph_diagnostics = validate_task_init_graph_sources(&proposed);
        if !init_graph_diagnostics.is_empty() {
            return self.record_response(
                command_id,
                request_hash,
                command::Response::Rejected(Rejection::InvalidGraph(
                    init_graph_diagnostics
                        .into_iter()
                        .map(|diagnostic| diagnostic.message)
                        .collect::<Vec<_>>()
                        .join("; "),
                )),
            );
        }

        let revision = self.state.revision + 1;
        proposed.set_revision_and_hash(revision);
        let record = CommitRecord::new(
            command_id.clone(),
            proposed.network_id.clone(),
            revision,
            previous_state_hash,
            proposed.state_hash.clone(),
            set,
        );
        self.state = proposed;
        self.journal.push(JournalRecord::Commit(record));
        self.record_response(
            command_id,
            request_hash,
            command::Response::Accepted {
                revision: self.state.revision,
                state_hash: self.state.state_hash.clone(),
            },
        )
    }

    fn claim_ready_task(
        &mut self,
        command_id: String,
        request_hash: String,
        request: dispatch::Request,
    ) -> command::Response {
        let Some(node) = self.state.tasks.get(&request.task_instance_id) else {
            return self.record_response(
                command_id,
                request_hash,
                command::Response::Rejected(Rejection::FailedPrecondition(
                    mutation::ReadPrecondition::NodeExists(request.task_instance_id),
                )),
            );
        };

        if self.state.statuses.get(&request.task_instance_id) != Some(&TaskStatus::Pending) {
            return self.record_response(
                command_id,
                request_hash,
                command::Response::Rejected(Rejection::InvalidLifecycleTransition(format!(
                    "task '{}' is not pending",
                    request.task_instance_id
                ))),
            );
        }

        let ready = compute_ready_set(&self.state);
        if !ready
            .task_instance_ids
            .iter()
            .any(|task_instance_id| task_instance_id == &request.task_instance_id)
        {
            return self.record_response(
                command_id,
                request_hash,
                command::Response::Rejected(Rejection::InvalidLifecycleTransition(format!(
                    "task '{}' is not ready",
                    request.task_instance_id
                ))),
            );
        }

        if let Err(error) = materialize_task_initialization(&self.state, &request.task_instance_id)
        {
            return self.record_response(
                command_id,
                request_hash,
                command::Response::Rejected(Rejection::InvalidLifecycleTransition(
                    error
                        .diagnostics
                        .into_iter()
                        .map(|diagnostic| diagnostic.message)
                        .collect::<Vec<_>>()
                        .join("; "),
                )),
            );
        }

        let revision = self.state.revision + 1;
        let claim = dispatch::Claim::accepted(
            self.state.network_id.clone(),
            &request,
            node.lifecycle_epoch,
            revision,
        );
        self.state.statuses.insert(
            request.task_instance_id.clone(),
            TaskStatus::Running {
                claim_id: claim.claim_id.clone(),
            },
        );
        self.state
            .claims
            .insert(claim.claim_id.clone(), claim.clone());
        self.state.set_revision_and_hash(revision);
        self.journal.push(JournalRecord::Claim(claim));
        self.record_response(
            command_id,
            request_hash,
            command::Response::Accepted {
                revision: self.state.revision,
                state_hash: self.state.state_hash.clone(),
            },
        )
    }

    fn record_task_outcome(
        &mut self,
        command_id: String,
        request_hash: String,
        outcome: dispatch::Outcome,
    ) -> command::Response {
        let Some(claim) = self.state.claims.get(&outcome.claim_id) else {
            return self.record_response(
                command_id,
                request_hash,
                command::Response::Rejected(Rejection::StaleClaim {
                    claim_id: outcome.claim_id,
                }),
            );
        };

        if claim.task_instance_id != outcome.task_instance_id
            || claim.lifecycle_epoch != outcome.lifecycle_epoch
            || claim.claim_revision != outcome.claim_revision
        {
            return self.record_response(
                command_id,
                request_hash,
                command::Response::Rejected(Rejection::StaleClaim {
                    claim_id: outcome.claim_id,
                }),
            );
        }

        if !matches!(
            self.state.statuses.get(&outcome.task_instance_id),
            Some(TaskStatus::Running { claim_id }) if claim_id == &outcome.claim_id
        ) {
            return self.record_response(
                command_id,
                request_hash,
                command::Response::Rejected(Rejection::InvalidLifecycleTransition(format!(
                    "task '{}' is not running under claim '{}'",
                    outcome.task_instance_id, outcome.claim_id
                ))),
            );
        }

        let revision = self.state.revision + 1;
        match outcome.status {
            OutcomeStatus::Succeeded => {
                self.state.statuses.insert(
                    outcome.task_instance_id.clone(),
                    TaskStatus::Succeeded {
                        outcome_id: outcome.outcome_id.clone(),
                    },
                );
            }
            OutcomeStatus::Failed => {
                self.state.statuses.insert(
                    outcome.task_instance_id.clone(),
                    TaskStatus::Failed {
                        outcome_id: outcome.outcome_id.clone(),
                        error: outcome
                            .error
                            .clone()
                            .unwrap_or_else(|| "task failed".to_string()),
                    },
                );
            }
        }
        self.state
            .artifact_availability
            .extend(dispatch::artifact_availability_for_outcome(&outcome));
        self.state
            .outcomes
            .insert(outcome.outcome_id.clone(), outcome.clone());
        let publication = Publication::pending_for_outcome(&self.state.network_id, &outcome);
        self.state
            .publications
            .insert(publication.publication_id.clone(), publication.clone());
        self.state.set_revision_and_hash(revision);
        self.journal.push(JournalRecord::Outcome { publication });
        self.record_response(
            command_id,
            request_hash,
            command::Response::Accepted {
                revision: self.state.revision,
                state_hash: self.state.state_hash.clone(),
            },
        )
    }

    fn mark_publication(
        &mut self,
        command_id: String,
        request_hash: String,
        publication: Publication,
    ) -> command::Response {
        let Some(current) = self
            .state
            .publications
            .get(&publication.publication_id)
            .cloned()
        else {
            return self.record_response(
                command_id,
                request_hash,
                command::Response::Rejected(Rejection::FailedPrecondition(
                    mutation::ReadPrecondition::PublicationPending(
                        publication.publication_id.clone(),
                    ),
                )),
            );
        };

        let upgrades_legacy_receipt = matches!(
            (&current.state, &publication.state),
            (
                PublicationState::Published { receipt: None, .. },
                PublicationState::Published {
                    receipt: Some(_),
                    legacy_event_seq: None,
                    ..
                }
            )
        );
        if matches!(
            (&current.state, &publication.state),
            (
                PublicationState::Pending | PublicationState::Failed { .. },
                PublicationState::Published { receipt: None, .. }
            )
        ) {
            return self.record_response(
                command_id,
                request_hash,
                command::Response::Rejected(Rejection::InvalidLifecycleTransition(format!(
                    "publication '{}' requires a canonical append receipt",
                    publication.publication_id
                ))),
            );
        }
        if matches!(current.state, PublicationState::Published { .. }) && !upgrades_legacy_receipt {
            return self.record_response(
                command_id,
                request_hash,
                command::Response::Rejected(Rejection::PublicationAlreadyMarked(
                    publication.publication_id,
                )),
            );
        }

        if current.network_id != publication.network_id || current.outcome != publication.outcome {
            return self.record_response(
                command_id,
                request_hash,
                command::Response::Rejected(Rejection::InvalidLifecycleTransition(format!(
                    "publication '{}' immutable fields do not match pending record",
                    publication.publication_id
                ))),
            );
        }

        if matches!(
            publication.state,
            PublicationState::Published {
                receipt: Some(_),
                legacy_event_seq: Some(_),
                ..
            }
        ) {
            return self.record_response(
                command_id,
                request_hash,
                command::Response::Rejected(Rejection::InvalidLifecycleTransition(format!(
                    "publication '{}' cannot mix canonical receipt with legacy event sequence",
                    publication.publication_id
                ))),
            );
        }

        let revision = self.state.revision + 1;
        let mut marked = current;
        marked.state = match publication.state {
            PublicationState::Published {
                receipt,
                legacy_event_seq,
                ..
            } => PublicationState::Published {
                marked_revision: revision,
                receipt,
                legacy_event_seq,
            },
            PublicationState::Failed { error } => PublicationState::Failed { error },
            PublicationState::Pending => {
                return self.record_response(
                    command_id,
                    request_hash,
                    command::Response::Rejected(Rejection::InvalidLifecycleTransition(format!(
                        "publication '{}' mark must be terminal",
                        publication.publication_id
                    ))),
                );
            }
        };
        self.state
            .publications
            .insert(marked.publication_id.clone(), marked.clone());
        self.state.set_revision_and_hash(revision);
        self.journal.push(JournalRecord::Publication(marked));
        self.record_response(
            command_id,
            request_hash,
            command::Response::Accepted {
                revision: self.state.revision,
                state_hash: self.state.state_hash.clone(),
            },
        )
    }

    fn validate_preconditions(
        &self,
        preconditions: &[mutation::ReadPrecondition],
    ) -> Result<(), Rejection> {
        for precondition in preconditions {
            let ok = match precondition {
                mutation::ReadPrecondition::RevisionIs(revision) => {
                    self.state.revision == *revision
                }
                mutation::ReadPrecondition::StateHashIs(hash) => self.state.state_hash == *hash,
                mutation::ReadPrecondition::NodeExists(task_instance_id) => {
                    self.state.tasks.contains_key(task_instance_id)
                }
                mutation::ReadPrecondition::NodeAbsent(task_instance_id) => {
                    !self.state.tasks.contains_key(task_instance_id)
                }
                mutation::ReadPrecondition::NodeStatusIs {
                    task_instance_id,
                    status,
                } => self.state.statuses.get(task_instance_id) == Some(status),
                mutation::ReadPrecondition::EdgeExists { from, to } => {
                    edge_exists(&self.state.edges, from, to)
                }
                mutation::ReadPrecondition::EdgeAbsent { from, to } => {
                    !edge_exists(&self.state.edges, from, to)
                }
                mutation::ReadPrecondition::ArtifactAvailable {
                    task_instance_id,
                    artifact_type_id,
                } => self.state.artifact_availability.iter().any(|artifact| {
                    artifact.task_instance_id == *task_instance_id
                        && artifact.artifact_type_id == *artifact_type_id
                }),
                mutation::ReadPrecondition::NoPath { from, to } => {
                    !path_exists(&self.state.edges, from, to)
                }
                mutation::ReadPrecondition::ClaimCurrent {
                    task_instance_id,
                    claim_id,
                    claim_revision,
                } => self.state.claims.get(claim_id).is_some_and(|claim| {
                    claim.task_instance_id == *task_instance_id
                        && claim.claim_revision == *claim_revision
                }),
                mutation::ReadPrecondition::PublicationPending(publication_id) => self
                    .state
                    .publications
                    .get(publication_id)
                    .is_some_and(|publication| {
                        matches!(
                            publication.state,
                            PublicationState::Pending
                                | PublicationState::Failed { .. }
                                | PublicationState::Published {
                                    // TODO compat-shim: remove after the minimum supported
                                    // persisted task-network schema guarantees identity-bearing
                                    // receipts. Until then,
                                    // persisted_legacy_receipt_reopens_and_upgrades proves the
                                    // receipt-less row is retried and durably upgraded.
                                    receipt: None,
                                    ..
                                }
                        )
                    }),
            };

            if !ok {
                return Err(Rejection::FailedPrecondition(precondition.clone()));
            }
        }

        Ok(())
    }

    pub(super) fn apply_journal_record_for_replay(
        &mut self,
        revision: u64,
        stored: &StoredJournalRecord,
    ) -> Result<(), TaskNetworkStoreError> {
        if stored
            .legacy_network_id
            .as_ref()
            .is_some_and(|network_id| network_id != &self.state.network_id)
        {
            return Err(decode_error(format!(
                "journal network '{}' does not match store '{}'",
                stored
                    .legacy_network_id
                    .as_deref()
                    .expect("legacy network id was checked"),
                self.state.network_id
            )));
        }
        let expected_revision = self.state.revision + 1;
        if revision != expected_revision
            || stored
                .legacy_revision
                .is_some_and(|legacy_revision| legacy_revision != revision)
        {
            return Err(decode_error(format!(
                "journal revision '{}' did not follow '{}'",
                revision, self.state.revision
            )));
        }

        match &stored.record {
            JournalRecord::Commit(record) => {
                if record.network_id != self.state.network_id
                    || record.revision != revision
                    || record.previous_state_hash != self.state.state_hash
                {
                    return Err(decode_error("commit journal record metadata mismatch"));
                }
                for mutation in &record.mutation_set.mutations {
                    match mutation {
                        mutation::Mutation::Inject(inject) => {
                            let task_instance_id = inject.task_node.task_instance_id.clone();
                            self.state
                                .statuses
                                .insert(task_instance_id.clone(), TaskStatus::Pending);
                            self.state
                                .tasks
                                .insert(task_instance_id, inject.task_node.clone());
                            self.state.edges.extend(inject.incoming_edges.clone());
                        }
                    }
                }
                dedupe_edges(&mut self.state.edges);
            }
            JournalRecord::Claim(claim) => {
                if claim.network_id != self.state.network_id || claim.claim_revision != revision {
                    return Err(decode_error("claim journal record metadata mismatch"));
                }
                self.state.statuses.insert(
                    claim.task_instance_id.clone(),
                    TaskStatus::Running {
                        claim_id: claim.claim_id.clone(),
                    },
                );
                self.state
                    .claims
                    .insert(claim.claim_id.clone(), claim.clone());
            }
            JournalRecord::Outcome { publication } => {
                if publication.network_id != self.state.network_id {
                    return Err(decode_error("outcome journal record metadata mismatch"));
                }
                let outcome = &publication.outcome;
                match outcome.status {
                    OutcomeStatus::Succeeded => {
                        self.state.statuses.insert(
                            outcome.task_instance_id.clone(),
                            TaskStatus::Succeeded {
                                outcome_id: outcome.outcome_id.clone(),
                            },
                        );
                    }
                    OutcomeStatus::Failed => {
                        self.state.statuses.insert(
                            outcome.task_instance_id.clone(),
                            TaskStatus::Failed {
                                outcome_id: outcome.outcome_id.clone(),
                                error: outcome
                                    .error
                                    .clone()
                                    .unwrap_or_else(|| "task failed".to_string()),
                            },
                        );
                    }
                }
                self.state
                    .artifact_availability
                    .extend(dispatch::artifact_availability_for_outcome(outcome));
                self.state
                    .outcomes
                    .insert(outcome.outcome_id.clone(), outcome.clone());
                self.state
                    .publications
                    .insert(publication.publication_id.clone(), publication.clone());
            }
            JournalRecord::Publication(publication) => {
                if publication.network_id != self.state.network_id {
                    return Err(decode_error("publication journal record metadata mismatch"));
                }
                self.state
                    .publications
                    .insert(publication.publication_id.clone(), publication.clone());
            }
        }

        self.state.set_revision_and_hash(revision);
        if let JournalRecord::Commit(record) = &stored.record {
            if self.state.state_hash != record.state_hash {
                return Err(decode_error(format!(
                    "commit state hash '{}' did not match replayed '{}'",
                    record.state_hash, self.state.state_hash
                )));
            }
        }
        if stored
            .legacy_state_hash
            .as_ref()
            .is_some_and(|state_hash| state_hash != &self.state.state_hash)
        {
            return Err(decode_error(format!(
                "journal state hash '{}' did not match replayed '{}'",
                stored
                    .legacy_state_hash
                    .as_deref()
                    .expect("legacy state hash was checked"),
                self.state.state_hash
            )));
        }
        self.journal.push(stored.record.clone());
        Ok(())
    }

    pub(super) fn insert_command_identity(
        &mut self,
        command_id: String,
        request_hash: String,
        response: command::Response,
    ) {
        self.command_requests
            .insert(command_id.clone(), request_hash);
        self.command_responses.insert(command_id, response);
    }
}

pub(super) fn command_request_hash(request: &command::Request) -> String {
    stable_hash(request)
}

pub(super) fn duplicate_or_replay(response: &command::Response) -> command::Response {
    match response {
        command::Response::Accepted {
            revision,
            state_hash,
        }
        | command::Response::Duplicate {
            revision,
            state_hash,
        } => command::Response::Duplicate {
            revision: *revision,
            state_hash: state_hash.clone(),
        },
        command::Response::Rejected(rejection) => command::Response::Rejected(rejection.clone()),
    }
}

fn edge_exists(edges: &[DependencyEdge], from: &str, to: &str) -> bool {
    edges.iter().any(|edge| edge.from == from && edge.to == to)
}

fn dedupe_edges(edges: &mut Vec<DependencyEdge>) {
    // Origin is recorded metadata, not edge identity: a legacy unrecorded
    // edge and its re-injected recorded twin are one edge. A recorded
    // origin wins over unrecorded; between recorded origins the lowest
    // rank wins deterministically.
    let mut best: BTreeMap<(String, String, DependencyKind), DependencyEdge> = BTreeMap::new();
    for edge in edges.drain(..) {
        let key = (edge.from.clone(), edge.to.clone(), edge.kind.clone());
        match best.get(&key) {
            Some(existing) if origin_rank(existing.origin) <= origin_rank(edge.origin) => {}
            _ => {
                best.insert(key, edge);
            }
        }
    }
    edges.extend(best.into_values());
    edges.sort();
}

fn origin_rank(origin: DependencyEdgeOrigin) -> u8 {
    match origin {
        DependencyEdgeOrigin::Semantic => 0,
        DependencyEdgeOrigin::Scheduling => 1,
        DependencyEdgeOrigin::Unrecorded => 2,
    }
}

fn path_exists(edges: &[DependencyEdge], from: &str, to: &str) -> bool {
    let mut adjacency: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for edge in edges {
        adjacency
            .entry(edge.from.as_str())
            .or_default()
            .push(edge.to.as_str());
    }

    let mut queue = VecDeque::from([from]);
    let mut seen = BTreeSet::new();
    while let Some(current) = queue.pop_front() {
        if current == to {
            return true;
        }
        if !seen.insert(current) {
            continue;
        }
        if let Some(next) = adjacency.get(current) {
            for item in next {
                queue.push_back(item);
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task_network::journal::JournalRecord;

    #[test]
    fn replay_rejects_claim_record_network_mismatch_even_when_hash_matches_mutated_state() {
        let mismatched_claim = dispatch::Claim {
            claim_id: "claim-alpha".to_string(),
            network_id: "other-network".to_string(),
            task_instance_id: "task-alpha".to_string(),
            lifecycle_epoch: 0,
            claim_revision: 1,
            worker_id: "worker-alpha".to_string(),
            idempotency_key: "claim-once".to_string(),
        };
        let mut mutated_state = NetworkState::empty("network-docs");
        mutated_state.statuses.insert(
            mismatched_claim.task_instance_id.clone(),
            TaskStatus::Running {
                claim_id: mismatched_claim.claim_id.clone(),
            },
        );
        mutated_state
            .claims
            .insert(mismatched_claim.claim_id.clone(), mismatched_claim.clone());
        mutated_state.set_revision_and_hash(1);
        let stored = StoredJournalRecord {
            record: JournalRecord::Claim(mismatched_claim.clone()),
            legacy_network_id: Some("network-docs".to_string()),
            legacy_revision: Some(1),
            legacy_state_hash: Some(mutated_state.state_hash),
        };
        let mut store = InMemoryTaskNetworkStore::new("network-docs");

        assert!(store.apply_journal_record_for_replay(1, &stored).is_err());
    }
}
