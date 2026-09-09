//! Installed Docs judgment instructions and explicitly selected guard operators.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use crate::error::ApiError;
use crate::provider::{ChatMessage, MessageRole};

/// Exact semantic resources admitted with a Docs policy revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocsSemanticTheory {
    pub schema_version: u32,
    /// Historical revisions can be read, but cannot author new observations without scope.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<super::scope::DocsScopePolicy>,
    pub source_extraction: String,
    pub readme_judgment: String,
    pub correspondence: String,
    pub drafting: String,
    pub revision: String,
    /// Provider response contracts selected by installed theory for each judgment.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub response_formats: BTreeMap<DocsJudgmentOperation, serde_json::Value>,
    pub claim_guards: Vec<DocsClaimGuard>,
    /// Absent in historical revisions. An explicit empty list refuses repair.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repair_actions: Option<Vec<DocsRepairAction>>,
}

/// Bounded responses inside an authorized validation invocation. These do not
/// admit Tasks or grant permission to publish workspace changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocsRepairAction {
    ReviseV1,
    PruneRejectedClaimsV1,
}

/// Named evaluator contracts. An empty selection delegates semantic entailment
/// to the configured judge; it never disables identity or citation integrity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocsClaimGuard {
    LiteralPresenceV1,
    CodeLineDirectPresenceV1,
    ClauseTermCoverageV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocsJudgmentOperation {
    SourceExtraction,
    ReadmeJudgment,
    Correspondence,
    Drafting,
    Revision,
}

pub(crate) struct DocsGeneration {
    pub request: crate::context::generation::contracts::GenerationOrchestrationRequest,
    pub messages: Vec<ChatMessage>,
}

impl DocsJudgmentOperation {
    fn frame_type(self) -> &'static str {
        match self {
            Self::SourceExtraction => "docs-source-claims",
            Self::ReadmeJudgment => "docs-claim-validation",
            Self::Correspondence => "docs-claim-correspondence",
            Self::Drafting => "docs-readme",
            Self::Revision => "docs-readme-revision",
        }
    }
}

impl DocsSemanticTheory {
    pub fn validate(&self) -> Result<(), ApiError> {
        self.repair_actions()?;
        self.scope()?.validate()?;
        if self.schema_version != 1 {
            return Err(invalid("unsupported Docs semantic theory schema"));
        }
        if [
            &self.source_extraction,
            &self.readme_judgment,
            &self.correspondence,
            &self.drafting,
            &self.revision,
        ]
        .iter()
        .any(|instruction| instruction.trim().is_empty())
        {
            return Err(invalid(
                "Docs semantic theory requires every judgment instruction",
            ));
        }
        if self.claim_guards.iter().collect::<BTreeSet<_>>().len() != self.claim_guards.len() {
            return Err(invalid("Docs semantic theory repeats a guard operator"));
        }
        Ok(())
    }

    pub(crate) fn scope(&self) -> Result<&super::scope::DocsScopePolicy, ApiError> {
        self.scope
            .as_ref()
            .ok_or_else(|| invalid("Docs theory requires an explicit scope selection"))
    }

    pub(crate) fn repair_actions(&self) -> Result<&[DocsRepairAction], ApiError> {
        self.repair_actions.as_deref().ok_or_else(|| {
            invalid("Docs semantic theory requires an explicit repair response selection")
        })
    }

    /// Fingerprint the actual bounded messages and selected bindings, so source
    /// or instruction changes cannot reuse a request identity for different input.
    pub(crate) fn generation(
        &self,
        config: &super::capability::DocsCapabilityConfig,
        policy_identity: &str,
        operation: DocsJudgmentOperation,
        input: serde_json::Value,
        retry_count: usize,
        batch_index: usize,
    ) -> Result<DocsGeneration, ApiError> {
        let messages = self.messages(operation, input.clone())?;
        let mut provider = config.provider.clone();
        if let Some(format) = self.response_formats.get(&operation) {
            let format = bind_evidence_choices(format, &input)?;
            if provider
                .runtime_overrides
                .extra_body_fields
                .get("response_format")
                .is_some_and(|configured| configured != &format)
            {
                return Err(invalid(
                    "provider response format conflicts with installed Docs theory",
                ));
            }
            provider
                .runtime_overrides
                .extra_body_fields
                .insert("response_format".into(), format);
        }
        let bytes = serde_json::to_vec(&(
            policy_identity,
            &config.subject_id,
            &config.agent_id,
            &config.target_root,
            &provider,
            operation.frame_type(),
            &messages,
            retry_count,
            batch_index,
        ))
        .map_err(|error| invalid(&error.to_string()))?;
        let digest = blake3::hash(&bytes);
        let mut request_bytes = [0; 8];
        request_bytes.copy_from_slice(&digest.as_bytes()[..8]);
        Ok(DocsGeneration {
            request: crate::context::generation::contracts::GenerationOrchestrationRequest {
                request_id: u64::from_le_bytes(request_bytes),
                node_id: *digest.as_bytes(),
                agent_id: config.agent_id.clone(),
                provider,
                frame_type: operation.frame_type().into(),
                retry_count,
                force: true,
            },
            messages,
        })
    }

