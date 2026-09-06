//! Native observation, publication recovery, and lifecycle ownership for Docs.

use super::observation_store::DocsObservationStore;
use super::publication::OBSERVATION_SCHEMA;
use crate::runtime::assembly::{
    owner_readiness_receipt, owner_release_receipt, owner_safe_point_receipt, owner_stop_receipt,
    owner_wait_receipt, verified_native_transition, NativeOwnerLifecycle,
    NativeOwnerLifecycleSnapshot,
};
use crate::runtime::contracts::*;
use crate::runtime::error::RuntimeAssemblyError;
use crate::runtime::lifecycle::*;
use meld_events::{AppendMode, DomainObjectRef, EventAppendCapability};
use meld_world_model::lifecycle::{NativeLifecycle, NativeLifecycleEvidence};
use meld_world_model::world_state::graph::contracts::OwnerPublicationScope;
use std::path::PathBuf;
use std::sync::Arc;

pub const RUNTIME_ID: &str = "docs.observation";

#[derive(Clone)]
pub(crate) struct DocsObservationBinding {
    pub root: PathBuf,
    pub subject: DomainObjectRef,
    pub scope: OwnerPublicationScope,
    pub session_id: String,
    pub store: Arc<DocsObservationStore>,
    pub events: EventAppendCapability,
    pub route: meld_world_model::world_state::graph::admission::GraphOwnerEventRoute,
}

pub(crate) struct DocsObservationActor {
    binding: DocsObservationBinding,
    binding_id: String,
    lifecycle: NativeLifecycle,
    running: bool,
}

impl DocsObservationActor {
    pub(crate) fn new(binding: DocsObservationBinding) -> Self {
        let binding_id = identity(
            "docs-binding",
            &(
                &binding.root,
                &binding.subject,
                &binding.scope,
                &binding.route,
                binding.events.ledger_identity(),
            ),
        )
        .expect("serializable Docs binding");
        Self {
            binding,
            binding_id,
            lifecycle: NativeLifecycle::new(RUNTIME_ID),
            running: false,
        }
    }

