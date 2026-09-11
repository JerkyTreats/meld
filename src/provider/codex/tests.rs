use super::*;

fn events(content: &str) -> String {
    [json!({"type":"thread.started","thread_id":"example"}),
        json!({"type":"turn.started"}),
        json!({"type":"item.completed","item":{"type":"agent_message","text":content}}),
        json!({"type":"turn.completed","usage":{"input_tokens":100,"cached_input_tokens":40,"output_tokens":10}})]
        .iter().map(Value::to_string).collect::<Vec<_>>().join("\n")
}

#[test]
fn completion_requires_native_success_and_preserves_reported_usage() {
    let parsed = response::parse(&events(r#"{"accepted":false}"#), "selected", true).unwrap();
    assert_eq!(parsed.content, r#"{"accepted":false}"#);
    assert_eq!(parsed.usage.prompt_tokens, 100);
    assert_eq!(parsed.usage.total_tokens, 110);
    let startup_error = "{\"type\":\"item.completed\",\"item\":{\"type\":\"error\",\"message\":\"required host unavailable\"}}\n";
    assert!(response::parse(
        &(startup_error.to_string() + &events("ok")),
        "selected",
        false
    )
    .unwrap_err()
    .to_string()
    .contains("required host unavailable"));
    assert!(response::parse(&events("not json"), "selected", true).is_err());
    assert!(response::parse(
        &events("ok").lines().take(3).collect::<Vec<_>>().join("\n"),
        "selected",
        false
    )
    .is_err());
    assert!(response::parse(
        &(events("ok") + "\n{\"type\":\"turn.failed\",\"error\":\"failed\"}"),
        "selected",
        false
    )
    .is_err());
    assert!(response::parse(
        &(events("ok")
            + "\n{\"type\":\"item.completed\",\"item\":{\"type\":\"command_execution\"}}"),
        "selected",
        false
    )
    .is_err());
}

#[test]
fn provider_options_refuse_silent_sampling_changes() {
    let mut options = CompletionOptions::default();
    assert!(validate_options(&options).is_ok());
    options.max_tokens = Some(8192);
    assert!(validate_options(&options).is_err());
    options.max_tokens = None;
    options
        .additional_json
        .insert("reasoning_effort".into(), json!("typo"));
    assert!(validate_options(&options).is_err());
    options.additional_json.clear();
    options
        .additional_json
        .insert("chat_template_kwargs".into(), json!({}));
    assert!(validate_options(&options).is_err());
}

#[test]
fn request_schema_cannot_replace_the_host_executable_or_authentication_home() {
    let mut fields = std::collections::BTreeMap::from([(
        "response_format".into(),
        json!({"type":"json_schema"}),
    )]);
    assert!(validate_request_overrides(&fields).is_ok());
    for key in ["codex_binary", "codex_home"] {
        fields.insert(key.into(), json!("/other/location"));
        assert!(validate_request_overrides(&fields).is_err());
        fields.remove(key);
    }
}

#[cfg(unix)]
#[tokio::test]
async fn refused_completion_retains_native_output_for_public_failure_evidence() {
    let root = tempfile::tempdir().unwrap();
    let binary = executable(
        root.path(),
        r#"#!/bin/sh
if [ "$1" = "--version" ]; then echo 'codex-cli fixture'; exit 0; fi
cat >/dev/null
echo '{"type":"item.completed","item":{"type":"error","message":"native startup diagnostic"}}'
"#,
    );
    let mut options = CompletionOptions::default();
    options.additional_json.extend([
        ("codex_binary".into(), json!(binary)),
        ("codex_home".into(), json!(root.path())),
    ]);
    let error = CodexClient::new("selected".into())
        .complete(
            vec![ChatMessage {
                role: MessageRole::User,
                content: "input".into(),
            }],
            options,
        )
        .await
        .unwrap_err();
    let ApiError::ProviderExecutionFailed { message, metadata } = error else {
        panic!("expected evidenced failure")
    };
    assert!(message.contains("native startup diagnostic"));
    assert!(metadata["codex"]["stdout"]
        .as_str()
        .unwrap()
        .contains("native startup diagnostic"));
    assert_eq!(metadata["codex"]["requested_model"], "selected");
}

#[test]
fn historical_completion_without_metadata_remains_readable() {
    let response: CompletionResponse = serde_json::from_value(json!({
        "content":"old", "model":"old", "usage":{"prompt_tokens":1,"completion_tokens":1,"total_tokens":2},"finish_reason":"stop"
    })).unwrap();
    assert!(response.execution_metadata.is_empty());
    assert!(serde_json::to_value(response)
        .unwrap()
        .get("execution_metadata")
        .is_none());
}

#[cfg(unix)]
fn executable(root: &Path, script: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = root.join("fake-codex");
    std::fs::write(&path, script).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    path
}

#[cfg(unix)]
#[tokio::test]
async fn subprocess_receives_exact_input_schema_and_isolated_context() {
    let root = tempfile::tempdir().unwrap();
    let binary = executable(
        root.path(),
        r#"#!/usr/bin/env python3
import json, os, pathlib, sys
if '--version' in sys.argv:
    print('codex-cli fixture')
    sys.exit(0)
assert '--ignore-user-config' in sys.argv and '--ephemeral' in sys.argv
assert pathlib.Path(os.environ['HOME']).parent == pathlib.Path.cwd()
assert 'T3CODE_HOME' not in os.environ
assert 'features.shell_tool=false' in sys.argv
schema=json.loads(pathlib.Path(sys.argv[sys.argv.index('--output-schema')+1]).read_text())
assert schema['required']==['accepted']
config=[sys.argv[i+1] for i,s in enumerate(sys.argv) if s=='--config']
instruction=json.loads(next(s.split('=',1)[1] for s in config if s.startswith('model_instructions_file=')))
assert pathlib.Path(instruction).read_text()=='Exact supplied standard.'
assert sys.stdin.read()=='Exact supplied evidence.'
print(json.dumps({'type':'item.completed','item':{'type':'agent_message','text':'{"accepted":false}'}}))
print(json.dumps({'type':'turn.completed','usage':{'input_tokens':12,'output_tokens':4}}))
print('fixture diagnostic',file=sys.stderr)
"#,
    );
    let mut options = CompletionOptions::default();
    options.additional_json.extend([
        ("codex_binary".into(), json!(binary)), ("codex_home".into(), json!(root.path())),
        ("response_format".into(), json!({"type":"json_schema","json_schema":{"schema":{"type":"object","properties":{"accepted":{"type":"boolean"}},"required":["accepted"],"additionalProperties":false}}})),
    ]);
    let result = CodexClient::new("selected-model".into())
        .complete(
            vec![
                ChatMessage {
                    role: MessageRole::System,
                    content: "Exact supplied standard.".into(),
                },
                ChatMessage {
                    role: MessageRole::User,
                    content: "Exact supplied evidence.".into(),
                },
            ],
            options,
        )
        .await
        .unwrap();
    assert_eq!(result.content, r#"{"accepted":false}"#);
    assert_eq!(
        result.execution_metadata["codex"]["version"],
        "codex-cli fixture"
    );
    assert_eq!(
        result.execution_metadata["codex"]["reasoning_effort"],
        "low"
    );
    assert_eq!(
        result.execution_metadata["codex"]["stderr"],
        "fixture diagnostic\n"
    );
    let args = result.execution_metadata["codex"]["argv"]
        .as_array()
        .unwrap();
    let instruction = args
        .iter()
        .filter_map(Value::as_str)
        .find(|arg| arg.starts_with("model_instructions_file="))
        .unwrap();
    let path: String = serde_json::from_str(instruction.split_once('=').unwrap().1).unwrap();
    assert!(
        !Path::new(&path).exists(),
        "temporary invocation artifacts are removed"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn nonzero_exit_and_oversized_output_cannot_be_completions() {
    let root = tempfile::tempdir().unwrap();
    let binary = executable(root.path(), "#!/bin/sh\necho native-failure >&2\nexit 7\n");
    let error = process::run(
        &binary,
        &[],
        root.path(),
        root.path(),
        "",
        Duration::from_secs(5),
    )
    .await
    .err()
    .unwrap()
    .to_string();
    assert!(error.contains("native-failure") && error.contains('7'));
    executable(
        root.path(),
        "#!/usr/bin/env python3\nprint('x' * (5 * 1024 * 1024))\n",
    );
    let error = process::run(
        &binary,
        &[],
        root.path(),
        root.path(),
        "",
        Duration::from_secs(5),
    )
    .await
    .err()
    .unwrap()
    .to_string();
    assert!(error.contains("capture limit"));
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn deadline_and_future_cancellation_kill_the_spawned_process() {
    for cancel in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let binary = executable(
            root.path(),
            "#!/bin/sh\necho $$ > child.pid\nexec sleep 30\n",
        );
        let deadline = if cancel {
            Duration::from_secs(10)
        } else {
            Duration::from_millis(100)
        };
        let task = process::run(&binary, &[], root.path(), root.path(), "", deadline);
        if cancel {
            assert!(tokio::time::timeout(Duration::from_millis(100), task)
                .await
                .is_err());
        } else {
            assert!(task.await.err().unwrap().to_string().contains("timed out"));
        }
        let pid = std::fs::read_to_string(root.path().join("child.pid")).unwrap();
        let proc_path = PathBuf::from(format!("/proc/{}", pid.trim()));
        for _ in 0..100 {
            if !proc_path.exists() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert!(!proc_path.exists(), "canceled Codex child must be reaped");
    }
}
