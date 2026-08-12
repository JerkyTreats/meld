//! Prompt assembly for context generation.
//!
//! Owns rendering of node context, prior frames, and (flag-gated) belief
//! conditioning into provider messages. Belief conditioning derives entirely
//! from the seeded [`BeliefContextBundle`]: assembly never reads live belief
//! state, so a prepared task run replays byte-identically regardless of
//! belief mutations after hydration. The world model never sees the prompt
//! text produced by this module.

use crate::agent::profile::prompt_contract::PromptContract;
use crate::context::belief_context::{
    classify_belief_assertion, BeliefContextAssertion, BeliefContextBundle, BeliefSelectionClass,
    BELIEF_CONTEXT_FAMILY_ID,
};
use crate::context::generation::contracts::{GenerationOrchestrationRequest, PromptAssemblyOutput};
use crate::error::ApiError;
use crate::execution::ContextReadPort;
use crate::provider::{ChatMessage, MessageRole};
use crate::store::{NodeRecord, NodeType};
use crate::views::{FrameFilter, OrderingPolicy};

const FILE_CONTEXT_MAX_BYTES: usize = 128 * 1024;

/// Rendered verbatim as the directory context payload when no child frames,
/// no file-child source, and no node-scoped frames exist. Sibling contract
/// with the workflow resolver: the model must be told context is missing
/// rather than receive a prompt with no context block at all.
pub const INSUFFICIENT_CONTEXT_MARKER: &str = "Insufficient context";

/// Builds prompt messages with belief conditioning disabled. Byte-identical
/// to pre-`belief_context` behavior.
pub fn build_prompt_messages(
    api: &(impl ContextReadPort + ?Sized),
    request: &GenerationOrchestrationRequest,
    node_record: &NodeRecord,
    prompt_contract: &PromptContract,
) -> Result<PromptAssemblyOutput, ApiError> {
    build_prompt_messages_with_belief(api, request, node_record, prompt_contract, None)
}

