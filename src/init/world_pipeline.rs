//! World-initialization pipeline over stages 2 through 4.
//!
//! Owner: root init. This is the command path behind the frozen shapes in
//! [`crate::init::world`]: stage 2 installs theory through the world-model
//! registry command, stage 3 creates genesis identities through the agent
//! registration commands, and stage 4 seeds the epistemic genesis fact
//! through the canonical event append. The pipeline never writes a domain
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
use meld_world_model::agent::{
    AgentCurationRuleBinding, AgentCurationRuleConfig, AgentRegistration, AgentStatus, AgentStore,
    AgentSubscription, SeedAgentRegistration, SubscribeAgentCommand,
};
use meld_world_model::belief::genesis::{
    UnobservedScopeDeclaration, EPISTEMIC_GENESIS_STREAM_ID, UNOBSERVED_SCOPE_EVENT_TYPE,
};
use meld_world_model::belief::{
    configured_belief_key, BeliefFamilyConfig, BeliefFamilyRegistry, BranchScope,
    TheoryInstallDisposition,
};
use meld_world_model::PerspectiveKey;
use thiserror::Error;

use crate::init::world::{
    StageDisposition, WorldInitReport, WorldInitRequest, WorldInitStage, WorldInitStageReport,
};

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
        }
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
        let rule_binding = AgentCurationRuleBinding::for_rule(content.curation_rule.clone())
            .map_err(|error| WorldInitError::Identity(error.to_string()))?;

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
                curation_rule: Some(rule_binding),
                created_at_seq: content.observed_seq,
            })
            .map_err(|error| WorldInitError::Identity(error.to_string()))?;

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
        if let Some(installed) = &record.curation_rule {
            record_ids.push(format!("curation_rule::{}", installed.content_hash));
        }
        let changed = !agent_existed || !subscription_existed || needs_operational;
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
