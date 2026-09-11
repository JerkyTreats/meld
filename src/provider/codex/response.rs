use crate::error::ApiError;
use crate::provider::{CompletionResponse, TokenUsage};
use serde_json::Value;

pub(super) fn parse(
    stdout: &str,
    model: &str,
    structured: bool,
) -> Result<CompletionResponse, ApiError> {
    let invalid =
        |detail: &str| ApiError::ProviderError(format!("Invalid Codex completion: {detail}"));
    let mut content = None;
    let mut usage = None;
    for line in stdout.lines().filter(|line| !line.trim().is_empty()) {
        let event: Value = serde_json::from_str(line).map_err(|_| invalid("malformed JSONL"))?;
        match event["type"].as_str() {
            Some("thread.started" | "turn.started") => {}
            Some("item.started" | "item.updated" | "item.completed") => {
                match event["item"]["type"].as_str() {
                    Some("agent_message") => {
                        if event["type"] == "item.completed" {
                            content = event["item"]["text"].as_str().map(str::to_owned);
                        }
                    }
                    Some("reasoning") => {}
                    Some("error") => {
                        return Err(ApiError::ProviderRequestFailed(format!(
                            "Codex item error: {}",
                            event["item"]["message"]
                        )))
                    }
                    _ => {
                        return Err(invalid(&format!(
                        "tool or unknown item observed: {}; refusing a tool-assisted completion",
                        event["item"]["type"]
                    )))
                    }
                }
            }
            Some("turn.completed") => {
                if usage.is_some() {
                    return Err(invalid("multiple completed turns"));
                }
                let tokens = |key: &str| {
                    event["usage"][key]
                        .as_u64()
                        .and_then(|n| u32::try_from(n).ok())
                        .ok_or_else(|| invalid("missing or overflowing token usage"))
                };
                let input = tokens("input_tokens")?;
                let output = tokens("output_tokens")?;
                usage = Some(TokenUsage {
                    prompt_tokens: input,
                    completion_tokens: output,
                    total_tokens: input
                        .checked_add(output)
                        .ok_or_else(|| invalid("overflowing total usage"))?,
                });
            }
            Some("error" | "turn.failed") => {
                return Err(ApiError::ProviderRequestFailed(format!(
                    "Codex reported failure: {event}"
                )))
            }
            _ => return Err(invalid("unknown event type")),
        }
    }
    let content = content
        .filter(|text| !text.trim().is_empty())
        .ok_or_else(|| invalid("missing final message"))?;
    if structured {
        serde_json::from_str::<Value>(&content)
            .map_err(|_| invalid("final message is not JSON"))?;
    }
    Ok(CompletionResponse {
        content,
        model: model.into(),
        usage: usage.ok_or_else(|| invalid("turn never completed"))?,
        finish_reason: Some("turn.completed".into()),
        execution_metadata: Default::default(),
    })
}