    pub(crate) fn messages(
        &self,
        operation: DocsJudgmentOperation,
        input: serde_json::Value,
    ) -> Result<Vec<ChatMessage>, ApiError> {
        self.validate()?;
        let instruction = match operation {
            DocsJudgmentOperation::SourceExtraction => &self.source_extraction,
            DocsJudgmentOperation::ReadmeJudgment => &self.readme_judgment,
            DocsJudgmentOperation::Correspondence => &self.correspondence,
            DocsJudgmentOperation::Drafting => &self.drafting,
            DocsJudgmentOperation::Revision => &self.revision,
        };
        Ok(vec![
            ChatMessage {
                role: MessageRole::System,
                content: instruction.clone(),
            },
            ChatMessage {
                role: MessageRole::User,
                content: serde_json::to_string(&input)
                    .map_err(|error| invalid(&error.to_string()))?,
            },
        ])
    }
}

/// Materialize a theory-selected enum from exact captured text. This constrains
/// quotation spelling; native citation checks and judgment still establish meaning.
fn bind_evidence_choices(
    value: &serde_json::Value,
    input: &serde_json::Value,
) -> Result<serde_json::Value, ApiError> {
    use serde_json::Value;
    match value {
        Value::Object(fields) => {
            if let Some(selection) = fields.get("x-meld-map-from-field") {
                let (items, field) = selected_items(selection, input)?;
                let template = selection
                    .get("value_schema")
                    .ok_or_else(|| invalid("keyed response contract has no value schema"))?;
                let bound = bind_evidence_choices(template, input)?;
                let mut properties = serde_json::Map::new();
                let mut required = Vec::new();
                for item in items {
                    let key = item
                        .pointer(field)
                        .and_then(Value::as_str)
                        .filter(|key| !key.is_empty())
                        .ok_or_else(|| invalid("keyed response input has no string identity"))?;
                    if properties.insert(key.into(), bound.clone()).is_some() {
                        return Err(invalid("keyed response input repeats an identity"));
                    }
                    required.push(key);
                }
                return Ok(serde_json::json!({"type":"object","properties":properties,
                    "required":required,"additionalProperties":false}));
            }
            if let Some(pointer) = fields.get("x-meld-if-text") {
                let text = pointer
                    .as_str()
                    .and_then(|pointer| input.pointer(pointer))
                    .and_then(Value::as_str)
                    .ok_or_else(|| invalid("conditional evidence input is absent or not text"))?;
                if text.trim().is_empty() {
                    return Ok(Value::Bool(false));
                }
            }
            let mut bound = serde_json::Map::new();
            for (key, value) in fields {
                if key == "x-meld-if-text" {
                    continue;
                } else if key == "x-meld-enum-from-field" {
                    let (items, field) = selected_items(value, input)?;
                    let choices = items
                        .iter()
                        .map(|item| {
                            item.pointer(field).and_then(Value::as_str).ok_or_else(|| {
                                invalid("response choice input has no string identity")
                            })
                        })
                        .collect::<Result<BTreeSet<_>, _>>()?;
                    if choices.is_empty() || fields.contains_key("enum") {
                        return Err(invalid(
                            "response identity choice contract has no choices or repeats enum",
                        ));
                    }
                    bound.insert(
                        "enum".into(),
                        serde_json::to_value(choices)
                            .map_err(|error| invalid(&error.to_string()))?,
                    );
                } else if key == "x-meld-enum-from-lines" {
                    let pointers: Vec<String> = serde_json::from_value(value.clone())
                        .map_err(|error| invalid(&error.to_string()))?;
                    let limit = fields
                        .get("maxLength")
                        .and_then(Value::as_u64)
                        .map(|limit| limit as usize)
                        .unwrap_or(usize::MAX);
                    if limit == 0 || pointers.is_empty() || fields.contains_key("enum") {
                        return Err(invalid("invalid evidence choice contract"));
                    }
                    let mut choices = BTreeSet::new();
                    for pointer in pointers {
                        let text =
                            input
                                .pointer(&pointer)
                                .and_then(Value::as_str)
                                .ok_or_else(|| {
                                    invalid("evidence choice input is absent or not text")
                                })?;
                        for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
                            let characters = line.chars().collect::<Vec<_>>();
                            for chunk in characters.chunks(limit) {
                                choices.insert(chunk.iter().collect::<String>());
                            }
                        }
                    }
                    if choices.is_empty() {
                        return Err(invalid("evidence choice contract has no captured text"));
                    }
                    bound.insert(
                        "enum".into(),
                        serde_json::to_value(choices)
                            .map_err(|error| invalid(&error.to_string()))?,
                    );
                } else {
                    let mut value = bind_evidence_choices(value, input)?;
                    if matches!(key.as_str(), "anyOf" | "oneOf") {
                        if let Some(alternatives) = value.as_array_mut() {
                            alternatives.retain(|alternative| *alternative != Value::Bool(false));
                            if alternatives.is_empty() {
                                return Err(invalid(
                                    "response contract has no available evidence alternatives",
                                ));
                            }
                        }
                    }
                    bound.insert(key.clone(), value);
                }
            }
            Ok(Value::Object(bound))
        }
        Value::Array(values) => values
            .iter()
            .map(|value| bind_evidence_choices(value, input))
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array),
        other => Ok(other.clone()),
    }
}

fn selected_items<'a>(
    selection: &'a serde_json::Value,
    input: &'a serde_json::Value,
) -> Result<(&'a Vec<serde_json::Value>, &'a str), ApiError> {
    let items = selection
        .get("array")
        .and_then(serde_json::Value::as_str)
        .and_then(|pointer| input.pointer(pointer))
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| invalid("response contract input is absent or not an array"))?;
    let field = selection
        .get("field")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| invalid("response contract has no identity field pointer"))?;
    Ok((items, field))
}

fn invalid(message: &str) -> ApiError {
    ApiError::ConfigError(message.into())
}

#[cfg(test)]
mod tests;
