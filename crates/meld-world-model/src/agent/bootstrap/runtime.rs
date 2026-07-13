//! Source-neutral one-shot bootstrap orchestration.

use sled::Db;

use super::contracts::{
    AgentBootstrapDiagnostic, AgentBootstrapError, AgentBootstrapProgress, AgentBootstrapReport,
    AgentBootstrapStage, MAX_BOOTSTRAP_DIAGNOSTICS,
};
use super::store::{deterministic_subscription_id, stage_at_least, BootstrapStore};
use crate::activation::{
    validate_world_model_activation, BeliefActivationReceipt, WorldModelActivationIdentity,
    WorldModelActivationInput,
};
use crate::belief::{BeliefActivation, BeliefActivationError};

/// One-shot world-model runtime that durably activates configured seed state.
pub struct AgentBootstrapRuntime {
    store: BootstrapStore,
    belief: BeliefActivation,
}

impl AgentBootstrapRuntime {
    /// Open bootstrap and belief activation state on one world-model database.
    pub fn new(db: Db) -> Result<Self, AgentBootstrapError> {
        Ok(Self {
            belief: BeliefActivation::new(db.clone()).map_err(map_belief_error)?,
            store: BootstrapStore::new(db)?,
        })
    }

    /// Run or recover one configured bootstrap through its final durable receipt.
    pub fn bootstrap(
        &self,
        input: &WorldModelActivationInput,
    ) -> Result<AgentBootstrapReport, AgentBootstrapError> {
        self.bootstrap_inner(input, |_| Ok(()))
    }

    /// Read durable progress without consulting supervisor state.
    pub fn progress(
        &self,
        bootstrap_id: &str,
    ) -> Result<Option<AgentBootstrapProgress>, AgentBootstrapError> {
        self.store.progress(bootstrap_id)
    }

    /// Read the final durable receipt for one logical bootstrap.
    pub fn receipt(
        &self,
        bootstrap_id: &str,
    ) -> Result<Option<crate::activation::AgentBootstrapReceipt>, AgentBootstrapError> {
        self.store.receipt(bootstrap_id)
    }

