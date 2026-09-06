use std::collections::{BTreeMap, BTreeSet};

use serde::{de::DeserializeOwned, Serialize};
use sled::transaction::{ConflictableTransactionError, TransactionError, Transactional};

use super::contracts::*;
use crate::runtime::registration::{RegistrationKind, RegistrationSet};
use crate::theory::{ParticipantKind, PreparedActivationClosureV1};

const TREE_ASSIGNMENTS: &str = "pds_assignment_lifecycle_v1";
const TREE_DECISIONS: &str = "pds_activation_lifecycle_decisions_v1";
const TREE_TRANSITIONS: &str = "pds_activation_lifecycle_transitions_v1";

/// Single durable authority for assignment generation, head, and admission.
#[derive(Clone)]
pub struct ActivationLifecycleStore {
    db: sled::Db,
    assignments: sled::Tree,
    decisions: sled::Tree,
    transitions: sled::Tree,
}

impl ActivationLifecycleStore {
    /// Open the lifecycle authority inside the existing product theory database.
    pub fn new(db: sled::Db) -> Result<Self, LifecycleError> {
        Ok(Self {
            assignments: db.open_tree(TREE_ASSIGNMENTS).map_err(storage)?,
            decisions: db.open_tree(TREE_DECISIONS).map_err(storage)?,
            transitions: db.open_tree(TREE_TRANSITIONS).map_err(storage)?,
            db,
        })
    }

    /// Accept or resume one exact prepared closure under a request key.
    pub fn accept(
        &self,
        request: ActivationLifecycleRequestV1,
    ) -> Result<LifecycleAcceptance, LifecycleError> {
        request
            .prepared
            .verify_identity()
            .map_err(|error| LifecycleError::Invalid(error.to_string()))?;
        let request_identity = content_hash(&request)?;
        if let Some(prior) = self.decision(&request.request_key)? {
            return Ok(LifecycleAcceptance {
                outcome: if prior.decision_id == request_identity {
                    LifecycleAcceptanceOutcome::Duplicate
                } else {
                    LifecycleAcceptanceOutcome::Conflicted
                },
                decision: prior,
            });
        }

        let assignment_id = request.prepared.assignment.assignment_id.clone();
        let mut assignment =
            self.assignment(&assignment_id)?
                .unwrap_or_else(|| AssignmentLifecycleV1 {
                    assignment_id: assignment_id.clone(),
                    lifecycle_revision: 0,
                    current_generation_id: None,
                    generations: BTreeMap::new(),
                });
        let current = assignment
            .current_generation_id
            .as_ref()
            .and_then(|id| assignment.generations.get(id));
        let resumed_id = match request.action {
            LifecycleAction::Recover => current
                .filter(|generation| {
                    generation.prepared_id == request.prepared.prepared_id
                        && generation.status == ActivationGenerationStatus::Interrupted
                })
                .map(|generation| generation.generation_id.clone()),
            LifecycleAction::Activate | LifecycleAction::Replace => assignment
                .generations
                .values()
                .find(|generation| {
                    generation.prepared_id == request.prepared.prepared_id
                        && generation.expected_prior_generation == request.expected_prior_generation
                        && matches!(
                            generation.status,
                            ActivationGenerationStatus::Preparing
                                | ActivationGenerationStatus::Ready
                        )
                })
                .map(|generation| generation.generation_id.clone()),
        };
        let valid = match request.action {
            LifecycleAction::Activate => assignment.current_generation_id.is_none(),
            LifecycleAction::Replace => {
                assignment.current_generation_id == request.expected_prior_generation
                    && assignment.current_generation_id.is_some()
            }
            LifecycleAction::Recover => resumed_id.is_some(),
        };
        if !valid {
            let decision = ActivationLifecycleDecisionV1 {
                decision_id: request_identity,
                request_key: request.request_key,
                action: request.action,
                prepared_id: request.prepared.prepared_id,
                expected_prior_generation: request.expected_prior_generation,
                generation_id: None,
                outcome: LifecycleAcceptanceOutcome::Rejected,
            };
            self.commit_decision(&decision)?;
            return Ok(LifecycleAcceptance {
                decision,
                outcome: LifecycleAcceptanceOutcome::Rejected,
            });
        }

        let generation_id = match resumed_id {
            Some(id) => id,
            None => {
                let generation_number = assignment.generations.len() as u64 + 1;
                let generation_id = content_hash(&(
                    &assignment_id,
                    &request.prepared.prepared_id,
                    generation_number,
                    &request.expected_prior_generation,
                ))?;
                let transition_ref = content_hash(&(&generation_id, "preparing"))?;
                assignment.generations.insert(
                    generation_id.clone(),
                    ActivationGenerationV1 {
                        generation_id: generation_id.clone(),
                        generation_number,
                        assignment_id: assignment_id.clone(),
                        activation_id: request.prepared.activation.activation_id.clone(),
                        prepared_id: request.prepared.prepared_id.clone(),
                        participant_plan_id: request.prepared.participant_plan.plan_id.clone(),
                        participant_specs: request
                            .prepared
                            .participant_plan
                            .participants
                            .iter()
                            .map(|specification| {
                                (specification.participant_id.clone(), specification.clone())
                            })
                            .collect(),
                        expected_prior_generation: request.expected_prior_generation.clone(),
                        status: ActivationGenerationStatus::Preparing,
                        realizations: BTreeMap::new(),
                        incarnations: BTreeMap::new(),
                        readiness: BTreeMap::new(),
                        registration_parity: None,
                        admission_epochs: Vec::new(),
                        waits: BTreeMap::new(),
                        safe_points: BTreeMap::new(),
                        passive_fences: BTreeMap::new(),
                        stop_receipts: BTreeMap::new(),
                        stop_order: Vec::new(),
                        release_receipts: BTreeMap::new(),
                        fenced_quiescence: None,
                        retirement: None,
                        last_transition_ref: transition_ref,
                    },
                );
                generation_id
            }
        };
        let decision = ActivationLifecycleDecisionV1 {
            decision_id: request_identity,
            request_key: request.request_key,
            action: request.action,
            prepared_id: request.prepared.prepared_id,
            expected_prior_generation: request.expected_prior_generation,
            generation_id: Some(generation_id),
            outcome: LifecycleAcceptanceOutcome::Accepted,
        };
        self.commit_assignment_and_decision(&mut assignment, &decision)?;
        Ok(LifecycleAcceptance {
            decision,
            outcome: LifecycleAcceptanceOutcome::Accepted,
        })
    }

    /// Realize the accepted participant plan through one exact registration set.
    pub fn realize(
        &self,
        prepared: &PreparedActivationClosureV1,
        generation_id: &str,
        registrations: &RegistrationSet,
        available_implementations: &BTreeSet<String>,
    ) -> Result<RegistrationParityReceiptV1, LifecycleError> {
        let mut assignment = self.required_assignment(&prepared.assignment.assignment_id)?;
        let generation = required_generation_mut(&mut assignment, generation_id)?;
        if !matches!(
            generation.status,
            ActivationGenerationStatus::Preparing
                | ActivationGenerationStatus::Ready
                | ActivationGenerationStatus::Interrupted
        ) {
            return Err(LifecycleError::Conflict(
                "generation cannot change realization from its current state".to_string(),
            ));
        }
        if generation.status == ActivationGenerationStatus::Ready {
            generation.status = ActivationGenerationStatus::Preparing;
        }
        generation.readiness.clear();
        generation.waits.clear();
        generation.safe_points.clear();
        generation.passive_fences.clear();
        generation.stop_receipts.clear();
        generation.stop_order.clear();
        generation.release_receipts.clear();
        generation.registration_parity = None;
        if generation.prepared_id != prepared.prepared_id {
            return Err(LifecycleError::Conflict(
                "generation belongs to another prepared closure".to_string(),
            ));
        }
        let declared = prepared
            .participant_plan
            .participants
            .iter()
            .map(|participant| participant.participant_id.as_str())
            .collect::<BTreeSet<_>>();
        if registrations
            .registrations
            .iter()
            .any(|registration| !declared.contains(registration.runtime_id.as_str()))
        {
            return Err(LifecycleError::Conflict(
                "runtime registrations contain an undeclared participant".to_string(),
            ));
        }
        let selected = registrations
            .registrations
            .iter()
            .map(|registration| registration.runtime_id.as_str())
            .collect::<BTreeSet<_>>();
        generation
            .realizations
            .retain(|participant, _| selected.contains(participant.as_str()));
        generation
            .incarnations
            .retain(|participant, _| selected.contains(participant.as_str()));

        let mut realization_ids = Vec::new();
        let mut registration_ids = Vec::new();
        for specification in &prepared.participant_plan.participants {
            let Some(registration) = registrations.get(&specification.participant_id) else {
                if specification.required {
                    return Err(LifecycleError::Conflict(format!(
                        "participant '{}' has no exact runtime registration",
                        specification.participant_id
                    )));
                }
                continue;
            };
            let expected_kind = match specification.kind {
                ParticipantKind::PassiveSource => RegistrationKind::PassiveService,
                ParticipantKind::BoundedActor | ParticipantKind::DurableOperationAdapter => {
                    RegistrationKind::ActiveActor
                }
            };
            if registration.kind != expected_kind
                || !available_implementations.contains(&specification.participant_id)
            {
                return Err(LifecycleError::Conflict(format!(
                    "participant '{}' is not exactly realizable",
                    specification.participant_id
                )));
            }
            let realization = ParticipantRealizationV1::new(
                generation_id.to_string(),
                specification,
                format!("runtime-factory::{}", specification.participant_id),
                registration.registration_id.clone(),
            )?;
            realization_ids.push(realization.realization_id.clone());
            registration_ids.push(registration.registration_id.clone());
            generation
                .realizations
                .insert(specification.participant_id.clone(), realization);
        }
        let receipt_id = content_hash(&(
            generation_id,
            &prepared.participant_plan.plan_id,
            &realization_ids,
            &registration_ids,
        ))?;
        let parity = RegistrationParityReceiptV1 {
            receipt_id,
            generation_id: generation_id.to_string(),
            participant_plan_id: prepared.participant_plan.plan_id.clone(),
            realization_ids,
            registration_ids,
        };
        generation.registration_parity = Some(parity.clone());
        generation.last_transition_ref = content_hash(&(generation_id, "realized"))?;
        self.commit_assignment(&mut assignment)?;
        Ok(parity)
    }

