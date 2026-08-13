//! World-initialization pipeline: theory installation, identity genesis,
//! and epistemic seeding.
//!
//! Owner: root init. This is the command path behind the frozen shapes in
//! [`crate::init::world`], executing the three world-initialization stages
//! of the staged contract in
//! `design/plan/integration/runtime_initialization.md`: theory installs
//! through the world-model registry command, genesis identities are created
//! through the agent registration commands, and the epistemic genesis fact
//! seeds through the canonical event append. The pipeline never writes a domain
//! store directly — every durable effect crosses a domain command or the
//! append capability, so each stage stays idempotent by the content
//! identity its owning domain defines.
//!
//! Stage selection is normalized to pipeline order without duplicates: a
//! request only chooses the subset, never the order. Re-running any subset
//! against unchanged content reports `Unchanged` with the same record ids.

use meld_events::{
    AppendDisposition, AppendMode, DomainObjectRef, EventAppendCapability, EventEnvelope,
};
use meld_execution::authority::AuthorityPolicyRegistryStore;
use meld_lang::AuthorityPolicy;
use meld_world_model::agent::{
    AgentCurationRuleBinding, AgentCurationRuleConfig, AgentCurationRuleRegistryStore,
    AgentMaintainedCondition, AgentMaintainedConditionRegistryStore, AgentRegistration,
    AgentStatus, AgentStore, AgentSubscription, SeedAgentRegistration, SubscribeAgentCommand,
};
use meld_world_model::belief::genesis::{
    UnobservedScopeDeclaration, EPISTEMIC_GENESIS_STREAM_ID, UNOBSERVED_SCOPE_EVENT_TYPE,
};
use meld_world_model::belief::{
    configured_belief_key, BeliefFamilyConfig, BeliefFamilyRegistry, BranchScope,
    OutcomeMappingRegistryStore, OutcomeMappingSetConfig, TheoryInstallDisposition,
};
use meld_world_model::strategy::{StrategyTheoryPackage, StrategyTheoryRegistryStore};
use meld_world_model::PerspectiveKey;
use thiserror::Error;

use crate::config::SelectedStewardshipPackage;
use crate::docs::claim_validation::{DocsClaimPolicy, DocsClaimPolicyRegistryStore};
use crate::init::world::{
    StageDisposition, WorldInitReport, WorldInitRequest, WorldInitStage, WorldInitStageReport,
};
use crate::runtime::theory::{TheoryInstallationReceipt, TheoryInstallationReceiptStore};
use meld_execution::capability::{CapabilityContractRegistryStore, CapabilityTypeContract};

/// Domain that owns the epistemic genesis fact appended by stage 4.
const GENESIS_DOMAIN_ID: &str = "world_model";

/// Canonical stage order of the world-initialization pipeline.
const PIPELINE_ORDER: [WorldInitStage; 3] = [
    WorldInitStage::InstallTheory,
    WorldInitStage::GenesisIdentities,
    WorldInitStage::SeedEpistemicFacts,
];

/// Failure of one world-initialization stage.
///
/// Every variant names the stage that failed so a partial run stays
/// diagnosable; stages already completed hold their durable effects, and
/// re-running the pipeline after the cause is fixed is always safe.
#[derive(Debug, Error)]
pub enum WorldInitError {
    /// Stage 2 could not install or resolve theory.
    #[error("install theory stage failed: {0}")]
    Theory(String),
    /// Stage 3 could not create or resolve genesis identities.
    #[error("genesis identities stage failed: {0}")]
    Identity(String),
    /// Stage 3 or 4 requires an installed theory revision that is missing.
    #[error(
        "belief family '{family_id}' has no installed revision; run the install-theory stage first"
    )]
    TheoryNotInstalled {
        /// Family the failing stage tried to resolve.
        family_id: String,
    },
    /// Stage 4 could not append the epistemic genesis fact.
    #[error("seed epistemic facts stage failed: {0}")]
    Genesis(String),
}

