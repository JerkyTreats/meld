//! Task package top-level contracts.

use crate::task::{CompiledTaskRecord, TaskInitializationPayload};
use crate::types::NodeID;
use crate::workflow::profile::WorkflowGate;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::task::package::prerequisite::PrerequisiteTemplateSpec;
use crate::task::package::region::RepeatedRegionSpec;
use crate::task::package::seed::InitialSeedSpec;
use crate::task::package::trigger::TaskTriggerSpec;

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
    pub node_id: Option<NodeID>,
    pub path: Option<PathBuf>,
    pub agent_id: String,
    pub provider: crate::provider::ProviderExecutionBinding,
    pub frame_type: String,
    pub force: bool,
    pub session_id: Option<String>,
}

/// Prepared compiled task and run payload for one package trigger.
#[derive(Debug, Clone)]
pub struct PreparedTaskRun {
    pub compiled_task: CompiledTaskRecord,
    pub init_payload: TaskInitializationPayload,
    pub target_node_id: NodeID,
}

/// Resolved package context shared with package-specific expansion lowering.
#[derive(Debug, Clone)]
pub struct PreparedWorkflowPackageContext {
    pub target_node_id: NodeID,
    pub target_path: String,
    pub prompts_by_turn_id: HashMap<String, String>,
    pub gates_by_id: HashMap<String, WorkflowGate>,
    pub traversal_expansion: TraversalPrerequisitePackageExpansionSpec,
}
