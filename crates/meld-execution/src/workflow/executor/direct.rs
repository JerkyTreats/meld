use std::collections::HashMap;
use std::fmt::Display;

use crate::error::ExecutionInvariantError;
use crate::execution::{ExecutionEventContext, ProviderPreparationView, ProviderValidationPort};
use crate::generation::NodeId;
use crate::workflow::{
    render_turn_prompt, resolve_prompt_template, resolve_turn_inputs, RegisteredWorkflowProfile,
    WorkflowExecutionRequest,
};

use super::{
    attempt::{execute_turn_with_retries, TurnAttemptContext, TurnExecutionInput},
    errors::config_error,
    WorkflowExecutorContext, WorkflowExecutorRuntime,
};

pub(super) struct DirectExecutionContext<'a, A, E>
where
    A: WorkflowExecutorContext<E>,
{
    pub api: &'a A,
    pub registered_profile: &'a RegisteredWorkflowProfile,
    pub request: &'a WorkflowExecutionRequest,
    pub runtime: &'a WorkflowExecutorRuntime<'a, A, E>,
    pub event_context: Option<&'a ExecutionEventContext>,
    pub thread_id: &'a str,
    pub target_path: &'a str,
    pub system_prompt: String,
    pub final_turn_seq: u32,
}

pub(super) struct DirectExecutionState {
    pub start_seq: u32,
    pub turn_outputs: HashMap<String, String>,
    pub completed_turns: usize,
    pub final_frame_id: Option<NodeId>,
}

pub(super) struct DirectExecutionResult {
    pub completed_turns: usize,
    pub final_frame_id: Option<NodeId>,
}

pub(super) async fn execute_direct_turns<A, E>(
    context: DirectExecutionContext<'_, A, E>,
    mut state: DirectExecutionState,
) -> Result<DirectExecutionResult, E>
where
    A: WorkflowExecutorContext<E> + 'static,
    E: From<ExecutionInvariantError> + Display + Clone + Send + Sync + 'static,
    <A as ProviderValidationPort>::ProviderPreparation: ProviderPreparationView + Sync,
{
    let profile = &context.registered_profile.profile;

    for turn in profile.ordered_turns() {
        if turn.seq < state.start_seq {
            continue;
        }

        let gate = profile
            .gates
            .iter()
            .find(|gate| gate.gate_id == turn.gate_id)
            .ok_or_else(|| {
                config_error(format!(
                    "Workflow '{}' missing gate '{}' for turn '{}'",
                    profile.workflow_id, turn.gate_id, turn.turn_id
                ))
            })?;

        let resolved_inputs = resolve_turn_inputs(
            context.api,
            context.request.node_id,
            &context.request.frame_type,
            &turn,
            &state.turn_outputs,
        )?;
        let prompt_template = resolve_prompt_template(
            context.api,
            context.registered_profile.source_path.as_deref(),
            &turn.prompt_ref,
        )?;
        let rendered_prompt = render_turn_prompt(&prompt_template, &turn, &resolved_inputs);

        let completed_turn = execute_turn_with_retries(
            TurnAttemptContext {
                api: context.api,
                profile,
                request: context.request,
                runtime: context.runtime,
                event_context: context.event_context,
                thread_id: context.thread_id,
                target_path: context.target_path,
            },
            TurnExecutionInput {
                turn: &turn,
                gate,
                system_prompt: &context.system_prompt,
                prompt_template: &prompt_template,
                rendered_prompt: &rendered_prompt,
                resolved_inputs: &resolved_inputs,
                final_turn_seq: context.final_turn_seq,
            },
            state.final_frame_id,
        )
        .await?;

        state
            .turn_outputs
            .insert(turn.output_type.clone(), completed_turn.content.clone());
        state
            .turn_outputs
            .insert(turn.turn_id.clone(), completed_turn.content);
        state.completed_turns += 1;
        state.final_frame_id = Some(completed_turn.frame_id);
    }

    Ok(DirectExecutionResult {
        completed_turns: state.completed_turns,
        final_frame_id: state.final_frame_id,
    })
}
