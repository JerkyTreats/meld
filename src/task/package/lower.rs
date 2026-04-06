//! Generic lowering from task package authroing specs into domain expansion templates.

use crate::error::ApiError;
use crate::merkle_traversal::expansion::{
    TraversalPrerequisiteExpansionTemplate, TraversalPrerequisiteTemplate, WorkflowRegionTemplate,
    WorkflowTurnTemplate,
};
use crate::task::expansion::TaskExpansionTemplate;
use crate::task::package::{
    PreparedWorkflowPackageContext, TraversalPrerequisitePackageExpansionSpec,
    WorkflowPackageTriggerRequest,
};
use crate::workflow::profile::{WorkflowGate, WorkflowProfile};
use std::collections::HashMap;

/// Lowers one traversal prerequisite package expansion into a task expansion template.
pub fn lower_traversal_prerequisite_expansion_template(
    profile: &WorkflowProfile,
    request: &WorkflowPackageTriggerRequest,
    expansion: &TraversalPrerequisitePackageExpansionSpec,
    context: &PreparedWorkflowPackageContext,
) -> Result<TaskExpansionTemplate, ApiError> {
    Ok(TaskExpansionTemplate {
        expansion_kind: expansion.expansion_kind.clone(),
        content: serde_json::to_value(TraversalPrerequisiteExpansionTemplate {
            repeated_region: lower_workflow_region_template(
                profile,
                request,
                expansion,
                &context.prompts_by_turn_id,
                &context.gates_by_id,
            )?,
            prerequisite_template: TraversalPrerequisiteTemplate {
                producer_turn_id: expansion.prerequisite.producer_turn_id.clone(),
                producer_stage_id: expansion.prerequisite.producer_stage_id.clone(),
                producer_output_slot_id: expansion.prerequisite.producer_output_slot_id.clone(),
                producer_artifact_type_id: expansion.prerequisite.producer_artifact_type_id.clone(),
                consumer_turn_id: expansion.prerequisite.consumer_turn_id.clone(),
                consumer_stage_id: expansion.prerequisite.consumer_stage_id.clone(),
                consumer_input_slot_id: expansion.prerequisite.consumer_input_slot_id.clone(),
            },
        })
        .map_err(|err| {
            ApiError::ConfigError(format!(
                "Failed to encode traversal expansion template '{}': {}",
                expansion.template_ref, err
            ))
        })?,
    })
}

/// Lowers one authored repeated region into a traversal-owned workflow region template.
pub fn lower_workflow_region_template(
    profile: &WorkflowProfile,
    request: &WorkflowPackageTriggerRequest,
    expansion: &TraversalPrerequisitePackageExpansionSpec,
    prompts: &HashMap<String, String>,
    gates: &HashMap<String, WorkflowGate>,
) -> Result<WorkflowRegionTemplate, ApiError> {
    validate_supported_stage_chain(profile, expansion)?;

    Ok(WorkflowRegionTemplate {
        workflow_id: profile.workflow_id.clone(),
        agent_id: request.agent_id.clone(),
        provider: request.provider.clone(),
        frame_type: request.frame_type.clone(),
        force: request.force,
        force_init_slot_id: expansion.repeated_region.force_init_slot_id.clone(),
        node_ref_slot_template: expansion.repeated_region.node_ref_slot_template.clone(),
        existing_output_slot_template: expansion
            .repeated_region
            .existing_output_slot_template
            .clone(),
        existing_output_artifact_type_id: expansion
            .repeated_region
            .existing_output_artifact_type_id
            .clone(),
        turns: expansion
            .repeated_region
            .turns
            .iter()
            .map(|turn| {
                let prompt_text = prompts.get(&turn.turn_id).cloned().ok_or_else(|| {
                    ApiError::ConfigError(format!(
                        "Workflow '{}' missing prompt text for turn '{}'",
                        profile.workflow_id, turn.turn_id
                    ))
                })?;
                let gate = gates.get(&turn.gate_id).cloned().ok_or_else(|| {
                    ApiError::ConfigError(format!(
                        "Workflow '{}' missing gate '{}'",
                        profile.workflow_id, turn.gate_id
                    ))
                })?;
                Ok(WorkflowTurnTemplate {
                    turn_id: turn.turn_id.clone(),
                    prompt_text,
                    output_type: turn.output_type.clone(),
                    gate,
                    persist_frame: turn.output_policy.persist_frame,
                    retry_limit: turn.retry_limit,
                    validate_json: turn.validate_json,
                })
            })
            .collect::<Result<Vec<_>, ApiError>>()?,
    })
}

