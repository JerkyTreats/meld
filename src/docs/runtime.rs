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
    pub claim_policy: Option<super::claim_validation::DocsClaimPolicyRevision>,
    pub claim_config: Option<super::capability::DocsCapabilityConfig>,
    pub claim_judge: super::claim_observation::DocsClaimJudgeSlot,
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
        let (mut revision, source_error) = if let Some(head) =
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
            let Some(policy) = &self.binding.claim_policy else {
                return Ok((false, source_error));
            };
            if revision
                .evidence
                .observation
                .as_ref()
                .is_none_or(|observation| observation.readmes.is_empty())
            {
                return Ok((false, source_error));
            }
            let completed_readme =
                if revision.source_claims.is_none() && revision.claim_report.is_none() {
                    self.binding
                        .store
                        .claim_progress(
                            &self.binding_id,
                            &revision
                                .evidence
                                .observation
                                .as_ref()
                                .ok_or("Docs source has no observation")?
                                .revision_id,
                            &policy.content_identity,
                        )?
                        .filter(|report| report.complete)
                } else {
                    None
                };
            let stale_policy = revision
                .claim_report
                .as_ref()
                .is_some_and(|report| report.policy_identity != policy.content_identity)
                || revision
                    .source_claims
                    .as_ref()
                    .is_some_and(|report| report.policy_identity != policy.content_identity);
            if stale_policy {
                revision = self
                    .binding
                    .store
                    .invalidate_judgments(&self.binding_id, &revision.revision_id)?;
            } else if let Some(report) = completed_readme {
                if !report
                    .matches_observation(&policy.policy, &revision.evidence)
                    .map_err(|error| error.to_string())?
                {
                    return Err("Docs retained README judgment changed input binding".into());
                }
                revision = self.binding.store.prepare_claim_report(
                    &self.binding_id,
                    &revision.revision_id,
                    report,
                )?;
            } else if revision.source_claims.is_none() {
                let input =
                    super::source_claims::source_input_identity(&policy.policy, &revision.evidence)
                        .map_err(|error| error.to_string())?;
                let prior = self
                    .binding
                    .store
                    .source_progress(&self.binding_id, &input)?;
                let report = if let Some(report) = prior.as_ref().filter(|report| report.complete) {
                    if !report
                        .matches_input(&policy.policy, &revision.evidence)
                        .map_err(|error| error.to_string())?
                    {
                        return Err("Docs retained source claims changed input binding".into());
                    }
                    report.clone()
                } else {
                    let judge = self
                        .binding
                        .claim_judge
                        .current()
                        .ok_or("Docs source-claim provider is not bound")?;
                    let runtime = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .map_err(|error| error.to_string())?;
                    runtime
                        .block_on(super::source_claims::advance_source_claims(
                            judge.as_ref(),
                            &policy.policy,
                            &revision.evidence,
                            prior.as_ref(),
                            1,
                        ))
                        .map_err(|error| error.to_string())?
                };
                self.binding.store.save_source_progress(
                    &self.binding_id,
                    &revision.revision_id,
                    &report,
                )?;
                if !report.complete {
                    return Ok((true, source_error));
                }
                revision = self.binding.store.prepare_source_claims(
                    &self.binding_id,
                    &revision.revision_id,
                    report,
                )?;
            } else if revision.claim_report.is_some() {
                if revision.correspondence.is_some() {
                    return Ok((false, source_error));
                }
                let source_claims = revision
                    .source_claims
                    .as_ref()
                    .expect("source claims prepared");
                let input = super::correspondence::input_identity(
                    &policy.policy,
                    &revision.evidence,
                    source_claims,
                )
                .map_err(|error| error.to_string())?;
                let prior = self
                    .binding
                    .store
                    .correspondence_progress(&self.binding_id, &input)?;
                let report = if let Some(report) = prior.as_ref().filter(|report| report.complete) {
                    report
                        .validate_capture(&revision.evidence, source_claims)
                        .map_err(|error| error.to_string())?;
                    report.clone()
                } else {
                    let judge = self
                        .binding
                        .claim_judge
                        .current()
                        .ok_or("Docs correspondence provider is not bound")?;
                    let runtime = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .map_err(|error| error.to_string())?;
                    runtime
                        .block_on(super::correspondence::advance_correspondence(
                            judge.as_ref(),
                            &policy.policy,
                            &revision.evidence,
                            source_claims,
                            prior.as_ref(),
                            1,
                        ))
                        .map_err(|error| error.to_string())?
                };
                self.binding.store.save_correspondence_progress(
                    &self.binding_id,
                    &revision.revision_id,
                    &report,
                )?;
                if !report.complete {
                    return Ok((true, source_error));
                }
                revision = self.binding.store.prepare_correspondence(
                    &self.binding_id,
                    &revision.revision_id,
                    report,
                )?;
            } else {
                let observation_id = &revision
                    .evidence
                    .observation
                    .as_ref()
                    .ok_or("Docs claim source has no observation")?
                    .revision_id;
                let prior = self.binding.store.claim_progress(
                    &self.binding_id,
                    observation_id,
                    &policy.content_identity,
                )?;
                let report = if let Some(report) = prior.as_ref().filter(|report| report.complete) {
                    if !report
                        .matches_observation(&policy.policy, &revision.evidence)
                        .map_err(|error| error.to_string())?
                    {
                        return Err("Docs retained claim report changed input binding".into());
                    }
                    report.clone()
                } else {
                    let judge = self
                        .binding
                        .claim_judge
                        .current()
                        .ok_or("Docs claim judgment provider is not bound")?;
                    let runtime = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .map_err(|error| error.to_string())?;
                    runtime
                        .block_on(super::claim_observation::advance_observed_claims(
                            judge.as_ref(),
                            &policy.policy,
                            &revision.evidence,
                            prior.as_ref(),
                            1,
                        ))
                        .map_err(|error| error.to_string())?
                };
                self.binding.store.save_claim_progress(
                    &self.binding_id,
                    &revision.revision_id,
                    &report,
                )?;
                if !report.complete {
                    return Ok((true, source_error));
                }
                revision = self.binding.store.prepare_claim_report(
                    &self.binding_id,
                    &revision.revision_id,
                    report,
                )?;
            }
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
        if let Some(policy) = &self.binding.claim_policy {
            policy
                .policy
                .validate()
                .map_err(|error| error.to_string())?;
            if policy.content_identity != policy.policy.content_identity() {
                return Err("Docs claim policy revision identity is invalid".into());
            }
        }
        let current = self.current_revision()?;
        let claim_progress = match current.as_ref().zip(self.binding.claim_policy.as_ref()) {
            Some((revision, policy)) => match &revision.evidence.observation {
                Some(observation) => self.binding.store.claim_progress(
                    &self.binding_id,
                    &observation.revision_id,
                    &policy.content_identity,
                )?,
                None => None,
            },
            None => None,
        };
        let source_progress = match current.as_ref().zip(self.binding.claim_policy.as_ref()) {
            Some((revision, policy)) => match super::source_claims::source_input_identity(
                &policy.policy,
                &revision.evidence,
            ) {
                Ok(input) => self
                    .binding
                    .store
                    .source_progress(&self.binding_id, &input)?,
                Err(_) => None,
            },
            None => None,
        };
        let correspondence_progress = match current.as_ref().zip(self.binding.claim_policy.as_ref())
        {
            Some((revision, policy)) => match revision.source_claims.as_ref() {
                Some(sources) => match super::correspondence::input_identity(
                    &policy.policy,
                    &revision.evidence,
                    sources,
                ) {
                    Ok(input) => self
                        .binding
                        .store
                        .correspondence_progress(&self.binding_id, &input)?,
                    Err(_) => None,
                },
                None => None,
            },
            None => None,
        };
        let checkpoint = identity(
            "docs-checkpoint",
            &(
                &self.binding_id,
                &head,
                &claim_progress,
                &source_progress,
                &correspondence_progress,
            ),
        )?;
        let pending = head.as_ref().filter(|head| head.publication.is_none());
        let claim_obligation = current
            .as_ref()
            .zip(self.binding.claim_policy.as_ref())
            .filter(|(revision, policy)| {
                revision
                    .evidence
                    .observation
                    .as_ref()
                    .is_some_and(|observation| !observation.readmes.is_empty())
                    && revision
                        .claim_report
                        .as_ref()
                        .is_none_or(|report| report.policy_identity != policy.content_identity)
            })
            .map(|(revision, policy)| (&revision.revision_id, &policy.content_identity));
        let source_obligation = current
            .as_ref()
            .zip(self.binding.claim_policy.as_ref())
            .filter(|(revision, policy)| {
                revision
                    .evidence
                    .observation
                    .as_ref()
                    .is_some_and(|observation| !observation.sources.is_empty())
                    && revision
                        .source_claims
                        .as_ref()
                        .is_none_or(|report| report.policy_identity != policy.content_identity)
            })
            .map(|(revision, policy)| (&revision.revision_id, &policy.content_identity));
        let correspondence_obligation = current
            .as_ref()
            .zip(self.binding.claim_policy.as_ref())
            .filter(|(revision, policy)| {
                revision
                    .evidence
                    .observation
                    .as_ref()
                    .is_some_and(|observation| !observation.readmes.is_empty())
                    && revision
                        .correspondence
                        .as_ref()
                        .is_none_or(|report| report.policy_identity != policy.content_identity)
            })
            .map(|(revision, policy)| (&revision.revision_id, &policy.content_identity));
        let mut binding_refs = vec![self.binding_id.clone()];
        if let Some(config) = &self.binding.claim_config {
            binding_refs.push(identity(
                "docs-claim-provider",
                &(&config.provider, &config.agent_id, &config.subject_id),
            )?);
        }
        Ok(NativeLifecycleEvidence {
            checkpoint_ref: checkpoint.clone(),
            installed_revision_refs: std::iter::once(OBSERVATION_SCHEMA.into())
                .chain(std::iter::once(format!(
                    "{}::{}::{}",
                    route.registry, route.id, route.content_hash
                )))
                .chain(
                    self.binding
                        .claim_policy
                        .as_ref()
                        .into_iter()
                        .flat_map(|_| {
                            [
                                super::source_claims::SOURCE_CLAIM_CONTRACT.into(),
                                super::correspondence::CORRESPONDENCE_CONTRACT.into(),
                            ]
                        }),
                )
                .chain(
                    self.binding
                        .claim_policy
                        .as_ref()
                        .map(|policy| policy.content_identity.clone()),
                )
                .collect(),
            binding_refs,
            subscription_refs: vec![format!("docs-source-poll::{}", self.binding_id)],
            proof_position_ref: checkpoint,
            unresolved_operation_summary_ref: identity(
                "docs-pending",
                &(
                    pending,
                    claim_obligation,
                    source_obligation,
                    correspondence_obligation,
                ),
            )?,
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
            claim_policy: None,
            claim_config: None,
            claim_judge: Default::default(),
        });
        actor.running = true;
        actor
    }

    fn enable_claims(
        owner: &mut DocsObservationActor,
        judge: Option<Arc<dyn super::super::claim_validation::DocsClaimJudge>>,
    ) {
        let policy = super::super::claim_observation::test_support::policy();
        owner.binding.claim_policy =
            Some(super::super::claim_validation::DocsClaimPolicyRevision {
                content_identity: policy.content_identity(),
                policy,
                installed_at_seq: 1,
            });
        if let Some(judge) = judge {
            assert!(owner.binding.claim_judge.bind(judge));
        }
    }

    #[test]
    fn correspondence_resumes_per_readme_and_publishes_omissions_without_repair() {
        use super::super::claim_observation::test_support::FixtureJudge;
        use std::sync::atomic::Ordering;
        let source = tempfile::tempdir().unwrap();
        let storage = tempfile::tempdir().unwrap();
        for directory in [".", "child"] {
            let root = source.path().join(directory);
            std::fs::create_dir_all(&root).unwrap();
            std::fs::write(root.join("lib.rs"), "pub fn run() {}\npub fn stop() {}\n").unwrap();
            std::fs::write(root.join("README.md"), "`run` exists.\n").unwrap();
        }
        let authority = EventAuthority::open(
            sled::open(storage.path().join("events")).unwrap(),
            EventAuthorityOpenOptions::default(),
        )
        .unwrap();
        let judge = Arc::new(FixtureJudge::default());
        let mut first = actor(source.path(), &storage.path().join("docs"), &authority);
        enable_claims(&mut first, Some(judge.clone()));
        for _ in 0..5 {
            let tick = first.tick(WorkBudget { max_items: 1 });
            assert!(tick.retryable_errors.is_empty(), "{tick:?}");
        }
        let assessed = first.current_revision().unwrap().unwrap();
        assert!(assessed.claim_report.is_some());
        let checkpoint = first.evidence().unwrap().proof_position_ref;
        first.tick(WorkBudget { max_items: 1 });
        assert_eq!(judge.correspondence_calls.load(Ordering::SeqCst), 1);
        assert_eq!(first.current_revision().unwrap(), Some(assessed.clone()));
        assert_ne!(first.evidence().unwrap().proof_position_ref, checkpoint);
        drop(first);
        let mut resumed = actor(source.path(), &storage.path().join("docs"), &authority);
        enable_claims(&mut resumed, Some(judge.clone()));
        let tick = resumed.tick(WorkBudget { max_items: 1 });
        assert!(tick.retryable_errors.is_empty(), "{tick:?}");
        assert_eq!(judge.correspondence_calls.load(Ordering::SeqCst), 2);
        let completed = resumed.current_revision().unwrap().unwrap();
        let report = completed.correspondence.as_ref().unwrap();
        assert!(report.complete);
        assert_eq!(report.readmes.len(), 2);
        assert!(report.readmes.iter().all(|readme| readme
            .claims
            .iter()
            .any(|claim| claim.readme_claim_ids.is_empty())));
        let operation = completed.publication().unwrap();
        assert!(operation.batch.objects.iter().any(|object| object
            .qualifications
            .get("correspondence")
            .is_some_and(|value| value == "missing")));
        assert!(operation
            .batch
            .relations
            .iter()
            .any(|relation| relation.relation_type == "docs_correspondence_match"));
        assert_eq!(
            std::fs::read_to_string(source.path().join("README.md")).unwrap(),
            "`run` exists.\n"
        );
        resumed.tick(WorkBudget { max_items: 1 });
        assert_eq!(judge.correspondence_calls.load(Ordering::SeqCst), 2);
        assert_eq!(resumed.current_revision().unwrap(), Some(completed.clone()));
        assert!(resumed
            .binding
            .store
            .save_correspondence_progress(&resumed.binding_id, &assessed.revision_id, report)
            .is_err());
        std::fs::write(
            source.path().join("README.md"),
            "`run` exists.\n`stop` exists.\n",
        )
        .unwrap();
        resumed.tick(WorkBudget { max_items: 1 });
        assert!(resumed
            .current_revision()
            .unwrap()
            .unwrap()
            .correspondence
            .is_none());
        assert_eq!(
            resumed
                .binding
                .store
                .revision(&completed.revision_id)
                .unwrap(),
            Some(completed)
        );
    }

    #[test]
    fn completed_correspondence_recovers_at_each_publication_boundary_without_a_judge() {
        use super::super::claim_observation::test_support::FixtureJudge;
        for boundary in 0..3 {
            let source = tempfile::tempdir().unwrap();
            let storage = tempfile::tempdir().unwrap();
            std::fs::write(
                source.path().join("lib.rs"),
                "pub fn run() {}\npub fn stop() {}\n",
            )
            .unwrap();
            std::fs::write(source.path().join("README.md"), "`run` exists.\n").unwrap();
            let authority = EventAuthority::open(
                sled::open(storage.path().join("events")).unwrap(),
                EventAuthorityOpenOptions::default(),
            )
            .unwrap();
            let mut first = actor(source.path(), &storage.path().join("docs"), &authority);
            enable_claims(&mut first, Some(Arc::new(FixtureJudge::default())));
            for _ in 0..3 {
                first.tick(WorkBudget { max_items: 1 });
            }
            let assessed = first.current_revision().unwrap().unwrap();
            assert!(assessed.claim_report.is_some());
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            let report = runtime
                .block_on(super::super::correspondence::advance_correspondence(
                    &FixtureJudge::default(),
                    &first.binding.claim_policy.as_ref().unwrap().policy,
                    &assessed.evidence,
                    assessed.source_claims.as_ref().unwrap(),
                    None,
                    1,
                ))
                .unwrap();
            first
                .binding
                .store
                .save_correspondence_progress(&first.binding_id, &assessed.revision_id, &report)
                .unwrap();
            let pending = if boundary > 0 {
                let pending = first
                    .binding
                    .store
                    .prepare_correspondence(
                        &first.binding_id,
                        &assessed.revision_id,
                        report.clone(),
                    )
                    .unwrap();
                if boundary == 2 {
                    authority
                        .append_capability()
                        .append_durable_proven(
                            pending.envelope("docs-observation-test").unwrap(),
                            AppendMode::Idempotent,
                        )
                        .unwrap();
                }
                Some(pending)
            } else {
                None
            };
            drop(first);
            let mut recovered = actor(source.path(), &storage.path().join("docs"), &authority);
            enable_claims(&mut recovered, None);
            let tick = recovered.tick(WorkBudget { max_items: 1 });
            assert!(tick.retryable_errors.is_empty(), "{boundary}: {tick:?}");
            assert_eq!(tick.items_committed, 1);
            let resumed = recovered.current_revision().unwrap().unwrap();
            assert_eq!(resumed.correspondence, Some(report));
            if let Some(pending) = pending {
                assert_eq!(resumed, pending);
            }
            assert_eq!(
                authority.watermark_capability().snapshot().unwrap().tip_seq,
                4
            );
        }
    }

    #[test]
    fn source_claim_work_resumes_per_file_and_ignores_readme_only_changes() {
        use super::super::claim_observation::test_support::FixtureJudge;
        use std::sync::atomic::Ordering;
        let source = tempfile::tempdir().unwrap();
        let storage = tempfile::tempdir().unwrap();
        for file in ["a.rs", "b.rs"] {
            std::fs::write(source.path().join(file), "pub fn run() {}\n").unwrap();
        }
        let authority = EventAuthority::open(
            sled::open(storage.path().join("events")).unwrap(),
            EventAuthorityOpenOptions::default(),
        )
        .unwrap();
        let judge = Arc::new(FixtureJudge::default());
        let mut first = actor(source.path(), &storage.path().join("docs"), &authority);
        enable_claims(&mut first, Some(judge.clone()));
        first.tick(WorkBudget { max_items: 1 });
        let raw = first.current_revision().unwrap().unwrap();
        let checkpoint = first.evidence().unwrap().proof_position_ref;
        first.tick(WorkBudget { max_items: 1 });
        assert_eq!(judge.source_calls.load(Ordering::SeqCst), 1);
        assert_eq!(first.current_revision().unwrap(), Some(raw.clone()));
        assert_ne!(first.evidence().unwrap().proof_position_ref, checkpoint);
        drop(first);
        let mut recovered = actor(source.path(), &storage.path().join("docs"), &authority);
        enable_claims(&mut recovered, Some(judge.clone()));
        recovered.tick(WorkBudget { max_items: 1 });
        let published = recovered.current_revision().unwrap().unwrap();
        assert!(published.source_claims.as_ref().unwrap().complete);
        assert_eq!(published.source_claims.as_ref().unwrap().files.len(), 2);
        assert_eq!(judge.source_calls.load(Ordering::SeqCst), 2);
        assert_eq!(judge.calls.load(Ordering::SeqCst), 0);
        std::fs::write(source.path().join("README.md"), "# run\n").unwrap();
        recovered.tick(WorkBudget { max_items: 1 });
        let raw_changed = recovered.current_revision().unwrap().unwrap();
        assert!(raw_changed.source_claims.is_none());
        recovered.tick(WorkBudget { max_items: 1 });
        let reused = recovered.current_revision().unwrap().unwrap();
        assert_eq!(reused.source_claims, published.source_claims);
        assert_eq!(judge.source_calls.load(Ordering::SeqCst), 2);
        assert_ne!(reused.revision_id, published.revision_id);
        assert!(recovered
            .binding
            .store
            .save_source_progress(
                &recovered.binding_id,
                &raw.revision_id,
                reused.source_claims.as_ref().unwrap()
            )
            .is_err());
    }

    #[test]
    fn completed_source_claims_recover_without_repeating_extraction() {
        use super::super::claim_observation::test_support::FixtureJudge;
        for boundary in 0..3 {
            let source = tempfile::tempdir().unwrap();
            let storage = tempfile::tempdir().unwrap();
            std::fs::write(source.path().join("lib.rs"), "pub fn run() {}\n").unwrap();
            let authority = EventAuthority::open(
                sled::open(storage.path().join("events")).unwrap(),
                EventAuthorityOpenOptions::default(),
            )
            .unwrap();
            let mut first = actor(source.path(), &storage.path().join("docs"), &authority);
            enable_claims(&mut first, None);
            first.tick(WorkBudget { max_items: 1 });
            let raw = first.current_revision().unwrap().unwrap();
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            let report = runtime
                .block_on(super::super::source_claims::advance_source_claims(
                    &FixtureJudge::default(),
                    &first.binding.claim_policy.as_ref().unwrap().policy,
                    &raw.evidence,
                    None,
                    1,
                ))
                .unwrap();
            first
                .binding
                .store
                .save_source_progress(&first.binding_id, &raw.revision_id, &report)
                .unwrap();
            let pending = if boundary > 0 {
                let pending = first
                    .binding
                    .store
                    .prepare_source_claims(&first.binding_id, &raw.revision_id, report.clone())
                    .unwrap();
                if boundary == 2 {
                    authority
                        .append_capability()
                        .append_durable_proven(
                            pending.envelope("docs-observation-test").unwrap(),
                            AppendMode::Idempotent,
                        )
                        .unwrap();
                }
                Some(pending)
            } else {
                None
            };
            drop(first);
            let mut recovered = actor(source.path(), &storage.path().join("docs"), &authority);
            enable_claims(&mut recovered, None);
            let tick = recovered.tick(WorkBudget { max_items: 1 });
            assert!(tick.retryable_errors.is_empty(), "{tick:?}");
            assert_eq!(tick.items_committed, 1);
            let resumed = recovered.current_revision().unwrap().unwrap();
            assert_eq!(resumed.source_claims, Some(report));
            if let Some(pending) = pending {
                assert_eq!(resumed, pending);
            }
            assert_eq!(
                authority.watermark_capability().snapshot().unwrap().tip_seq,
                2
            );
        }
    }

    #[test]
    fn bounded_claim_judgments_resume_after_reopen_without_repeating_completed_readmes() {
        use super::super::claim_observation::test_support::FixtureJudge;
        use std::sync::atomic::Ordering;
        let source = tempfile::tempdir().unwrap();
        let storage = tempfile::tempdir().unwrap();
        for directory in [".", "a", "b"] {
            let root = source.path().join(directory);
            std::fs::create_dir_all(&root).unwrap();
            std::fs::write(root.join("lib.rs"), "pub fn run() {}\n").unwrap();
            std::fs::write(root.join("README.md"), "# run\n").unwrap();
        }
        let authority = EventAuthority::open(
            sled::open(storage.path().join("events")).unwrap(),
            EventAuthorityOpenOptions::default(),
        )
        .unwrap();
        let judge = Arc::new(FixtureJudge::default());
        let mut first = actor(source.path(), &storage.path().join("docs"), &authority);
        enable_claims(&mut first, Some(judge.clone()));
        first.tick(WorkBudget { max_items: 1 });
        for _ in 0..3 {
            first.tick(WorkBudget { max_items: 1 });
        }
        let raw = first.current_revision().unwrap().unwrap();
        let before = first.evidence().unwrap().proof_position_ref;
        let tick = first.tick(WorkBudget { max_items: 1 });
        assert_eq!(tick.items_committed, 1, "{tick:?}");
        assert_eq!(judge.calls.load(Ordering::SeqCst), 1);
        assert_eq!(first.current_revision().unwrap(), Some(raw.clone()));
        assert_ne!(before, first.evidence().unwrap().proof_position_ref);
        let progress = first
            .binding
            .store
            .claim_progress(
                &first.binding_id,
                &raw.evidence.observation.as_ref().unwrap().revision_id,
                &first
                    .binding
                    .claim_policy
                    .as_ref()
                    .unwrap()
                    .content_identity,
            )
            .unwrap()
            .unwrap();
        assert!(!progress.complete);
        assert_eq!(progress.readmes.len(), 1);
        assert!(raw.clone().with_claim_report(progress).is_err());
        drop(first);
        let mut recovered = actor(source.path(), &storage.path().join("docs"), &authority);
        enable_claims(&mut recovered, Some(judge.clone()));
        recovered.tick(WorkBudget { max_items: 1 });
        assert_eq!(judge.calls.load(Ordering::SeqCst), 2);
        assert!(recovered
            .current_revision()
            .unwrap()
            .unwrap()
            .claim_report
            .is_none());
        recovered.tick(WorkBudget { max_items: 1 });
        assert_eq!(judge.calls.load(Ordering::SeqCst), 3);
        let complete = recovered.current_revision().unwrap().unwrap();
        assert!(complete.claim_report.as_ref().unwrap().complete);
        assert_eq!(complete.claim_report.as_ref().unwrap().readmes.len(), 3);
        recovered.tick(WorkBudget { max_items: 1 });
        assert_eq!(judge.calls.load(Ordering::SeqCst), 3);
        assert_eq!(
            authority.watermark_capability().snapshot().unwrap().tip_seq,
            3
        );
        let selected = recovered.binding.claim_policy.as_mut().unwrap();
        selected.policy.title_weight += 1.0;
        selected.content_identity = selected.policy.content_identity();
        recovered.tick(WorkBudget { max_items: 1 });
        let invalidated = recovered.current_revision().unwrap().unwrap();
        assert!(invalidated.claim_report.is_none());
        assert_eq!(invalidated.predecessor, Some(complete.revision_id));
        assert_eq!(judge.calls.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn completed_claim_publication_recovers_without_a_judge_or_repeated_semantic_work() {
        use super::super::claim_observation::{assess_observed_claims, test_support::FixtureJudge};
        for boundary in 0..3 {
            let source = tempfile::tempdir().unwrap();
            let storage = tempfile::tempdir().unwrap();
            std::fs::write(source.path().join("lib.rs"), "pub fn run() {}\n").unwrap();
            std::fs::write(source.path().join("README.md"), "# run\n").unwrap();
            let authority = EventAuthority::open(
                sled::open(storage.path().join("events")).unwrap(),
                EventAuthorityOpenOptions::default(),
            )
            .unwrap();
            let mut first = actor(source.path(), &storage.path().join("docs"), &authority);
            enable_claims(&mut first, None);
            first.tick(WorkBudget { max_items: 1 });
            let raw = first.current_revision().unwrap().unwrap();
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            let report = runtime
                .block_on(assess_observed_claims(
                    &FixtureJudge::default(),
                    &first.binding.claim_policy.as_ref().unwrap().policy,
                    &raw.evidence,
                ))
                .unwrap();
            first
                .binding
                .store
                .save_claim_progress(&first.binding_id, &raw.revision_id, &report)
                .unwrap();
            let pending = if boundary > 0 {
                let pending = first
                    .binding
                    .store
                    .prepare_claim_report(&first.binding_id, &raw.revision_id, report.clone())
                    .unwrap();
                if boundary == 2 {
                    authority
                        .append_capability()
                        .append_durable_proven(
                            pending.envelope("docs-observation-test").unwrap(),
                            AppendMode::Idempotent,
                        )
                        .unwrap();
                }
                Some(pending)
            } else {
                None
            };
            drop(first);
            let mut recovered = actor(source.path(), &storage.path().join("docs"), &authority);
            enable_claims(&mut recovered, None);
            let tick = recovered.tick(WorkBudget { max_items: 1 });
            assert_eq!(tick.items_committed, 1, "{tick:?}");
            assert!(tick.retryable_errors.is_empty());
            let resumed = recovered.current_revision().unwrap().unwrap();
            assert_eq!(resumed.claim_report, Some(report));
            if let Some(pending) = pending {
                assert_eq!(resumed, pending);
            }
            assert_eq!(
                authority.watermark_capability().snapshot().unwrap().tip_seq,
                2
            );
            assert!(recovered
                .binding
                .store
                .save_claim_progress(
                    &recovered.binding_id,
                    &raw.revision_id,
                    resumed.claim_report.as_ref().unwrap()
                )
                .is_err());
        }
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