    /// Record a new participant incarnation and clear stale owner evidence.
    pub fn create_incarnation(
        &self,
        assignment_id: &str,
        generation_id: &str,
        participant_id: &str,
        lease_ref: String,
    ) -> Result<ParticipantIncarnationV1, LifecycleError> {
        let mut assignment = self.required_assignment(assignment_id)?;
        let generation = required_generation_mut(&mut assignment, generation_id)?;
        if !matches!(
            generation.status,
            ActivationGenerationStatus::Preparing | ActivationGenerationStatus::Interrupted
        ) {
            return Err(LifecycleError::Conflict(
                "generation cannot create an incarnation from its current state".to_string(),
            ));
        }
        let realization = generation.realizations.get(participant_id).ok_or_else(|| {
            LifecycleError::Invalid("participant realization is absent".to_string())
        })?;
        let incarnation_number = generation
            .incarnations
            .get(participant_id)
            .map(|incarnation| incarnation.incarnation_number + 1)
            .unwrap_or(1);
        let incarnation = ParticipantIncarnationV1::new(
            generation_id.to_string(),
            participant_id.to_string(),
            realization.realization_id.clone(),
            lease_ref,
            incarnation_number,
        )?;
        generation
            .incarnations
            .insert(participant_id.to_string(), incarnation.clone());
        generation.readiness.remove(participant_id);
        generation.waits.remove(participant_id);
        generation.safe_points.remove(participant_id);
        generation.last_transition_ref = content_hash(&(
            generation_id,
            participant_id,
            "incarnation",
            incarnation_number,
        ))?;
        self.commit_assignment(&mut assignment)?;
        Ok(incarnation)
    }

    /// Accept structurally exact native-owner readiness.
    pub fn record_readiness(
        &self,
        assignment_id: &str,
        generation_id: &str,
        participant_id: &str,
        receipt: OwnerReadinessReceiptV1,
        prepared: &PreparedActivationClosureV1,
    ) -> Result<(), LifecycleError> {
        let mut assignment = self.required_assignment(assignment_id)?;
        let generation = required_generation_mut(&mut assignment, generation_id)?;
        prepared
            .verify_identity()
            .map_err(|error| LifecycleError::Invalid(error.to_string()))?;
        if generation.prepared_id != prepared.prepared_id
            || generation.assignment_id != prepared.assignment.assignment_id
        {
            return Err(LifecycleError::Conflict(
                "owner readiness prepared closure differs from the generation".to_string(),
            ));
        }
        if !matches!(
            generation.status,
            ActivationGenerationStatus::Preparing | ActivationGenerationStatus::Interrupted
        ) {
            return Err(LifecycleError::Conflict(
                "generation cannot accept readiness from its current state".to_string(),
            ));
        }
        let specification = prepared
            .participant_plan
            .participants
            .iter()
            .find(|participant| participant.participant_id == participant_id)
            .ok_or_else(|| LifecycleError::Invalid("participant is not declared".to_string()))?;
        let incarnation = generation.incarnations.get(participant_id).ok_or_else(|| {
            LifecycleError::Invalid("participant incarnation is absent".to_string())
        })?;
        let realization = generation.realizations.get(participant_id).ok_or_else(|| {
            LifecycleError::Invalid("participant realization is absent".to_string())
        })?;
        let context = ParticipantLifecycleContextV1::new(specification, realization, incarnation)?;
        let expected_receipt =
            OwnerReadinessReceiptV1::from_native(&context, receipt.native_evidence.clone())?;
        if receipt.generation_id != generation_id
            || receipt.incarnation_id != incarnation.incarnation_id
            || receipt.realization_id != realization.realization_id
            || receipt.participant_id != specification.participant_id
            || receipt.owner_domain != specification.owner_domain
            || receipt.readiness_contract_ref != specification.readiness_contract_ref
            || receipt.native_evidence.participant_id != specification.participant_id
            || receipt.native_evidence.owner_domain != specification.owner_domain
            || receipt != expected_receipt
        {
            return Err(LifecycleError::Conflict(
                "owner readiness does not match the exact participant position".to_string(),
            ));
        }
        generation
            .readiness
            .insert(participant_id.to_string(), receipt);
        if generation.status != ActivationGenerationStatus::Interrupted
            && generation
                .realizations
                .keys()
                .all(|participant| generation.readiness.contains_key(participant))
            && generation.registration_parity.is_some()
        {
            generation.status = ActivationGenerationStatus::Ready;
        }
        generation.last_transition_ref = content_hash(&(generation_id, participant_id, "ready"))?;
        self.commit_assignment(&mut assignment)
    }

    /// Atomically publish a ready generation as the current head with open admission.
    pub fn publish_current(
        &self,
        assignment_id: &str,
        generation_id: &str,
    ) -> Result<AdmissionEpochV1, LifecycleError> {
        let mut assignment = self.required_assignment(assignment_id)?;
        let expected = assignment
            .generations
            .get(generation_id)
            .ok_or_else(|| LifecycleError::Invalid("activation generation is absent".to_string()))?
            .expected_prior_generation
            .clone();
        if assignment.current_generation_id != expected {
            return Err(LifecycleError::Conflict(
                "assignment head differs from the expected prior generation".to_string(),
            ));
        }
        if let Some(prior_id) = assignment.current_generation_id.clone() {
            close_generation_admission(&mut assignment, &prior_id, "replaced")?;
            required_generation_mut(&mut assignment, &prior_id)?.status =
                ActivationGenerationStatus::Draining;
        }
        let generation = required_generation_mut(&mut assignment, generation_id)?;
        if generation.status != ActivationGenerationStatus::Ready {
            return Err(LifecycleError::Conflict(
                "generation is not structurally ready".to_string(),
            ));
        }
        let epoch = open_epoch(generation)?;
        generation.status = ActivationGenerationStatus::Current;
        generation.last_transition_ref = epoch.transition_ref.clone();
        assignment.current_generation_id = Some(generation_id.to_string());
        self.commit_assignment(&mut assignment)?;
        Ok(epoch)
    }

    /// Fence a current generation before participant recovery.
    pub fn interrupt(
        &self,
        assignment_id: &str,
        generation_id: &str,
    ) -> Result<AdmissionEpochV1, LifecycleError> {
        let mut assignment = self.required_assignment(assignment_id)?;
        if assignment.current_generation_id.as_deref() != Some(generation_id) {
            return Err(LifecycleError::Conflict(
                "only the current generation can be interrupted".to_string(),
            ));
        }
        let epoch = close_generation_admission(&mut assignment, generation_id, "interrupted")?;
        required_generation_mut(&mut assignment, generation_id)?.status =
            ActivationGenerationStatus::Interrupted;
        self.commit_assignment(&mut assignment)?;
        Ok(epoch)
    }

    /// Reopen the same current generation under a successor admission epoch.
    pub fn reopen(
        &self,
        assignment_id: &str,
        generation_id: &str,
        prepared: &PreparedActivationClosureV1,
    ) -> Result<AdmissionEpochV1, LifecycleError> {
        let mut assignment = self.required_assignment(assignment_id)?;
        if assignment.current_generation_id.as_deref() != Some(generation_id) {
            return Err(LifecycleError::Conflict(
                "assignment head changed during participant recovery".to_string(),
            ));
        }
        let generation = required_generation_mut(&mut assignment, generation_id)?;
        if generation.prepared_id != prepared.prepared_id {
            return Err(LifecycleError::Conflict(
                "recovery prepared closure differs from the generation".to_string(),
            ));
        }
        if generation.status != ActivationGenerationStatus::Interrupted
            || !generation
                .realizations
                .keys()
                .all(|participant| generation.readiness.contains_key(participant))
        {
            return Err(LifecycleError::Conflict(
                "recovery readiness is incomplete".to_string(),
            ));
        }
        let epoch = open_epoch(generation)?;
        generation.status = ActivationGenerationStatus::Current;
        generation.last_transition_ref = epoch.transition_ref.clone();
        self.commit_assignment(&mut assignment)?;
        Ok(epoch)
    }