fn validate_supported_stage_chain(
    profile: &WorkflowProfile,
    expansion: &TraversalPrerequisitePackageExpansionSpec,
) -> Result<(), ApiError> {
    let expected_stage_chain = ["prepare", "execute", "finalize"];
    let actual_stage_chain = expansion
        .repeated_region
        .stage_chain
        .stages
        .iter()
        .map(|stage| stage.stage_id.as_str())
        .collect::<Vec<_>>();
    if actual_stage_chain != expected_stage_chain {
        return Err(ApiError::ConfigError(format!(
            "Package '{}' region '{}' has unsupported stage chain '{}'",
            profile.workflow_id,
            expansion.repeated_region.region_id,
            actual_stage_chain.join(" -> ")
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::{ProviderExecutionBinding, ProviderRuntimeOverrides};
    use crate::task::package::{
        PrerequisiteTemplateSpec, RepeatedRegionSpec, StageChainSpec, StageSpec,
        TraversalPrerequisitePackageExpansionSpec, TurnOutputPolicySpec, TurnSpec,
    };
    use crate::workflow::profile::{
        WorkflowArtifactPolicy, WorkflowFailurePolicy, WorkflowProfile, WorkflowThreadPolicy,
    };
    use serde_json::json;

    fn profile() -> WorkflowProfile {
        WorkflowProfile {
            workflow_id: "docs_writer_thread_v1".to_string(),
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
        }
    }

    fn request() -> WorkflowPackageTriggerRequest {
        WorkflowPackageTriggerRequest {
            package_id: "docs_writer".to_string(),
            workflow_id: "docs_writer_thread_v1".to_string(),
            node_id: None,
            path: Some("src".into()),
            agent_id: "docs-writer".to_string(),
            provider: ProviderExecutionBinding {
                provider_name: "test-provider".to_string(),
                runtime_overrides: ProviderRuntimeOverrides::default(),
            },
            frame_type: "context-docs-writer".to_string(),
            force: true,
            session_id: None,
        }
    }

    fn expansion() -> TraversalPrerequisitePackageExpansionSpec {
        TraversalPrerequisitePackageExpansionSpec {
            expansion_kind: "traversal_prerequisite_expansion".to_string(),
            template_ref: "docs_writer_bottom_up".to_string(),
            traversal_strategy: "bottom_up".to_string(),
            repeated_region: RepeatedRegionSpec {
                region_id: "docs_writer_node".to_string(),
                force_init_slot_id: "force_posture".to_string(),
                node_ref_slot_template: "node_ref::{node_id_prefix}".to_string(),
                existing_output_slot_template: "existing_readme::{node_id_prefix}".to_string(),
                existing_output_artifact_type_id: "readme_final".to_string(),
                stage_chain: StageChainSpec {
                    stages: vec![
                        StageSpec {
                            stage_id: "prepare".to_string(),
                            capability_type_id: "context_generate_prepare".to_string(),
                            capability_version: 1,
                        },
                        StageSpec {
                            stage_id: "execute".to_string(),
                            capability_type_id: "provider_execute_chat".to_string(),
                            capability_version: 1,
                        },
                        StageSpec {
                            stage_id: "finalize".to_string(),
                            capability_type_id: "context_generate_finalize".to_string(),
                            capability_version: 1,
                        },
                    ],
                },
                turns: vec![TurnSpec {
                    turn_id: "style_refine".to_string(),
                    prompt_ref: "prompts/docs_writer/style_refine.md".to_string(),
                    output_type: "readme_final".to_string(),
                    gate_id: "style_gate".to_string(),
                    output_policy: TurnOutputPolicySpec {
                        persist_frame: true,
                    },
                    retry_limit: 1,
                    validate_json: false,
                }],
            },
            prerequisite: PrerequisiteTemplateSpec {
                producer_turn_id: "style_refine".to_string(),
                producer_stage_id: "finalize".to_string(),
                producer_output_slot_id: "generation_output".to_string(),
                producer_artifact_type_id: "readme_final".to_string(),
                consumer_turn_id: "style_refine".to_string(),
                consumer_stage_id: "prepare".to_string(),
                consumer_input_slot_id: "upstream_artifact".to_string(),
            },
        }
    }

    fn context() -> PreparedWorkflowPackageContext {
        PreparedWorkflowPackageContext {
            target_node_id: [7u8; 32],
            target_path: "src".to_string(),
            prompts_by_turn_id: HashMap::from([(
                "style_refine".to_string(),
                "Refine the README".to_string(),
            )]),
            gates_by_id: HashMap::from([(
                "style_gate".to_string(),
                WorkflowGate {
                    gate_id: "style_gate".to_string(),
                    gate_type: "schema_required_fields".to_string(),
                    required_fields: vec!["title".to_string()],
                    rules: json!({}),
                    fail_on_violation: true,
                },
            )]),
            traversal_expansion: expansion(),
        }
    }

    #[test]
    fn lower_region_maps_resolved_prompt_and_gate_data() {
        let region = lower_workflow_region_template(
            &profile(),
            &request(),
            &expansion(),
            &context().prompts_by_turn_id,
            &context().gates_by_id,
        )
        .unwrap();

        assert_eq!(region.turns.len(), 1);
        assert_eq!(region.turns[0].prompt_text, "Refine the README");
        assert_eq!(region.turns[0].gate.gate_id, "style_gate");
        assert!(region.turns[0].persist_frame);
    }

    #[test]
    fn lower_region_rejects_unsupported_stage_chain() {
        let mut expansion = expansion();
        expansion.repeated_region.stage_chain.stages = vec![StageSpec {
            stage_id: "execute".to_string(),
            capability_type_id: "provider_execute_chat".to_string(),
            capability_version: 1,
        }];

        let error = lower_workflow_region_template(
            &profile(),
            &request(),
            &expansion,
            &context().prompts_by_turn_id,
            &context().gates_by_id,
        )
        .unwrap_err();

        assert!(matches!(error, ApiError::ConfigError(_)));
        assert!(error.to_string().contains("unsupported stage chain"));
    }
}