/// Semantic content for one world-initialization run.
///
/// Resolved by the caller before the pipeline runs: identities come from
/// the stage 0 stewardship selection, theory bodies from the XDG theory
/// configuration source, and `observed_seq` from the ledger. The pipeline
/// itself resolves nothing from the environment, so isolate boots can
/// inject synthetic content.
#[derive(Debug, Clone)]
pub struct WorldInitContent {
    /// Belief family definition installed by stage 2.
    pub family_config: BeliefFamilyConfig,
    /// Curation rule installed on the seed agent record by stage 3.
    pub curation_rule: AgentCurationRuleConfig,
    /// Durable seed agent identity.
    pub agent_id: String,
    /// Selected subtree subject the world is about.
    pub subject: DomainObjectRef,
    /// Perspective registered for the seed agent.
    pub perspective: PerspectiveKey,
    /// Branch whose belief stream the seed agent owns.
    pub branch_scope: BranchScope,
    /// Named observation scope stored on the agent record.
    pub observation_scope: String,
    /// Compatibility directive string bound at stage 3.
    pub directive: String,
    /// Provenance recorded on the agent record and the genesis fact.
    pub provenance: String,
    /// Session partition for the stage 4 ledger append.
    pub session_id: String,
    /// Ledger sequence observed when this run started; recorded on new
    /// registrations and revisions, ignored by unchanged re-runs.
    pub observed_seq: u64,
}

/// Complete validated semantic image supplied to owner installation commands.
#[derive(Debug, Clone)]
pub struct WorldInitTheoryBundle {
    pub family: BeliefFamilyConfig,
    pub curation_rule: AgentCurationRuleConfig,
    pub maintained_condition: AgentMaintainedCondition,
    pub outcome_mapping: OutcomeMappingSetConfig,
    pub strategy_theory: StrategyTheoryPackage,
    pub executable_contracts: Vec<CapabilityTypeContract>,
    pub authority_policy: AuthorityPolicy,
    pub claim_policy: DocsClaimPolicy,
}

/// Exact owner stores needed to install and activate a complete image.
pub struct CompleteTheoryInstall<'a> {
    pub selection: SelectedStewardshipPackage,
    pub bundle: WorldInitTheoryBundle,
    pub curation_rules: &'a AgentCurationRuleRegistryStore,
    pub maintained_conditions: &'a AgentMaintainedConditionRegistryStore,
    pub outcome_mappings: &'a OutcomeMappingRegistryStore,
    pub strategy_theories: &'a StrategyTheoryRegistryStore,
    pub executable_contracts: &'a CapabilityContractRegistryStore,
    pub authority_policies: &'a AuthorityPolicyRegistryStore,
    pub claim_policies: &'a DocsClaimPolicyRegistryStore,
    pub receipts: &'a TheoryInstallationReceiptStore,
}

/// Normalize a stage selection to pipeline order without duplicates.
///
/// The request only selects the subset: duplicates collapse and request
/// order is ignored, so `[seed, install, seed]` runs install first and
/// seeds once.
pub fn normalized_stages(request: &WorldInitRequest) -> Vec<WorldInitStage> {
    PIPELINE_ORDER
        .into_iter()
        .filter(|stage| request.stages.contains(stage))
        .collect()
}

/// The stage 2 through 4 command pipeline over injected domain capabilities.
///
/// Holds no storage of its own: the belief-family registry, the agent
/// store, and the append capability are the only write surfaces, and all
/// three are domain-owned.
pub struct WorldInitPipeline<'a> {
    registry: &'a mut dyn BeliefFamilyRegistry,
    agent_store: &'a AgentStore,
    append: &'a EventAppendCapability,
    complete_theory: Option<CompleteTheoryInstall<'a>>,
}

impl<'a> WorldInitPipeline<'a> {
    /// Bind the pipeline to its domain command surfaces.
    pub fn new(
        registry: &'a mut dyn BeliefFamilyRegistry,
        agent_store: &'a AgentStore,
        append: &'a EventAppendCapability,
    ) -> Self {
        Self {
            registry,
            agent_store,
            append,
            complete_theory: None,
        }
    }

    /// Bind the complete image and all owner stores for the elevated path.
    pub fn with_complete_theory(mut self, install: CompleteTheoryInstall<'a>) -> Self {
        self.complete_theory = Some(install);
        self
    }