    /// Record a native-owner no-work position and its complete structural wakes.
    pub fn record_wait(
        &self,
        assignment_id: &str,
        generation_id: &str,
        participant_id: &str,
        receipt: OwnerWaitReceiptV1,
    ) -> Result<(), LifecycleError> {
        let mut assignment = self.required_assignment(assignment_id)?;
        let generation = required_generation_mut(&mut assignment, generation_id)?;
        let incarnation = generation.incarnations.get(participant_id).ok_or_else(|| {
            LifecycleError::Invalid("participant incarnation is absent".to_string())
        })?;
        if receipt.generation_id != generation_id
            || receipt.incarnation_id != incarnation.incarnation_id
        {
            return Err(LifecycleError::Conflict(
                "owner wait does not match the current incarnation".to_string(),
            ));
        }
        generation.waits.insert(participant_id.to_string(), receipt);
        generation.last_transition_ref = content_hash(&(generation_id, participant_id, "wait"))?;
        self.commit_assignment(&mut assignment)
    }

    /// Remove stale wait evidence after an owner reports activity or failure.
    pub fn clear_wait(
        &self,
        assignment_id: &str,
        generation_id: &str,
        participant_id: &str,
    ) -> Result<(), LifecycleError> {
        let mut assignment = self.required_assignment(assignment_id)?;
        let generation = required_generation_mut(&mut assignment, generation_id)?;
        if generation.waits.remove(participant_id).is_some() {
            generation.last_transition_ref =
                content_hash(&(generation_id, participant_id, "wait-cleared"))?;
            self.commit_assignment(&mut assignment)?;
        }
        Ok(())
    }

    /// Project lifecycle state from native waits and structural wake resolution.
    pub fn project_liveness(
        &self,
        assignment_id: &str,
        generation_id: &str,
        working_participants: &BTreeSet<String>,
        idle_participants: &BTreeSet<String>,
        failed_participants: &BTreeSet<String>,
        resolvable_wakes: &BTreeSet<StructuralWakeRef>,
    ) -> Result<AssignmentLifecycleProjectionV1, LifecycleError> {
        let assignment = self.required_assignment(assignment_id)?;
        let generation = assignment.generations.get(generation_id).ok_or_else(|| {
            LifecycleError::Invalid("activation generation is absent".to_string())
        })?;
        let mut incomplete_participants = Vec::new();
        let mut broken_wake_refs = BTreeSet::new();
        let realized_ids = generation.realizations.keys().collect::<BTreeSet<_>>();
        for participant_id in working_participants
            .iter()
            .chain(idle_participants)
            .chain(failed_participants)
        {
            if !realized_ids.contains(participant_id) {
                incomplete_participants.push(participant_id.clone());
            }
        }
        for realization in generation.realizations.values() {
            let participant_id = &realization.participant_id;
            if !generation.readiness.contains_key(participant_id) {
                incomplete_participants.push(participant_id.clone());
                continue;
            }
            if failed_participants.contains(participant_id) {
                incomplete_participants.push(participant_id.clone());
                continue;
            }
            if working_participants.contains(participant_id) {
                continue;
            }
            let Some(wait) = generation.waits.get(participant_id) else {
                if idle_participants.contains(participant_id) {
                    continue;
                }
                incomplete_participants.push(participant_id.clone());
                continue;
            };
            for wake_ref in &wait.wake_refs {
                if !resolvable_wakes.contains(wake_ref) {
                    broken_wake_refs.insert(wake_ref.clone());
                }
            }
        }
        incomplete_participants.sort();
        let broken_wake_refs = broken_wake_refs.into_iter().collect::<Vec<_>>();
        let all_realized_waiting = generation
            .realizations
            .keys()
            .all(|participant_id| generation.waits.contains_key(participant_id));
        let state = match generation.status {
            ActivationGenerationStatus::Interrupted => ProjectedLifecycleState::Interrupted,
            ActivationGenerationStatus::Draining | ActivationGenerationStatus::FencedQuiescent => {
                ProjectedLifecycleState::Draining
            }
            ActivationGenerationStatus::Retired => ProjectedLifecycleState::Retired,
            _ if !failed_participants.is_empty()
                || !incomplete_participants.is_empty()
                || !broken_wake_refs.is_empty() =>
            {
                ProjectedLifecycleState::Stalled
            }
            _ if working_participants
                .iter()
                .any(|participant| realized_ids.contains(participant)) =>
            {
                ProjectedLifecycleState::ActiveWork
            }
            ActivationGenerationStatus::Current if all_realized_waiting => {
                ProjectedLifecycleState::Quiescent
            }
            _ if idle_participants
                .iter()
                .any(|participant| realized_ids.contains(participant)) =>
            {
                ProjectedLifecycleState::ActiveIdle
            }
            _ => ProjectedLifecycleState::Stalled,
        };
        Ok(AssignmentLifecycleProjectionV1 {
            assignment_id: assignment_id.to_string(),
            generation_id: Some(generation_id.to_string()),
            state,
            incomplete_participants,
            broken_wake_refs,
            last_transition_ref: generation.last_transition_ref.clone(),
        })
    }

    /// Close current admission and begin owner drain.
    pub fn begin_drain(
        &self,
        assignment_id: &str,
        generation_id: &str,
    ) -> Result<AdmissionEpochV1, LifecycleError> {
        let mut assignment = self.required_assignment(assignment_id)?;
        if let Some(generation) = assignment.generations.get(generation_id) {
            if generation.status == ActivationGenerationStatus::Draining {
                return generation
                    .current_admission_epoch()
                    .filter(|epoch| epoch.status == AdmissionEpochStatus::Closed)
                    .cloned()
                    .ok_or_else(|| {
                        LifecycleError::Conflict(
                            "draining generation has no closed admission epoch".to_string(),
                        )
                    });
            }
        }
        if assignment.current_generation_id.as_deref() != Some(generation_id) {
            return Err(LifecycleError::Conflict(
                "only the current or already-draining generation can begin drain".to_string(),
            ));
        }
        let epoch = close_generation_admission(&mut assignment, generation_id, "draining")?;
        required_generation_mut(&mut assignment, generation_id)?.status =
            ActivationGenerationStatus::Draining;
        assignment.current_generation_id = None;
        self.commit_assignment(&mut assignment)?;
        Ok(epoch)
    }

    /// Record one native-owner stop acknowledgement after admission closes.
    pub fn record_stop(
        &self,
        assignment_id: &str,
        generation_id: &str,
        receipt: OwnerStopReceiptV1,
    ) -> Result<(), LifecycleError> {
        let mut assignment = self.required_assignment(assignment_id)?;
        let generation = required_generation_mut(&mut assignment, generation_id)?;
        validate_stop_receipt(generation, &receipt)?;
        if generation.status != ActivationGenerationStatus::FencedQuiescent {
            return Err(LifecycleError::Conflict(
                "native owner stop precedes fenced quiescence".to_string(),
            ));
        }
        if let Some(existing) = generation.stop_receipts.get(&receipt.participant_id) {
            if existing == &receipt {
                return Ok(());
            }
            return Err(LifecycleError::Conflict(
                "native stop receipt conflicts with existing evidence".to_string(),
            ));
        }
        generation
            .stop_receipts
            .insert(receipt.participant_id.clone(), receipt.clone());
        generation.stop_order.push(receipt.participant_id.clone());
        generation.last_transition_ref = receipt.receipt_id;
        self.commit_assignment(&mut assignment)
    }

    /// Record one native passive-source fence at an exact delivery position.
    pub fn record_passive_fence(
        &self,
        assignment_id: &str,
        generation_id: &str,
        receipt: PassiveFenceReceiptV1,
    ) -> Result<(), LifecycleError> {
        let mut assignment = self.required_assignment(assignment_id)?;
        let generation = required_generation_mut(&mut assignment, generation_id)?;
        validate_passive_fence(generation, &receipt)?;
        if generation.status != ActivationGenerationStatus::Draining {
            return Err(LifecycleError::Conflict(
                "passive fence is outside generation drain".to_string(),
            ));
        }
        if let Some(existing) = generation.passive_fences.get(&receipt.participant_id) {
            if existing == &receipt {
                return Ok(());
            }
            return Err(LifecycleError::Conflict(
                "passive fence receipt conflicts with existing evidence".to_string(),
            ));
        }
        generation
            .passive_fences
            .insert(receipt.participant_id.clone(), receipt.clone());
        generation.last_transition_ref = receipt.receipt_id;
        self.commit_assignment(&mut assignment)
    }

