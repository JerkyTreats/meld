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
    attempt::execute_turn_with_retries, errors::config_error, WorkflowExecutorContext,
    WorkflowExecutorRuntime,
};

pub(super) struct DirectExecutionResult {
    pub completed_turns: usize,
    pub final_frame_id: Option<NodeId>,
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn execute_direct_turns<A, E>(
    api: &A,
    registered_profile: &RegisteredWorkflowProfile,
    request: &WorkflowExecutionRequest,
    runtime: &WorkflowExecutorRuntime<'_, A, E>,
    event_context: Option<&ExecutionEventContext>,
    thread_id: &str,
    target_path: &str,
    start_seq: u32,
    mut turn_outputs: HashMap<String, String>,
    mut completed_turns: usize,
    mut final_frame_id: Option<NodeId>,
    system_prompt: String,
    final_turn_seq: u32,
) -> Result<DirectExecutionResult, E>
where
    A: WorkflowExecutorContext<E> + 'static,
    E: From<ExecutionInvariantError> + Display + Clone + Send + Sync + 'static,
    <A as ProviderValidationPort>::ProviderPreparation: ProviderPreparationView + Sync,
{
    let profile = &registered_profile.profile;

    for turn in profile.ordered_turns() {
        if turn.seq < start_seq {
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
            api,
            request.node_id,
            &request.frame_type,
            &turn,
            &turn_outputs,
        )?;
        let prompt_template = resolve_prompt_template(
            api,
            registered_profile.source_path.as_deref(),
            &turn.prompt_ref,
        )?;
        let rendered_prompt = render_turn_prompt(&prompt_template, &turn, &resolved_inputs);

        let completed_turn = execute_turn_with_retries(
            api,
            profile,
            &turn,
            gate,
            request,
            runtime,
            event_context,
            thread_id,
            target_path,
            &system_prompt,
            &prompt_template,
            &rendered_prompt,
            &resolved_inputs,
            final_turn_seq,
            final_frame_id,
        )
        .await?;

        turn_outputs.insert(turn.output_type.clone(), completed_turn.content.clone());
        turn_outputs.insert(turn.turn_id.clone(), completed_turn.content);
        completed_turns += 1;
        final_frame_id = Some(completed_turn.frame_id);
    }

    Ok(DirectExecutionResult {
        completed_turns,
        final_frame_id,
    })
}