    /// Run the selected stages in pipeline order and report per stage.
    ///
    /// Stops at the first failing stage; completed stages keep their
    /// durable effects and the run is safe to repeat.
    pub fn run(
        &mut self,
        request: &WorldInitRequest,
        content: &WorldInitContent,
    ) -> Result<WorldInitReport, WorldInitError> {
        let mut stage_reports = Vec::new();
        for stage in normalized_stages(request) {
            let report = match stage {
                WorldInitStage::InstallTheory => self.install_theory(content)?,
                WorldInitStage::GenesisIdentities => self.genesis_identities(content)?,
                WorldInitStage::SeedEpistemicFacts => self.seed_epistemic_facts(content)?,
            };
            stage_reports.push(report);
        }
        Ok(WorldInitReport { stage_reports })
    }

    /// Stage 2: install the selected belief family as a content-hash
    /// revision through the world-model registry command.
    ///
    /// Only the belief family has a durable registry today. The evidence
    /// mapping selected by id and the curation rule body are carried as
    /// configuration: the mapping is consumed by the ingestion actor's
    /// configured mapping and the rule is installed on the agent record in
    /// stage 3. Durable registries for those theory kinds are deferred.
    fn install_theory(
        &mut self,
        content: &WorldInitContent,
    ) -> Result<WorldInitStageReport, WorldInitError> {
        if self.complete_theory.is_some() {
            return self.install_complete_theory(content.observed_seq, &content.subject);
        }
        let (disposition, revision) = self
            .registry
            .install(content.family_config.clone(), content.observed_seq)
            .map_err(|error| WorldInitError::Theory(error.to_string()))?;
        let revision_ref = revision.revision_ref();
        Ok(WorldInitStageReport {
            stage: WorldInitStage::InstallTheory,
            disposition: match disposition {
                TheoryInstallDisposition::Installed => StageDisposition::Applied,
                TheoryInstallDisposition::Unchanged => StageDisposition::Unchanged,
            },
            record_ids: vec![format!(
                "{}::{}::{}",
                revision_ref.registry, revision_ref.id, revision_ref.content_hash
            )],
        })
    }