    /// Record one exact native-owner safe point before final stop.
    pub fn record_safe_point(
        &self,
        assignment_id: &str,
        generation_id: &str,
        receipt: OwnerSafePointReceiptV1,
    ) -> Result<(), LifecycleError> {
        let mut assignment = self.required_assignment(assignment_id)?;
        let generation = required_generation_mut(&mut assignment, generation_id)?;
        validate_safe_point_receipt(generation, &receipt)?;
        if generation.status != ActivationGenerationStatus::Draining {
            return Err(LifecycleError::Conflict(
                "owner safe point is outside generation drain".to_string(),
            ));
        }
        if let Some(existing) = generation.safe_points.get(&receipt.participant_id) {
            if existing == &receipt {
                return Ok(());
            }
            return Err(LifecycleError::Conflict(
                "owner safe-point receipt conflicts with existing evidence".to_string(),
            ));
        }
        generation
            .safe_points
            .insert(receipt.participant_id.clone(), receipt.clone());
        generation.last_transition_ref = receipt.receipt_id;
        self.commit_assignment(&mut assignment)
    }

    /// Record one native owner or passive binding release after fencing.
    pub fn record_release(
        &self,
        assignment_id: &str,
        generation_id: &str,
        receipt: OwnerReleaseReceiptV1,
    ) -> Result<(), LifecycleError> {
        let mut assignment = self.required_assignment(assignment_id)?;
        let generation = required_generation_mut(&mut assignment, generation_id)?;
        validate_release_receipt(generation, &receipt)?;
        if generation.status != ActivationGenerationStatus::FencedQuiescent {
            return Err(LifecycleError::Conflict(
                "owner release precedes fenced quiescence".to_string(),
            ));
        }
        if !generation
            .stop_receipts
            .contains_key(&receipt.participant_id)
        {
            return Err(LifecycleError::Conflict(
                "native release precedes the participant stop receipt".to_string(),
            ));
        }
        if let Some(existing) = generation.release_receipts.get(&receipt.participant_id) {
            if existing == &receipt {
                return Ok(());
            }
            return Err(LifecycleError::Conflict(
                "native release receipt conflicts with existing evidence".to_string(),
            ));
        }
        generation
            .release_receipts
            .insert(receipt.participant_id.clone(), receipt.clone());
        generation.last_transition_ref = receipt.receipt_id;
        self.commit_assignment(&mut assignment)
    }

    /// Aggregate complete owner and passive fencing evidence.
    pub fn commit_fenced_quiescence(
        &self,
        assignment_id: &str,
        generation_id: &str,
    ) -> Result<FencedQuiescenceReceiptV1, LifecycleError> {
        let mut assignment = self.required_assignment(assignment_id)?;
        let generation = required_generation_mut(&mut assignment, generation_id)?;
        if generation.status != ActivationGenerationStatus::Draining {
            return Err(LifecycleError::Conflict(
                "generation is not draining".to_string(),
            ));
        }
        let active: BTreeSet<_> = generation
            .realizations
            .values()
            .filter(|realization| realization.kind != ParticipantKind::PassiveSource)
            .map(|realization| realization.participant_id.as_str())
            .collect();
        let passive: BTreeSet<_> = generation
            .realizations
            .values()
            .filter(|realization| realization.kind == ParticipantKind::PassiveSource)
            .map(|realization| realization.participant_id.as_str())
            .collect();
        if !active
            .iter()
            .all(|participant| generation.safe_points.contains_key(*participant))
            || !passive
                .iter()
                .all(|participant| generation.passive_fences.contains_key(*participant))
        {
            return Err(LifecycleError::Conflict(
                "fenced quiescence evidence is incomplete".to_string(),
            ));
        }
        let admission_epoch_id = generation
            .current_admission_epoch()
            .filter(|epoch| epoch.status == AdmissionEpochStatus::Closed)
            .map(|epoch| epoch.epoch_id.clone())
            .ok_or_else(|| LifecycleError::Conflict("admission is not closed".to_string()))?;
        let mut owner_safe_point_receipt_ids = generation
            .safe_points
            .values()
            .map(|receipt| receipt.receipt_id.clone())
            .collect::<Vec<_>>();
        owner_safe_point_receipt_ids.sort();
        let mut passive_fence_receipt_ids = generation
            .passive_fences
            .values()
            .map(|receipt| receipt.receipt_id.clone())
            .collect::<Vec<_>>();
        passive_fence_receipt_ids.sort();
        let receipt_id = content_hash(&(
            generation_id,
            &admission_epoch_id,
            &owner_safe_point_receipt_ids,
            &passive_fence_receipt_ids,
        ))?;
        let receipt = FencedQuiescenceReceiptV1 {
            receipt_id,
            generation_id: generation_id.to_string(),
            admission_epoch_id,
            owner_safe_point_receipt_ids,
            passive_fence_receipt_ids,
        };
        generation.fenced_quiescence = Some(receipt.clone());
        generation.status = ActivationGenerationStatus::FencedQuiescent;
        generation.last_transition_ref = receipt.receipt_id.clone();
        self.commit_assignment(&mut assignment)?;
        Ok(receipt)
    }

    /// Publish immutable retirement after reverse-order stop and release.
    pub fn retire(
        &self,
        assignment_id: &str,
        generation_id: &str,
    ) -> Result<RetirementReceiptV1, LifecycleError> {
        let mut assignment = self.required_assignment(assignment_id)?;
        let generation = required_generation_mut(&mut assignment, generation_id)?;
        if let Some(retirement) = &generation.retirement {
            return Ok(retirement.clone());
        }
        let fenced = generation.fenced_quiescence.as_ref().ok_or_else(|| {
            LifecycleError::Conflict("generation is not fenced quiescent".to_string())
        })?;
        if generation.stop_order != reverse_structural_order(generation)?
            || generation.stop_receipts.len() != generation.incarnations.len()
            || generation.release_receipts.len() != generation.incarnations.len()
        {
            return Err(LifecycleError::Conflict(
                "stop and release evidence is incomplete".to_string(),
            ));
        }
        let stop_receipts = generation
            .stop_order
            .iter()
            .map(|participant| generation.stop_receipts[participant].clone())
            .collect::<Vec<_>>();
        let release_receipts = generation
            .stop_order
            .iter()
            .map(|participant| generation.release_receipts[participant].clone())
            .collect::<Vec<_>>();
        let receipt_id = content_hash(&(
            generation_id,
            &fenced.receipt_id,
            &stop_receipts,
            &release_receipts,
        ))?;
        let receipt = RetirementReceiptV1 {
            receipt_id,
            generation_id: generation_id.to_string(),
            fenced_quiescence_receipt_id: fenced.receipt_id.clone(),
            stop_receipts,
            release_receipts,
        };
        generation.retirement = Some(receipt.clone());
        generation.status = ActivationGenerationStatus::Retired;
        generation.last_transition_ref = receipt.receipt_id.clone();
        self.commit_assignment(&mut assignment)?;
        Ok(receipt)
    }

    /// Read one assignment lifecycle aggregate.
    pub fn assignment(
        &self,
        assignment_id: &str,
    ) -> Result<Option<AssignmentLifecycleV1>, LifecycleError> {
        read_optional(&self.assignments, assignment_id.as_bytes())
    }

    /// Read one request decision.
    pub fn decision(
        &self,
        request_key: &str,
    ) -> Result<Option<ActivationLifecycleDecisionV1>, LifecycleError> {
        read_optional(&self.decisions, request_key.as_bytes())
    }

    /// Read the sole current generation for an assignment.
    pub fn current_generation(
        &self,
        assignment_id: &str,
    ) -> Result<Option<ActivationGenerationV1>, LifecycleError> {
        let Some(assignment) = self.assignment(assignment_id)? else {
            return Ok(None);
        };
        Ok(assignment
            .current_generation_id
            .as_ref()
            .and_then(|id| assignment.generations.get(id))
            .cloned())
    }

    /// Read one generation by exact identity.
    pub fn generation(
        &self,
        assignment_id: &str,
        generation_id: &str,
    ) -> Result<Option<ActivationGenerationV1>, LifecycleError> {
        Ok(self
            .assignment(assignment_id)?
            .and_then(|assignment| assignment.generations.get(generation_id).cloned()))
    }

    /// Flush every lifecycle record family.
    pub fn flush(&self) -> Result<(), LifecycleError> {
        self.db.flush().map_err(storage)?;
        Ok(())
    }

    fn required_assignment(
        &self,
        assignment_id: &str,
    ) -> Result<AssignmentLifecycleV1, LifecycleError> {
        self.assignment(assignment_id)?
            .ok_or_else(|| LifecycleError::Invalid("assignment lifecycle is absent".to_string()))
    }