    fn bootstrap_inner(
        &self,
        input: &WorldModelActivationInput,
        mut after_stage: impl FnMut(AgentBootstrapStage) -> Result<(), AgentBootstrapError>,
    ) -> Result<AgentBootstrapReport, AgentBootstrapError> {
        let identity = validate_world_model_activation(input).map_err(|error| {
            AgentBootstrapError::Validation {
                field: error.field,
                message: error.message,
            }
        })?;
        let existing_progress = self.store.validated_progress(&identity)?;
        let resumed_from = existing_progress.as_ref().map(|progress| progress.stage);
        let mut diagnostics = Vec::with_capacity(MAX_BOOTSTRAP_DIAGNOSTICS);

        let mut confirmed_belief = None;
        if let Some(progress) = existing_progress.as_ref() {
            if stage_at_least(progress.stage, AgentBootstrapStage::BeliefConfigured) {
                confirmed_belief = Some(
                    self.belief
                        .confirm(input, &identity)
                        .map_err(map_belief_error)?,
                );
            }
            let confirmed_receipt = self.store.confirm_products(
                input,
                &identity,
                progress,
                confirmed_belief.as_ref(),
            )?;
            if progress.stage == AgentBootstrapStage::Completed {
                push_diagnostic(
                    &mut diagnostics,
                    AgentBootstrapStage::Completed,
                    "confirmed_no_work",
                );
                return Ok(AgentBootstrapReport {
                    receipt: confirmed_receipt.ok_or_else(|| AgentBootstrapError::Storage {
                        message: "completed bootstrap receipt is missing".to_string(),
                    })?,
                    progress: progress.clone(),
                    work_performed: false,
                    resumed_from,
                    diagnostics,
                });
            }
        }

        let mut progress = match existing_progress {
            Some(progress) => progress,
            None => {
                let progress = self.store.start(input, &identity)?;
                self.store.flush()?;
                push_diagnostic(&mut diagnostics, AgentBootstrapStage::Started, "advanced");
                after_stage(AgentBootstrapStage::Started)?;
                progress
            }
        };

        let belief = if let Some(receipt) = confirmed_belief {
            receipt
        } else {
            let receipt = self
                .belief
                .activate(input, &identity)
                .map_err(map_belief_error)?;
            progress = self.store.mark_belief_configured(&identity)?;
            self.store.flush()?;
            push_diagnostic(
                &mut diagnostics,
                AgentBootstrapStage::BeliefConfigured,
                "advanced",
            );
            after_stage(AgentBootstrapStage::BeliefConfigured)?;
            receipt
        };
        require_belief_receipt(&belief, input, &identity)?;

        if !stage_at_least(progress.stage, AgentBootstrapStage::AgentRegistered) {
            progress = self.store.register_agent(input, &identity)?;
            self.store.flush()?;
            push_diagnostic(
                &mut diagnostics,
                AgentBootstrapStage::AgentRegistered,
                "advanced",
            );
            after_stage(AgentBootstrapStage::AgentRegistered)?;
        }

        if !stage_at_least(progress.stage, AgentBootstrapStage::RuleRegistered) {
            progress = self.store.register_rule(&input.curation_rule, &identity)?;
            self.store.flush()?;
            push_diagnostic(
                &mut diagnostics,
                AgentBootstrapStage::RuleRegistered,
                "advanced",
            );
            after_stage(AgentBootstrapStage::RuleRegistered)?;
        }

        let subscription_id = deterministic_subscription_id(input);
        if !stage_at_least(progress.stage, AgentBootstrapStage::SubscriptionBound) {
            let next = self.store.bind_subscription(input, &identity)?;
            progress = next.0;
            if next.1.subscription_id != subscription_id {
                return Err(AgentBootstrapError::Conflict {
                    field: "agent_subscription.subscription_id".to_string(),
                    configured_value_hash: hash(subscription_id.as_bytes()),
                    durable_value_hash: hash(next.1.subscription_id.as_bytes()),
                });
            }
            self.store.flush()?;
            push_diagnostic(
                &mut diagnostics,
                AgentBootstrapStage::SubscriptionBound,
                "advanced",
            );
            after_stage(AgentBootstrapStage::SubscriptionBound)?;
        }

        if !stage_at_least(progress.stage, AgentBootstrapStage::ProductsConfirmed) {
            progress = self.store.confirm_seed_products(input, &identity)?;
            self.store.flush()?;
            push_diagnostic(
                &mut diagnostics,
                AgentBootstrapStage::ProductsConfirmed,
                "advanced",
            );
            after_stage(AgentBootstrapStage::ProductsConfirmed)?;
        }

        self.store
            .confirm_products(input, &identity, &progress, Some(&belief))?;
        let (progress, receipt) = self
            .store
            .complete(input, &identity, belief, subscription_id)?;
        self.store.flush()?;
        let confirmed = self
            .belief
            .confirm(input, &identity)
            .map_err(map_belief_error)?;
        let durable_receipt = self
            .store
            .confirm_products(input, &identity, &progress, Some(&confirmed))?
            .ok_or_else(|| AgentBootstrapError::Storage {
                message: "completed bootstrap receipt is missing".to_string(),
            })?;
        if durable_receipt != receipt {
            return Err(AgentBootstrapError::Storage {
                message: "bootstrap receipt changed during completion".to_string(),
            });
        }
        push_diagnostic(&mut diagnostics, AgentBootstrapStage::Completed, "advanced");
        after_stage(AgentBootstrapStage::Completed)?;

        Ok(AgentBootstrapReport {
            receipt,
            progress,
            work_performed: true,
            resumed_from,
            diagnostics,
        })
    }
}

fn expected_belief_receipt(
    input: &WorldModelActivationInput,
    identity: &WorldModelActivationIdentity,
) -> BeliefActivationReceipt {
    BeliefActivationReceipt {
        family_id: input.belief_family.family_id.clone(),
        config_snapshot_hash: identity.belief_config_hash.clone(),
        activation_hash: identity.activation_hash.clone(),
        activation_id: identity.activation_id.clone(),
    }
}

fn require_belief_receipt(
    receipt: &BeliefActivationReceipt,
    input: &WorldModelActivationInput,
    identity: &WorldModelActivationIdentity,
) -> Result<(), AgentBootstrapError> {
    let expected = expected_belief_receipt(input, identity);
    if *receipt == expected {
        Ok(())
    } else {
        let configured =
            serde_json::to_vec(&expected).map_err(|error| AgentBootstrapError::Storage {
                message: error.to_string(),
            })?;
        let durable =
            serde_json::to_vec(receipt).map_err(|error| AgentBootstrapError::Storage {
                message: error.to_string(),
            })?;
        Err(AgentBootstrapError::Conflict {
            field: "belief_activation_receipt".to_string(),
            configured_value_hash: hash(&configured),
            durable_value_hash: hash(&durable),
        })
    }
}

fn push_diagnostic(
    diagnostics: &mut Vec<AgentBootstrapDiagnostic>,
    stage: AgentBootstrapStage,
    disposition: &str,
) {
    if diagnostics.len() < MAX_BOOTSTRAP_DIAGNOSTICS {
        diagnostics.push(AgentBootstrapDiagnostic {
            stage,
            disposition: disposition.to_string(),
        });
    }
}

fn map_belief_error(error: BeliefActivationError) -> AgentBootstrapError {
    match error {
        BeliefActivationError::Conflict {
            field,
            configured_value_hash,
            durable_value_hash,
        } => AgentBootstrapError::Conflict {
            field,
            configured_value_hash,
            durable_value_hash,
        },
        BeliefActivationError::Storage { message } => AgentBootstrapError::Storage { message },
    }
}