    fn install_complete_theory(
        &mut self,
        observed_seq: u64,
        subject: &DomainObjectRef,
    ) -> Result<WorldInitStageReport, WorldInitError> {
        let install = self
            .complete_theory
            .as_ref()
            .expect("complete theory checked");
        validate_selected_bundle(&install.selection, &install.bundle, subject)
            .map_err(WorldInitError::Theory)?;

        let (family_disposition, family) = self
            .registry
            .install(install.bundle.family.clone(), observed_seq)
            .map_err(|error| WorldInitError::Theory(error.to_string()))?;
        let (curation_disposition, curation) = install
            .curation_rules
            .install(
                &install.selection.curation_rule_id,
                install.bundle.curation_rule.clone(),
                observed_seq,
            )
            .map_err(|error| WorldInitError::Theory(error.to_string()))?;
        let (condition_disposition, condition) = install
            .maintained_conditions
            .install(install.bundle.maintained_condition.clone(), observed_seq)
            .map_err(|error| WorldInitError::Theory(error.to_string()))?;
        let (mapping_disposition, mapping) = install
            .outcome_mappings
            .install(install.bundle.outcome_mapping.clone(), observed_seq)
            .map_err(|error| WorldInitError::Theory(error.to_string()))?;

        let mut capability_changed = false;
        let mut capabilities = Vec::new();
        for contract in &install.bundle.executable_contracts {
            let (changed, revision) = install
                .executable_contracts
                .install(contract.clone(), observed_seq)
                .map_err(|error| WorldInitError::Theory(error.to_string()))?;
            capability_changed |= changed;
            capabilities.push(revision);
        }
        validate_strategy_contracts(&install.bundle.strategy_theory, &capabilities)
            .map_err(WorldInitError::Theory)?;
        let (strategy_disposition, strategy) = install
            .strategy_theories
            .install(install.bundle.strategy_theory.clone(), observed_seq)
            .map_err(|error| WorldInitError::Theory(error.to_string()))?;
        let (authority_changed, authority) = install
            .authority_policies
            .install(install.bundle.authority_policy.clone(), observed_seq)
            .map_err(|error| WorldInitError::Theory(error.to_string()))?;
        let (claim_changed, claim) = install
            .claim_policies
            .install(install.bundle.claim_policy.clone(), observed_seq)
            .map_err(|error| WorldInitError::Theory(error.to_string()))?;

        if self
            .registry
            .resolve(&family.family_id, &family.content_hash)
            .map_err(|error| WorldInitError::Theory(error.to_string()))?
            .as_ref()
            != Some(&family)
            || install
                .curation_rules
                .resolve(&curation.rule_id, &curation.content_hash)
                .map_err(|error| WorldInitError::Theory(error.to_string()))?
                .as_ref()
                != Some(&curation)
            || install
                .maintained_conditions
                .resolve(&condition.condition_id, &condition.content_hash)
                .map_err(|error| WorldInitError::Theory(error.to_string()))?
                .as_ref()
                != Some(&condition)
            || install
                .outcome_mappings
                .resolve(&mapping.mapping_id, &mapping.content_hash)
                .map_err(|error| WorldInitError::Theory(error.to_string()))?
                .as_ref()
                != Some(&mapping)
            || install
                .strategy_theories
                .resolve(&strategy.theory_id, &strategy.content_hash)
                .map_err(|error| WorldInitError::Theory(error.to_string()))?
                .as_ref()
                != Some(&strategy)
            || install
                .authority_policies
                .resolve(&authority.policy.policy_id, &authority.content_hash)
                .map_err(|error| WorldInitError::Theory(error.to_string()))?
                .as_ref()
                != Some(&authority)
            || install
                .claim_policies
                .resolve(&claim.revision_ref())
                .map_err(|error| WorldInitError::Theory(error.to_string()))?
                .as_ref()
                != Some(&claim)
        {
            return Err(WorldInitError::Theory(
                "an owner did not resolve the exact revision it installed".to_string(),
            ));
        }
        for capability in &capabilities {
            if install
                .executable_contracts
                .resolve(&capability.revision_ref())
                .map_err(|error| WorldInitError::Theory(error.to_string()))?
                .as_ref()
                != Some(capability)
            {
                return Err(WorldInitError::Theory(
                    "execution did not resolve the exact contract revision it installed"
                        .to_string(),
                ));
            }
        }

        let receipt = TheoryInstallationReceipt::new(
            install.selection.clone(),
            family.revision_ref(),
            curation.revision_ref(),
            condition.revision_ref(),
            mapping.revision_ref(),
            strategy.revision_ref(),
            capabilities
                .iter()
                .map(|item| item.revision_ref())
                .collect(),
            authority.revision_ref(),
            claim.revision_ref(),
            observed_seq,
        )
        .map_err(|error| WorldInitError::Theory(error.to_string()))?;
        let receipt_changed = install
            .receipts
            .install(receipt.clone())
            .map_err(|error| WorldInitError::Theory(error.to_string()))?;

        let owner_changed = [
            family_disposition,
            curation_disposition,
            condition_disposition,
            mapping_disposition,
            strategy_disposition,
        ]
        .contains(&TheoryInstallDisposition::Installed)
            || capability_changed
            || authority_changed
            || claim_changed;
        let mut record_ids = vec![
            format_revision(&family.revision_ref()),
            format_revision(&curation.revision_ref()),
            format_revision(&condition.revision_ref()),
            format_revision(&mapping.revision_ref()),
            format_revision(&strategy.revision_ref()),
        ];
        record_ids.extend(capabilities.iter().map(|revision| {
            let reference = revision.revision_ref();
            format!(
                "capability_contract::{}::{}::{}",
                reference.selector.capability_type_id,
                reference.selector.capability_version,
                reference.content_identity
            )
        }));
        record_ids.push(format!(
            "authority_policy::{}::{}",
            authority.policy.policy_id, authority.content_hash
        ));
        record_ids.push(format!(
            "claim_policy::{}::{}",
            claim.policy.policy_id, claim.content_identity
        ));
        record_ids.push(format!("theory_receipt::{}", receipt.receipt_id));
        Ok(WorldInitStageReport {
            stage: WorldInitStage::InstallTheory,
            disposition: if owner_changed || receipt_changed {
                StageDisposition::Applied
            } else {
                StageDisposition::Unchanged
            },
            record_ids,
        })
    }

