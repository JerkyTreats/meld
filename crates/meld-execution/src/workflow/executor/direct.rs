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
    /// Execution API used by direct workflow turns.
    pub api: &'a A,
    /// Registered profile being executed.
    pub registered_profile: &'a RegisteredWorkflowProfile,
    /// Caller request for the workflow target.
    pub request: &'a WorkflowExecutionRequest,
    /// Runtime services shared across workflow execution.
    pub runtime: &'a WorkflowExecutorRuntime<'a, A, E>,
    /// Optional event context used for publishing progress.
    pub event_context: Option<&'a ExecutionEventContext>,
    /// Workflow thread identifier within workflow runtime state.
    pub thread_id: &'a str,
    /// Workspace target path used in progress events.
    pub target_path: &'a str,
    /// System prompt text supplied to provider execution.
    pub system_prompt: String,
    /// Sequence number of the final workflow turn.
    pub final_turn_seq: u32,
}

pub(super) struct DirectExecutionState {
    /// First turn sequence to execute.
    pub start_seq: u32,
    /// Completed turn outputs keyed by output type and turn id.
    pub turn_outputs: HashMap<String, String>,
    /// Number of turns completed before this direct execution pass.
    pub completed_turns: usize,
    /// Final frame identifier produced by the workflow thread when available.
    pub final_frame_id: Option<NodeId>,
}

pub(super) struct DirectExecutionResult {
    /// Number of turns completed after direct execution.
    pub completed_turns: usize,
    /// Final frame identifier produced by the workflow thread when available.
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
