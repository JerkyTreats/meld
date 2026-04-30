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
    pub package_id: String,
    pub workflow_id: String,
    pub trigger: TaskTriggerSpec,
    pub seed: InitialSeedSpec,
    pub expansions: Vec<PackageExpansionSpec>,
}

/// Package-authored traversal prerequisite expansion entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraversalPrerequisitePackageExpansionSpec {
    pub expansion_kind: String,
    pub template_ref: String,
    pub traversal_strategy: String,
    pub repeated_region: RepeatedRegionSpec,
    pub prerequisite: PrerequisiteTemplateSpec,
    #[serde(default)]
    pub publish: Option<TraversalPublishSpec>,
}

/// Optional publish policy authored alongside a traversal expansion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraversalPublishSpec {
    pub file_name: String,
    pub strategy: String,
}

/// Package-authored expansion entries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PackageExpansionSpec {
    TraversalPrerequisite(TraversalPrerequisitePackageExpansionSpec),
}

/// Compatibility-facing trigger request for one workflow-backed task package.
#[derive(Debug, Clone)]
pub struct WorkflowPackageTriggerRequest {
    pub package_id: String,
    pub workflow_id: String,
    pub node_id: Option<NodeId>,
    pub path: Option<PathBuf>,
    pub agent_id: String,
    pub provider: crate::execution::ProviderExecutionBinding,
    pub frame_type: String,
    pub force: bool,
    pub session_id: Option<String>,
}

/// Prepared compiled task and run payload for one package trigger.
#[derive(Debug, Clone)]
pub struct PreparedTaskRun {
    pub compiled_task: CompiledTaskRecord,
    pub init_payload: TaskInitializationPayload,
    pub target_node_id: NodeId,
}

/// Resolved package context shared with package-specific expansion lowering.
#[derive(Debug, Clone)]
pub struct PreparedWorkflowPackageContext {
    pub target_node_id: NodeId,
    pub target_path: String,
    pub prompts_by_turn_id: HashMap<String, String>,
    pub gates_by_id: HashMap<String, WorkflowGate>,
    pub traversal_expansion: TraversalPrerequisitePackageExpansionSpec,
}