    /// Stage 3: create the seed agent, its curation-rule binding, and its
    /// subscription to the configured belief key, all idempotent by
    /// registration identity.
    ///
    /// The belief key derives from the currently installed theory revision
    /// so the subscription binds exactly what stage 2 made resolvable.
    /// Registration identity idempotency means an existing agent record is
    /// returned unchanged even when the loaded rule content differs; rule
    /// revision migration is not part of this compatibility surface, so
    /// the report cites the rule hash actually installed on the record.
    fn genesis_identities(
        &self,
        content: &WorldInitContent,
    ) -> Result<WorldInitStageReport, WorldInitError> {
        let family_id = &content.family_config.family_id;
        let theory = self
            .registry
            .current(family_id)
            .map_err(|error| WorldInitError::Identity(error.to_string()))?
            .ok_or_else(|| WorldInitError::TheoryNotInstalled {
                family_id: family_id.clone(),
            })?;
        let belief_key = configured_belief_key(
            &theory,
            &content.subject,
            &content.perspective,
            &content.branch_scope,
        );
        let rule_revision = if let Some(install) = &self.complete_theory {
            let receipt = install
                .receipts
                .current(&install.selection)
                .map_err(|error| WorldInitError::Identity(error.to_string()))?;
            Some(receipt.curation_rule)
        } else {
            None
        };
        let maintained_condition_binding = if let Some(install) = &self.complete_theory {
            let receipt = install
                .receipts
                .current(&install.selection)
                .map_err(|error| WorldInitError::Identity(error.to_string()))?;
            let revision = install
                .maintained_conditions
                .resolve(
                    &receipt.maintained_condition.id,
                    &receipt.maintained_condition.content_hash,
                )
                .map_err(|error| WorldInitError::Identity(error.to_string()))?
                .ok_or_else(|| {
                    WorldInitError::Identity(
                        "installed maintained condition revision is missing".to_string(),
                    )
                })?;
            Some(
                revision
                    .binding()
                    .map_err(|error| WorldInitError::Identity(error.to_string()))?,
            )
        } else {
            None
        };
        let rule_binding = if rule_revision.is_none() {
            Some(
                AgentCurationRuleBinding::for_rule(content.curation_rule.clone())
                    .map_err(|error| WorldInitError::Identity(error.to_string()))?,
            )
        } else {
            None
        };

        // Existence is checked before each command because the idempotent
        // commands return the same record whether or not they created it;
        // the disposition must distinguish those outcomes truthfully.
        let agent_existed = self
            .agent_store
            .get_agent(&content.agent_id)
            .map_err(|error| WorldInitError::Identity(error.to_string()))?
            .is_some();
        let registration = AgentRegistration::new(self.agent_store);
        let record = registration
            .register_seed_agent(SeedAgentRegistration {
                agent_id: content.agent_id.clone(),
                perspective_key: content.perspective.clone(),
                subject: content.subject.clone(),
                branch_scope: content.branch_scope.clone(),
                observation_scope: content.observation_scope.clone(),
                directive: content.directive.clone(),
                seed_provenance: content.provenance.clone(),
                curation_rule: rule_binding,
                curation_rule_revision: rule_revision.clone(),
                maintained_condition: maintained_condition_binding.clone(),
                maintained_condition_revision: maintained_condition_binding
                    .as_ref()
                    .map(|binding| binding.revision.clone()),
                created_at_seq: content.observed_seq,
            })
            .map_err(|error| WorldInitError::Identity(error.to_string()))?;
        let needs_rule_migration = rule_revision.as_ref().is_some_and(|revision| {
            record.curation_rule_revision.as_ref() != Some(revision)
                || record.curation_rule.is_some()
        });
        let record = match rule_revision {
            Some(revision) if needs_rule_migration => registration
                .bind_curation_rule_revision(&content.agent_id, revision, content.observed_seq)
                .map_err(|error| WorldInitError::Identity(error.to_string()))?,
            _ => record,
        };
        let needs_condition_migration = maintained_condition_binding
            .as_ref()
            .is_some_and(|binding| record.maintained_condition.as_ref() != Some(binding));
        let record = match maintained_condition_binding {
            Some(binding) if needs_condition_migration => registration
                .bind_maintained_condition_revision(
                    &content.agent_id,
                    binding,
                    content.observed_seq,
                )
                .map_err(|error| WorldInitError::Identity(error.to_string()))?,
            _ => record,
        };

        let subscription_existed = self
            .agent_store
            .subscription_by_agent_and_key(&content.agent_id, &belief_key)
            .map_err(|error| WorldInitError::Identity(error.to_string()))?
            .is_some();
        let subscription = AgentSubscription::new(self.agent_store)
            .subscribe(SubscribeAgentCommand {
                agent_id: content.agent_id.clone(),
                belief_key,
                created_at_seq: content.observed_seq,
            })
            .map_err(|error| WorldInitError::Identity(error.to_string()))?;

        // The operational transition runs only when the record is not
        // already operational, so an unchanged re-run rewrites nothing.
        let needs_operational = record.status != AgentStatus::Operational;
        if needs_operational {
            registration
                .mark_operational(&content.agent_id, content.observed_seq)
                .map_err(|error| WorldInitError::Identity(error.to_string()))?;
        }

        let mut record_ids = vec![
            format!("agent::{}", record.agent_id),
            format!("agent_subscription::{}", subscription.subscription_id),
        ];
        if let Some(installed) = &record.curation_rule_revision {
            record_ids.push(format_revision(installed));
        } else if let Some(installed) = &record.curation_rule {
            record_ids.push(format!("curation_rule::{}", installed.content_hash));
        }
        if let Some(installed) = &record.maintained_condition_revision {
            record_ids.push(format_revision(installed));
        }
        let changed = !agent_existed
            || !subscription_existed
            || needs_operational
            || needs_rule_migration
            || needs_condition_migration;
        Ok(WorldInitStageReport {
            stage: WorldInitStage::GenesisIdentities,
            disposition: if changed {
                StageDisposition::Applied
            } else {
                StageDisposition::Unchanged
            },
            record_ids,
        })
    }

