//! Native Security source observation and lifecycle evidence owned by Security.

use super::{capability::SecurityCapability, observation};
use crate::runtime::assembly::{
    owner_readiness_receipt, owner_release_receipt, owner_safe_point_receipt, owner_stop_receipt,
    owner_wait_receipt, verified_native_transition, NativeOwnerLifecycle,
    NativeOwnerLifecycleSnapshot,
};
use crate::runtime::{contracts::*, error::RuntimeAssemblyError, lifecycle::*};
use meld_events::EventAppendCapability;
use meld_lang::AuthorityPolicyBinding;
use meld_world_model::lifecycle::{NativeLifecycle, NativeLifecycleEvidence};

pub const RUNTIME_ID: &str = "dependency_security.observation";

#[derive(Clone)]
pub(crate) struct SecurityObservationBinding {
    pub sources: Vec<SecurityCapability>,
    pub authority: AuthorityPolicyBinding,
    pub events: EventAppendCapability,
    pub route: meld_world_model::world_state::graph::admission::GraphOwnerEventRoute,
}

pub(crate) struct SecurityObservationActor {
    binding: SecurityObservationBinding,
    binding_id: String,
    lifecycle: NativeLifecycle,
    running: bool,
    fence: Option<(String, String)>,
    next_source: usize,
    executor: Option<tokio::runtime::Runtime>,
}

impl SecurityObservationActor {
    pub(crate) fn new(binding: SecurityObservationBinding) -> Self {
        let source_refs: Vec<_> = binding
            .sources
            .iter()
            .map(|source| {
                (
                    &source.id,
                    &source.subject,
                    &source.policy,
                    &source.workspace,
                    &source.cargo,
                    &source.advisories,
                    &source.limits,
                )
            })
            .collect();
        let binding_id = super::contracts::content_hash(&(
            source_refs,
            &binding.authority,
            binding.events.ledger_identity(),
        ))
        .expect("Security source binding serializes");
        let executor = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .ok();
        Self {
            binding,
            binding_id,
            lifecycle: NativeLifecycle::new(RUNTIME_ID),
            running: false,
            fence: None,
            next_source: 0,
            executor,
        }
    }

    fn current(&self) -> Result<Option<super::condition::CurrentSecurityCondition>, String> {
        super::condition::current(
            self.binding
                .sources
                .first()
                .ok_or("Security observer has no selected sources")?,
            &self.binding.events.replay_capability(),
        )
        .map_err(|e| e.to_string())
    }

    fn checkpoint(&self) -> Result<u64, String> {
        Ok(self
            .current()?
            .into_iter()
            .flat_map(|condition| condition.products.into_values())
            .map(|position| position.receipt_seq)
            .max()
            .unwrap_or(0))
    }

    fn wake(&self) -> StructuralWakeRef {
        StructuralWakeRef::PassiveSubscription(format!("security-source-poll::{}", self.binding_id))
    }

