//! Bootstrap-owned durable mutation surface.

use serde::Serialize;
use sled::transaction::{
    ConflictableTransactionError, TransactionError, Transactional, TransactionalTree,
};
use sled::{Db, Tree};

use super::compat::{decode_legacy_agent_record, DecodedLegacyAgent};
use super::contracts::{
    AgentBootstrapError, AgentBootstrapProgress, AgentBootstrapProgressStatus, AgentBootstrapStage,
};
use crate::activation::{
    AgentBootstrapReceipt, AgentCurationRuleRecord, BeliefActivationReceipt, DirectiveRecord,
    LegacyDirectiveMigrationConflict, LegacyDirectiveMigrationConflictField,
    LegacyDirectiveMigrationIdentity, LegacyDirectiveMigrationReceipt,
    WorldModelActivationIdentity, WorldModelActivationInput,
    LEGACY_DIRECTIVE_MIGRATION_SCHEMA_VERSION,
};
use crate::agent::contracts::deterministic_id;
use crate::agent::{AgentRecord, AgentStatus, AgentSubscriptionRecord, AgentSubscriptionStatus};

const TREE_AGENT_RECORDS: &str = "agent_records";
const TREE_AGENT_BY_STATUS: &str = "agent_by_status";
const TREE_SUBSCRIPTIONS: &str = "agent_subscriptions";
const TREE_SUBSCRIPTIONS_BY_AGENT: &str = "agent_subscriptions_by_agent";
const TREE_SUBSCRIPTIONS_BY_KEY: &str = "agent_subscriptions_by_key";
const TREE_DIRECTIVES: &str = "agent_directives";
const TREE_CURATION_RULES: &str = "agent_curation_rules";
const TREE_BOOTSTRAP_PROGRESS: &str = "agent_bootstrap_progress";
const TREE_BOOTSTRAP_RECEIPTS: &str = "agent_bootstrap_receipts";
const TREE_LEGACY_MIGRATION_RECEIPTS: &str = "agent_legacy_directive_migration_receipts";
const TREE_BOOTSTRAP_SEQUENCE: &str = "agent_bootstrap_sequence";
const KEY_NEXT_SEQUENCE: &[u8] = b"next";
const KEY_PAD: usize = 20;

pub(super) struct BootstrapStore {
    db: Db,
    agents: Tree,
    agent_by_status: Tree,
    subscriptions: Tree,
    subscriptions_by_agent: Tree,
    subscriptions_by_key: Tree,
    directives: Tree,
    curation_rules: Tree,
    progress: Tree,
    receipts: Tree,
    migration_receipts: Tree,
    sequence: Tree,
}

impl BootstrapStore {
    pub(super) fn new(db: Db) -> Result<Self, AgentBootstrapError> {
        Ok(Self {
            agents: open(&db, TREE_AGENT_RECORDS)?,
            agent_by_status: open(&db, TREE_AGENT_BY_STATUS)?,
            subscriptions: open(&db, TREE_SUBSCRIPTIONS)?,
            subscriptions_by_agent: open(&db, TREE_SUBSCRIPTIONS_BY_AGENT)?,
            subscriptions_by_key: open(&db, TREE_SUBSCRIPTIONS_BY_KEY)?,
            directives: open(&db, TREE_DIRECTIVES)?,
            curation_rules: open(&db, TREE_CURATION_RULES)?,
            progress: open(&db, TREE_BOOTSTRAP_PROGRESS)?,
            receipts: open(&db, TREE_BOOTSTRAP_RECEIPTS)?,
            migration_receipts: open(&db, TREE_LEGACY_MIGRATION_RECEIPTS)?,
            sequence: open(&db, TREE_BOOTSTRAP_SEQUENCE)?,
            db,
        })
    }

    pub(super) fn progress(
        &self,
        bootstrap_id: &str,
    ) -> Result<Option<AgentBootstrapProgress>, AgentBootstrapError> {
        decode_optional(
            self.progress
                .get(bootstrap_id.as_bytes())
                .map_err(storage)?,
        )
    }

    pub(super) fn validated_progress(
        &self,
        identity: &WorldModelActivationIdentity,
    ) -> Result<Option<AgentBootstrapProgress>, AgentBootstrapError> {
        let Some(progress) = self.progress(&identity.bootstrap_id)? else {
            return Ok(None);
        };
        require_progress_identity_read(&progress, identity)?;
        require_progress_shape(&progress)?;
        Ok(Some(progress))
    }

    pub(super) fn receipt(
        &self,
        bootstrap_id: &str,
    ) -> Result<Option<AgentBootstrapReceipt>, AgentBootstrapError> {
        decode_optional(
            self.receipts
                .get(bootstrap_id.as_bytes())
                .map_err(storage)?,
        )
    }