    pub(crate) fn tick(&mut self, budget: WorkBudget) -> WorkerTickReport {
        let input = self.checkpoint().unwrap_or(0);
        let mut report = WorkerTickReport {
            actor_id: RUNTIME_ID.into(),
            scope: WorkerScope {
                domain_id: "docs".into(),
                stream_id: Some(self.binding.scope.scope_id.clone()),
                work_key: Some(self.binding_id.clone()),
                agent_id: None,
                perspective_key: self.binding.scope.perspective_id.clone(),
                branch_id: self.binding.scope.branch_id.clone(),
                subject_key: Some(self.binding.subject.index_key()),
            },
            input_checkpoint: WorkerCheckpoint {
                name: "docs_observation_sequence".into(),
                value: input,
            },
            output_checkpoint: WorkerCheckpoint {
                name: "docs_observation_sequence".into(),
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
                code: "docs_owner_fenced".into(),
                message: "Docs owner is not running".into(),
            });
            return report;
        }
        if budget.max_items == 0 {
            return report;
        }
        report.items_attempted = 1;
        match self.observe_and_publish() {
            Ok((committed, source_error)) => {
                report.items_committed = usize::from(committed);
                if let Some(message) = source_error {
                    report.retryable_errors.push(WorkerTickIssue {
                        item_id: None,
                        code: "docs_source_unavailable".into(),
                        message,
                    });
                }
            }
            Err(message) => report.retryable_errors.push(WorkerTickIssue {
                item_id: None,
                code: "docs_observation_unresolved".into(),
                message,
            }),
        }
        match self.checkpoint() {
            Ok(value) => report.output_checkpoint.value = value,
            Err(message) => report.retryable_errors.push(WorkerTickIssue {
                item_id: None,
                code: "docs_checkpoint_unavailable".into(),
                message,
            }),
        }
        if !report.made_progress() && report.retryable_errors.is_empty() {
            report.waiting_on.push(WaitingOnDeclaration {
                condition: "docs_source_unchanged".into(),
                subject_key: Some(self.binding.subject.index_key()),
                detail: "Awaiting changed bytes on the bound Docs source poll".into(),
                wake_refs: vec![self.wake()],
            });
        }
        report
    }

    fn observe_and_publish(&self) -> Result<(bool, Option<String>), String> {
        let previous = self.binding.store.head(&self.binding_id)?;
        let (revision, source_error) = if let Some(head) =
            previous.as_ref().filter(|head| head.publication.is_none())
        {
            (
                self.binding
                    .store
                    .revision(&head.revision_id)?
                    .ok_or("pending Docs revision is absent")?,
                None,
            )
        } else {
            let (evidence, error) = match super::observation::inspect_scope(&self.binding.root) {
                Ok(evidence) => (evidence, None),
                Err(error) => {
                    let message = error.to_string();
                    let observation = super::observation::DocsScopeObservation {
                        revision_id: identity("docs-source-failure", &message)?,
                        sources: vec![],
                        readmes: vec![],
                        exclusions: vec![],
                        coverage_gaps: vec![super::observation::ObservationExclusion {
                            path: ".".into(),
                            reason: message.clone(),
                        }],
                    };
                    (
                        super::capability::DocsEvidenceBundle {
                            source_fingerprint: String::new(),
                            directories: vec![],
                            observation: Some(observation),
                        },
                        Some(message),
                    )
                }
            };
            (
                self.binding.store.prepare(
                    &self.binding_id,
                    self.binding.events.ledger_identity(),
                    &self.binding.subject,
                    &self.binding.scope,
                    evidence,
                )?,
                error,
            )
        };
        if previous.as_ref().is_some_and(|head| {
            head.revision_id == revision.revision_id && head.publication.is_some()
        }) {
            return Ok((false, source_error));
        }
        let proof = self
            .binding
            .events
            .append_durable_proven(
                revision.envelope(&self.binding.session_id)?,
                AppendMode::Idempotent,
            )
            .map_err(|e| e.to_string())?;
        self.binding
            .store
            .record_publication(&self.binding_id, &revision.revision_id, &proof)?;
        Ok((true, source_error))
    }

    pub(crate) fn current_revision(
        &self,
    ) -> Result<Option<super::publication::DocsObservationRevision>, String> {
        self.binding
            .store
            .head(&self.binding_id)?
            .map(|head| {
                self.binding
                    .store
                    .revision(&head.revision_id)
                    .and_then(|revision| {
                        revision.ok_or_else(|| "Docs head revision is absent".into())
                    })
            })
            .transpose()
    }

    fn checkpoint(&self) -> Result<u64, String> {
        Ok(self
            .current_revision()?
            .map_or(0, |revision| revision.sequence))
    }

    fn wake(&self) -> StructuralWakeRef {
        StructuralWakeRef::PassiveSubscription(format!("docs-source-poll::{}", self.binding_id))
    }

    fn evidence(&self) -> Result<NativeLifecycleEvidence, String> {
        if self.binding.route != super::publication::graph_route() {
            return Err(
                "Docs observation Graph route does not match the native publication contract"
                    .into(),
            );
        }
        let route = self
            .binding
            .route
            .revision_ref()
            .map_err(|e| e.to_string())?;
        let head = self.binding.store.head(&self.binding_id)?;
        if let Some(head) = &head {
            self.binding
                .store
                .revision(&head.revision_id)?
                .ok_or("Docs lifecycle head revision is absent")?;
        }
        let checkpoint = identity("docs-checkpoint", &(&self.binding_id, &head))?;
        let pending = head.as_ref().filter(|head| head.publication.is_none());
        Ok(NativeLifecycleEvidence {
            checkpoint_ref: checkpoint.clone(),
            installed_revision_refs: vec![
                OBSERVATION_SCHEMA.into(),
                format!("{}::{}::{}", route.registry, route.id, route.content_hash),
            ],
            binding_refs: vec![self.binding_id.clone()],
            subscription_refs: vec![format!("docs-source-poll::{}", self.binding_id)],
            proof_position_ref: checkpoint,
            unresolved_operation_summary_ref: identity("docs-pending", &pending)?,
        })
    }
}

