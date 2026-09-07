//! Task template materialization entrypoints.

use crate::capability::CapabilityCatalog;
use crate::error::ApiError;
use crate::execution::{ContextReadPort, NodeResolutionPort};
use crate::generation::NodeId;
use crate::task::expansion::TaskExpansionTemplate;
use crate::task::package::{
    load_task_package_spec_for_workflow, lower_traversal_prerequisite_expansion_template,
    prepare_workflow_task_run, workflow_task_run_id, PackageRunResolvers, PreparedTaskRun,
    PreparedWorkflowPackageContext, WorkflowPackageTriggerRequest,
};
use crate::workflow::registry::RegisteredWorkflowProfile;
use std::path::Path;

/// Returns true when a registered workflow has a task package route.
pub fn workflow_uses_task_package_path<E>(
    registered_profile: &RegisteredWorkflowProfile,
    default_package_dir: Option<&Path>,
) -> Result<bool, E>
where
    E: From<ApiError>,
{
    Ok(load_task_package_spec_for_workflow(registered_profile, default_package_dir)?.is_some())
}

/// Returns the deterministic task run id for one workflow target.
pub fn workflow_task_run_id_for_target(
    registered_profile: &RegisteredWorkflowProfile,
    node_id: NodeId,
) -> String {
    workflow_task_run_id(&registered_profile.profile.workflow_id, node_id)
}

/// Builds the standard resolver set for one registered workflow trigger,
/// pairing caller-supplied prompt and belief resolvers with the built-in
/// traversal prerequisite expansion lowering.
pub fn traversal_package_run_resolvers<'a, E, PromptResolver, BundleResolver>(
    registered_profile: &'a RegisteredWorkflowProfile,
    request: &'a WorkflowPackageTriggerRequest,
    resolve_prompt: PromptResolver,
    resolve_belief_bundle: BundleResolver,
) -> PackageRunResolvers<
    PromptResolver,
    impl FnOnce(&PreparedWorkflowPackageContext) -> Result<TaskExpansionTemplate, E> + 'a,
    BundleResolver,
>
where
    E: From<ApiError>,
    PromptResolver: FnMut(&str) -> Result<String, E>,
    BundleResolver: FnOnce(NodeId) -> Result<serde_json::Value, E>,
{
    PackageRunResolvers {
        resolve_prompt,
        build_expansion_template: |context: &PreparedWorkflowPackageContext| {
            Ok(lower_traversal_prerequisite_expansion_template(
                &registered_profile.profile,
                request,
                &context.traversal_expansion,
                context,
            )?)
        },
        resolve_belief_bundle,
    }
}

/// Prepares one registered workflow through the generic task package path.
///
/// `resolvers` carries the trigger-time closures; the standard traversal
/// set comes from [`traversal_package_run_resolvers`]. The belief bundle
/// resolver is invoked only when the workflow's `belief_context` flag is on
/// and the package authors a `goal_belief_hydration` seed.
pub fn prepare_registered_workflow_task_run<E, A, PromptResolver, TemplateBuilder, BundleResolver>(
    api: &A,
    workspace_root: &Path,
    registered_profile: &RegisteredWorkflowProfile,
    request: &WorkflowPackageTriggerRequest,
    catalog: &CapabilityCatalog,
    default_package_dir: Option<&Path>,
    resolvers: PackageRunResolvers<PromptResolver, TemplateBuilder, BundleResolver>,
) -> Result<PreparedTaskRun, E>
where
    E: From<ApiError>,
    A: ContextReadPort<Error = E, NodeId = NodeId>
        + NodeResolutionPort<Error = E, NodeId = NodeId>
        + ?Sized,
    PromptResolver: FnMut(&str) -> Result<String, E>,
    TemplateBuilder: FnOnce(&PreparedWorkflowPackageContext) -> Result<TaskExpansionTemplate, E>,
    BundleResolver: FnOnce(NodeId) -> Result<serde_json::Value, E>,
{
    let package_spec =
        load_task_package_spec_for_workflow(registered_profile, default_package_dir)?.ok_or_else(
            || {
                ApiError::ConfigError(format!(
                    "Workflow '{}' does not have a task package route",
                    registered_profile.profile.workflow_id
                ))
            },
        )?;

    prepare_workflow_task_run(
        api,
        workspace_root,
        registered_profile,
        request,
        catalog,
        &package_spec,
        resolvers,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::profile::{
        WorkflowArtifactPolicy, WorkflowFailurePolicy, WorkflowProfile, WorkflowThreadPolicy,
    };
    use serde_json::json;

    fn registered_profile(workflow_id: &str) -> RegisteredWorkflowProfile {
        RegisteredWorkflowProfile {
            profile: WorkflowProfile {
                workflow_id: workflow_id.to_string(),
                version: 1,
                title: "Docs Writer".to_string(),
                description: "Writes docs".to_string(),
                thread_policy: WorkflowThreadPolicy {
                    start_conditions: json!({}),
                    dedupe_key_fields: Vec::new(),
                    max_turn_retries: 1,
                },
                turns: Vec::new(),
                gates: Vec::new(),
                artifact_policy: WorkflowArtifactPolicy {
                    store_output: true,
                    store_prompt_render: true,
                    store_context_payload: true,
                    max_output_bytes: 1024,
                },
                failure_policy: WorkflowFailurePolicy {
                    mode: "fail_fast".to_string(),
                    resume_from_failed_turn: false,
                    stop_on_gate_fail: true,
                },
                thread_profile: None,
                target_agent_id: None,
                target_frame_type: None,
                final_artifact_type: None,
                belief_context: None,
            },
            source_path: None,
        }
    }

    #[test]
    fn workflow_name_cannot_select_an_implicit_product_package() {
        let profile = registered_profile("docs_writer_thread_v1");

        assert!(!workflow_uses_task_package_path::<ApiError>(&profile, None).unwrap());
    }

    #[test]
    fn workflow_task_run_id_for_target_includes_workflow_and_node_prefix() {
        let profile = registered_profile("docs_writer_thread_v1");

        let task_run_id = workflow_task_run_id_for_target(&profile, [1u8; 32]);

        assert_eq!(
            task_run_id,
            "taskrun::docs_writer_thread_v1::0101010101010101"
        );
    }
}