    fn commit_assignment(
        &self,
        assignment: &mut AssignmentLifecycleV1,
    ) -> Result<(), LifecycleError> {
        let expected_revision = assignment.lifecycle_revision;
        assignment.lifecycle_revision = expected_revision.checked_add(1).ok_or_else(|| {
            LifecycleError::Conflict("assignment lifecycle revision overflow".to_string())
        })?;
        let transition_ref = assignment
            .generations
            .values()
            .max_by_key(|generation| generation.generation_number)
            .map(|generation| generation.last_transition_ref.clone())
            .unwrap_or_else(|| assignment.assignment_id.clone());
        let assignment_bytes = encode(assignment)?;
        let transition_bytes = assignment_bytes.clone();
        (&self.assignments, &self.transitions)
            .transaction(|(assignments, transitions)| {
                require_assignment_revision(
                    assignments,
                    assignment.assignment_id.as_bytes(),
                    expected_revision,
                )?;
                assignments.insert(
                    assignment.assignment_id.as_bytes(),
                    assignment_bytes.clone(),
                )?;
                transitions.insert(transition_ref.as_bytes(), transition_bytes.clone())?;
                Ok(())
            })
            .map_err(transaction)?;
        self.flush()
    }

    fn commit_assignment_and_decision(
        &self,
        assignment: &mut AssignmentLifecycleV1,
        decision: &ActivationLifecycleDecisionV1,
    ) -> Result<(), LifecycleError> {
        let expected_revision = assignment.lifecycle_revision;
        assignment.lifecycle_revision = expected_revision.checked_add(1).ok_or_else(|| {
            LifecycleError::Conflict("assignment lifecycle revision overflow".to_string())
        })?;
        let assignment_bytes = encode(assignment)?;
        let decision_bytes = encode(decision)?;
        let generation = decision
            .generation_id
            .as_ref()
            .and_then(|id| assignment.generations.get(id))
            .ok_or_else(|| LifecycleError::Invalid("accepted generation is absent".to_string()))?;
        let transition_bytes = assignment_bytes.clone();
        (&self.assignments, &self.decisions, &self.transitions)
            .transaction(|(assignments, decisions, transitions)| {
                if decisions.get(decision.request_key.as_bytes())?.is_some() {
                    return Err(ConflictableTransactionError::Abort(
                        LifecycleError::Conflict(
                            "lifecycle request was accepted concurrently".to_string(),
                        ),
                    ));
                }
                require_assignment_revision(
                    assignments,
                    assignment.assignment_id.as_bytes(),
                    expected_revision,
                )?;
                assignments.insert(
                    assignment.assignment_id.as_bytes(),
                    assignment_bytes.clone(),
                )?;
                decisions.insert(decision.request_key.as_bytes(), decision_bytes.clone())?;
                transitions.insert(
                    generation.last_transition_ref.as_bytes(),
                    transition_bytes.clone(),
                )?;
                Ok(())
            })
            .map_err(transaction)?;
        self.flush()
    }

    fn commit_decision(
        &self,
        decision: &ActivationLifecycleDecisionV1,
    ) -> Result<(), LifecycleError> {
        let decision_bytes = encode(decision)?;
        self.decisions
            .transaction(|decisions| {
                if decisions.get(decision.request_key.as_bytes())?.is_some() {
                    return Err(ConflictableTransactionError::Abort(
                        LifecycleError::Conflict(
                            "lifecycle request was decided concurrently".to_string(),
                        ),
                    ));
                }
                decisions.insert(decision.request_key.as_bytes(), decision_bytes.clone())?;
                Ok(())
            })
            .map_err(transaction)?;
        self.flush()
    }
}

fn require_assignment_revision(
    assignments: &sled::transaction::TransactionalTree,
    key: &[u8],
    expected_revision: u64,
) -> Result<(), ConflictableTransactionError<LifecycleError>> {
    let actual_revision = match assignments.get(key)? {
        Some(bytes) => {
            decode::<AssignmentLifecycleV1>(&bytes)
                .map_err(ConflictableTransactionError::Abort)?
                .lifecycle_revision
        }
        None => 0,
    };
    if actual_revision != expected_revision
        || expected_revision == 0 && assignments.get(key)?.is_some()
    {
        return Err(ConflictableTransactionError::Abort(
            LifecycleError::Conflict("assignment lifecycle changed concurrently".to_string()),
        ));
    }
    Ok(())
}

fn open_epoch(generation: &mut ActivationGenerationV1) -> Result<AdmissionEpochV1, LifecycleError> {
    let epoch_number = generation.admission_epochs.len() as u64 + 1;
    let transition_ref = content_hash(&(&generation.generation_id, epoch_number, "open"))?;
    let epoch = AdmissionEpochV1 {
        epoch_id: content_hash(&(&generation.generation_id, epoch_number))?,
        generation_id: generation.generation_id.clone(),
        epoch_number,
        status: AdmissionEpochStatus::Open,
        transition_ref,
    };
    generation.admission_epochs.push(epoch.clone());
    Ok(epoch)
}

fn close_generation_admission(
    assignment: &mut AssignmentLifecycleV1,
    generation_id: &str,
    reason: &str,
) -> Result<AdmissionEpochV1, LifecycleError> {
    let generation = required_generation_mut(assignment, generation_id)?;
    let current = generation
        .admission_epochs
        .last_mut()
        .ok_or_else(|| LifecycleError::Conflict("generation has no admission epoch".to_string()))?;
    if current.status == AdmissionEpochStatus::Closed {
        return Ok(current.clone());
    }
    current.status = AdmissionEpochStatus::Closed;
    current.transition_ref = content_hash(&(generation_id, current.epoch_number, reason))?;
    generation.last_transition_ref = current.transition_ref.clone();
    Ok(current.clone())
}

fn required_generation_mut<'a>(
    assignment: &'a mut AssignmentLifecycleV1,
    generation_id: &str,
) -> Result<&'a mut ActivationGenerationV1, LifecycleError> {
    assignment
        .generations
        .get_mut(generation_id)
        .ok_or_else(|| LifecycleError::Invalid("activation generation is absent".to_string()))
}

fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>, LifecycleError> {
    bincode::serialize(value).map_err(|error| LifecycleError::Storage(error.to_string()))
}

fn decode<T: DeserializeOwned>(value: &[u8]) -> Result<T, LifecycleError> {
    bincode::deserialize(value).map_err(|error| LifecycleError::Storage(error.to_string()))
}

fn read_optional<T: DeserializeOwned>(
    tree: &sled::Tree,
    key: &[u8],
) -> Result<Option<T>, LifecycleError> {
    tree.get(key)
        .map_err(storage)?
        .map(|value| decode(&value))
        .transpose()
}

fn storage(error: impl ToString) -> LifecycleError {
    LifecycleError::Storage(error.to_string())
}

fn transaction(error: TransactionError<LifecycleError>) -> LifecycleError {
    match error {
        TransactionError::Abort(error) => error,
        TransactionError::Storage(error) => storage(error),
    }
}

fn lifecycle_context(
    generation: &ActivationGenerationV1,
    participant_id: &str,
) -> Result<ParticipantLifecycleContextV1, LifecycleError> {
    let specification = generation
        .participant_specs
        .get(participant_id)
        .ok_or_else(|| {
            LifecycleError::Invalid("participant specification is absent".to_string())
        })?;
    let realization = generation
        .realizations
        .get(participant_id)
        .ok_or_else(|| LifecycleError::Invalid("participant realization is absent".to_string()))?;
    let incarnation = generation
        .incarnations
        .get(participant_id)
        .ok_or_else(|| LifecycleError::Invalid("participant incarnation is absent".to_string()))?;
    ParticipantLifecycleContextV1::new(specification, realization, incarnation)
}

fn validate_stop_receipt(
    generation: &ActivationGenerationV1,
    receipt: &OwnerStopReceiptV1,
) -> Result<(), LifecycleError> {
    let context = lifecycle_context(generation, &receipt.participant_id)?;
    let expected = OwnerStopReceiptV1::new(
        &context,
        receipt.owner_checkpoint_ref.clone(),
        receipt.owner_proof_ref.clone(),
    )?;
    if &expected != receipt {
        return Err(LifecycleError::Conflict(
            "native stop receipt does not match the exact participant position".to_string(),
        ));
    }
    Ok(())
}

fn validate_passive_fence(
    generation: &ActivationGenerationV1,
    receipt: &PassiveFenceReceiptV1,
) -> Result<(), LifecycleError> {
    let context = lifecycle_context(generation, &receipt.participant_id)?;
    let expected = PassiveFenceReceiptV1::new(
        &context,
        receipt.source_checkpoint_ref.clone(),
        receipt.owner_proof_ref.clone(),
    )?;
    if &expected != receipt {
        return Err(LifecycleError::Conflict(
            "passive fence receipt does not match the exact source position".to_string(),
        ));
    }
    Ok(())
}