    /// Stage 4: append the unobserved-scope declaration for the selected
    /// subtree through the canonical append capability.
    ///
    /// The frozen record identity is per scope and carries no time, so the
    /// events constructor's wall-clock timestamp does not affect
    /// idempotency: a re-run dedupes on the record id and the original
    /// recorded time stands. An injected-time constructor variant for
    /// isolate boots is a recorded events-domain request.
    fn seed_epistemic_facts(
        &self,
        content: &WorldInitContent,
    ) -> Result<WorldInitStageReport, WorldInitError> {
        let declaration = UnobservedScopeDeclaration {
            subject: content.subject.clone(),
            declared_by: content.provenance.clone(),
        };
        let payload = serde_json::to_value(&declaration)
            .map_err(|error| WorldInitError::Genesis(error.to_string()))?;
        let envelope = EventEnvelope::epistemic_genesis(
            content.session_id.clone(),
            GENESIS_DOMAIN_ID,
            EPISTEMIC_GENESIS_STREAM_ID,
            &declaration.scope_key(),
            UNOBSERVED_SCOPE_EVENT_TYPE,
            payload,
        )
        .with_graph(vec![declaration.subject.clone()], Vec::new());
        // The constructor and the declaration derive the same frozen id;
        // the assertion keeps the joint events/world-model contract honest
        // if either side ever drifts.
        debug_assert_eq!(
            envelope.record_id.as_deref(),
            Some(declaration.record_id()).as_deref()
        );
        let receipt = self
            .append
            .append_durable(envelope, AppendMode::Idempotent)
            .map_err(|error| WorldInitError::Genesis(error.to_string()))?;
        Ok(WorldInitStageReport {
            stage: WorldInitStage::SeedEpistemicFacts,
            disposition: match receipt.disposition {
                AppendDisposition::Inserted => StageDisposition::Applied,
                AppendDisposition::Duplicate => StageDisposition::Unchanged,
            },
            record_ids: vec![declaration.record_id()],
        })
    }
}