    pub(super) fn confirm_products(
        &self,
        input: &WorldModelActivationInput,
        identity: &WorldModelActivationIdentity,
        observed_progress: &AgentBootstrapProgress,
        belief: Option<&BeliefActivationReceipt>,
    ) -> Result<Option<AgentBootstrapReceipt>, AgentBootstrapError> {
        require_progress_identity_read(observed_progress, identity)?;
        let progress =
            self.validated_progress(identity)?
                .ok_or_else(|| AgentBootstrapError::Storage {
                    message: "bootstrap progress disappeared during confirmation".to_string(),
                })?;
        require_progress_shape(&progress)?;
        if !stage_at_least(progress.stage, observed_progress.stage) {
            return Err(AgentBootstrapError::Storage {
                message: "bootstrap progress regressed during confirmation".to_string(),
            });
        }

        if stage_at_least(progress.stage, AgentBootstrapStage::AgentRegistered) {
            let directive: DirectiveRecord =
                required_record(&self.directives, &input.directive.directive_id, "directive")?;
            require_exact_read("directive.text", &input.directive, &directive)?;
            let agent: AgentRecord =
                required_record(&self.agents, &input.seed_agent.agent_id, "agent")?;
            require_canonical_agent_matches_read(&agent, input)?;
            agent
                .validate()
                .map_err(|error| AgentBootstrapError::Storage {
                    message: error.to_string(),
                })?;
        }
        if stage_at_least(progress.stage, AgentBootstrapStage::RuleRegistered) {
            let rule: AgentCurationRuleRecord = required_record(
                &self.curation_rules,
                &input.curation_rule.rule_id,
                "curation rule",
            )?;
            require_exact_read("curation_rule", &input.curation_rule, &rule)?;
        }
        if stage_at_least(progress.stage, AgentBootstrapStage::SubscriptionBound) {
            let subscription_id = deterministic_subscription_id(input);
            let subscription: AgentSubscriptionRecord =
                required_record(&self.subscriptions, &subscription_id, "agent subscription")?;
            require_subscription_matches_read(&subscription, input, &subscription_id)?;
            subscription
                .validate()
                .map_err(|error| AgentBootstrapError::Storage {
                    message: error.to_string(),
                })?;
            let natural_key =
                AgentSubscriptionRecord::natural_key(&input.seed_agent.agent_id, &input.belief_key);
            let indexed = self
                .subscriptions_by_key
                .get(natural_key.as_bytes())
                .map_err(storage)?
                .ok_or_else(|| AgentBootstrapError::Storage {
                    message: "subscription natural-key index is missing".to_string(),
                })?;
            if indexed.as_ref() != subscription_id.as_bytes() {
                return Err(conflict_bytes(
                    "agent_subscription.natural_key",
                    subscription_id.as_bytes(),
                    indexed.as_ref(),
                ));
            }
        }
        if stage_at_least(progress.stage, AgentBootstrapStage::Completed) {
            let belief = belief.ok_or_else(|| AgentBootstrapError::Storage {
                message: "completed bootstrap confirmation requires belief receipt".to_string(),
            })?;
            let receipt: AgentBootstrapReceipt =
                required_record(&self.receipts, &identity.bootstrap_id, "bootstrap receipt")?;
            require_receipt_identity_read(&receipt, input, identity, belief)?;
            Ok(Some(receipt))
        } else if let Some(receipt) = self.receipt(&identity.bootstrap_id)? {
            let current =
                self.validated_progress(identity)?
                    .ok_or_else(|| AgentBootstrapError::Storage {
                        message: "bootstrap progress disappeared while confirming receipt"
                            .to_string(),
                    })?;
            if current.stage != AgentBootstrapStage::Completed {
                return Err(AgentBootstrapError::Storage {
                    message: "bootstrap receipt exists before completed progress".to_string(),
                });
            }
            if let Some(belief) = belief {
                require_receipt_identity_read(&receipt, input, identity, belief)?;
                Ok(Some(receipt))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    pub(super) fn start(
        &self,
        input: &WorldModelActivationInput,
        identity: &WorldModelActivationIdentity,
    ) -> Result<AgentBootstrapProgress, AgentBootstrapError> {
        let bootstrap_id = identity.bootstrap_id.as_bytes();
        let subscription_id = deterministic_subscription_id(input);

        (
            &self.sequence,
            &self.progress,
            &self.agents,
            &self.subscriptions,
        )
            .transaction(|(sequence, progress, agents, subscriptions)| {
                if let Some(raw) = progress.get(bootstrap_id)? {
                    let durable: AgentBootstrapProgress = decode_tx(&raw)?;
                    require_progress_identity(&durable, identity)?;
                    return Ok(durable);
                }
                let floor = existing_sequence_floor_tx(
                    agents,
                    subscriptions,
                    &input.seed_agent.agent_id,
                    &subscription_id,
                )?;
                let seq = allocate_sequence_after(sequence, floor)?;
                let durable = AgentBootstrapProgress {
                    bootstrap_id: identity.bootstrap_id.clone(),
                    activation_id: identity.activation_id.clone(),
                    activation_hash: identity.activation_hash.clone(),
                    input_hash: identity.input_hash.clone(),
                    stage: AgentBootstrapStage::Started,
                    status: AgentBootstrapProgressStatus::Started,
                    updated_at_seq: seq,
                };
                progress.insert(bootstrap_id, encode_tx(&durable)?.as_slice())?;
                Ok(durable)
            })
            .map_err(map_transaction)
    }

    pub(super) fn mark_belief_configured(
        &self,
        identity: &WorldModelActivationIdentity,
    ) -> Result<AgentBootstrapProgress, AgentBootstrapError> {
        self.advance_progress(identity, AgentBootstrapStage::BeliefConfigured)
    }

    pub(super) fn register_agent(
        &self,
        input: &WorldModelActivationInput,
        identity: &WorldModelActivationIdentity,
    ) -> Result<AgentBootstrapProgress, AgentBootstrapError> {
        let bootstrap_id = identity.bootstrap_id.as_bytes();
        (
            &self.sequence,
            &self.progress,
            &self.directives,
            &self.agents,
            &self.agent_by_status,
            &self.migration_receipts,
        )
            .transaction(
                |(sequence, progress, directives, agents, status_index, migrations)| {
                    let current = load_progress_tx(progress, bootstrap_id, identity)?;
                    if stage_at_least(current.stage, AgentBootstrapStage::AgentRegistered) {
                        return Ok(current);
                    }
                    let seq = allocate_sequence(sequence)?;
                    confirm_directive_tx(directives, &input.directive)?;
                    let agent =
                        confirm_or_migrate_agent_tx(agents, migrations, input, identity, seq)?;
                    status_index.insert(
                        agent_status_key(
                            agent.status.index_key(),
                            agent.updated_at_seq,
                            &agent.agent_id,
                        )
                        .as_bytes(),
                        agent.agent_id.as_bytes(),
                    )?;
                    let next = next_progress(&current, AgentBootstrapStage::AgentRegistered, seq);
                    progress.insert(bootstrap_id, encode_tx(&next)?.as_slice())?;
                    Ok(next)
                },
            )
            .map_err(map_transaction)
    }

    pub(super) fn register_rule(
        &self,
        rule: &AgentCurationRuleRecord,
        identity: &WorldModelActivationIdentity,
    ) -> Result<AgentBootstrapProgress, AgentBootstrapError> {
        let bootstrap_id = identity.bootstrap_id.as_bytes();
        (&self.sequence, &self.progress, &self.curation_rules)
            .transaction(|(sequence, progress, rules)| {
                let current = load_progress_tx(progress, bootstrap_id, identity)?;
                if stage_at_least(current.stage, AgentBootstrapStage::RuleRegistered) {
                    return Ok(current);
                }
                confirm_exact_tx(rules, &rule.rule_id, rule, "curation_rule")?;
                let seq = allocate_sequence(sequence)?;
                let next = next_progress(&current, AgentBootstrapStage::RuleRegistered, seq);
                progress.insert(bootstrap_id, encode_tx(&next)?.as_slice())?;
                Ok(next)
            })
            .map_err(map_transaction)
    }

    pub(super) fn bind_subscription(
        &self,
        input: &WorldModelActivationInput,
        identity: &WorldModelActivationIdentity,
    ) -> Result<(AgentBootstrapProgress, AgentSubscriptionRecord), AgentBootstrapError> {
        let bootstrap_id = identity.bootstrap_id.as_bytes();
        let natural_key =
            AgentSubscriptionRecord::natural_key(&input.seed_agent.agent_id, &input.belief_key);
        let subscription_id = deterministic_id("subscription", &natural_key);
        (
            &self.sequence,
            &self.progress,
            &self.subscriptions,
            &self.subscriptions_by_agent,
            &self.subscriptions_by_key,
        )
            .transaction(|(sequence, progress, subscriptions, by_agent, by_key)| {
                let current = load_progress_tx(progress, bootstrap_id, identity)?;
                if stage_at_least(current.stage, AgentBootstrapStage::SubscriptionBound) {
                    let raw = subscriptions
                        .get(subscription_id.as_bytes())?
                        .ok_or_else(|| {
                            abort_storage("subscription progress exists without record")
                        })?;
                    return Ok((current, decode_tx(&raw)?));
                }
                let seq = allocate_sequence(sequence)?;
                let record = match subscriptions.get(subscription_id.as_bytes())? {
                    Some(raw) => {
                        let durable: AgentSubscriptionRecord = decode_tx(&raw)?;
                        require_subscription_matches(&durable, input, &subscription_id)?;
                        durable
                    }
                    None => {
                        let record = AgentSubscriptionRecord {
                            subscription_id: subscription_id.clone(),
                            agent_id: input.seed_agent.agent_id.clone(),
                            belief_key: input.belief_key.clone(),
                            status: AgentSubscriptionStatus::Active,
                            last_delivered_revision_id: None,
                            last_delivered_seq: 0,
                            created_at_seq: seq,
                            updated_at_seq: seq,
                        };
                        subscriptions
                            .insert(subscription_id.as_bytes(), encode_tx(&record)?.as_slice())?;
                        record
                    }
                };
                by_agent.insert(
                    subscription_agent_key(
                        &record.agent_id,
                        record.created_at_seq,
                        &subscription_id,
                    )
                    .as_bytes(),
                    subscription_id.as_bytes(),
                )?;
                if let Some(raw) = by_key.get(natural_key.as_bytes())? {
                    if raw.as_ref() != subscription_id.as_bytes() {
                        return Err(abort_conflict_bytes(
                            "agent_subscription.natural_key",
                            subscription_id.as_bytes(),
                            raw.as_ref(),
                        ));
                    }
                }
                by_key.insert(natural_key.as_bytes(), subscription_id.as_bytes())?;
                let next = next_progress(&current, AgentBootstrapStage::SubscriptionBound, seq);
                progress.insert(bootstrap_id, encode_tx(&next)?.as_slice())?;
                Ok((next, record))
            })
            .map_err(map_transaction)
    }

    pub(super) fn confirm_seed_products(
        &self,
        input: &WorldModelActivationInput,
        identity: &WorldModelActivationIdentity,
    ) -> Result<AgentBootstrapProgress, AgentBootstrapError> {
        let bootstrap_id = identity.bootstrap_id.as_bytes();
        let subscription_id = deterministic_subscription_id(input);
        let natural_key =
            AgentSubscriptionRecord::natural_key(&input.seed_agent.agent_id, &input.belief_key);
        (
            &self.sequence,
            &self.progress,
            &self.directives,
            &self.agents,
            &self.curation_rules,
            &self.subscriptions,
            &self.subscriptions_by_key,
        )
            .transaction(
                |(
                    sequence,
                    progress,
                    directives,
                    agents,
                    rules,
                    subscriptions,
                    subscriptions_by_key,
                )| {
                    let current = load_progress_tx(progress, bootstrap_id, identity)?;
                    let raw = directives
                        .get(input.directive.directive_id.as_bytes())?
                        .ok_or_else(|| abort_storage("configured directive is missing"))?;
                    let directive: DirectiveRecord = decode_tx(&raw)?;
                    require_field("directive.text", &input.directive, &directive)?;
                    let raw = agents
                        .get(input.seed_agent.agent_id.as_bytes())?
                        .ok_or_else(|| abort_storage("configured agent is missing"))?;
                    let agent: AgentRecord = decode_tx(&raw)?;
                    require_canonical_agent_matches(&agent, input)?;
                    require_agent_sequence_shape_tx(&agent)?;
                    let raw = rules
                        .get(input.curation_rule.rule_id.as_bytes())?
                        .ok_or_else(|| abort_storage("configured curation rule is missing"))?;
                    let rule: AgentCurationRuleRecord = decode_tx(&raw)?;
                    require_field("curation_rule", &input.curation_rule, &rule)?;
                    let raw = subscriptions
                        .get(subscription_id.as_bytes())?
                        .ok_or_else(|| abort_storage("configured subscription is missing"))?;
                    let subscription: AgentSubscriptionRecord = decode_tx(&raw)?;
                    require_subscription_matches(&subscription, input, &subscription_id)?;
                    let indexed = subscriptions_by_key
                        .get(natural_key.as_bytes())?
                        .ok_or_else(|| {
                            abort_storage("subscription natural-key index is missing")
                        })?;
                    if indexed.as_ref() != subscription_id.as_bytes() {
                        return Err(abort_conflict_bytes(
                            "agent_subscription.natural_key",
                            subscription_id.as_bytes(),
                            indexed.as_ref(),
                        ));
                    }
                    if stage_at_least(current.stage, AgentBootstrapStage::ProductsConfirmed) {
                        return Ok(current);
                    }
                    let floor = durable_product_sequence_floor_tx(&agent, &subscription)?
                        .max(current.updated_at_seq);
                    let seq = allocate_sequence_after(sequence, floor)?;
                    let next = next_progress(&current, AgentBootstrapStage::ProductsConfirmed, seq);
                    progress.insert(bootstrap_id, encode_tx(&next)?.as_slice())?;
                    Ok(next)
                },
            )
            .map_err(map_transaction)
    }

    pub(super) fn complete(
        &self,
        input: &WorldModelActivationInput,
        identity: &WorldModelActivationIdentity,
        belief: BeliefActivationReceipt,
        subscription_id: String,
    ) -> Result<(AgentBootstrapProgress, AgentBootstrapReceipt), AgentBootstrapError> {
        let bootstrap_id = identity.bootstrap_id.as_bytes();
        (
            &self.sequence,
            &self.progress,
            &self.receipts,
            &self.agents,
            &self.subscriptions,
        )
            .transaction(|(sequence, progress, receipts, agents, subscriptions)| {
                let current = load_progress_tx(progress, bootstrap_id, identity)?;
                if let Some(raw) = receipts.get(bootstrap_id)? {
                    let receipt: AgentBootstrapReceipt = decode_tx(&raw)?;
                    require_receipt_identity(&receipt, input, identity, &belief, &subscription_id)?;
                    return Ok((current, receipt));
                }
                if !stage_at_least(current.stage, AgentBootstrapStage::ProductsConfirmed) {
                    return Err(abort_storage(
                        "bootstrap receipt requires confirmed seed products",
                    ));
                }
                let raw = agents
                    .get(input.seed_agent.agent_id.as_bytes())?
                    .ok_or_else(|| abort_storage("configured agent is missing"))?;
                let agent: AgentRecord = decode_tx(&raw)?;
                require_canonical_agent_matches(&agent, input)?;
                let raw = subscriptions
                    .get(subscription_id.as_bytes())?
                    .ok_or_else(|| abort_storage("configured subscription is missing"))?;
                let subscription: AgentSubscriptionRecord = decode_tx(&raw)?;
                require_subscription_matches(&subscription, input, &subscription_id)?;
                let floor = durable_product_sequence_floor_tx(&agent, &subscription)?
                    .max(current.updated_at_seq);
                let seq = allocate_sequence_after(sequence, floor)?;
                let receipt = AgentBootstrapReceipt {
                    receipt_id: deterministic_id("agent-bootstrap-receipt", &identity.bootstrap_id),
                    bootstrap_id: identity.bootstrap_id.clone(),
                    activation_hash: identity.activation_hash.clone(),
                    activation_id: identity.activation_id.clone(),
                    input_hash: identity.input_hash.clone(),
                    belief: belief.clone(),
                    directive_id: input.directive.directive_id.clone(),
                    agent_id: input.seed_agent.agent_id.clone(),
                    rule_id: input.curation_rule.rule_id.clone(),
                    subscription_id: subscription_id.clone(),
                    completed_at_seq: seq,
                };
                let next = next_progress(&current, AgentBootstrapStage::Completed, seq);
                receipts.insert(bootstrap_id, encode_tx(&receipt)?.as_slice())?;
                progress.insert(bootstrap_id, encode_tx(&next)?.as_slice())?;
                Ok((next, receipt))
            })
            .map_err(map_transaction)
    }

    pub(super) fn flush(&self) -> Result<(), AgentBootstrapError> {
        self.db.flush().map_err(storage)?;
        Ok(())
    }

    fn advance_progress(
        &self,
        identity: &WorldModelActivationIdentity,
        stage: AgentBootstrapStage,
    ) -> Result<AgentBootstrapProgress, AgentBootstrapError> {
        let bootstrap_id = identity.bootstrap_id.as_bytes();
        (&self.sequence, &self.progress)
            .transaction(|(sequence, progress)| {
                let current = load_progress_tx(progress, bootstrap_id, identity)?;
                if stage_at_least(current.stage, stage) {
                    return Ok(current);
                }
                let seq = allocate_sequence(sequence)?;
                let next = next_progress(&current, stage, seq);
                progress.insert(bootstrap_id, encode_tx(&next)?.as_slice())?;
                Ok(next)
            })
            .map_err(map_transaction)
    }
}

fn confirm_directive_tx(
    directives: &TransactionalTree,
    directive: &DirectiveRecord,
) -> Result<(), ConflictableTransactionError<BootstrapAbort>> {
    confirm_exact_tx(
        directives,
        &directive.directive_id,
        directive,
        "directive.text",
    )
}

fn confirm_or_migrate_agent_tx(
    agents: &TransactionalTree,
    migrations: &TransactionalTree,
    input: &WorldModelActivationInput,
    identity: &WorldModelActivationIdentity,
    seq: u64,
) -> Result<AgentRecord, ConflictableTransactionError<BootstrapAbort>> {
    let key = input.seed_agent.agent_id.as_bytes();
    let Some(raw) = agents.get(key)? else {
        let agent = configured_agent(input, seq);
        agents.insert(key, encode_tx(&agent)?.as_slice())?;
        return Ok(agent);
    };
    if let Ok(agent) = serde_json::from_slice::<AgentRecord>(&raw) {
        require_canonical_agent_matches(&agent, input)?;
        require_agent_sequence_shape_tx(&agent)?;
        return Ok(agent);
    }

    let legacy = decode_legacy_agent_record(&raw).map_err(abort_storage)?;
    let migration_identity = migration_identity(input, identity);
    require_legacy_matches(&legacy, input, &migration_identity)?;
    let canonical = legacy.canonical(input.directive.directive_id.clone());
    let receipt = LegacyDirectiveMigrationReceipt {
        receipt_id: deterministic_id(
            "legacy-directive-migration",
            &serde_json::to_string(&migration_identity)
                .map_err(|error| abort_storage(error.to_string()))?,
        ),
        identity: migration_identity,
        legacy_agent_record_hash: legacy.raw_hash().to_string(),
        directive_record_hash: semantic_hash(&input.directive)?,
        canonical_agent_record_hash: semantic_hash(&canonical)?,
        completed_at_seq: seq,
    };
    if let Some(raw_receipt) = migrations.get(receipt.receipt_id.as_bytes())? {
        let durable: LegacyDirectiveMigrationReceipt = decode_tx(&raw_receipt)?;
        if durable != receipt {
            return Err(abort_conflict(
                "legacy_directive_migration_receipt",
                &receipt,
                &durable,
            ));
        }
    }
    agents.insert(key, encode_tx(&canonical)?.as_slice())?;
    migrations.insert(
        receipt.receipt_id.as_bytes(),
        encode_tx(&receipt)?.as_slice(),
    )?;
    Ok(canonical)
}

fn configured_agent(input: &WorldModelActivationInput, seq: u64) -> AgentRecord {
    AgentRecord {
        agent_id: input.seed_agent.agent_id.clone(),
        perspective_key: input.seed_agent.perspective_key.clone(),
        subject: input.seed_agent.subject.clone(),
        branch_scope: input.seed_agent.branch_scope.clone(),
        observation_scope: input.seed_agent.observation_scope.clone(),
        directive_id: input.seed_agent.directive_id.clone(),
        seed_provenance: input.seed_agent.seed_provenance.clone(),
        status: AgentStatus::Registered,
        created_at_seq: seq,
        updated_at_seq: seq,
    }
}

fn require_canonical_agent_matches(
    agent: &AgentRecord,
    input: &WorldModelActivationInput,
) -> Result<(), ConflictableTransactionError<BootstrapAbort>> {
    require_field(
        "agent.agent_id",
        input.seed_agent.agent_id.as_str(),
        &agent.agent_id,
    )?;
    require_field(
        "agent.perspective_key",
        &input.seed_agent.perspective_key,
        &agent.perspective_key,
    )?;
    require_field("agent.subject", &input.seed_agent.subject, &agent.subject)?;
    require_field(
        "agent.branch_scope",
        &input.seed_agent.branch_scope,
        &agent.branch_scope,
    )?;
    require_field(
        "agent.observation_scope",
        input.seed_agent.observation_scope.as_str(),
        &agent.observation_scope,
    )?;
    require_field(
        "agent.directive_id",
        &input.seed_agent.directive_id,
        &agent.directive_id,
    )?;
    require_field(
        "agent.seed_provenance",
        &input.seed_agent.seed_provenance,
        &agent.seed_provenance,
    )
}

fn require_agent_sequence_shape_tx(
    agent: &AgentRecord,
) -> Result<(), ConflictableTransactionError<BootstrapAbort>> {
    if agent.updated_at_seq < agent.created_at_seq {
        Err(abort_storage(
            "agent updated sequence precedes creation sequence",
        ))
    } else {
        Ok(())
    }
}

fn durable_product_sequence_floor_tx(
    agent: &AgentRecord,
    subscription: &AgentSubscriptionRecord,
) -> Result<u64, ConflictableTransactionError<BootstrapAbort>> {
    require_agent_sequence_shape_tx(agent)?;
    if subscription.updated_at_seq < subscription.created_at_seq {
        return Err(abort_storage(
            "subscription updated sequence precedes creation sequence",
        ));
    }
    Ok(agent
        .created_at_seq
        .max(agent.updated_at_seq)
        .max(subscription.created_at_seq)
        .max(subscription.updated_at_seq)
        .max(subscription.last_delivered_seq))
}

fn require_subscription_matches(
    subscription: &AgentSubscriptionRecord,
    input: &WorldModelActivationInput,
    subscription_id: &str,
) -> Result<(), ConflictableTransactionError<BootstrapAbort>> {
    require_field(
        "agent_subscription.subscription_id",
        subscription_id,
        subscription.subscription_id.as_str(),
    )?;
    require_field(
        "agent_subscription.agent_id",
        input.seed_agent.agent_id.as_str(),
        subscription.agent_id.as_str(),
    )?;
    require_field(
        "agent_subscription.belief_key",
        &input.belief_key,
        &subscription.belief_key,
    )?;
    if subscription.updated_at_seq < subscription.created_at_seq {
        return Err(abort_storage(
            "subscription updated sequence precedes creation sequence",
        ));
    }
    Ok(())
}

fn require_canonical_agent_matches_read(
    agent: &AgentRecord,
    input: &WorldModelActivationInput,
) -> Result<(), AgentBootstrapError> {
    require_exact_read(
        "agent.agent_id",
        input.seed_agent.agent_id.as_str(),
        agent.agent_id.as_str(),
    )?;
    require_exact_read(
        "agent.perspective_key",
        &input.seed_agent.perspective_key,
        &agent.perspective_key,
    )?;
    require_exact_read("agent.subject", &input.seed_agent.subject, &agent.subject)?;
    require_exact_read(
        "agent.branch_scope",
        &input.seed_agent.branch_scope,
        &agent.branch_scope,
    )?;
    require_exact_read(
        "agent.observation_scope",
        input.seed_agent.observation_scope.as_str(),
        agent.observation_scope.as_str(),
    )?;
    require_exact_read(
        "agent.directive_id",
        input.seed_agent.directive_id.as_str(),
        agent.directive_id.as_str(),
    )?;
    require_exact_read(
        "agent.seed_provenance",
        input.seed_agent.seed_provenance.as_str(),
        agent.seed_provenance.as_str(),
    )
}

fn require_subscription_matches_read(
    subscription: &AgentSubscriptionRecord,
    input: &WorldModelActivationInput,
    subscription_id: &str,
) -> Result<(), AgentBootstrapError> {
    require_exact_read(
        "agent_subscription.subscription_id",
        subscription_id,
        subscription.subscription_id.as_str(),
    )?;
    require_exact_read(
        "agent_subscription.agent_id",
        input.seed_agent.agent_id.as_str(),
        subscription.agent_id.as_str(),
    )?;
    require_exact_read(
        "agent_subscription.belief_key",
        &input.belief_key,
        &subscription.belief_key,
    )
}

fn require_legacy_matches(
    legacy: &DecodedLegacyAgent,
    input: &WorldModelActivationInput,
    identity: &LegacyDirectiveMigrationIdentity,
) -> Result<(), ConflictableTransactionError<BootstrapAbort>> {
    require_legacy_field(
        identity,
        LegacyDirectiveMigrationConflictField::AgentId,
        input.seed_agent.agent_id.as_str(),
        legacy.agent_id(),
    )?;
    require_legacy_field(
        identity,
        LegacyDirectiveMigrationConflictField::PerspectiveKey,
        &input.seed_agent.perspective_key,
        legacy.perspective_key(),
    )?;
    require_legacy_field(
        identity,
        LegacyDirectiveMigrationConflictField::Subject,
        &input.seed_agent.subject,
        legacy.subject(),
    )?;
    require_legacy_field(
        identity,
        LegacyDirectiveMigrationConflictField::BranchScope,
        &input.seed_agent.branch_scope,
        legacy.branch_scope(),
    )?;
    require_legacy_field(
        identity,
        LegacyDirectiveMigrationConflictField::ObservationScope,
        input.seed_agent.observation_scope.as_str(),
        legacy.observation_scope(),
    )?;
    require_legacy_field(
        identity,
        LegacyDirectiveMigrationConflictField::DirectiveText,
        input.directive.text.as_str(),
        legacy.directive_text(),
    )?;
    require_legacy_field(
        identity,
        LegacyDirectiveMigrationConflictField::SeedProvenance,
        input.seed_agent.seed_provenance.as_str(),
        legacy.seed_provenance(),
    )
}

fn require_legacy_field<T: Serialize + PartialEq + ?Sized>(
    identity: &LegacyDirectiveMigrationIdentity,
    field: LegacyDirectiveMigrationConflictField,
    configured: &T,
    legacy: &T,
) -> Result<(), ConflictableTransactionError<BootstrapAbort>> {
    if configured == legacy {
        return Ok(());
    }
    Err(ConflictableTransactionError::Abort(
        BootstrapAbort::LegacyConflict(Box::new(LegacyDirectiveMigrationConflict {
            identity: identity.clone(),
            field,
            configured_value_hash: semantic_hash(configured)?,
            legacy_value_hash: semantic_hash(legacy)?,
        })),
    ))
}

fn require_field<T: Serialize + PartialEq + ?Sized>(
    field: &str,
    configured: &T,
    durable: &T,
) -> Result<(), ConflictableTransactionError<BootstrapAbort>> {
    if configured == durable {
        Ok(())
    } else {
        Err(abort_conflict(field, configured, durable))
    }
}

fn confirm_exact_tx<T: Serialize + serde::de::DeserializeOwned + PartialEq>(
    tree: &TransactionalTree,
    key: &str,
    configured: &T,
    field: &str,
) -> Result<(), ConflictableTransactionError<BootstrapAbort>> {
    if let Some(raw) = tree.get(key.as_bytes())? {
        let durable: T = decode_tx(&raw)?;
        if durable != *configured {
            return Err(abort_conflict(field, configured, &durable));
        }
        return Ok(());
    }
    tree.insert(key.as_bytes(), encode_tx(configured)?.as_slice())?;
    Ok(())
}

fn require_progress_identity(
    progress: &AgentBootstrapProgress,
    identity: &WorldModelActivationIdentity,
) -> Result<(), ConflictableTransactionError<BootstrapAbort>> {
    require_field(
        "bootstrap.bootstrap_id",
        &identity.bootstrap_id,
        &progress.bootstrap_id,
    )?;
    require_field(
        "bootstrap.activation_id",
        &identity.activation_id,
        &progress.activation_id,
    )?;
    require_field(
        "bootstrap.activation_hash",
        &identity.activation_hash,
        &progress.activation_hash,
    )?;
    require_field(
        "bootstrap.input_hash",
        &identity.input_hash,
        &progress.input_hash,
    )
}

fn require_progress_identity_read(
    progress: &AgentBootstrapProgress,
    identity: &WorldModelActivationIdentity,
) -> Result<(), AgentBootstrapError> {
    require_exact_read(
        "bootstrap.bootstrap_id",
        identity.bootstrap_id.as_str(),
        progress.bootstrap_id.as_str(),
    )?;
    require_exact_read(
        "bootstrap.activation_id",
        identity.activation_id.as_str(),
        progress.activation_id.as_str(),
    )?;
    require_exact_read(
        "bootstrap.activation_hash",
        identity.activation_hash.as_str(),
        progress.activation_hash.as_str(),
    )?;
    require_exact_read(
        "bootstrap.input_hash",
        identity.input_hash.as_str(),
        progress.input_hash.as_str(),
    )
}

fn require_progress_shape(progress: &AgentBootstrapProgress) -> Result<(), AgentBootstrapError> {
    let expected_status = if progress.stage == AgentBootstrapStage::Completed {
        AgentBootstrapProgressStatus::Completed
    } else {
        AgentBootstrapProgressStatus::Started
    };
    require_exact_read(
        "bootstrap.progress_status",
        &expected_status,
        &progress.status,
    )?;
    if progress.updated_at_seq == 0 {
        return Err(AgentBootstrapError::Storage {
            message: "bootstrap progress sequence must be positive".to_string(),
        });
    }
    Ok(())
}

fn require_receipt_identity(
    receipt: &AgentBootstrapReceipt,
    input: &WorldModelActivationInput,
    identity: &WorldModelActivationIdentity,
    belief: &BeliefActivationReceipt,
    subscription_id: &str,
) -> Result<(), ConflictableTransactionError<BootstrapAbort>> {
    let expected = AgentBootstrapReceipt {
        receipt_id: deterministic_id("agent-bootstrap-receipt", &identity.bootstrap_id),
        bootstrap_id: identity.bootstrap_id.clone(),
        activation_hash: identity.activation_hash.clone(),
        activation_id: identity.activation_id.clone(),
        input_hash: identity.input_hash.clone(),
        belief: belief.clone(),
        directive_id: input.directive.directive_id.clone(),
        agent_id: input.seed_agent.agent_id.clone(),
        rule_id: input.curation_rule.rule_id.clone(),
        subscription_id: subscription_id.to_string(),
        completed_at_seq: receipt.completed_at_seq,
    };
    if *receipt == expected {
        Ok(())
    } else {
        Err(abort_conflict("bootstrap_receipt", &expected, receipt))
    }
}

fn require_receipt_identity_read(
    receipt: &AgentBootstrapReceipt,
    input: &WorldModelActivationInput,
    identity: &WorldModelActivationIdentity,
    belief: &BeliefActivationReceipt,
) -> Result<(), AgentBootstrapError> {
    let expected = AgentBootstrapReceipt {
        receipt_id: deterministic_id("agent-bootstrap-receipt", &identity.bootstrap_id),
        bootstrap_id: identity.bootstrap_id.clone(),
        activation_hash: identity.activation_hash.clone(),
        activation_id: identity.activation_id.clone(),
        input_hash: identity.input_hash.clone(),
        belief: belief.clone(),
        directive_id: input.directive.directive_id.clone(),
        agent_id: input.seed_agent.agent_id.clone(),
        rule_id: input.curation_rule.rule_id.clone(),
        subscription_id: deterministic_subscription_id(input),
        completed_at_seq: receipt.completed_at_seq,
    };
    require_exact_read("bootstrap_receipt", &expected, receipt)
}

fn migration_identity(
    input: &WorldModelActivationInput,
    identity: &WorldModelActivationIdentity,
) -> LegacyDirectiveMigrationIdentity {
    LegacyDirectiveMigrationIdentity {
        schema_version: LEGACY_DIRECTIVE_MIGRATION_SCHEMA_VERSION,
        activation_id: identity.activation_id.clone(),
        bootstrap_id: identity.bootstrap_id.clone(),
        agent_id: input.seed_agent.agent_id.clone(),
        directive_id: input.directive.directive_id.clone(),
    }
}

fn load_progress_tx(
    tree: &TransactionalTree,
    bootstrap_id: &[u8],
    identity: &WorldModelActivationIdentity,
) -> Result<AgentBootstrapProgress, ConflictableTransactionError<BootstrapAbort>> {
    let raw = tree
        .get(bootstrap_id)?
        .ok_or_else(|| abort_storage("bootstrap progress is missing"))?;
    let progress: AgentBootstrapProgress = decode_tx(&raw)?;
    require_progress_identity(&progress, identity)?;
    Ok(progress)
}

fn next_progress(
    current: &AgentBootstrapProgress,
    stage: AgentBootstrapStage,
    seq: u64,
) -> AgentBootstrapProgress {
    AgentBootstrapProgress {
        stage,
        status: if stage_at_least(stage, AgentBootstrapStage::Completed) {
            AgentBootstrapProgressStatus::Completed
        } else {
            AgentBootstrapProgressStatus::Started
        },
        updated_at_seq: seq,
        ..current.clone()
    }
}

pub(super) fn stage_at_least(current: AgentBootstrapStage, expected: AgentBootstrapStage) -> bool {
    stage_rank(current) >= stage_rank(expected)
}

fn stage_rank(stage: AgentBootstrapStage) -> u8 {
    match stage {
        AgentBootstrapStage::Started => 0,
        AgentBootstrapStage::BeliefConfigured => 1,
        AgentBootstrapStage::AgentRegistered => 2,
        AgentBootstrapStage::RuleRegistered => 3,
        AgentBootstrapStage::SubscriptionBound => 4,
        AgentBootstrapStage::ProductsConfirmed => 5,
        AgentBootstrapStage::Completed => 6,
    }
}

fn existing_sequence_floor_tx(
    agents: &TransactionalTree,
    subscriptions: &TransactionalTree,
    agent_id: &str,
    subscription_id: &str,
) -> Result<u64, ConflictableTransactionError<BootstrapAbort>> {
    let mut floor = 0;
    if let Some(raw) = agents.get(agent_id.as_bytes())? {
        if let Ok(agent) = serde_json::from_slice::<AgentRecord>(&raw) {
            if agent.updated_at_seq < agent.created_at_seq {
                return Err(abort_storage(
                    "agent updated sequence precedes creation sequence",
                ));
            }
            floor = floor.max(agent.updated_at_seq);
        } else {
            let legacy = decode_legacy_agent_record(&raw).map_err(abort_storage)?;
            floor = floor.max(legacy.sequence_floor().map_err(abort_storage)?);
        }
    }
    if let Some(raw) = subscriptions.get(subscription_id.as_bytes())? {
        let subscription: AgentSubscriptionRecord = decode_tx(&raw)?;
        if subscription.updated_at_seq < subscription.created_at_seq {
            return Err(abort_storage(
                "subscription updated sequence precedes creation sequence",
            ));
        }
        floor = floor
            .max(subscription.created_at_seq)
            .max(subscription.updated_at_seq)
            .max(subscription.last_delivered_seq);
    }
    Ok(floor)
}

fn allocate_sequence(
    tree: &TransactionalTree,
) -> Result<u64, ConflictableTransactionError<BootstrapAbort>> {
    allocate_sequence_after(tree, 0)
}

fn allocate_sequence_after(
    tree: &TransactionalTree,
    floor: u64,
) -> Result<u64, ConflictableTransactionError<BootstrapAbort>> {
    let current = match tree.get(KEY_NEXT_SEQUENCE)? {
        Some(raw) => u64::from_be_bytes(
            raw.as_ref()
                .try_into()
                .map_err(|_| abort_storage("bootstrap sequence has invalid width"))?,
        ),
        None => 0,
    };
    let next = current
        .max(floor)
        .checked_add(1)
        .ok_or_else(|| abort_storage("bootstrap sequence exhausted"))?;
    tree.insert(KEY_NEXT_SEQUENCE, &next.to_be_bytes())?;
    Ok(next)
}

#[derive(Debug, Clone)]
enum BootstrapAbort {
    Conflict {
        field: String,
        configured_value_hash: String,
        durable_value_hash: String,
    },
    LegacyConflict(Box<LegacyDirectiveMigrationConflict>),
    Storage(String),
}

fn abort_conflict<T: Serialize + ?Sized>(
    field: &str,
    configured: &T,
    durable: &T,
) -> ConflictableTransactionError<BootstrapAbort> {
    match (semantic_hash(configured), semantic_hash(durable)) {
        (Ok(configured_value_hash), Ok(durable_value_hash)) => {
            ConflictableTransactionError::Abort(BootstrapAbort::Conflict {
                field: field.to_string(),
                configured_value_hash,
                durable_value_hash,
            })
        }
        (Err(error), _) | (_, Err(error)) => error,
    }
}

fn abort_conflict_bytes(
    field: &str,
    configured: &[u8],
    durable: &[u8],
) -> ConflictableTransactionError<BootstrapAbort> {
    ConflictableTransactionError::Abort(BootstrapAbort::Conflict {
        field: field.to_string(),
        configured_value_hash: blake3::hash(configured).to_hex().to_string(),
        durable_value_hash: blake3::hash(durable).to_hex().to_string(),
    })
}

fn abort_storage(message: impl Into<String>) -> ConflictableTransactionError<BootstrapAbort> {
    ConflictableTransactionError::Abort(BootstrapAbort::Storage(message.into()))
}

fn map_transaction(error: TransactionError<BootstrapAbort>) -> AgentBootstrapError {
    match error {
        TransactionError::Abort(BootstrapAbort::Conflict {
            field,
            configured_value_hash,
            durable_value_hash,
        }) => AgentBootstrapError::Conflict {
            field,
            configured_value_hash,
            durable_value_hash,
        },
        TransactionError::Abort(BootstrapAbort::LegacyConflict(conflict)) => {
            AgentBootstrapError::LegacyDirectiveConflict(conflict)
        }
        TransactionError::Abort(BootstrapAbort::Storage(message)) => {
            AgentBootstrapError::Storage { message }
        }
        TransactionError::Storage(error) => storage(error),
    }
}

fn semantic_hash<T: Serialize + ?Sized>(
    value: &T,
) -> Result<String, ConflictableTransactionError<BootstrapAbort>> {
    let bytes = serde_json::to_vec(value).map_err(|error| abort_storage(error.to_string()))?;
    Ok(blake3::hash(&bytes).to_hex().to_string())
}

fn encode_tx<T: Serialize + ?Sized>(
    value: &T,
) -> Result<Vec<u8>, ConflictableTransactionError<BootstrapAbort>> {
    serde_json::to_vec(value).map_err(|error| abort_storage(error.to_string()))
}

fn decode_tx<T: serde::de::DeserializeOwned>(
    raw: &[u8],
) -> Result<T, ConflictableTransactionError<BootstrapAbort>> {
    serde_json::from_slice(raw).map_err(|error| abort_storage(error.to_string()))
}

fn decode_optional<T: serde::de::DeserializeOwned>(
    raw: Option<sled::IVec>,
) -> Result<Option<T>, AgentBootstrapError> {
    raw.map(|value| {
        serde_json::from_slice(&value).map_err(|error| AgentBootstrapError::Storage {
            message: error.to_string(),
        })
    })
    .transpose()
}

fn required_record<T: serde::de::DeserializeOwned>(
    tree: &Tree,
    key: &str,
    record_name: &str,
) -> Result<T, AgentBootstrapError> {
    let raw =
        tree.get(key.as_bytes())
            .map_err(storage)?
            .ok_or_else(|| AgentBootstrapError::Storage {
                message: format!("{record_name} record is missing"),
            })?;
    serde_json::from_slice(&raw).map_err(|error| AgentBootstrapError::Storage {
        message: format!("cannot decode {record_name} record: {error}"),
    })
}

fn require_exact_read<T: Serialize + PartialEq + ?Sized>(
    field: &str,
    configured: &T,
    durable: &T,
) -> Result<(), AgentBootstrapError> {
    if configured == durable {
        Ok(())
    } else {
        Err(conflict(field, configured, durable))
    }
}

fn conflict<T: Serialize + ?Sized>(
    field: &str,
    configured: &T,
    durable: &T,
) -> AgentBootstrapError {
    AgentBootstrapError::Conflict {
        field: field.to_string(),
        configured_value_hash: semantic_hash_read(configured),
        durable_value_hash: semantic_hash_read(durable),
    }
}

fn conflict_bytes(field: &str, configured: &[u8], durable: &[u8]) -> AgentBootstrapError {
    AgentBootstrapError::Conflict {
        field: field.to_string(),
        configured_value_hash: blake3::hash(configured).to_hex().to_string(),
        durable_value_hash: blake3::hash(durable).to_hex().to_string(),
    }
}

fn semantic_hash_read<T: Serialize + ?Sized>(value: &T) -> String {
    let bytes = serde_json::to_vec(value).unwrap_or_else(|error| error.to_string().into_bytes());
    blake3::hash(&bytes).to_hex().to_string()
}

pub(super) fn deterministic_subscription_id(input: &WorldModelActivationInput) -> String {
    deterministic_id(
        "subscription",
        &AgentSubscriptionRecord::natural_key(&input.seed_agent.agent_id, &input.belief_key),
    )
}

fn agent_status_key(status: &str, seq: u64, agent_id: &str) -> String {
    format!("{status}::{seq:0KEY_PAD$}::{agent_id}")
}

fn subscription_agent_key(agent_id: &str, seq: u64, subscription_id: &str) -> String {
    format!("{agent_id}::{seq:0KEY_PAD$}::{subscription_id}")
}

fn open(db: &Db, name: &str) -> Result<Tree, AgentBootstrapError> {
    db.open_tree(name).map_err(storage)
}

fn storage(error: sled::Error) -> AgentBootstrapError {
    AgentBootstrapError::Storage {
        message: error.to_string(),
    }
}