fn validate_safe_point_receipt(
    generation: &ActivationGenerationV1,
    receipt: &OwnerSafePointReceiptV1,
) -> Result<(), LifecycleError> {
    let context = lifecycle_context(generation, &receipt.participant_id)?;
    if context.kind == ParticipantKind::PassiveSource {
        return Err(LifecycleError::Conflict(
            "passive source cannot publish an active-owner safe point".to_string(),
        ));
    }
    let expected = OwnerSafePointReceiptV1::new(
        &context,
        receipt.owner_checkpoint_ref.clone(),
        receipt.unresolved_operation_summary_ref.clone(),
        receipt.proof_refs.clone(),
    )?;
    if &expected != receipt {
        return Err(LifecycleError::Conflict(
            "owner safe-point receipt does not match the exact participant position".to_string(),
        ));
    }
    Ok(())
}

fn validate_release_receipt(
    generation: &ActivationGenerationV1,
    receipt: &OwnerReleaseReceiptV1,
) -> Result<(), LifecycleError> {
    let context = lifecycle_context(generation, &receipt.participant_id)?;
    let expected = OwnerReleaseReceiptV1::new(&context, receipt.owner_proof_ref.clone())?;
    if &expected != receipt {
        return Err(LifecycleError::Conflict(
            "native release receipt does not match the exact participant binding".to_string(),
        ));
    }
    Ok(())
}

