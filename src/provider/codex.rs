//! Structured completion through an isolated, noninteractive Codex invocation.

mod process;
mod response;
#[cfg(test)]
mod tests;

use super::{
    ChatMessage, CompletionOptions, CompletionResponse, CompletionStream, MessageRole,
    ModelProviderClient,
};
use crate::error::ApiError;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub(super) struct CodexClient {
    model: String,
}

struct Settings {
    binary: PathBuf,
    home: PathBuf,
    effort: String,
    timeout: Duration,
    schema: Option<Value>,
}

fn option_string(options: &CompletionOptions, key: &str) -> Result<Option<String>, ApiError> {
    options
        .additional_json
        .get(key)
        .map(|value| {
            value
                .as_str()
                .filter(|text| !text.trim().is_empty())
                .map(str::to_owned)
                .ok_or_else(|| {
                    ApiError::ConfigError(format!("Codex {key} must be a nonempty string"))
                })
        })
        .transpose()
}

pub(super) fn validate_options(options: &CompletionOptions) -> Result<(), ApiError> {
    // The shared default temperature is not an explicit Codex sampling control.
    if options.temperature.is_some_and(|value| value != 1.0)
        || options.max_tokens.is_some()
        || options.top_p.is_some()
        || options.frequency_penalty.is_some()
        || options.presence_penalty.is_some()
        || options.stop.is_some()
    {
        return Err(ApiError::ConfigError(
            "Codex exec does not expose these sampling or output-token controls; omit them".into(),
        ));
    }
    for key in options.additional_json.keys() {
        if ![
            "codex_binary",
            "codex_home",
            "reasoning_effort",
            "timeout_ms",
            "response_format",
        ]
        .contains(&key.as_str())
        {
            return Err(ApiError::ConfigError(format!(
                "Unsupported Codex completion option: {key}"
            )));
        }
    }
    for key in ["codex_binary", "codex_home", "reasoning_effort"] {
        option_string(options, key)?;
    }
    if let Some(effort) = option_string(options, "reasoning_effort")? {
        if !["none", "minimal", "low", "medium", "high", "xhigh", "max"].contains(&effort.as_str())
        {
            return Err(ApiError::ConfigError(
                "Invalid Codex reasoning_effort".into(),
            ));
        }
    }
    if let Some(timeout) = options.additional_json.get("timeout_ms") {
        if !timeout
            .as_u64()
            .is_some_and(|value| value > 0 && value <= 600_000)
        {
            return Err(ApiError::ConfigError(
                "Codex timeout_ms must be between 1 and 600000".into(),
            ));
        }
    }
    if let Some(format) = options.additional_json.get("response_format") {
        if format["type"] != "json_schema" || !format["json_schema"]["schema"].is_object() {
            return Err(ApiError::ConfigError(
                "Codex response_format requires a json_schema object".into(),
            ));
        }
    }
    Ok(())
}

pub(super) fn validate_request_overrides(
    fields: &std::collections::BTreeMap<String, Value>,
) -> Result<(), ApiError> {
    if fields.contains_key("codex_binary") || fields.contains_key("codex_home") {
        return Err(ApiError::ConfigError("Codex executable and authentication home belong in the provider configuration, not request overrides".into()));
    }
    Ok(())
}

impl Settings {
    fn resolve(options: &CompletionOptions) -> Result<Self, ApiError> {
        validate_options(options)?;
        let selected = option_string(options, "codex_binary")?.unwrap_or_else(|| "codex".into());
        let binary = if Path::new(&selected).is_absolute() {
            PathBuf::from(selected)
        } else if Path::new(&selected).components().count() == 1 {
            std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
                .map(|base| base.join(&selected))
                .find(|path| path.is_file())
                .ok_or_else(|| {
                    ApiError::ProviderNotConfigured(
                        "Codex executable not found on PATH; set codex_binary".into(),
                    )
                })?
        } else {
            return Err(ApiError::ConfigError(
                "codex_binary must be absolute or a command name on PATH".into(),
            ));
        }
        .canonicalize()
        .map_err(process::io_error)?;
        let home = option_string(options, "codex_home")?
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("CODEX_HOME").map(PathBuf::from))
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".codex")))
            .ok_or_else(|| {
                ApiError::ProviderNotConfigured(
                    "Set codex_home or CODEX_HOME for Codex login".into(),
                )
            })?;
        if !home.is_absolute() {
            return Err(ApiError::ConfigError("codex_home must be absolute".into()));
        }
        Ok(Self {
            binary,
            home,
            effort: option_string(options, "reasoning_effort")?.unwrap_or_else(|| "low".into()),
            timeout: Duration::from_millis(
                options
                    .additional_json
                    .get("timeout_ms")
                    .and_then(Value::as_u64)
                    .unwrap_or(120_000),
            ),
            schema: options
                .additional_json
                .get("response_format")
                .map(|format| format["json_schema"]["schema"].clone()),
        })
    }
}

impl CodexClient {
    pub(super) fn new(model: String) -> Self {
        Self { model }
    }
}