    pub(crate) fn tick(&mut self, budget: WorkBudget) -> WorkerTickReport {
        let input = self.checkpoint().unwrap_or(0);
        let mut report = WorkerTickReport {
            actor_id: RUNTIME_ID.into(),
            scope: WorkerScope {
                domain_id: "dependency-security".into(),
                stream_id: Some(self.binding_id.clone()),
                work_key: Some(self.binding_id.clone()),
                agent_id: None,
                perspective_key: None,
                branch_id: None,
                subject_key: self
                    .binding
                    .sources
                    .first()
                    .map(|source| source.subject.subject.index_key()),
            },
            input_checkpoint: WorkerCheckpoint {
                name: "security_source_position".into(),
                value: input,
            },
            output_checkpoint: WorkerCheckpoint {
                name: "security_source_position".into(),
                value: input,
            },
            items_attempted: 0,
            items_committed: 0,
            retryable_errors: vec![],
            fatal_errors: vec![],
            budget_exhausted: budget.max_items == 0,
            waiting_on: vec![],
        };
        if !self.running {
            report.fatal_errors.push(WorkerTickIssue {
                item_id: None,
                code: "security_observer_fenced".into(),
                message: "Security source owner is not running".into(),
            });
            return report;
        }
        if budget.max_items == 0 {
            return report;
        }
        let source = &self.binding.sources[self.next_source];
        self.next_source = (self.next_source + 1) % self.binding.sources.len();
        if let Ok(_owner) = source.publication_gate.try_lock() {
            report.items_attempted = 1;
            let (generation, incarnation) = self
                .fence
                .as_ref()
                .expect("running native source has a fence");
            let result = if source.id == super::capability::OBSERVE_INVENTORY {
                self.executor
                    .as_ref()
                    .expect("ready source has an executor")
                    .block_on(observation::poll_inventory(
                        source,
                        &self.binding.authority,
                        &self.binding_id,
                        generation,
                        incarnation,
                        &self.binding.events,
                    ))
            } else {
                observation::poll(
                    source,
                    &self.binding.authority,
                    &self.binding_id,
                    generation,
                    incarnation,
                    &self.binding.events,
                )
            };
            match result {
                Ok((committed, error)) => {
                    report.items_committed = usize::from(committed);
                    if let Some(message) = error {
                        report.retryable_errors.push(WorkerTickIssue {
                            item_id: None,
                            code: "security_source_unavailable".into(),
                            message,
                        });
                    }
                }
                Err(message) => report.retryable_errors.push(WorkerTickIssue {
                    item_id: None,
                    code: "security_observation_unresolved".into(),
                    message,
                }),
            }
        }
        match self.checkpoint() {
            Ok(position) => report.output_checkpoint.value = position,
            Err(message) => report.retryable_errors.push(WorkerTickIssue {
                item_id: None,
                code: "security_source_checkpoint_unavailable".into(),
                message,
            }),
        }
        if !report.made_progress() {
            report.waiting_on.push(WaitingOnDeclaration {
                condition: "security_source_poll".into(),
                subject_key: self
                    .binding
                    .sources
                    .first()
                    .map(|source| source.subject.subject.index_key()),
                detail: "Awaiting the bound Security source poll".into(),
                wake_refs: vec![self.wake()],
            });
        }
        report
    }

    fn evidence(&self) -> Result<NativeLifecycleEvidence, String> {
        let primary = self
            .binding
            .sources
            .first()
            .ok_or("Security observer has no selected sources")?;
        let _owner = primary
            .publication_gate
            .try_lock()
            .map_err(|_| "Security source publication is still in flight")?;
        if self.executor.is_none() {
            return Err("Security source executor is unavailable".into());
        }
        for source in &self.binding.sources {
            observation::validate_permission(
                &self.binding.authority,
                &source.subject.subject,
                &source.id,
            )?;
            source.limits.validate()?;
            if source.subject != primary.subject
                || source.policy != primary.policy
                || !std::sync::Arc::ptr_eq(&source.publication_gate, &primary.publication_gate)
            {
                return Err(
                    "Security sources must share their exact subject, policy and publication owner"
                        .into(),
                );
            }
            let paths = match source.id.as_str() {
                super::capability::ACQUIRE_ADVISORIES => vec![source.advisories.as_ref()],
                super::capability::OBSERVE_INVENTORY => {
                    vec![source.workspace.as_ref(), source.cargo.as_ref()]
                }
                _ => return Err("Security observer has a non-source Capability".into()),
            };
            if paths
                .into_iter()
                .any(|path| path.is_none_or(|path| !path.is_absolute()))
            {
                return Err("Security observation has no exact absolute source binding".into());
            }
        }
        if self.binding.route != super::condition::graph_route() {
            return Err("Security current-condition Graph route is not installed".into());
        }
        let current = self.current()?;
        let checkpoint = format!(
            "security-source::{}::{}",
            self.binding_id,
            self.checkpoint()?
        );
        let pending = current.is_some()
            && !super::condition::is_published(
                &self.binding.sources[0],
                &self.binding.events.replay_capability(),
            )
            .map_err(|e| e.to_string())?;
        Ok(NativeLifecycleEvidence {
            checkpoint_ref: checkpoint.clone(),
            installed_revision_refs: vec![
                observation::EVENT.into(),
                {
                    let policy = primary.policy.revision_ref()?;
                    format!(
                        "{}::{}::{}",
                        policy.registry, policy.id, policy.content_hash
                    )
                },
                observation::INVENTORY_EVENT.into(),
                self.binding.authority.content_hash.clone(),
                self.binding
                    .route
                    .revision_ref()
                    .map_err(|e| e.to_string())?
                    .content_hash,
            ]
            .into_iter()
            .chain(
                self.binding
                    .sources
                    .iter()
                    .map(|source| super::capability::contract(&source.id).content_identity()),
            )
            .collect(),
            binding_refs: vec![self.binding_id.clone()],
            subscription_refs: vec![format!("security-source-poll::{}", self.binding_id)],
            proof_position_ref: checkpoint,
            unresolved_operation_summary_ref: super::contracts::content_hash(&(current, pending))?,
        })
    }
}