fn identity(kind: &str, value: &impl serde::Serialize) -> Result<String, String> {
    Ok(format!(
        "{kind}::{}",
        blake3::hash(&serde_json::to_vec(value).map_err(|e| e.to_string())?).to_hex()
    ))
}

fn failure(error: impl ToString) -> RuntimeAssemblyError {
    RuntimeAssemblyError::SupervisorHandoff(error.to_string())
}

impl NativeOwnerLifecycle for DocsObservationActor {
    fn native_snapshot(&self) -> Result<NativeOwnerLifecycleSnapshot, RuntimeAssemblyError> {
        self.evidence().map(Into::into).map_err(failure)
    }

    fn native_readiness(
        &mut self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<OwnerReadinessReceiptV1, RuntimeAssemblyError> {
        if context.participant_id != RUNTIME_ID || context.owner_domain != "docs" {
            return Err(failure("Docs readiness belongs to another native owner"));
        }
        if !self.binding.root.is_dir() {
            return Err(failure("Docs source directory is unavailable"));
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
        self.binding.store.flush().map_err(failure)?;
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
        Ok(self.running && *wake == self.wake() && self.binding.root.is_dir())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use meld_events::{EventAuthority, EventAuthorityOpenOptions};

    fn actor(
        root: &std::path::Path,
        db: &std::path::Path,
        authority: &EventAuthority,
    ) -> DocsObservationActor {
        let mut actor = DocsObservationActor::new(DocsObservationBinding {
            root: root.into(),
            subject: DomainObjectRef::new("workspace_fs", "node", "scope").unwrap(),
            scope: OwnerPublicationScope {
                scope_id: "scope".into(),
                branch_id: None,
                perspective_id: None,
                valid_at: None,
            },
            session_id: "docs-observation-test".into(),
            store: Arc::new(DocsObservationStore::new(sled::open(db).unwrap()).unwrap()),
            events: authority.append_capability(),
            route: super::super::publication::graph_route(),
        });
        actor.running = true;
        actor
    }

    #[test]
    fn pending_publication_reopens_exactly_before_observing_new_source() {
        let source = tempfile::tempdir().unwrap();
        let storage = tempfile::tempdir().unwrap();
        let authority = EventAuthority::open(
            sled::open(storage.path().join("events")).unwrap(),
            EventAuthorityOpenOptions::default(),
        )
        .unwrap();
        std::fs::write(source.path().join("lib.rs"), "original source\n").unwrap();
        let first = actor(source.path(), &storage.path().join("docs"), &authority);
        let revision = first
            .binding
            .store
            .prepare(
                &first.binding_id,
                authority.ledger_identity(),
                &first.binding.subject,
                &first.binding.scope,
                super::super::observation::inspect_scope(source.path()).unwrap(),
            )
            .unwrap();
        authority
            .append_capability()
            .append_durable_proven(
                revision.envelope("docs-observation-test").unwrap(),
                AppendMode::Idempotent,
            )
            .unwrap();
        drop(first);
        std::fs::write(source.path().join("lib.rs"), "successor source\n").unwrap();
        let mut recovered = actor(source.path(), &storage.path().join("docs"), &authority);
        assert_eq!(
            recovered.current_revision().unwrap(),
            Some(revision.clone())
        );
        let before = authority.watermark_capability().snapshot().unwrap();
        let resumed = recovered.tick(WorkBudget { max_items: 1 });
        assert_eq!(resumed.items_committed, 1, "{resumed:?}");
        assert_eq!(authority.watermark_capability().snapshot().unwrap(), before);
        assert_eq!(
            recovered.current_revision().unwrap(),
            Some(revision.clone())
        );
        let changed = recovered.tick(WorkBudget { max_items: 1 });
        assert_eq!(changed.items_committed, 1, "{changed:?}");
        let next = recovered.current_revision().unwrap().unwrap();
        assert_eq!(next.predecessor, Some(revision.revision_id.clone()));
        assert_eq!(next.sequence, 2);
        assert_eq!(
            authority.watermark_capability().snapshot().unwrap().tip_seq,
            2
        );
        let idle = recovered.tick(WorkBudget { max_items: 1 });
        assert_eq!(idle.items_committed, 0);
        assert_eq!(recovered.current_revision().unwrap(), Some(next));
        assert_eq!(
            recovered
                .binding
                .store
                .revision(&revision.revision_id)
                .unwrap(),
            Some(revision)
        );
    }

    #[test]
    fn reused_content_gets_a_successor_and_foreign_receipt_cannot_close_publication() {
        let source = tempfile::tempdir().unwrap();
        let storage = tempfile::tempdir().unwrap();
        let authority = EventAuthority::open(
            sled::open(storage.path().join("events")).unwrap(),
            EventAuthorityOpenOptions::default(),
        )
        .unwrap();
        let foreign = EventAuthority::open(
            sled::open(storage.path().join("foreign")).unwrap(),
            EventAuthorityOpenOptions::default(),
        )
        .unwrap();
        std::fs::write(source.path().join("lib.rs"), "A\n").unwrap();
        let mut owner = actor(source.path(), &storage.path().join("docs"), &authority);
        let first = owner
            .binding
            .store
            .prepare(
                &owner.binding_id,
                authority.ledger_identity(),
                &owner.binding.subject,
                &owner.binding.scope,
                super::super::observation::inspect_scope(source.path()).unwrap(),
            )
            .unwrap();
        let proof = foreign
            .append_capability()
            .append_durable_proven(first.envelope("foreign").unwrap(), AppendMode::Idempotent)
            .unwrap();
        assert!(owner
            .binding
            .store
            .record_publication(&owner.binding_id, &first.revision_id, &proof)
            .is_err());
        assert!(owner
            .binding
            .store
            .head(&owner.binding_id)
            .unwrap()
            .unwrap()
            .publication
            .is_none());
        assert!(owner
            .binding
            .store
            .prepare(
                &owner.binding_id,
                foreign.ledger_identity(),
                &owner.binding.subject,
                &owner.binding.scope,
                first.evidence.clone()
            )
            .is_err());
        owner.tick(WorkBudget { max_items: 1 });
        std::fs::write(source.path().join("lib.rs"), "B\n").unwrap();
        owner.tick(WorkBudget { max_items: 1 });
        let second = owner.current_revision().unwrap().unwrap();
        std::fs::write(source.path().join("lib.rs"), "A\n").unwrap();
        owner.tick(WorkBudget { max_items: 1 });
        let third = owner.current_revision().unwrap().unwrap();
        assert_eq!(third.evidence, first.evidence);
        assert_ne!(third.revision_id, first.revision_id);
        assert_eq!(third.predecessor, Some(second.revision_id));
        assert_eq!(third.sequence, 3);
        assert_eq!(
            authority.watermark_capability().snapshot().unwrap().tip_seq,
            3
        );
        let mut altered = third.clone();
        altered.sequence = 1;
        assert!(altered.publication().is_err());
    }

    #[test]
    fn wake_resolution_is_bound_to_the_exact_source_and_running_owner() {
        let source = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        let storage = tempfile::tempdir().unwrap();
        let authority = EventAuthority::open(
            sled::open(storage.path().join("events")).unwrap(),
            EventAuthorityOpenOptions::default(),
        )
        .unwrap();
        let mut owner = actor(source.path(), &storage.path().join("docs"), &authority);
        let peer = actor(other.path(), &storage.path().join("peer"), &authority);
        assert!(owner.native_resolves_wake(&owner.wake()).unwrap());
        assert!(!owner.native_resolves_wake(&peer.wake()).unwrap());
        assert!(!owner
            .native_resolves_wake(&StructuralWakeRef::PassiveSubscription(
                "docs-source-poll::foreign".into()
            ))
            .unwrap());
        owner.running = false;
        assert!(!owner.native_resolves_wake(&owner.wake()).unwrap());
    }
}