fn validate_selected_bundle(
    selection: &SelectedStewardshipPackage,
    bundle: &WorldInitTheoryBundle,
    subject: &DomainObjectRef,
) -> Result<(), String> {
    if bundle.family.family_id != selection.belief_family_id
        || bundle.maintained_condition.condition_id != selection.maintained_condition_id
        || bundle.outcome_mapping.mapping_id != selection.evidence_mapping_id
        || bundle.strategy_theory.snapshot.theory_id != selection.strategy_theory_id
        || bundle.authority_policy.policy_id != selection.authority_policy_id
        || bundle.claim_policy.policy_id != selection.claim_policy_id
    {
        return Err("selected theory identities do not match the loaded owner bodies".to_string());
    }
    if bundle.authority_policy.principal_id != selection.principal_id
        || &bundle.authority_policy.subject != subject
    {
        return Err(
            "authority policy principal or subject does not match the stewardship binding"
                .to_string(),
        );
    }
    if bundle.curation_rule.dimension_id != bundle.family.dimension_id {
        return Err("curation rule dimension does not match the belief family".to_string());
    }
    if bundle.maintained_condition.dimension_id != bundle.family.dimension_id
        || bundle.curation_rule.maintained_condition_id.as_deref()
            != Some(bundle.maintained_condition.condition_id.as_str())
    {
        return Err(
            "curation rule, maintained condition, and belief family do not agree".to_string(),
        );
    }
    if bundle
        .strategy_theory
        .requested_dimensions
        .iter()
        .any(|dimension| dimension != &bundle.family.dimension_id)
    {
        return Err(
            "Strategy requested dimension is unavailable from the belief family".to_string(),
        );
    }
    if bundle.strategy_theory.requested_authority.is_empty() {
        return Err("Strategy package requests no authority".to_string());
    }
    if bundle
        .strategy_theory
        .requested_authority
        .iter()
        .any(|action| {
            !bundle
                .executable_contracts
                .iter()
                .any(|contract| &contract.capability_type_id == action)
        })
    {
        return Err(
            "Strategy requested authority cites an unavailable executable action".to_string(),
        );
    }
    for rule in &bundle.strategy_theory.snapshot.settlement_rules {
        if !bundle
            .family
            .evidence_schemas
            .iter()
            .any(|schema| schema.schema_id == rule.evidence_route.evidence_schema_id)
        {
            return Err(format!(
                "Strategy evidence route '{}' cites an unknown belief evidence schema",
                rule.evidence_route.route_id
            ));
        }
    }
    Ok(())
}

fn validate_strategy_contracts(
    strategy: &StrategyTheoryPackage,
    contracts: &[meld_execution::capability::CapabilityContractRevision],
) -> Result<(), String> {
    for capability in &strategy.capabilities {
        let selector = capability
            .operator
            .resolution
            .specific
            .as_ref()
            .ok_or_else(|| {
                format!(
                    "Strategy capability '{}' has no exact execution selector",
                    capability.operator.operator_id
                )
            })?;
        if !contracts.iter().any(|revision| {
            revision.contract.capability_type_id == selector.capability_type_id
                && revision.contract.capability_version == selector.capability_version
                && revision.content_identity == capability.contract_id
        }) {
            return Err(format!(
                "Strategy capability '{}' cites an unknown executable contract revision",
                capability.operator.operator_id
            ));
        }
    }
    Ok(())
}

fn format_revision(reference: &meld_world_model::belief::TheoryRevisionRef) -> String {
    format!(
        "{}::{}::{}",
        reference.registry, reference.id, reference.content_hash
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalization_orders_and_dedupes_stage_selection() {
        let request = WorldInitRequest {
            stages: vec![
                WorldInitStage::SeedEpistemicFacts,
                WorldInitStage::InstallTheory,
                WorldInitStage::SeedEpistemicFacts,
                WorldInitStage::GenesisIdentities,
                WorldInitStage::InstallTheory,
            ],
        };
        assert_eq!(
            normalized_stages(&request),
            vec![
                WorldInitStage::InstallTheory,
                WorldInitStage::GenesisIdentities,
                WorldInitStage::SeedEpistemicFacts,
            ]
        );
    }

    #[test]
    fn normalization_keeps_a_partial_selection_partial() {
        let request = WorldInitRequest {
            stages: vec![WorldInitStage::SeedEpistemicFacts],
        };
        assert_eq!(
            normalized_stages(&request),
            vec![WorldInitStage::SeedEpistemicFacts]
        );
    }

    #[test]
    fn normalization_of_an_empty_selection_runs_nothing() {
        let request = WorldInitRequest { stages: Vec::new() };
        assert!(normalized_stages(&request).is_empty());
    }
}