fn reverse_structural_order(
    generation: &ActivationGenerationV1,
) -> Result<Vec<String>, LifecycleError> {
    fn visit(
        participant_id: &str,
        specifications: &BTreeMap<String, crate::theory::ActivationParticipantSpec>,
        realized: &BTreeSet<&str>,
        visited: &mut BTreeSet<String>,
        ordered: &mut Vec<String>,
    ) -> Result<(), LifecycleError> {
        if !visited.insert(participant_id.to_string()) {
            return Ok(());
        }
        let specification = specifications.get(participant_id).ok_or_else(|| {
            LifecycleError::Invalid("participant specification is absent".to_string())
        })?;
        for dependency in &specification.depends_on {
            if realized.contains(dependency.as_str()) {
                visit(dependency, specifications, realized, visited, ordered)?;
            }
        }
        ordered.push(participant_id.to_string());
        Ok(())
    }

    let realized = generation
        .realizations
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let mut visited = BTreeSet::new();
    let mut ordered = Vec::new();
    for participant_id in generation.realizations.keys() {
        visit(
            participant_id,
            &generation.participant_specs,
            &realized,
            &mut visited,
            &mut ordered,
        )?;
    }
    ordered.reverse();
    Ok(ordered)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        AdapterPlacement, AssignedAgentPositionV1, OperationalLimits, RuntimeIsolationRequirements,
        StewardshipActivationV1, StewardshipAssignmentV1,
    };
    use crate::runtime::registration::RuntimeRegistration;
    use crate::theory::{
        ActivationParticipantPlanV1, ActivationParticipantSpec, EffectiveAuthorityInputRefs,
        PreparedDomainActivationRef,
    };
    use meld_events::DomainObjectRef;
    use tempfile::TempDir;

    struct Fixture {
        _directory: TempDir,
        store: ActivationLifecycleStore,
        prepared: PreparedActivationClosureV1,
        registrations: RegistrationSet,
    }

    fn fixture() -> Fixture {
        let directory = TempDir::new().unwrap();
        let store = ActivationLifecycleStore::new(sled::open(directory.path()).unwrap()).unwrap();
        let assignment = StewardshipAssignmentV1::new(
            "compilation".into(),
            "product-revision".into(),
            "principal".into(),
            DomainObjectRef::new("workspace_fs", "node", "docs").unwrap(),
            "perspective".into(),
            "main".into(),
            "topology".into(),
            vec![AssignedAgentPositionV1 {
                position_id: "steward".into(),
                agent_id: "agent".into(),
            }],
            "authority".into(),
            "grant".into(),
        )
        .unwrap();
        let activation = StewardshipActivationV1::new(
            assignment.assignment_id.clone(),
            BTreeMap::new(),
            BTreeMap::new(),
            AdapterPlacement::InProcess,
            RuntimeIsolationRequirements::default(),
            OperationalLimits::default(),
        )
        .unwrap();
        let participant_plan = ActivationParticipantPlanV1::new(vec![
            participant("actor.one", "owner.one", ParticipantKind::BoundedActor),
            participant("source.one", "source", ParticipantKind::PassiveSource),
        ])
        .unwrap();
        let prepared = PreparedActivationClosureV1::new(
            assignment,
            activation,
            "content".into(),
            vec![PreparedDomainActivationRef {
                owner_domain: "owner.one".into(),
                receipt_ref: "owner-receipt".into(),
            }],
            "capability".into(),
            participant_plan,
            vec![],
            EffectiveAuthorityInputRefs {
                requested_authority_ref: "authority".into(),
                principal_grant_ref: "grant".into(),
                current_judgment_ref: "judgment".into(),
            },
            None,
        )
        .unwrap();
        let registrations = RegistrationSet {
            registrations: vec![
                registration("actor.one", RegistrationKind::ActiveActor),
                registration("source.one", RegistrationKind::PassiveService),
            ],
        };
        Fixture {
            _directory: directory,
            store,
            prepared,
            registrations,
        }
    }

    fn participant(
        participant_id: &str,
        owner_domain: &str,
        kind: ParticipantKind,
    ) -> ActivationParticipantSpec {
        ActivationParticipantSpec {
            participant_id: participant_id.into(),
            owner_domain: owner_domain.into(),
            kind,
            required: true,
            depends_on: BTreeSet::new(),
            readiness_contract_ref: format!("{participant_id}.ready.v1"),
            wake_contract_ref: format!("{participant_id}.wake.v1"),
            safe_point_contract_ref: format!("{participant_id}.safe.v1"),
            stop_contract_ref: format!("{participant_id}.stop.v1"),
        }
    }

    fn registration(runtime_id: &str, kind: RegistrationKind) -> RuntimeRegistration {
        RuntimeRegistration {
            registration_id: format!("prepared::test::{runtime_id}"),
            runtime_id: runtime_id.into(),
            kind,
            required_resources: Vec::new(),
        }
    }

    fn context(
        fixture: &Fixture,
        prepared: &PreparedActivationClosureV1,
        generation_id: &str,
        participant_id: &str,
    ) -> ParticipantLifecycleContextV1 {
        let generation = fixture
            .store
            .generation(&prepared.assignment.assignment_id, generation_id)
            .unwrap()
            .unwrap();
        ParticipantLifecycleContextV1::new(
            &generation.participant_specs[participant_id],
            &generation.realizations[participant_id],
            &generation.incarnations[participant_id],
        )
        .unwrap()
    }

    fn readiness(context: &ParticipantLifecycleContextV1, suffix: &str) -> OwnerReadinessReceiptV1 {
        let checkpoint = format!("checkpoint::{}::{suffix}", context.participant_id);
        OwnerReadinessReceiptV1::from_native(
            context,
            NativeOwnerReadinessEvidenceV1::new(
                context.participant_id.clone(),
                context.owner_domain.clone(),
                checkpoint.clone(),
                vec![format!("revision::{}", context.participant_id)],
                vec![format!("binding::{}", context.participant_id)],
                vec![format!("subscription::{}", context.participant_id)],
                checkpoint,
            )
            .unwrap(),
        )
        .unwrap()
    }

    fn accept_and_ready(fixture: &Fixture) -> String {
        prepare_generation(
            fixture,
            &fixture.prepared,
            "request-one",
            LifecycleAction::Activate,
            None,
        )
    }

    fn prepare_generation(
        fixture: &Fixture,
        prepared: &PreparedActivationClosureV1,
        request_key: &str,
        action: LifecycleAction,
        expected_prior_generation: Option<String>,
    ) -> String {
        let acceptance = fixture
            .store
            .accept(
                ActivationLifecycleRequestV1::new(
                    request_key.into(),
                    action,
                    prepared.clone(),
                    expected_prior_generation,
                )
                .unwrap(),
            )
            .unwrap();
        let generation_id = acceptance.decision.generation_id.unwrap();
        fixture
            .store
            .realize(
                prepared,
                &generation_id,
                &fixture.registrations,
                &BTreeSet::from(["actor.one".into(), "source.one".into()]),
            )
            .unwrap();
        for specification in &prepared.participant_plan.participants {
            let incarnation = fixture
                .store
                .create_incarnation(
                    &prepared.assignment.assignment_id,
                    &generation_id,
                    &specification.participant_id,
                    format!("lease::{}", specification.participant_id),
                )
                .unwrap();
            let context = context(
                fixture,
                prepared,
                &generation_id,
                &specification.participant_id,
            );
            assert_eq!(context.incarnation_id, incarnation.incarnation_id);
            let readiness = readiness(&context, "ready");
            fixture
                .store
                .record_readiness(
                    &prepared.assignment.assignment_id,
                    &generation_id,
                    &specification.participant_id,
                    readiness,
                    prepared,
                )
                .unwrap();
        }
        generation_id
    }

    #[test]
    fn exact_retry_returns_one_generation_and_changed_request_conflicts() {
        let fixture = fixture();
        let request = ActivationLifecycleRequestV1::new(
            "request-one".into(),
            LifecycleAction::Activate,
            fixture.prepared.clone(),
            None,
        )
        .unwrap();
        let accepted = fixture.store.accept(request.clone()).unwrap();
        let duplicate = fixture.store.accept(request).unwrap();
        assert_eq!(duplicate.outcome, LifecycleAcceptanceOutcome::Duplicate);
        assert_eq!(
            accepted.decision.generation_id,
            duplicate.decision.generation_id
        );

        let changed = ActivationLifecycleRequestV1::new(
            "request-one".into(),
            LifecycleAction::Recover,
            fixture.prepared.clone(),
            accepted.decision.generation_id,
        )
        .unwrap();
        assert_eq!(
            fixture.store.accept(changed).unwrap().outcome,
            LifecycleAcceptanceOutcome::Conflicted
        );
    }

    #[test]
    fn rejected_request_is_terminal_and_retries_return_its_exact_decision() {
        let fixture = fixture();
        let generation_id = accept_and_ready(&fixture);
        fixture
            .store
            .publish_current(&fixture.prepared.assignment.assignment_id, &generation_id)
            .unwrap();
        let request = ActivationLifecycleRequestV1::new(
            "invalid-second-activate".into(),
            LifecycleAction::Activate,
            fixture.prepared.clone(),
            None,
        )
        .unwrap();
        let rejected = fixture.store.accept(request.clone()).unwrap();
        assert_eq!(rejected.outcome, LifecycleAcceptanceOutcome::Rejected);
        assert!(rejected.decision.generation_id.is_none());
        let duplicate = fixture.store.accept(request).unwrap();
        assert_eq!(duplicate.outcome, LifecycleAcceptanceOutcome::Duplicate);
        assert_eq!(duplicate.decision, rejected.decision);
    }

    #[test]
    fn publication_commits_one_current_head_with_an_open_epoch() {
        let fixture = fixture();
        let generation_id = accept_and_ready(&fixture);
        let epoch = fixture
            .store
            .publish_current(&fixture.prepared.assignment.assignment_id, &generation_id)
            .unwrap();
        let assignment = fixture
            .store
            .assignment(&fixture.prepared.assignment.assignment_id)
            .unwrap()
            .unwrap();
        let current = assignment.generations.get(&generation_id).unwrap();
        assert_eq!(
            assignment.current_generation_id.as_deref(),
            Some(generation_id.as_str())
        );
        assert_eq!(epoch.status, AdmissionEpochStatus::Open);
        assert!(current.admission_open());
    }

    #[test]
    fn replacement_switches_head_and_fences_the_predecessor_atomically() {
        let fixture = fixture();
        let predecessor_id = accept_and_ready(&fixture);
        fixture
            .store
            .publish_current(&fixture.prepared.assignment.assignment_id, &predecessor_id)
            .unwrap();
        let successor = PreparedActivationClosureV1::new(
            fixture.prepared.assignment.clone(),
            fixture.prepared.activation.clone(),
            "content-two".into(),
            fixture.prepared.owner_receipts.clone(),
            fixture.prepared.capability_preparation_receipt_id.clone(),
            fixture.prepared.participant_plan.clone(),
            fixture.prepared.binding_revision_refs.clone(),
            fixture.prepared.effective_authority_inputs.clone(),
            Some(fixture.prepared.prepared_id.clone()),
        )
        .unwrap();
        let successor_id = prepare_generation(
            &fixture,
            &successor,
            "request-two",
            LifecycleAction::Replace,
            Some(predecessor_id.clone()),
        );
        fixture
            .store
            .publish_current(&fixture.prepared.assignment.assignment_id, &successor_id)
            .unwrap();

        let assignment = fixture
            .store
            .assignment(&fixture.prepared.assignment.assignment_id)
            .unwrap()
            .unwrap();
        assert_eq!(
            assignment.current_generation_id.as_deref(),
            Some(successor_id.as_str())
        );
        assert!(assignment.generations[&successor_id].admission_open());
        let predecessor = &assignment.generations[&predecessor_id];
        assert_eq!(predecessor.status, ActivationGenerationStatus::Draining);
        assert_eq!(
            predecessor.current_admission_epoch().unwrap().status,
            AdmissionEpochStatus::Closed
        );
        fixture
            .store
            .begin_drain(&fixture.prepared.assignment.assignment_id, &predecessor_id)
            .unwrap();
    }

    #[test]
    fn recovery_requires_successor_incarnation_readiness_before_reopen() {
        let fixture = fixture();
        let generation_id = accept_and_ready(&fixture);
        fixture
            .store
            .publish_current(&fixture.prepared.assignment.assignment_id, &generation_id)
            .unwrap();
        fixture
            .store
            .interrupt(&fixture.prepared.assignment.assignment_id, &generation_id)
            .unwrap();
        let incarnation = fixture
            .store
            .create_incarnation(
                &fixture.prepared.assignment.assignment_id,
                &generation_id,
                "actor.one",
                "lease::actor.one::two".into(),
            )
            .unwrap();
        assert!(fixture
            .store
            .reopen(
                &fixture.prepared.assignment.assignment_id,
                &generation_id,
                &fixture.prepared,
            )
            .is_err());
        let context = context(&fixture, &fixture.prepared, &generation_id, "actor.one");
        assert_eq!(context.incarnation_id, incarnation.incarnation_id);
        fixture
            .store
            .record_readiness(
                &fixture.prepared.assignment.assignment_id,
                &generation_id,
                "actor.one",
                readiness(&context, "two"),
                &fixture.prepared,
            )
            .unwrap();
        let epoch = fixture
            .store
            .reopen(
                &fixture.prepared.assignment.assignment_id,
                &generation_id,
                &fixture.prepared,
            )
            .unwrap();
        assert_eq!(epoch.epoch_number, 2);
        assert_eq!(epoch.status, AdmissionEpochStatus::Open);
    }

    #[test]
    fn waits_distinguish_quiescence_from_a_broken_wake() {
        let fixture = fixture();
        let generation_id = accept_and_ready(&fixture);
        fixture
            .store
            .publish_current(&fixture.prepared.assignment.assignment_id, &generation_id)
            .unwrap();
        let generation = fixture
            .store
            .generation(&fixture.prepared.assignment.assignment_id, &generation_id)
            .unwrap()
            .unwrap();
        let wake = StructuralWakeRef::OwnerRevision("owner.one::next".into());
        let passive_wake = StructuralWakeRef::PassiveSubscription("source.one::next".into());
        fixture
            .store
            .record_wait(
                &fixture.prepared.assignment.assignment_id,
                &generation_id,
                "actor.one",
                OwnerWaitReceiptV1::new(
                    generation_id.clone(),
                    generation.incarnations["actor.one"].incarnation_id.clone(),
                    "checkpoint::actor.one".into(),
                    "no-eligible-work".into(),
                    vec![wake.clone()],
                )
                .unwrap(),
            )
            .unwrap();
        fixture
            .store
            .record_wait(
                &fixture.prepared.assignment.assignment_id,
                &generation_id,
                "source.one",
                OwnerWaitReceiptV1::new(
                    generation_id.clone(),
                    generation.incarnations["source.one"].incarnation_id.clone(),
                    "checkpoint::source.one".into(),
                    "awaiting-source-delivery".into(),
                    vec![passive_wake.clone()],
                )
                .unwrap(),
            )
            .unwrap();
        let broken = fixture
            .store
            .project_liveness(
                &fixture.prepared.assignment.assignment_id,
                &generation_id,
                &BTreeSet::new(),
                &BTreeSet::new(),
                &BTreeSet::new(),
                &BTreeSet::from([passive_wake.clone()]),
            )
            .unwrap();
        assert_eq!(broken.state, ProjectedLifecycleState::Stalled);
        assert_eq!(broken.broken_wake_refs, vec![wake.clone()]);

        let failed_despite_other_work = fixture
            .store
            .project_liveness(
                &fixture.prepared.assignment.assignment_id,
                &generation_id,
                &BTreeSet::from(["source.one".to_string()]),
                &BTreeSet::new(),
                &BTreeSet::from(["actor.one".to_string()]),
                &BTreeSet::from([wake.clone(), passive_wake.clone()]),
            )
            .unwrap();
        assert_eq!(
            failed_despite_other_work.state,
            ProjectedLifecycleState::Stalled
        );
        assert_eq!(
            failed_despite_other_work.incomplete_participants,
            vec!["actor.one"]
        );

        fixture
            .store
            .clear_wait(
                &fixture.prepared.assignment.assignment_id,
                &generation_id,
                "actor.one",
            )
            .unwrap();
        let active_idle = fixture
            .store
            .project_liveness(
                &fixture.prepared.assignment.assignment_id,
                &generation_id,
                &BTreeSet::new(),
                &BTreeSet::from(["actor.one".to_string()]),
                &BTreeSet::new(),
                &BTreeSet::from([passive_wake.clone()]),
            )
            .unwrap();
        assert_eq!(active_idle.state, ProjectedLifecycleState::ActiveIdle);

        fixture
            .store
            .record_wait(
                &fixture.prepared.assignment.assignment_id,
                &generation_id,
                "actor.one",
                OwnerWaitReceiptV1::new(
                    generation_id.clone(),
                    generation.incarnations["actor.one"].incarnation_id.clone(),
                    "checkpoint::actor.one".into(),
                    "no-eligible-work".into(),
                    vec![wake.clone()],
                )
                .unwrap(),
            )
            .unwrap();
        let quiescent = fixture
            .store
            .project_liveness(
                &fixture.prepared.assignment.assignment_id,
                &generation_id,
                &BTreeSet::new(),
                &BTreeSet::new(),
                &BTreeSet::new(),
                &BTreeSet::from([wake, passive_wake]),
            )
            .unwrap();
        assert_eq!(quiescent.state, ProjectedLifecycleState::Quiescent);
    }

    #[test]
    fn readiness_rejects_missing_or_tampered_native_owner_evidence() {
        let fixture = fixture();
        assert!(NativeOwnerReadinessEvidenceV1::new(
            "actor.one".into(),
            "owner.one".into(),
            "checkpoint".into(),
            vec!["revision".into()],
            vec!["binding".into()],
            Vec::new(),
            "proof".into(),
        )
        .is_err());
        assert!(OwnerWaitReceiptV1::new(
            "generation".into(),
            "incarnation".into(),
            "checkpoint".into(),
            "condition".into(),
            vec![StructuralWakeRef::OwnerRevision(String::new())],
        )
        .is_err());

        let acceptance = fixture
            .store
            .accept(
                ActivationLifecycleRequestV1::new(
                    "tampered-readiness".into(),
                    LifecycleAction::Activate,
                    fixture.prepared.clone(),
                    None,
                )
                .unwrap(),
            )
            .unwrap();
        let generation_id = acceptance.decision.generation_id.unwrap();
        fixture
            .store
            .realize(
                &fixture.prepared,
                &generation_id,
                &fixture.registrations,
                &BTreeSet::from(["actor.one".into(), "source.one".into()]),
            )
            .unwrap();
        fixture
            .store
            .create_incarnation(
                &fixture.prepared.assignment.assignment_id,
                &generation_id,
                "actor.one",
                "lease::actor.one".into(),
            )
            .unwrap();
        let context = context(&fixture, &fixture.prepared, &generation_id, "actor.one");
        let mut fabricated = readiness(&context, "native");
        fabricated.native_evidence.proof_position_ref = "fabricated-position".into();
        assert!(fixture
            .store
            .record_readiness(
                &fixture.prepared.assignment.assignment_id,
                &generation_id,
                "actor.one",
                fabricated,
                &fixture.prepared,
            )
            .is_err());
    }

    #[test]
    fn closure_rejects_fabricated_or_missing_owner_evidence() {
        let fixture = fixture();
        let generation_id = accept_and_ready(&fixture);
        fixture
            .store
            .publish_current(&fixture.prepared.assignment.assignment_id, &generation_id)
            .unwrap();
        fixture
            .store
            .begin_drain(&fixture.prepared.assignment.assignment_id, &generation_id)
            .unwrap();
        let actor = context(&fixture, &fixture.prepared, &generation_id, "actor.one");
        assert!(PassiveFenceReceiptV1::new(
            &actor,
            "fabricated-checkpoint".into(),
            "fabricated-proof".into(),
        )
        .is_err());

        let stop = OwnerStopReceiptV1::new(
            &actor,
            "checkpoint::actor.one::stopped".into(),
            "proof::actor.one::stopped".into(),
        )
        .unwrap();
        let mut wrong_owner = stop.clone();
        wrong_owner.owner_domain = "other.owner".into();
        assert!(fixture
            .store
            .record_stop(
                &fixture.prepared.assignment.assignment_id,
                &generation_id,
                wrong_owner,
            )
            .is_err());
        let mut wrong_safe_point = OwnerSafePointReceiptV1::new(
            &actor,
            "checkpoint::actor.one::drained".into(),
            "operations::none".into(),
            vec!["proof::actor.one::drained".into()],
        )
        .unwrap();
        wrong_safe_point.owner_domain = "other.owner".into();
        assert!(fixture
            .store
            .record_safe_point(
                &fixture.prepared.assignment.assignment_id,
                &generation_id,
                wrong_safe_point,
            )
            .is_err());
        assert!(fixture
            .store
            .commit_fenced_quiescence(&fixture.prepared.assignment.assignment_id, &generation_id,)
            .is_err());

        let generation = fixture
            .store
            .generation(&fixture.prepared.assignment.assignment_id, &generation_id)
            .unwrap()
            .unwrap();
        let mut wrong_release =
            OwnerReleaseReceiptV1::new(&actor, "proof::actor.one::released".into()).unwrap();
        wrong_release.released_binding_ref = "another-binding".into();
        assert!(validate_release_receipt(&generation, &wrong_release).is_err());
    }

    #[test]
    fn retirement_requires_owner_safe_point_and_passive_fence() {
        let fixture = fixture();
        let generation_id = accept_and_ready(&fixture);
        fixture
            .store
            .publish_current(&fixture.prepared.assignment.assignment_id, &generation_id)
            .unwrap();
        fixture
            .store
            .begin_drain(&fixture.prepared.assignment.assignment_id, &generation_id)
            .unwrap();
        assert!(fixture
            .store
            .commit_fenced_quiescence(&fixture.prepared.assignment.assignment_id, &generation_id,)
            .is_err());
        let actor = context(&fixture, &fixture.prepared, &generation_id, "actor.one");
        let source = context(&fixture, &fixture.prepared, &generation_id, "source.one");
        let actor_stop = OwnerStopReceiptV1::new(
            &actor,
            "checkpoint::actor.one::stopped".into(),
            "proof::actor.one::stopped".into(),
        )
        .unwrap();
        let source_stop = OwnerStopReceiptV1::new(
            &source,
            "checkpoint::source.one::stopped".into(),
            "proof::source.one::stopped".into(),
        )
        .unwrap();
        assert!(fixture
            .store
            .record_stop(
                &fixture.prepared.assignment.assignment_id,
                &generation_id,
                actor_stop.clone(),
            )
            .is_err());
        fixture
            .store
            .record_safe_point(
                &fixture.prepared.assignment.assignment_id,
                &generation_id,
                OwnerSafePointReceiptV1::new(
                    &actor,
                    "checkpoint::actor.one::drained".into(),
                    "operations::none".into(),
                    vec!["proof::actor.one::drained".into()],
                )
                .unwrap(),
            )
            .unwrap();
        fixture
            .store
            .record_passive_fence(
                &fixture.prepared.assignment.assignment_id,
                &generation_id,
                PassiveFenceReceiptV1::new(
                    &source,
                    "checkpoint::source.one::fenced".into(),
                    "proof::source.one::fenced".into(),
                )
                .unwrap(),
            )
            .unwrap();
        fixture
            .store
            .commit_fenced_quiescence(&fixture.prepared.assignment.assignment_id, &generation_id)
            .unwrap();
        fixture
            .store
            .record_stop(
                &fixture.prepared.assignment.assignment_id,
                &generation_id,
                source_stop,
            )
            .unwrap();
        fixture
            .store
            .record_stop(
                &fixture.prepared.assignment.assignment_id,
                &generation_id,
                actor_stop,
            )
            .unwrap();
        for owner in [&actor, &source] {
            fixture
                .store
                .record_release(
                    &fixture.prepared.assignment.assignment_id,
                    &generation_id,
                    OwnerReleaseReceiptV1::new(
                        owner,
                        format!("proof::{}::released", owner.participant_id),
                    )
                    .unwrap(),
                )
                .unwrap();
        }
        fixture
            .store
            .retire(&fixture.prepared.assignment.assignment_id, &generation_id)
            .unwrap();
        let duplicate = fixture
            .store
            .retire(&fixture.prepared.assignment.assignment_id, &generation_id)
            .unwrap();
        assert_eq!(duplicate.generation_id, generation_id);
        let assignment = fixture
            .store
            .assignment(&fixture.prepared.assignment.assignment_id)
            .unwrap()
            .unwrap();
        assert!(assignment.current_generation_id.is_none());
        assert_eq!(
            assignment.generations[&generation_id].status,
            ActivationGenerationStatus::Retired
        );
        assert_eq!(
            assignment.generations[&generation_id].stop_order,
            vec!["source.one", "actor.one"]
        );
    }
}
