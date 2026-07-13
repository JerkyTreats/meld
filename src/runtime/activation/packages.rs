use meld_execution::activation::{
    CapabilityContractRef, ExecutionActivationSelection, ExecutionTargetSelector,
    PublicationMapping, RequiredArtifactContract,
};
use meld_world_model::activation::{
    AgentCurationRuleRecord, DirectiveRecord, SeedAgentActivation, WorldModelActivationInput,
};
use meld_world_model::agent::AgentCurationRuleConfig;
use meld_world_model::belief::{BeliefKey, BranchScope};
use meld_world_model::events::DomainObjectRef;
use meld_world_model::PerspectiveKey;
use serde::{Deserialize, Serialize};

use super::contracts::DocsFreshnessActivationConfig;

/// Root runtime selection package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeActivationInput {
    /// Stable activation id.
    pub activation_id: String,
    /// Canonical content and deployment hash.
    pub activation_hash: String,
    /// One-shot bootstrap runtime id.
    pub bootstrap_runtime_id: String,
    /// Canonically sorted enabled runtime ids.
    pub enabled_runtime_ids: Vec<String>,
}

/// Independent owner packages derived from one validated activation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProductActivationRuntimeInputs {
    /// Root runtime selection package.
    pub runtime: RuntimeActivationInput,
    /// Canonical world-model owner package.
    pub world_model: WorldModelActivationInput,
    /// Source-neutral selection for the execution-owned asset binder.
    pub execution: ExecutionActivationSelection,
}

pub(crate) fn build_runtime_inputs(
    config: &DocsFreshnessActivationConfig,
    activation_hash: &str,
    canonical_target: String,
) -> ProductActivationRuntimeInputs {
    let activation_id = config.activation_id.clone();
    let hash = activation_hash.to_string();
    let subject = DomainObjectRef {
        domain_id: config.subject.domain_id.clone(),
        object_kind: config.subject.object_kind.clone(),
        object_id: config.subject.object_id.clone(),
    };
    let perspective = PerspectiveKey {
        perspective_kind: config.perspective.perspective_kind.clone(),
        perspective_id: config.perspective.perspective_id.clone(),
    };
    let branch_scope = BranchScope {
        branch_id: config.branch_scope.branch_id.clone(),
    };
    let family_config = config.belief_family.to_owner_config();
    let belief_key = BeliefKey {
        subject: subject.clone(),
        dimension_id: family_config.dimension_id.clone(),
        predicate_id: family_config.predicate_id.clone(),
        perspective: perspective.clone(),
        branch_scope: branch_scope.clone(),
        evidence_policy_id: family_config.evidence_policy_id.clone(),
    };
    let curation_rule = AgentCurationRuleConfig {
        dimension_id: config.curation_rule.dimension_id.clone(),
        threshold: config.curation_rule.threshold,
        priority_urgency: config.curation_rule.priority_urgency,
        desired_summary: config.curation_rule.desired_summary.clone(),
        source_kind: config.curation_rule.source_kind.clone(),
    };

    ProductActivationRuntimeInputs {
        runtime: RuntimeActivationInput {
            activation_id: activation_id.clone(),
            activation_hash: hash.clone(),
            bootstrap_runtime_id: config.runtime.bootstrap_runtime_id.clone(),
            enabled_runtime_ids: config.runtime.enabled_runtime_ids.clone(),
        },
        world_model: WorldModelActivationInput {
            activation_hash: hash.clone(),
            activation_id: activation_id.clone(),
            bootstrap_id: config.runtime.bootstrap_runtime_id.clone(),
            belief_family: family_config,
            directive: DirectiveRecord {
                directive_id: config.directive.directive_id.clone(),
                text: config.directive.text.clone(),
            },
            seed_agent: SeedAgentActivation {
                agent_id: config.seed_agent.agent_id.clone(),
                perspective_key: perspective,
                subject,
                branch_scope,
                observation_scope: config.seed_agent.observation_scope.clone(),
                directive_id: config.seed_agent.directive_id.clone(),
                seed_provenance: config.seed_agent.seed_provenance.clone(),
            },
            curation_rule: AgentCurationRuleRecord {
                rule_id: config.curation_rule.rule_id.clone(),
                agent_id: config.seed_agent.agent_id.clone(),
                config: curation_rule,
            },
            belief_key,
        },
        execution: ExecutionActivationSelection {
            activation_hash: hash.clone(),
            activation_id: activation_id.clone(),
            method_id: config.execution.method_id.clone(),
            workspace_scan_step_id: config.execution.workspace_scan_step_id.clone(),
            task_package_id: config.execution.task_package_id.clone(),
            workflow_id: config.execution.workflow_id.clone(),
            workspace_scan: CapabilityContractRef {
                capability_type_id: config.execution.workspace_scan_capability_type_id.clone(),
                capability_version: config.execution.workspace_scan_capability_version,
            },
            task_network_id: config.execution.task_network_id.clone(),
            required_artifact: RequiredArtifactContract {
                artifact_type_id: config.execution.required_artifact_type_id.clone(),
                schema_version: 1,
            },
            provider_binding_ref: config.execution.provider_binding_ref.clone(),
            frame_type: config.execution.frame_type.clone(),
            force_policy: config.execution.force_policy,
            target: ExecutionTargetSelector {
                kind: config.execution.target.kind,
                canonical_value: canonical_target,
            },
            publication: PublicationMapping {
                mapping_id: format!("publication.{}", config.activation_id),
                success_event_type: config.publication.success_event_type.clone(),
                failure_event_type: config.publication.failure_event_type.clone(),
                content_source_kind: config.publication.content_source_kind.clone(),
            },
        },
    }
}
