//! Task package top-level contracts.

use crate::generation::NodeId;
use crate::task::{CompiledTaskRecord, TaskInitializationPayload};
use crate::workflow::profile::WorkflowGate;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use super::prerequisite::PrerequisiteTemplateSpec;
use super::region::RepeatedRegionSpec;
use super::seed::InitialSeedSpec;
use super::trigger::TaskTriggerSpec;

/// Top-level authored task package document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskPackageSpec {
    /// Package identifier carried across the execution boundary.
    pub package_id: String,
    /// Workflow profile identifier that owns this execution record.
    pub workflow_id: String,
    /// Trigger contract accepted by this package.
    pub trigger: TaskTriggerSpec,
    /// Initial seed artifacts declared by this package.
    pub seed: InitialSeedSpec,
    /// Expansion entries authored by this package.
    pub expansions: Vec<PackageExpansionSpec>,
}

impl TaskPackageSpec {
    /// Derive a source-format-neutral digest from the typed package semantics.
    pub fn semantic_digest(&self) -> String {
        let encoded = serde_json::to_vec(self)
            .expect("task package must remain serializable for activation identity");
        blake3::hash(&encoded).to_hex().to_string()
    }
}

/// Package-authored traversal prerequisite expansion entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraversalPrerequisitePackageExpansionSpec {
    /// Expansion kind handled by package lowering.
    pub expansion_kind: String,
    /// Template reference resolved by the package registry.
    pub template_ref: String,
    /// Traversal strategy name used to produce node batches.
    pub traversal_strategy: String,
    /// Workflow region template expanded for each traversal batch.
    pub repeated_region: RepeatedRegionSpec,
    /// Prerequisite template that wires traversal dependencies.
    pub prerequisite: PrerequisiteTemplateSpec,
    /// Optional frame head publish policy carried with the expansion.
    #[serde(default)]
    pub publish: Option<TraversalPublishSpec>,
}

/// Optional publish policy authored alongside a traversal expansion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraversalPublishSpec {
    /// Publish file name selected for frame head output.
    pub file_name: String,
    /// Publish strategy selected for frame head output.
    pub strategy: String,
}

/// Package-authored expansion entries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PackageExpansionSpec {
    /// Traversal prerequisite expansion authored by the package.
    TraversalPrerequisite(TraversalPrerequisitePackageExpansionSpec),
}

/// Compatibility-facing trigger request for one workflow-backed task package.
#[derive(Debug, Clone)]
pub struct WorkflowPackageTriggerRequest {
    /// Package identifier carried across the execution boundary.
    pub package_id: String,
    /// Workflow profile identifier that owns this execution record.
    pub workflow_id: String,
    /// Workspace node identifier carried across execution boundaries.
    pub node_id: Option<NodeId>,
    /// Workspace path associated with this execution record.
    pub path: Option<PathBuf>,
    /// Agent identifier responsible for this execution request.
    pub agent_id: String,
    /// Provider binding selected for execution.
    pub provider: crate::execution::ProviderExecutionBinding,
    /// Context frame type produced or consumed by this execution path.
    pub frame_type: String,
    /// True when execution should bypass cached or existing output.
    pub force: bool,
    /// Session identifier carried across the execution boundary.
    pub session_id: Option<String>,
}

/// Prepared compiled task and run payload for one package trigger.
#[derive(Debug, Clone)]
pub struct PreparedTaskRun {
    /// Compiled task produced by package lowering.
    pub compiled_task: CompiledTaskRecord,
    /// Initialization payload paired with the compiled task.
    pub init_payload: TaskInitializationPayload,
    /// Target node identifier carried across the execution boundary.
    pub target_node_id: NodeId,
}

/// Resolved package context shared with package-specific expansion lowering.
#[derive(Debug, Clone)]
pub struct PreparedWorkflowPackageContext {
    /// Target node identifier carried across the execution boundary.
    pub target_node_id: NodeId,
    /// Target workspace path used while lowering the package.
    pub target_path: String,
    /// Prompts by turn identifier carried across the execution boundary.
    pub prompts_by_turn_id: HashMap<String, String>,
    /// Gates by identifier carried across the execution boundary.
    pub gates_by_id: HashMap<String, WorkflowGate>,
    /// Traversal expansion being lowered for this target.
    pub traversal_expansion: TraversalPrerequisitePackageExpansionSpec,
    /// Init slot that carries the belief context bundle, resolved only when
    /// the workflow's `belief_context` flag is on and the package authors a
    /// `goal_belief_hydration` seed. `None` keeps lowering byte-identical.
    pub belief_init_slot_id: Option<String>,
}