fn failure(error: impl ToString) -> RuntimeAssemblyError {
    RuntimeAssemblyError::SupervisorHandoff(error.to_string())
}

impl NativeOwnerLifecycle for SecurityObservationActor {
    fn native_snapshot(&self) -> Result<NativeOwnerLifecycleSnapshot, RuntimeAssemblyError> {
        self.evidence().map(Into::into).map_err(failure)
    }

    fn native_readiness(
        &mut self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<OwnerReadinessReceiptV1, RuntimeAssemblyError> {
        if context.participant_id != RUNTIME_ID || context.owner_domain != "dependency-security" {
            return Err(failure(
                "Security readiness belongs to another native owner",
            ));
        }
        let evidence = self.evidence().map_err(failure)?;
        let transition = self
            .lifecycle
            .start(
                (
                    context.generation_id.clone(),
                    context.incarnation_id.clone(),
                )
                    .into(),
                &evidence.proof_position_ref,
            )
            .map_err(failure)?;
        let receipt = owner_readiness_receipt(
            context,
            evidence.into(),
            verified_native_transition(context, transition)?,
        )?;
        self.fence = Some((
            context.generation_id.clone(),
            context.incarnation_id.clone(),
        ));
        self.running = true;
        Ok(receipt)
    }

    fn native_wait(
        &self,
        context: &ParticipantLifecycleContextV1,
        report: &WorkerTickReport,
    ) -> Result<OwnerWaitReceiptV1, RuntimeAssemblyError> {
        owner_wait_receipt(context, self.native_snapshot()?, report)
    }

    fn native_safe_point(
        &mut self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<OwnerSafePointReceiptV1, RuntimeAssemblyError> {
        let evidence = self.evidence().map_err(failure)?;
        let transition = self
            .lifecycle
            .safe_point(
                (
                    context.generation_id.clone(),
                    context.incarnation_id.clone(),
                )
                    .into(),
                &evidence.proof_position_ref,
            )
            .map_err(failure)?;
        self.running = false;
        owner_safe_point_receipt(
            context,
            evidence.into(),
            verified_native_transition(context, transition)?,
        )
    }

    fn native_stop(
        &mut self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<OwnerStopReceiptV1, RuntimeAssemblyError> {
        let evidence = self.evidence().map_err(failure)?;
        let transition = self
            .lifecycle
            .stop(
                (
                    context.generation_id.clone(),
                    context.incarnation_id.clone(),
                )
                    .into(),
                &evidence.proof_position_ref,
            )
            .map_err(failure)?;
        self.running = false;
        owner_stop_receipt(
            context,
            evidence.into(),
            verified_native_transition(context, transition)?,
        )
    }

    fn native_release(
        &mut self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<OwnerReleaseReceiptV1, RuntimeAssemblyError> {
        let evidence = self.evidence().map_err(failure)?;
        let transition = self
            .lifecycle
            .release(
                (
                    context.generation_id.clone(),
                    context.incarnation_id.clone(),
                )
                    .into(),
                &evidence.proof_position_ref,
            )
            .map_err(failure)?;
        owner_release_receipt(
            context,
            evidence.into(),
            verified_native_transition(context, transition)?,
        )
    }

    fn native_resolves_wake(&self, wake: &StructuralWakeRef) -> Result<bool, RuntimeAssemblyError> {
        Ok(self.running && *wake == self.wake())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use meld_events::{EventAuthority, EventAuthorityOpenOptions};

    #[test]
    fn native_observer_authors_safe_points_and_resolves_only_its_bound_poll() {
        let root = tempfile::tempdir().unwrap();
        let events = EventAuthority::open(
            sled::open(root.path().join("events")).unwrap(),
            EventAuthorityOpenOptions::default(),
        )
        .unwrap();
        let (source, authority, _) =
            observation::tests::fixture(&root.path().join("missing-source.json"));
        let mut inventory = source.clone();
        inventory.id = super::super::capability::OBSERVE_INVENTORY.into();
        inventory.workspace = Some(root.path().join("workspace"));
        inventory.cargo = Some(env!("CARGO").into());
        inventory.advisories = None;
        let mut policy = authority.policy;
        policy
            .principal_granted_action_ids
            .push(inventory.id.clone());
        policy.runtime_allowed_action_ids.push(inventory.id.clone());
        let authority =
            AuthorityPolicyBinding::new(policy.clone(), policy.content_hash().unwrap()).unwrap();
        let binding = SecurityObservationBinding {
            sources: vec![source, inventory],
            authority,
            events: events.append_capability(),
            route: super::super::condition::graph_route(),
        };
        let mut owner = SecurityObservationActor::new(binding.clone());
        let context = ParticipantLifecycleContextV1 {
            generation_id: "generation".into(),
            incarnation_id: "incarnation".into(),
            realization_id: "realization".into(),
            participant_id: RUNTIME_ID.into(),
            owner_domain: "dependency-security".into(),
            kind: crate::theory::ParticipantKind::BoundedActor,
            readiness_contract_ref: "dependency-security.readiness.v1".into(),
            wake_contract_ref: "dependency-security.wake.v1".into(),
            safe_point_contract_ref: "dependency-security.safe-point.v1".into(),
            stop_contract_ref: "dependency-security.stop.v1".into(),
            lease_ref: "lease".into(),
        };
        owner.native_readiness(&context).unwrap();
        assert!(
            owner.native_resolves_wake(&owner.wake()).unwrap(),
            "missing files remain observable through the bound poll"
        );
        for variant in 0..3 {
            let mut foreign = binding.clone();
            match variant {
                0 => foreign.sources[0].advisories = Some(root.path().join("foreign-source.json")),
                1 => foreign.sources[1].workspace = Some(root.path().join("foreign-workspace")),
                _ => foreign.sources[1].cargo = Some(root.path().join("foreign-cargo")),
            }
            let other = SecurityObservationActor::new(foreign);
            assert!(!owner.native_resolves_wake(&other.wake()).unwrap());
        }
        let before = owner.evidence().unwrap();
        let tick = owner.tick(WorkBudget { max_items: 1 });
        assert_eq!(tick.items_committed, 1, "{tick:?}");
        assert!(tick
            .retryable_errors
            .iter()
            .any(|issue| issue.code == "security_source_unavailable"));
        let inventory_failure = owner.tick(WorkBudget { max_items: 1 });
        assert_eq!(
            inventory_failure.items_committed, 1,
            "{inventory_failure:?}"
        );
        let unchanged_failure = owner.tick(WorkBudget { max_items: 1 });
        assert_eq!(unchanged_failure.items_committed, 0);
        assert!(unchanged_failure
            .waiting_on
            .iter()
            .any(|wait| wait.wake_refs.contains(&owner.wake())));
        let after = owner.evidence().unwrap();
        assert_ne!(before.proof_position_ref, after.proof_position_ref);
        assert!(after.installed_revision_refs.contains(
            &super::super::capability::contract(super::super::capability::ACQUIRE_ADVISORIES)
                .content_identity()
        ));
        assert!(after.installed_revision_refs.contains(
            &super::super::capability::contract(super::super::capability::OBSERVE_INVENTORY)
                .content_identity()
        ));
        let mut predecessor = context.clone();
        predecessor.incarnation_id = "predecessor".into();
        assert!(owner.native_safe_point(&predecessor).is_err());
        owner.native_safe_point(&context).unwrap();
        assert!(!owner.native_resolves_wake(&owner.wake()).unwrap());
        assert!(!owner
            .tick(WorkBudget { max_items: 1 })
            .fatal_errors
            .is_empty());
        owner.native_stop(&context).unwrap();
        owner.native_release(&context).unwrap();
    }
}