/// Builds prompt messages, optionally conditioned on a belief context bundle.
///
/// `belief_bundle: Some(_)` marks a `belief_context`-enabled run: prior-frame
/// selection switches to [`OrderingPolicy::BeliefEndorsed`] and a belief brief
/// is rendered into the context payload when the bundle carries an assertion.
/// An explicitly empty bundle (no belief for the subject) still enables
/// belief-endorsed selection but leaves the rendered prompt otherwise
/// unchanged.
pub fn build_prompt_messages_with_belief(
    api: &(impl ContextReadPort + ?Sized),
    request: &GenerationOrchestrationRequest,
    node_record: &NodeRecord,
    prompt_contract: &PromptContract,
    belief_bundle: Option<&BeliefContextBundle>,
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
                collect_directory_child_context_text(api, node_record, request, belief_bundle)?;
            if child_context_text.is_empty() {
                let node_context_text =
                    collect_scoped_node_frame_context(api, request, belief_bundle)?;
                if node_context_text.is_empty() {
                    Some(INSUFFICIENT_CONTEXT_MARKER.to_string())
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

    let mut context_payload = prompt_context.unwrap_or_default();
    // Belief brief precedes other context so settled belief frames the rest.
    if let Some(section) = belief_bundle.and_then(render_belief_context_section) {
        context_payload = if context_payload.is_empty() {
            section
        } else {
            format!("{}\n\n---\n\n{}", section, context_payload)
        };
    }
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

/// Renders the belief brief from the trigger subject's seeded assertion.
/// Bundles that do not cover the trigger subject render nothing so the
/// prompt stays unchanged when no belief covers the subject.
fn render_belief_context_section(bundle: &BeliefContextBundle) -> Option<String> {
    let assertion = bundle.assertion_for_subject(&bundle.subject_node_id)?;
    let mut lines = vec![
        format!("Belief Context (subject: {}):", bundle.subject_path),
        // Bundle-level sequence is the subtree high-water mark; the per-
        // subject currency readers should trust is on the Assertion line.
        format!(
            "- Family: {} (high-water sequence {})",
            bundle.family_id, bundle.as_of_seq
        ),
        format!(
            "- Assertion: status={}, confidence={} (as of sequence {}){}",
            assertion.status,
            assertion.confidence,
            assertion.as_of_seq,
            if assertion.stale { ", stale" } else { "" }
        ),
        "- Settled beliefs above are authoritative over model recall; do not override them from memory.".to_string(),
    ];
    if assertion.contradicted || !assertion.contradicted_claims.is_empty() {
        lines.push("- Unresolved contradicted claims:".to_string());
        if assertion.contradicted_claims.is_empty() {
            lines.push("  - (contradiction flagged without hydrated claim refs)".to_string());
        }
        for claim in &assertion.contradicted_claims {
            lines.push(format!("  - {} (unresolved)", claim));
        }
    }
    if let Some(revision_id) = &assertion.revision_id {
        lines.push(format!("- Belief revision: {}", revision_id));
    }
    if !assertion.evidence_refs.is_empty() {
        lines.push(format!(
            "- Evidence refs: {}",
            assertion.evidence_refs.join(", ")
        ));
    }
    if !assertion.source_fact_refs.is_empty() {
        lines.push(format!(
            "- Source facts: {}",
            assertion.source_fact_refs.join(", ")
        ));
    }
    Some(lines.join("\n"))
}

fn belief_annotation_line(
    class: &BeliefSelectionClass,
    assertion: &BeliefContextAssertion,
) -> String {
    match class {
        BeliefSelectionClass::Endorsed => format!(
            "Belief: {} endorsed, status={}, confidence={} (as of sequence {})",
            BELIEF_CONTEXT_FAMILY_ID, assertion.status, assertion.confidence, assertion.as_of_seq
        ),
        BeliefSelectionClass::Stale => format!(
            "Belief: {} stale, status={}, confidence={} (as of sequence {})",
            BELIEF_CONTEXT_FAMILY_ID, assertion.status, assertion.confidence, assertion.as_of_seq
        ),
        BeliefSelectionClass::Contradicted => format!(
            "Belief: {} contradicted (unresolved as of sequence {}) — prior content withheld",
            BELIEF_CONTEXT_FAMILY_ID, assertion.as_of_seq
        ),
        BeliefSelectionClass::Uncovered => String::new(),
    }
}

fn collect_directory_child_context_text(
    api: &(impl ContextReadPort + ?Sized),
    node_record: &NodeRecord,
    request: &GenerationOrchestrationRequest,
    belief_bundle: Option<&BeliefContextBundle>,
) -> Result<String, ApiError> {
    if !matches!(node_record.node_type, NodeType::Directory) {
        return Ok(String::new());
    }

    let child_view = crate::context::query::view::ContextView {
        max_frames: 1,
        ordering: if belief_bundle.is_some() {
            OrderingPolicy::BeliefEndorsed
        } else {
            OrderingPolicy::Recency
        },
        filters: vec![
            FrameFilter::ByType(request.frame_type.clone()),
            FrameFilter::ByAgent(request.agent_id.clone()),
        ],
    };

    // (selection rank, authored child order, section text). Flag-off runs
    // only ever produce rank 0 in child order, preserving legacy output.
    let mut child_sections: Vec<(u8, usize, String)> = Vec::new();
    for (child_order, child_id) in node_record.children.iter().enumerate() {
        let child_context = api.get_node(*child_id, child_view.clone())?;
        // Frameless file children fall back to on-disk source so a cold run
        // still sees real content — sibling contract with the workflow
        // resolver. Frameless directory children stay skipped: bottom-up
        // ordering guarantees a traversed subdirectory has frames by the
        // time its parent assembles.
        let child_text = if child_context.frames.is_empty() {
            match child_context.node_record.node_type {
                NodeType::File { .. } => file_source_text(&child_context.node_record)?,
                NodeType::Directory => continue,
            }
        } else {
            joined_frame_text(&child_context.frames)
        };

        let child_kind = match child_context.node_record.node_type {
            NodeType::File { .. } => "File",
            NodeType::Directory => "Directory",
        };
        let child_path = child_context.node_record.path.display().to_string();

        if let Some(bundle) = belief_bundle {
            // Belief-endorsed selection: deterministic function of the
            // seeded bundle assertions plus the authored child order. Live
            // belief state is never consulted here.
            let assertion = bundle.assertion_for_subject(&hex::encode(child_id));
            let class = classify_belief_assertion(assertion);
            let section = match (&class, assertion) {
                (BeliefSelectionClass::Contradicted, Some(assertion)) => {
                    // Contradicted subject: content excluded, flagged as
                    // unresolved instead of silently dropped.
                    format!(
                        "Path: {}\nType: {}\n{}",
                        child_path,
                        child_kind,
                        belief_annotation_line(&class, assertion)
                    )
                }
                (_, assertion) => match assertion {
                    Some(assertion) if class != BeliefSelectionClass::Uncovered => format!(
                        "Path: {}\nType: {}\n{}\nContent:\n{}",
                        child_path,
                        child_kind,
                        belief_annotation_line(&class, assertion),
                        child_text
                    ),
                    _ => format!(
                        "Path: {}\nType: {}\nContent:\n{}",
                        child_path, child_kind, child_text
                    ),
                },
            };
            child_sections.push((class.rank(), child_order, section));
        } else {
            child_sections.push((
                0,
                child_order,
                format!(
                    "Path: {}\nType: {}\nContent:\n{}",
                    child_path, child_kind, child_text
                ),
            ));
        }
    }

    child_sections.sort_by_key(|(rank, order, _)| (*rank, *order));

    Ok(child_sections
        .into_iter()
        .map(|(_, _, section)| section)
        .collect::<Vec<_>>()
        .join("\n\n---\n\n"))
}

fn collect_scoped_node_frame_context(
    api: &(impl ContextReadPort + ?Sized),
    request: &GenerationOrchestrationRequest,
    belief_bundle: Option<&BeliefContextBundle>,
) -> Result<String, ApiError> {
    let view = crate::context::query::view::ContextView {
        max_frames: 10,
        ordering: if belief_bundle.is_some() {
            OrderingPolicy::BeliefEndorsed
        } else {
            OrderingPolicy::Recency
        },
        filters: vec![
            FrameFilter::ByType(request.frame_type.clone()),
            FrameFilter::ByAgent(request.agent_id.clone()),
        ],
    };
    let context = api.get_node(request.node_id, view)?;
    let joined = joined_frame_text(&context.frames);

    let Some(bundle) = belief_bundle else {
        return Ok(joined);
    };
    if joined.is_empty() {
        return Ok(joined);
    }

    // Node scope has a single candidate subject: the request node itself,
    // classified from its seeded bundle assertion.
    let assertion = bundle.assertion_for_subject(&hex::encode(request.node_id));
    let class = classify_belief_assertion(assertion);
    Ok(match (&class, assertion) {
        (BeliefSelectionClass::Contradicted, Some(assertion)) => {
            belief_annotation_line(&class, assertion)
        }
        (BeliefSelectionClass::Uncovered, _) | (_, None) => joined,
        (_, Some(assertion)) => {
            format!("{}\n{}", belief_annotation_line(&class, assertion), joined)
        }
    })
}

fn joined_frame_text(frames: &[crate::context::frame::Frame]) -> String {
    frames
        .iter()
        .map(|f| String::from_utf8_lossy(&f.content))
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn collect_file_source_context(node_record: &NodeRecord) -> Result<String, ApiError> {
    if !matches!(node_record.node_type, NodeType::File { .. }) {
        return Ok(String::new());
    }

    Ok(format!(
        "Path: {}\nType: File\nContent:\n{}",
        node_record.path.display(),
        file_source_text(node_record)?
    ))
}

/// Bounded on-disk source read shared by file-target context and the
/// frameless file-child fallback.
fn file_source_text(node_record: &NodeRecord) -> Result<String, ApiError> {
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

    Ok(text)
}