#[async_trait]
impl ModelProviderClient for CodexClient {
    async fn complete(
        &self,
        messages: Vec<ChatMessage>,
        options: CompletionOptions,
    ) -> Result<CompletionResponse, ApiError> {
        let settings = Settings::resolve(&options)?;
        let (instructions, prompt) = match messages.as_slice() {
            [system, user] if system.role == MessageRole::System && user.role == MessageRole::User =>
                (system.content.as_str(), user.content.as_str()),
            [user] if user.role == MessageRole::User =>
                ("Complete the supplied request using only its content.", user.content.as_str()),
            _ => return Err(ApiError::ConfigError("Codex completion accepts one user message with an optional preceding system message".into())),
        };
        let root = process::invocation_directory()?;
        let cwd = root.path();
        let instruction_path = cwd.join("instructions.txt");
        std::fs::write(&instruction_path, instructions).map_err(process::io_error)?;
        let mut args = vec![
            "exec".to_string(),
            "--ephemeral".into(),
            "--ignore-user-config".into(),
            "--ignore-rules".into(),
            "--skip-git-repo-check".into(),
            "--json".into(),
            "--color".into(),
            "never".into(),
            "--sandbox".into(),
            "read-only".into(),
            "--model".into(),
            self.model.clone(),
        ];
        let mut config = vec![
            "approval_policy=\"never\"".to_string(),
            "project_doc_max_bytes=0".into(),
            "web_search=\"disabled\"".into(),
            "tools.view_image=false".into(),
            "mcp_servers={}".into(),
            "developer_instructions=\"\"".into(),
            format!("model_reasoning_effort={}", json!(settings.effort)),
            format!("model_instructions_file={}", json!(instruction_path)),
            format!("sqlite_home={}", json!(cwd.join("state"))),
            format!("log_dir={}", json!(cwd.join("logs"))),
        ];
        for feature in [
            "shell_tool",
            "unified_exec",
            "shell_snapshot",
            "apps",
            "plugins",
            "hooks",
            "memories",
            "multi_agent",
            "multi_agent_v2",
            "goals",
            "browser_use",
            "computer_use",
            "image_generation",
            "view_image",
            "skill_search",
            "code_mode",
        ] {
            config.push(format!("features.{feature}=false"));
        }
        // CODEX_HOME supplies authentication, but its user-authored skills must
        // not become additional instructions for a provider completion.
        let skill_root = settings.home.join("skills");
        let mut disabled_skills = Vec::new();
        if skill_root.is_dir() {
            for entry in walkdir::WalkDir::new(&skill_root).follow_links(false) {
                let entry = entry.map_err(|error| ApiError::ConfigError(error.to_string()))?;
                if entry.file_name() == "SKILL.md" {
                    disabled_skills.push(format!(
                        "{{path={},enabled=false}}",
                        json!(entry.path().parent())
                    ));
                }
            }
        }
        config.push(format!("skills.config=[{}]", disabled_skills.join(",")));
        for value in &config {
            args.extend(["--config".into(), value.clone()]);
        }
        let schema_bytes = settings
            .schema
            .as_ref()
            .map(serde_json::to_vec)
            .transpose()
            .map_err(|error| ApiError::ConfigError(error.to_string()))?;
        if let Some(bytes) = &schema_bytes {
            let path = cwd.join("schema.json");
            std::fs::write(&path, bytes).map_err(process::io_error)?;
            args.extend([
                "--output-schema".into(),
                path.to_string_lossy().into_owned(),
            ]);
        }
        args.push("-".into());
        let binary_hash =
            blake3::hash(&std::fs::read(&settings.binary).map_err(process::io_error)?)
                .to_hex()
                .to_string();
        let started = Instant::now();
        let mut evidence = json!({
            "binary": settings.binary, "binary_blake3": binary_hash,
            "requested_model": self.model, "reasoning_effort": settings.effort, "argv": args,
            "instruction_blake3": blake3::hash(instructions.as_bytes()).to_hex().to_string(),
            "prompt_blake3": blake3::hash(prompt.as_bytes()).to_hex().to_string(),
            "schema_blake3": schema_bytes.as_ref().map(|bytes| blake3::hash(bytes).to_hex().to_string()),
            "timeout_ms": settings.timeout.as_millis(),
            "context_policy": "isolated-cwd-home-env; explicit-instructions; ignore-user-config-rules; disabled-action-tools; reject-tool-events",
            "model_identity_source": "requested CLI argument; exec JSONL does not report resolved model identity",
            "sampling_controls": "Codex defaults; shared default temperature is not forwarded"
        });
        let version = process::run(
            &settings.binary,
            &["--version".into()],
            cwd,
            &settings.home,
            "",
            Duration::from_secs(10),
        )
        .await
        .map_err(|error| execution_failure(error, evidence.clone()))?;
        evidence["version"] = json!(version.stdout.trim());
        let output = process::run(
            &settings.binary,
            &args,
            cwd,
            &settings.home,
            prompt,
            settings.timeout,
        )
        .await
        .map_err(|error| execution_failure(error, evidence.clone()))?;
        evidence["elapsed_ms"] = json!(started.elapsed().as_millis());
        evidence["stderr"] = json!(output.stderr);
        evidence["stdout"] = json!(output.stdout);
        let mut completion =
            response::parse(&output.stdout, &self.model, settings.schema.is_some())
                .map_err(|error| execution_failure(error, evidence.clone()))?;
        completion
            .execution_metadata
            .insert("codex".into(), evidence);
        Ok(completion)
    }

    async fn stream(
        &self,
        _: Vec<ChatMessage>,
        _: CompletionOptions,
    ) -> Result<CompletionStream, ApiError> {
        Err(ApiError::ProviderError(
            "Codex streaming completion is not supported".into(),
        ))
    }
    fn provider_name(&self) -> &str {
        "codex"
    }
    fn model_name(&self) -> &str {
        &self.model
    }
    async fn list_models(&self) -> Result<Vec<String>, ApiError> {
        Err(ApiError::ProviderError("Codex exec does not expose a model catalog; model availability is established by a completion".into()))
    }
}

fn execution_failure(error: ApiError, mut evidence: Value) -> ApiError {
    if let ApiError::ProviderExecutionFailed { metadata, .. } = &error {
        evidence["process"] = json!(metadata);
    }
    ApiError::ProviderExecutionFailed {
        message: error.to_string(),
        metadata: std::collections::BTreeMap::from([("codex".into(), evidence)]),
    }
}
