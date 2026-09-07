//! Native advisory source observation and lifecycle evidence owned by Security.

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
    pub source: SecurityCapability,
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
}

impl SecurityObservationActor {
    pub(crate) fn new(binding: SecurityObservationBinding) -> Self {
        let binding_id = super::contracts::content_hash(&(
            &binding.source.subject,
            &binding.source.policy,
            &binding.source.advisories,
            &binding.source.limits,
            &binding.authority,
            binding.events.ledger_identity(),
        ))
        .expect("Security source binding serializes");
        Self {
            binding,
            binding_id,
            lifecycle: NativeLifecycle::new(RUNTIME_ID),
            running: false,
            fence: None,
        }
    }

    fn current(&self) -> Result<Option<super::condition::CurrentSecurityCondition>, String> {
        super::condition::current(
            &self.binding.source,
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
        StructuralWakeRef::PassiveSubscription(format!(
            "security-advisory-poll::{}",
            self.binding_id
        ))
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
                subject_key: Some(self.binding.source.subject.subject.index_key()),
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
        if let Ok(_owner) = self.binding.source.publication_gate.try_lock() {
            report.items_attempted = 1;
            let (generation, incarnation) = self
                .fence
                .as_ref()
                .expect("running native source has a fence");
            match observation::poll(
                &self.binding.source,
                &self.binding.authority,
                &self.binding_id,
                generation,
                incarnation,
                &self.binding.events,
            ) {
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
                subject_key: Some(self.binding.source.subject.subject.index_key()),
                detail: "Awaiting the bound Security source poll".into(),
                wake_refs: vec![self.wake()],
            });
        }
        report
    }

    fn evidence(&self) -> Result<NativeLifecycleEvidence, String> {
        let _owner = self
            .binding
            .source
            .publication_gate
            .try_lock()
            .map_err(|_| "Security source publication is still in flight")?;
        observation::validate_permission(
            &self.binding.authority,
            &self.binding.source.subject.subject,
        )?;
        self.binding.source.limits.validate()?;
        if self.binding.source.id != super::capability::ACQUIRE_ADVISORIES {
            return Err("Security observer is not bound to its selected read capability".into());
        }
        if self
            .binding
            .source
            .advisories
            .as_ref()
            .is_none_or(|path| !path.is_absolute())
        {
            return Err("Security observation has no exact absolute source binding".into());
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
                &self.binding.source,
                &self.binding.events.replay_capability(),
            )
            .map_err(|e| e.to_string())?;
        Ok(NativeLifecycleEvidence {
            checkpoint_ref: checkpoint.clone(),
            installed_revision_refs: vec![
                observation::EVENT.into(),
                {
                    let policy = self.binding.source.policy.revision_ref()?;
                    format!(
                        "{}::{}::{}",
                        policy.registry, policy.id, policy.content_hash
                    )
                },
                super::capability::contract(super::capability::ACQUIRE_ADVISORIES)
                    .content_identity(),
                self.binding.authority.content_hash.clone(),
                self.binding
                    .route
                    .revision_ref()
                    .map_err(|e| e.to_string())?
                    .content_hash,
            ],
            binding_refs: vec![self.binding_id.clone()],
            subscription_refs: vec![format!("security-advisory-poll::{}", self.binding_id)],
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
        let binding = SecurityObservationBinding {
            source,
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
        let mut foreign = binding;
        foreign.source.advisories = Some(root.path().join("foreign-source.json"));
        let other = SecurityObservationActor::new(foreign);
        assert!(!owner.native_resolves_wake(&other.wake()).unwrap());
        let before = owner.evidence().unwrap();
        let tick = owner.tick(WorkBudget { max_items: 1 });
        assert_eq!(tick.items_committed, 1, "{tick:?}");
        assert!(tick
            .retryable_errors
            .iter()
            .any(|issue| issue.code == "security_source_unavailable"));
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