fn hash(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::Path;

    use super::*;
    use crate::activation::{
        AgentCurationRuleRecord, DirectiveRecord, LegacyDirectiveMigrationConflictField,
        SeedAgentActivation,
    };
    use crate::agent::bootstrap::compat::encode_legacy_agent_record;
    use crate::agent::{
        AgentCurationRuleConfig, AgentRecord, AgentStatus, AgentStore, AgentSubscription,
        AgentSubscriptionRecord, AgentSubscriptionStatus, SubscribeAgentCommand,
    };
    use crate::belief::{BeliefFamilyConfig, BeliefKey, BranchScope};
    use crate::events::DomainObjectRef;
    use crate::world_state::graph::PerspectiveKey;

    type DbSnapshot = BTreeMap<Vec<u8>, Vec<(Vec<u8>, Vec<u8>)>>;

    fn input() -> WorldModelActivationInput {
        let belief_family: BeliefFamilyConfig = serde_json::from_str(
            r#"{
                "family_id":"family_a",
                "dimension_id":"family_a",
                "predicate_id":"confidence",
                "evidence_policy_id":"policy-a",
                "evidence_schemas":[{"schema_id":"content","required":true,"role":"Support","reliability":1.0,"precision":1.0}],
                "source_mappings":[{"mapping_id":"content","source_kind":"content_written","evidence_schema_id":"content","subject_from":"record.subject","value_field":"stale_probability","factor_id":"freshness"}],
                "comparator":{"engine_id":"weighted_bayesian","engine_version":"1","factors":[{"factor_id":"freshness","evidence_schema_id":"content","weight":1.0,"polarity":"Supports"}],"missing_evidence_uncertainty":0.5},
                "default_prior":0.8,
                "planner_projection":{"confidence_field":"confidence","threshold":0.7,"posterior_meaning":"stale_probability"},
                "config_version":"1"
            }"#,
        )
        .unwrap();
        let subject = DomainObjectRef::new("workspace_fs", "node", "node-a").unwrap();
        let perspective = PerspectiveKey::new("default", "default").unwrap();
        let branch_scope = BranchScope::main();
        WorldModelActivationInput {
            activation_hash: "a".repeat(64),
            activation_id: "activation-a".to_string(),
            bootstrap_id: "bootstrap-a".to_string(),
            belief_family,
            directive: DirectiveRecord {
                directive_id: "directive-a".to_string(),
                text: "curate configured family goals".to_string(),
            },
            seed_agent: SeedAgentActivation {
                agent_id: "seed-a".to_string(),
                perspective_key: perspective.clone(),
                subject: subject.clone(),
                branch_scope: branch_scope.clone(),
                observation_scope: "family_a".to_string(),
                directive_id: "directive-a".to_string(),
                seed_provenance: "trusted init".to_string(),
            },
            curation_rule: AgentCurationRuleRecord {
                rule_id: "rule-a".to_string(),
                agent_id: "seed-a".to_string(),
                config: AgentCurationRuleConfig {
                    dimension_id: "family_a".to_string(),
                    threshold: 0.7,
                    priority_urgency: 10,
                    desired_summary: "confidence above 0.7".to_string(),
                    source_kind: "belief_divergence".to_string(),
                },
            },
            belief_key: BeliefKey {
                subject,
                dimension_id: "family_a".to_string(),
                predicate_id: "confidence".to_string(),
                perspective,
                branch_scope,
                evidence_policy_id: "policy-a".to_string(),
            },
        }
    }

    fn open(path: &Path) -> AgentBootstrapRuntime {
        AgentBootstrapRuntime::new(sled::open(path).unwrap()).unwrap()
    }

    fn snapshot_db(path: &Path) -> DbSnapshot {
        let db = sled::open(path).unwrap();
        let mut snapshot = BTreeMap::new();
        for tree_name in db.tree_names() {
            let tree = db.open_tree(tree_name.clone()).unwrap();
            let records = tree
                .iter()
                .map(|entry| {
                    let (key, value) = entry.unwrap();
                    (key.to_vec(), value.to_vec())
                })
                .collect();
            snapshot.insert(tree_name.to_vec(), records);
        }
        snapshot
    }

    #[test]
    fn bootstrap_is_staged_durable_and_exact_replay_is_no_work() {
        let temp = tempfile::tempdir().unwrap();
        let runtime = open(temp.path());
        let first = runtime.bootstrap(&input()).unwrap();

        assert!(first.work_performed);
        assert_eq!(first.progress.stage, AgentBootstrapStage::Completed);
        assert_eq!(first.diagnostics.len(), 7);
        let replay = runtime.bootstrap(&input()).unwrap();
        assert!(!replay.work_performed);
        assert_eq!(replay.receipt, first.receipt);
        assert_eq!(replay.diagnostics.len(), 1);
        assert_eq!(
            replay.diagnostics[0].disposition,
            "confirmed_no_work".to_string()
        );

        drop(runtime);
        let db = sled::open(temp.path()).unwrap();
        let store = AgentStore::new(db.clone()).unwrap();
        let agent = store.get_agent("seed-a").unwrap().unwrap();
        assert_eq!(agent.directive_id, "directive-a");
        assert_eq!(agent.status, AgentStatus::Registered);
        assert_eq!(
            store.get_directive("directive-a").unwrap().unwrap(),
            input().directive
        );
        assert_eq!(
            store.get_curation_rule("rule-a").unwrap().unwrap(),
            input().curation_rule
        );
        assert_eq!(
            store
                .get_bootstrap_progress("bootstrap-a")
                .unwrap()
                .unwrap(),
            first.progress
        );
        assert_eq!(
            store.get_bootstrap_receipt("bootstrap-a").unwrap().unwrap(),
            first.receipt
        );
        assert!(store.activations_for_agent("seed-a").unwrap().is_empty());
        let subscription = store
            .subscription_by_agent_and_key("seed-a", &input().belief_key)
            .unwrap()
            .unwrap();
        assert_eq!(subscription.subscription_id, first.receipt.subscription_id);
        let belief_store = crate::belief::BeliefStore::new(db).unwrap();
        assert!(belief_store
            .get_config_snapshot(&first.receipt.belief.config_snapshot_hash)
            .unwrap()
            .is_some());
    }

    #[test]
    fn bootstrap_reopens_and_resumes_after_every_durable_stage() {
        let stages = [
            AgentBootstrapStage::Started,
            AgentBootstrapStage::BeliefConfigured,
            AgentBootstrapStage::AgentRegistered,
            AgentBootstrapStage::RuleRegistered,
            AgentBootstrapStage::SubscriptionBound,
            AgentBootstrapStage::ProductsConfirmed,
            AgentBootstrapStage::Completed,
        ];
        for fault_stage in stages {
            let temp = tempfile::tempdir().unwrap();
            {
                let runtime = open(temp.path());
                let result = runtime.bootstrap_inner(&input(), |stage| {
                    if stage == fault_stage {
                        Err(AgentBootstrapError::Storage {
                            message: format!("injected after {stage:?}"),
                        })
                    } else {
                        Ok(())
                    }
                });
                assert!(result.is_err(), "fault at {fault_stage:?} must interrupt");
            }

            let runtime = open(temp.path());
            let recovered = runtime.bootstrap(&input()).unwrap();
            assert_eq!(recovered.progress.stage, AgentBootstrapStage::Completed);
            assert_eq!(recovered.resumed_from, Some(fault_stage));
            assert_eq!(
                recovered.work_performed,
                fault_stage != AgentBootstrapStage::Completed
            );
            let replay = runtime.bootstrap(&input()).unwrap();
            assert!(!replay.work_performed);
            assert_eq!(replay.receipt, recovered.receipt);
        }
    }

    #[test]
    fn same_bootstrap_identity_with_divergent_content_fails_closed() {
        let temp = tempfile::tempdir().unwrap();
        let runtime = open(temp.path());
        runtime.bootstrap(&input()).unwrap();
        let mut divergent = input();
        divergent.directive.text = "different intent".to_string();

        let error = runtime.bootstrap(&divergent).unwrap_err();
        assert!(matches!(
            error,
            AgentBootstrapError::Conflict { ref field, .. }
                if field == "bootstrap.input_hash"
        ));
    }

    #[test]
    fn divergent_started_retry_writes_nothing() {
        let temp = tempfile::tempdir().unwrap();
        {
            let runtime = open(temp.path());
            let result = runtime.bootstrap_inner(&input(), |stage| {
                if stage == AgentBootstrapStage::Started {
                    Err(AgentBootstrapError::Storage {
                        message: "injected after progress start".to_string(),
                    })
                } else {
                    Ok(())
                }
            });
            assert!(result.is_err());
        }
        let before = snapshot_db(temp.path());

        let mut divergent = input();
        divergent.directive.text = "different intent".to_string();
        {
            let runtime = open(temp.path());
            let error = runtime.bootstrap(&divergent).unwrap_err();
            assert!(matches!(
                error,
                AgentBootstrapError::Conflict { ref field, .. }
                    if field == "bootstrap.input_hash"
            ));
        }

        assert_eq!(snapshot_db(temp.path()), before);
    }

    #[test]
    fn resumed_bootstrap_reconfirms_intermediate_belief_snapshot() {
        let temp = tempfile::tempdir().unwrap();
        let configured = input();
        {
            let runtime = open(temp.path());
            let result = runtime.bootstrap_inner(&configured, |stage| {
                if stage == AgentBootstrapStage::BeliefConfigured {
                    Err(AgentBootstrapError::Storage {
                        message: "injected after belief configuration".to_string(),
                    })
                } else {
                    Ok(())
                }
            });
            assert!(result.is_err());
        }
        let identity = validate_world_model_activation(&configured).unwrap();
        {
            let db = sled::open(temp.path()).unwrap();
            let store = crate::belief::BeliefStore::new(db).unwrap();
            store
                .put_config_snapshot(&identity.belief_config_hash, r#"{"drift":true}"#)
                .unwrap();
            store.flush().unwrap();
        }

        let runtime = open(temp.path());
        let error = runtime.bootstrap(&configured).unwrap_err();
        assert!(matches!(
            error,
            AgentBootstrapError::Conflict { ref field, .. }
                if field == "belief_config_snapshot.content"
        ));
        drop(runtime);
        let store = AgentStore::new(sled::open(temp.path()).unwrap()).unwrap();
        assert!(store.get_agent("seed-a").unwrap().is_none());
    }

    #[test]
    fn completed_replay_reconfirms_immutable_agent_identity() {
        let temp = tempfile::tempdir().unwrap();
        {
            let runtime = open(temp.path());
            runtime.bootstrap(&input()).unwrap();
        }
        {
            let db = sled::open(temp.path()).unwrap();
            let store = AgentStore::new(db).unwrap();
            let mut agent = store.get_agent("seed-a").unwrap().unwrap();
            agent.observation_scope = "drifted-family".to_string();
            store.put_agent(&agent).unwrap();
            store.flush().unwrap();
        }

        let runtime = open(temp.path());
        let error = runtime.bootstrap(&input()).unwrap_err();
        assert!(matches!(
            error,
            AgentBootstrapError::Conflict { ref field, .. }
                if field == "agent.observation_scope"
        ));
    }

    #[test]
    fn completed_replay_preserves_mutable_lifecycle_and_cursor_without_writes() {
        let temp = tempfile::tempdir().unwrap();
        let completed_at_seq = {
            let runtime = open(temp.path());
            runtime
                .bootstrap(&input())
                .unwrap()
                .receipt
                .completed_at_seq
        };
        {
            let db = sled::open(temp.path()).unwrap();
            let store = AgentStore::new(db).unwrap();
            let mut agent = store.get_agent("seed-a").unwrap().unwrap();
            agent.status = AgentStatus::Operational;
            agent.updated_at_seq = completed_at_seq + 1;
            store.put_agent(&agent).unwrap();
            let mut subscription = store
                .subscription_by_agent_and_key("seed-a", &input().belief_key)
                .unwrap()
                .unwrap();
            subscription.status = AgentSubscriptionStatus::Suspended;
            subscription.last_delivered_revision_id = Some("revision-after-ready".to_string());
            subscription.last_delivered_seq = completed_at_seq + 1;
            subscription.updated_at_seq = completed_at_seq + 2;
            store.put_subscription(&subscription).unwrap();
            store.flush().unwrap();
        }
        let before = snapshot_db(temp.path());

        {
            let runtime = open(temp.path());
            let replay = runtime.bootstrap(&input()).unwrap();
            assert!(!replay.work_performed);
        }

        assert_eq!(snapshot_db(temp.path()), before);
    }

    #[test]
    fn same_directive_id_with_divergent_text_fails_closed() {
        let temp = tempfile::tempdir().unwrap();
        let runtime = open(temp.path());
        runtime.bootstrap(&input()).unwrap();
        let mut divergent = input();
        divergent.bootstrap_id = "bootstrap-b".to_string();
        divergent.activation_id = "activation-b".to_string();
        divergent.activation_hash = "b".repeat(64);
        divergent.directive.text = "different intent".to_string();

        let error = runtime.bootstrap(&divergent).unwrap_err();
        assert!(matches!(
            error,
            AgentBootstrapError::Conflict { ref field, .. } if field == "directive.text"
        ));
    }

    #[test]
    fn same_belief_family_with_divergent_config_fails_closed() {
        let temp = tempfile::tempdir().unwrap();
        let runtime = open(temp.path());
        runtime.bootstrap(&input()).unwrap();
        let mut divergent = input();
        divergent.bootstrap_id = "bootstrap-b".to_string();
        divergent.activation_id = "activation-b".to_string();
        divergent.activation_hash = "b".repeat(64);
        divergent.belief_family.default_prior = 0.6;

        let error = runtime.bootstrap(&divergent).unwrap_err();
        assert!(matches!(
            error,
            AgentBootstrapError::Conflict { ref field, .. }
                if field == "belief_family.config_snapshot_hash"
        ));
    }

    #[test]
    fn same_agent_id_with_divergent_subject_fails_closed() {
        let temp = tempfile::tempdir().unwrap();
        let runtime = open(temp.path());
        runtime.bootstrap(&input()).unwrap();
        let mut divergent = input();
        divergent.bootstrap_id = "bootstrap-b".to_string();
        divergent.activation_id = "activation-b".to_string();
        divergent.activation_hash = "b".repeat(64);
        let subject = DomainObjectRef::new("workspace_fs", "node", "node-b").unwrap();
        divergent.seed_agent.subject = subject.clone();
        divergent.belief_key.subject = subject;

        let error = runtime.bootstrap(&divergent).unwrap_err();
        assert!(matches!(
            error,
            AgentBootstrapError::Conflict { ref field, .. } if field == "agent.subject"
        ));
    }

    #[test]
    fn same_rule_id_with_divergent_config_fails_closed() {
        let temp = tempfile::tempdir().unwrap();
        let runtime = open(temp.path());
        runtime.bootstrap(&input()).unwrap();
        let mut divergent = input();
        divergent.bootstrap_id = "bootstrap-b".to_string();
        divergent.activation_id = "activation-b".to_string();
        divergent.activation_hash = "b".repeat(64);
        divergent.curation_rule.config.priority_urgency = 99;

        let error = runtime.bootstrap(&divergent).unwrap_err();
        assert!(matches!(
            error,
            AgentBootstrapError::Conflict { ref field, .. } if field == "curation_rule"
        ));
    }

    #[test]
    fn legacy_embedded_directive_migrates_atomically_with_parity_receipt() {
        let temp = tempfile::tempdir().unwrap();
        let db = sled::open(temp.path()).unwrap();
        let configured = input();
        let legacy_canonical = AgentRecord {
            agent_id: configured.seed_agent.agent_id.clone(),
            perspective_key: configured.seed_agent.perspective_key.clone(),
            subject: configured.seed_agent.subject.clone(),
            branch_scope: configured.seed_agent.branch_scope.clone(),
            observation_scope: configured.seed_agent.observation_scope.clone(),
            directive_id: "unused-by-legacy-wire".to_string(),
            seed_provenance: configured.seed_agent.seed_provenance.clone(),
            status: AgentStatus::Registered,
            created_at_seq: 41,
            updated_at_seq: 41,
        };
        let legacy_bytes =
            encode_legacy_agent_record(&legacy_canonical, &configured.directive.text);
        db.open_tree("agent_records")
            .unwrap()
            .insert(
                configured.seed_agent.agent_id.as_bytes(),
                legacy_bytes.clone(),
            )
            .unwrap();
        db.flush().unwrap();
        drop(db);

        let runtime = open(temp.path());
        let report = runtime.bootstrap(&configured).unwrap();
        drop(runtime);
        let db = sled::open(temp.path()).unwrap();
        let agent = AgentStore::new(db.clone())
            .unwrap()
            .get_agent(&configured.seed_agent.agent_id)
            .unwrap()
            .unwrap();
        assert_eq!(agent.directive_id, configured.directive.directive_id);
        assert_eq!(agent.created_at_seq, 41);
        assert_eq!(agent.updated_at_seq, 41);
        assert_eq!(agent.status, AgentStatus::Registered);
        assert!(report.receipt.completed_at_seq > 41);

        let migrations = db
            .open_tree("agent_legacy_directive_migration_receipts")
            .unwrap();
        assert_eq!(migrations.len(), 1);
        let receipt: crate::activation::LegacyDirectiveMigrationReceipt =
            serde_json::from_slice(&migrations.iter().next().unwrap().unwrap().1).unwrap();
        assert_eq!(receipt.identity.schema_version, 1);
        assert!(receipt.completed_at_seq > 41);
        assert_eq!(
            receipt.identity.directive_id,
            configured.directive.directive_id
        );
        assert_eq!(receipt.legacy_agent_record_hash.len(), 64);
        assert_eq!(receipt.directive_record_hash.len(), 64);
        assert_eq!(receipt.canonical_agent_record_hash.len(), 64);
        assert_eq!(
            receipt.legacy_agent_record_hash,
            blake3::hash(&legacy_bytes).to_hex().to_string()
        );
        assert_eq!(
            receipt.directive_record_hash,
            blake3::hash(&serde_json::to_vec(&configured.directive).unwrap())
                .to_hex()
                .to_string()
        );
        let migrated = AgentRecord {
            directive_id: configured.directive.directive_id.clone(),
            ..legacy_canonical
        };
        assert_eq!(
            receipt.canonical_agent_record_hash,
            blake3::hash(&serde_json::to_vec(&migrated).unwrap())
                .to_hex()
                .to_string()
        );
    }

    #[test]
    fn legacy_migration_names_divergent_field_and_leaves_record_unchanged() {
        let temp = tempfile::tempdir().unwrap();
        let db = sled::open(temp.path()).unwrap();
        let configured = input();
        let legacy = AgentRecord {
            agent_id: configured.seed_agent.agent_id.clone(),
            perspective_key: configured.seed_agent.perspective_key.clone(),
            subject: configured.seed_agent.subject.clone(),
            branch_scope: configured.seed_agent.branch_scope.clone(),
            observation_scope: configured.seed_agent.observation_scope.clone(),
            directive_id: "unused".to_string(),
            seed_provenance: configured.seed_agent.seed_provenance.clone(),
            status: AgentStatus::Registered,
            created_at_seq: 7,
            updated_at_seq: 7,
        };
        let legacy_bytes = encode_legacy_agent_record(&legacy, "different embedded intent");
        db.open_tree("agent_records")
            .unwrap()
            .insert(
                configured.seed_agent.agent_id.as_bytes(),
                legacy_bytes.clone(),
            )
            .unwrap();
        db.flush().unwrap();
        drop(db);

        let runtime = open(temp.path());
        let error = runtime.bootstrap(&configured).unwrap_err();
        assert!(matches!(
            error,
            AgentBootstrapError::LegacyDirectiveConflict(ref conflict)
                if conflict.field == LegacyDirectiveMigrationConflictField::DirectiveText
        ));
        drop(runtime);
        let db = sled::open(temp.path()).unwrap();
        let raw = db
            .open_tree("agent_records")
            .unwrap()
            .get(configured.seed_agent.agent_id.as_bytes())
            .unwrap()
            .unwrap();
        assert_eq!(raw.as_ref(), legacy_bytes.as_slice());
        assert!(db
            .open_tree("agent_directives")
            .unwrap()
            .get(configured.directive.directive_id.as_bytes())
            .unwrap()
            .is_none());
        assert!(db
            .open_tree("agent_legacy_directive_migration_receipts")
            .unwrap()
            .is_empty());
    }

    #[test]
    fn deterministic_subscription_binding_matches_agent_natural_key() {
        let configured = input();
        let expected = super::deterministic_subscription_id(&configured);
        let natural_key = AgentSubscriptionRecord::natural_key(
            &configured.seed_agent.agent_id,
            &configured.belief_key,
        );
        assert_eq!(
            expected,
            crate::agent::contracts::deterministic_id("subscription", &natural_key)
        );
    }

    #[test]
    fn bootstrap_reuses_compatible_preexisting_agent_and_subscription() {
        let temp = tempfile::tempdir().unwrap();
        let db = sled::open(temp.path()).unwrap();
        let configured = input();
        let store = AgentStore::new(db.clone()).unwrap();
        store
            .put_agent(&AgentRecord {
                agent_id: configured.seed_agent.agent_id.clone(),
                perspective_key: configured.seed_agent.perspective_key.clone(),
                subject: configured.seed_agent.subject.clone(),
                branch_scope: configured.seed_agent.branch_scope.clone(),
                observation_scope: configured.seed_agent.observation_scope.clone(),
                directive_id: configured.directive.directive_id.clone(),
                seed_provenance: configured.seed_agent.seed_provenance.clone(),
                status: AgentStatus::Registered,
                created_at_seq: 77,
                updated_at_seq: 77,
            })
            .unwrap();
        let existing = AgentSubscription::new(&store)
            .subscribe(SubscribeAgentCommand {
                agent_id: configured.seed_agent.agent_id.clone(),
                belief_key: configured.belief_key.clone(),
                created_at_seq: 78,
            })
            .unwrap();
        store.flush().unwrap();
        drop(store);

        let runtime = AgentBootstrapRuntime::new(db).unwrap();
        let report = runtime.bootstrap(&configured).unwrap();
        assert_eq!(report.receipt.subscription_id, existing.subscription_id);
        assert!(report.receipt.completed_at_seq > 78);
        drop(runtime);
        let store = AgentStore::new(sled::open(temp.path()).unwrap()).unwrap();
        let agent = store
            .get_agent(&configured.seed_agent.agent_id)
            .unwrap()
            .unwrap();
        assert_eq!(agent.status, AgentStatus::Registered);
        assert_eq!(agent.created_at_seq, 77);
        assert_eq!(agent.updated_at_seq, 77);
        let durable = store
            .get_subscription(&existing.subscription_id)
            .unwrap()
            .unwrap();
        assert_eq!(durable.created_at_seq, 78);
        assert_eq!(durable.belief_key, configured.belief_key);
    }

    #[test]
    fn concurrent_exact_bootstraps_converge_on_one_receipt() {
        let temp = tempfile::tempdir().unwrap();
        let db = sled::open(temp.path()).unwrap();
        let left = AgentBootstrapRuntime::new(db.clone()).unwrap();
        let right = AgentBootstrapRuntime::new(db).unwrap();
        let left_input = input();
        let right_input = left_input.clone();

        let left = std::thread::spawn(move || left.bootstrap(&left_input).unwrap());
        let right = std::thread::spawn(move || right.bootstrap(&right_input).unwrap());
        let left = left.join().unwrap();
        let right = right.join().unwrap();

        assert_eq!(left.receipt, right.receipt);
        assert_eq!(left.progress, right.progress);
        let db = sled::open(temp.path()).unwrap();
        let store = AgentStore::new(db.clone()).unwrap();
        let agent = store.get_agent("seed-a").unwrap().unwrap();
        let subscription = store
            .subscription_by_agent_and_key("seed-a", &input().belief_key)
            .unwrap()
            .unwrap();
        assert!(left.receipt.completed_at_seq > agent.updated_at_seq);
        assert!(left.receipt.completed_at_seq > subscription.updated_at_seq);
        let runtime = AgentBootstrapRuntime::new(db).unwrap();
        let replay = runtime.bootstrap(&input()).unwrap();
        assert!(!replay.work_performed);
        assert_eq!(replay.receipt, left.receipt);
    }

    #[test]
    fn completion_refloors_after_lifecycle_mutation_following_product_confirmation() {
        let temp = tempfile::tempdir().unwrap();
        let db = sled::open(temp.path()).unwrap();
        let store = AgentStore::new(db.clone()).unwrap();
        let runtime = AgentBootstrapRuntime::new(db).unwrap();
        let report = runtime
            .bootstrap_inner(&input(), |stage| {
                if stage == AgentBootstrapStage::ProductsConfirmed {
                    let mut agent = store.get_agent("seed-a").unwrap().unwrap();
                    agent.status = AgentStatus::Operational;
                    agent.updated_at_seq = 120;
                    store.put_agent(&agent).unwrap();
                    let mut subscription = store
                        .subscription_by_agent_and_key("seed-a", &input().belief_key)
                        .unwrap()
                        .unwrap();
                    subscription.status = AgentSubscriptionStatus::Suspended;
                    subscription.last_delivered_revision_id =
                        Some("revision-after-confirmation".to_string());
                    subscription.last_delivered_seq = 120;
                    subscription.updated_at_seq = 121;
                    store.put_subscription(&subscription).unwrap();
                    store.flush().unwrap();
                }
                Ok(())
            })
            .unwrap();

        assert!(report.receipt.completed_at_seq > 121);
        assert_eq!(
            store.get_agent("seed-a").unwrap().unwrap().status,
            AgentStatus::Operational
        );
    }

    #[test]
    fn resume_preserves_mutable_subscription_lifecycle_and_cursor() {
        let temp = tempfile::tempdir().unwrap();
        {
            let runtime = open(temp.path());
            let result = runtime.bootstrap_inner(&input(), |stage| {
                if stage == AgentBootstrapStage::SubscriptionBound {
                    Err(AgentBootstrapError::Storage {
                        message: "injected before activation".to_string(),
                    })
                } else {
                    Ok(())
                }
            });
            assert!(result.is_err());
        }
        {
            let db = sled::open(temp.path()).unwrap();
            let store = AgentStore::new(db).unwrap();
            let mut subscription = store
                .subscription_by_agent_and_key("seed-a", &input().belief_key)
                .unwrap()
                .unwrap();
            subscription.status = AgentSubscriptionStatus::Suspended;
            subscription.last_delivered_revision_id = Some("revision-before-resume".to_string());
            subscription.last_delivered_seq = 91;
            subscription.updated_at_seq = 92;
            store.put_subscription(&subscription).unwrap();
            store.flush().unwrap();
        }

        let runtime = open(temp.path());
        let interrupted = runtime.bootstrap_inner(&input(), |stage| {
            if stage == AgentBootstrapStage::ProductsConfirmed {
                Err(AgentBootstrapError::Storage {
                    message: "injected before final receipt".to_string(),
                })
            } else {
                Ok(())
            }
        });
        assert!(interrupted.is_err());
        let confirmed = runtime.progress("bootstrap-a").unwrap().unwrap();
        assert_eq!(confirmed.stage, AgentBootstrapStage::ProductsConfirmed);
        assert!(confirmed.updated_at_seq > 92);
        let report = runtime.bootstrap(&input()).unwrap();
        assert_eq!(report.progress.stage, AgentBootstrapStage::Completed);
        assert!(report.receipt.completed_at_seq > 92);
        assert!(report.receipt.completed_at_seq > confirmed.updated_at_seq);
        drop(runtime);
        let store = AgentStore::new(sled::open(temp.path()).unwrap()).unwrap();
        assert_eq!(
            store.get_agent("seed-a").unwrap().unwrap().status,
            AgentStatus::Registered
        );
        assert!(store.activations_for_agent("seed-a").unwrap().is_empty());
        let subscription = store
            .subscription_by_agent_and_key("seed-a", &input().belief_key)
            .unwrap()
            .unwrap();
        assert_eq!(subscription.status, AgentSubscriptionStatus::Suspended);
        assert_eq!(
            subscription.last_delivered_revision_id.as_deref(),
            Some("revision-before-resume")
        );
        assert_eq!(subscription.last_delivered_seq, 91);
        assert_eq!(subscription.updated_at_seq, 92);
    }
}
