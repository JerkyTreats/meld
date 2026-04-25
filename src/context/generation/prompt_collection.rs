use crate::agent::profile::prompt_contract::PromptContract;
use crate::context::generation::contracts::{GenerationOrchestrationRequest, PromptAssemblyOutput};
use crate::error::ApiError;
use crate::execution::ContextReadPort;
use crate::provider::{ChatMessage, MessageRole};
use crate::store::{NodeRecord, NodeType};
use crate::views::{FrameFilter, OrderingPolicy};

const FILE_CONTEXT_MAX_BYTES: usize = 128 * 1024;

pub fn build_prompt_messages(
    api: &(impl ContextReadPort + ?Sized),
    request: &GenerationOrchestrationRequest,
    node_record: &NodeRecord,
    prompt_contract: &PromptContract,
) -> Result<PromptAssemblyOutput, ApiError> {
    let user_prompt_template = match node_record.node_type {
        NodeType::File { .. } => prompt_contract.user_prompt_file.clone(),
        NodeType::Directory => prompt_contract.user_prompt_directory.clone(),
    };

    let rendered_prompt = prompt_contract.render_user_prompt(
        node_record.node_type.clone(),
        &node_record.path.display().to_string(),
        match node_record.node_type {
            NodeType::File { size, .. } => Some(size),
            NodeType::Directory => None,
        },
    );

    let prompt_context = match node_record.node_type {
        NodeType::File { .. } => Some(collect_file_source_context(node_record)?),
        NodeType::Directory => {
            let child_context_text =
                collect_directory_child_context_text(api, node_record, request)?;
            if child_context_text.is_empty() {
                let node_context_text = collect_scoped_node_frame_context(api, request)?;
                if node_context_text.is_empty() {
                    None
                } else {
                    Some(node_context_text)
                }
            } else {
                Some(child_context_text)
            }
        }
    };

    let mut messages = vec![ChatMessage {
        role: MessageRole::System,
        content: prompt_contract.system_prompt.clone(),
    }];

    let context_payload = prompt_context.unwrap_or_default();
    let rendered_user_message = if context_payload.is_empty() {
        rendered_prompt.clone()
    } else {
        format!("Context:\n{}\n\nTask: {}", context_payload, rendered_prompt)
    };

    if context_payload.is_empty() {
        messages.push(ChatMessage {
            role: MessageRole::User,
            content: rendered_prompt.clone(),
        });
    } else {
        messages.push(ChatMessage {
            role: MessageRole::User,
            content: rendered_user_message,
        });
    }

    Ok(PromptAssemblyOutput {
        system_prompt: prompt_contract.system_prompt.clone(),
        user_prompt_template,
        rendered_prompt,
        context_payload,
        messages,
    })
}

fn collect_directory_child_context_text(
    api: &(impl ContextReadPort + ?Sized),
    node_record: &NodeRecord,
    request: &GenerationOrchestrationRequest,
) -> Result<String, ApiError> {
    if !matches!(node_record.node_type, NodeType::Directory) {
        return Ok(String::new());
    }

    let child_view = crate::context::query::view::ContextView {
        max_frames: 1,
        ordering: OrderingPolicy::Recency,
        filters: vec![
            FrameFilter::ByType(request.frame_type.clone()),
            FrameFilter::ByAgent(request.agent_id.clone()),
        ],
    };

    let mut child_sections = Vec::new();
    for child_id in &node_record.children {
        let child_context = api.get_node(*child_id, child_view.clone())?;
        if child_context.frames.is_empty() {
            continue;
        }

        let child_kind = match child_context.node_record.node_type {
            NodeType::File { .. } => "File",
            NodeType::Directory => "Directory",
        };
        let child_text = child_context
            .frames
            .iter()
            .map(|f| String::from_utf8_lossy(&f.content))
            .collect::<Vec<_>>()
            .join("\n\n");

        child_sections.push(format!(
            "Path: {}\nType: {}\nContent:\n{}",
            child_context.node_record.path.display(),
            child_kind,
            child_text
        ));
    }

    Ok(child_sections.join("\n\n---\n\n"))
}

fn collect_scoped_node_frame_context(
    api: &(impl ContextReadPort + ?Sized),
    request: &GenerationOrchestrationRequest,
) -> Result<String, ApiError> {
    let view = crate::context::query::view::ContextView {
        max_frames: 10,
        ordering: OrderingPolicy::Recency,
        filters: vec![
            FrameFilter::ByType(request.frame_type.clone()),
            FrameFilter::ByAgent(request.agent_id.clone()),
        ],
    };
    let context = api.get_node(request.node_id, view)?;
    Ok(context
        .frames
        .iter()
        .map(|f| String::from_utf8_lossy(&f.content))
        .collect::<Vec<_>>()
        .join("\n\n"))
}

fn collect_file_source_context(node_record: &NodeRecord) -> Result<String, ApiError> {
    if !matches!(node_record.node_type, NodeType::File { .. }) {
        return Ok(String::new());
    }

    let bytes = std::fs::read(&node_record.path).map_err(|e| {
        ApiError::StorageError(crate::error::StorageError::IoError(std::io::Error::new(
            e.kind(),
            format!(
                "Failed to read file source content for generation {}: {}",
                node_record.path.display(),
                e
            ),
        )))
    })?;

    let truncated = bytes.len() > FILE_CONTEXT_MAX_BYTES;
    let slice = if truncated {
        &bytes[..FILE_CONTEXT_MAX_BYTES]
    } else {
        &bytes
    };
    let mut text = String::from_utf8_lossy(slice).to_string();
    if truncated {
        text.push_str(&format!(
            "\n\n[Truncated to {} bytes for prompt safety]",
            FILE_CONTEXT_MAX_BYTES
        ));
    }

    Ok(format!(
        "Path: {}\nType: File\nContent:\n{}",
        node_record.path.display(),
        text
    ))
}
