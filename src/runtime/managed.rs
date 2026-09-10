//! Process launch and public control transport. Native activation, work and drain
//! stay in the foreground supervisor; this module never opens an assembly.

use std::fs::OpenOptions;
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use super::assembly::{ProductRuntimeAssembly, ProductRuntimeDescription};
use super::control::{shutdown_id, ControlStatus, StopRequest};
use super::supervisor::{RuntimeInstanceStatus, RuntimeShutdownStatus, SupervisorStore};
use crate::cli::{Cli, Commands, RunContext, RuntimeCommands};
use crate::error::ApiError;
use serde::Serialize;

const DEADLINE: Duration = Duration::from_secs(15);

fn error(message: impl ToString) -> ApiError {
    ApiError::ConfigError(message.to_string())
}

pub fn execute(cli: &Cli) -> Option<Result<String, ApiError>> {
    let Commands::Runtime { command } = &cli.command else {
        return None;
    };
    if !matches!(
        command,
        RuntimeCommands::Start {
            foreground: false,
            ..
        } | RuntimeCommands::Stop { .. }
            | RuntimeCommands::Restart { .. }
            | RuntimeCommands::Follow { .. }
    ) {
        return None;
    }
    Some((|| {
        let config = RunContext::selected_config(
            &cli.workspace,
            cli.config.as_deref(),
            cli.assignment.as_deref(),
        )?;
        let target = ProductRuntimeAssembly::describe_for_workspace(&cli.workspace, &config)
            .map_err(error)?;
        match command {
            RuntimeCommands::Start { tick_ms, json, .. } => start(cli, &target, *tick_ms, *json),
            RuntimeCommands::Stop { instance, json } => stop(&target, instance.as_deref(), *json),
            RuntimeCommands::Restart { tick_ms, json } => {
                stop(&target, None, true)?;
                start(cli, &target, *tick_ms, *json)
            }
            RuntimeCommands::Follow { instance, json } => {
                follow(&target, instance.as_deref(), *json)
            }
            _ => unreachable!(),
        }
    })())
}

/// An advertisement is only a route hint. The live owner must confirm its root
/// and process identity before callers issue an exact-instance operation.
fn live(target: &ProductRuntimeDescription) -> Result<Option<(String, ControlStatus)>, ApiError> {
    let root = &target.product_root;
    let Some(discovery) = crate::serve::discovery::read(root) else {
        return Ok(None);
    };
    let addr: std::net::SocketAddr = discovery.addr.parse().map_err(error)?;
    if !addr.ip().is_loopback() {
        return Err(error("runtime discovery is not a loopback address"));
    }
    let url = format!("http://{addr}");
    let response = match ureq::get(&format!("{url}/v1/runtime/status"))
        .timeout(Duration::from_secs(2))
        .call()
    {
        Ok(response) => response,
        Err(ureq::Error::Transport(_)) => return Ok(None),
        Err(e) => {
            return Err(error(format!(
                "advertised runtime control is unavailable: {e}"
            )))
        }
    };
    let status: ControlStatus = response.into_json().map_err(error)?;
    if status.product_root != *root
        || status.instance.product_root != *root
        || status.process_id != discovery.process_id
    {
        return Err(error("runtime discovery does not match the live owner"));
    }
    if status.supervisor_store_path != target.supervisor_store_path {
        return Err(error("live runtime belongs to another assignment in this product; select its assignment explicitly"));
    }
    Ok(Some((url, status)))
}

fn render(
    value: &impl Serialize,
    json: bool,
    text: impl FnOnce() -> String,
) -> Result<String, ApiError> {
    if json {
        serde_json::to_string_pretty(value).map_err(error)
    } else {
        Ok(text())
    }
}

fn start(
    cli: &Cli,
    target: &ProductRuntimeDescription,
    tick_ms: u64,
    json: bool,
) -> Result<String, ApiError> {
    if tick_ms == 0 {
        return Err(error("tick-ms must be greater than 0"));
    }
    if let Some((_, status)) = live(target)? {
        if status.instance.status != RuntimeInstanceStatus::Running || status.stop_received {
            return Err(error(
                "the current runtime is draining; wait for its shutdown before starting",
            ));
        }
        return render(&status, json, || {
            format!("Already running: {}", status.instance.instance_id)
        });
    }
    if !target.supervisor_store_path.exists() {
        return Err(error("runtime is not prepared; run meld init first"));
    }
    let instance = format!("runtime-{}", uuid::Uuid::new_v4());
    let logs = target.product_root.join("process-logs");
    std::fs::create_dir_all(&logs).map_err(error)?;
    let stdout_path = logs.join(format!("{instance}.jsonl"));
    let stderr_path = logs.join(format!("{instance}.stderr"));
    let stdout = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&stdout_path)
        .map_err(error)?;
    let stderr = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&stderr_path)
        .map_err(error)?;
    let mut child_command = Command::new(std::env::current_exe().map_err(error)?);
    child_command.arg("--workspace").arg(&cli.workspace);
    if let Some(path) = &cli.config {
        child_command.arg("--config").arg(path);
    }
    if let Some(id) = &cli.assignment {
        child_command.arg("--assignment").arg(id);
    }
    for id in &cli.enable_runtime {
        child_command.arg("--enable-runtime").arg(id);
    }
    child_command
        .args([
            "runtime",
            "run",
            "--instance-id",
            &instance,
            "--tick-ms",
            &tick_ms.to_string(),
            "--format",
            "json",
        ])
        .stdin(Stdio::null())
        .stdout(stdout)
        .stderr(stderr);
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        child_command.process_group(0);
    }
    let mut child = child_command.spawn().map_err(error)?;
    let deadline = Instant::now() + DEADLINE;
    while Instant::now() < deadline {
        if let Some(exit) = child.try_wait().map_err(error)? {
            return Err(error(format!(
                "runtime failed before readiness: {exit}; inspect {}",
                stderr_path.display()
            )));
        }
        if let Some((_, status)) = live(target)? {
            if status.instance.instance_id == instance
                && status.process_id == child.id()
                && status.instance.status == RuntimeInstanceStatus::Running
                && !status.stop_received
            {
                return render(&status, json, || {
                    format!(
                        "Started {}\nFollow: meld runtime follow\nStop: meld runtime stop",
                        instance
                    )
                });
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    Err(error(format!("readiness unresolved for {instance}, pid {}; no successful start claimed; inspect {} and use runtime stop --instance {instance} when control is available", child.id(), stderr_path.display())))
}

fn stop(
    target: &ProductRuntimeDescription,
    expected: Option<&str>,
    json: bool,
) -> Result<String, ApiError> {
    let current = live(target)?;
    let instance = if let Some((url, status)) = current {
        if expected.is_some_and(|id| id != status.instance.instance_id) {
            return Err(error(
                "selected instance is not the live instance; stop was not sent",
            ));
        }
        let request = StopRequest {
            product_root: target.product_root.clone(),
            instance_id: status.instance.instance_id.clone(),
        };
        ureq::post(&format!("{url}/v1/runtime/stop"))
            .timeout(Duration::from_secs(2))
            .send_json(&request)
            .map_err(|e| {
                error(format!(
                    "stop outcome unknown for {}; inspect that exact instance before retrying: {e}",
                    request.instance_id
                ))
            })?;
        status.instance
    } else {
        if !target.supervisor_store_path.exists() {
            return Err(error("no prepared runtime instance"));
        }
        let store = SupervisorStore::open(&target.supervisor_store_path).map_err(|e| {
            error(format!(
                "runtime control unavailable and store ownership unresolved: {e}"
            ))
        })?;
        match expected {
            Some(id) => store.get_runtime_instance(id).map_err(error)?,
            None => store.latest_runtime_instance().map_err(error)?,
        }
        .ok_or_else(|| error("no runtime instance to stop"))?
    };
    let key = shutdown_id(&instance);
    let deadline = Instant::now() + DEADLINE;
    while Instant::now() < deadline {
        // A successful exclusive store open proves the foreground owner has
        // released its stores. Retained completion alone is insufficient for restart.
        if let Ok(store) = SupervisorStore::open(&target.supervisor_store_path) {
            if let Some(shutdown) = store.get_shutdown_state(&key).map_err(error)? {
                match shutdown.status {
                    RuntimeShutdownStatus::Completed => {
                        return render(&shutdown, json, || {
                            format!("Stopped {}\nShutdown: {}", instance.instance_id, key)
                        })
                    }
                    RuntimeShutdownStatus::Failed | RuntimeShutdownStatus::TimedOut => {
                        return Err(error(format!(
                            "shutdown {key} ended {:?}; successor start refused",
                            shutdown.status
                        )))
                    }
                    _ => {}
                }
            } else {
                return Err(error(format!(
                    "instance {} has no completed native shutdown; its outcome is unresolved",
                    instance.instance_id
                )));
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    Err(error(format!(
        "shutdown {key} remains pending or unknown; successor start refused"
    )))
}

fn follow(
    target: &ProductRuntimeDescription,
    expected: Option<&str>,
    json: bool,
) -> Result<String, ApiError> {
    let (url, status) = live(target)?.ok_or_else(|| error("no live runtime to follow"))?;
    if expected.is_some_and(|id| id != status.instance.instance_id) {
        return Err(error("selected instance is not live"));
    }
    let mut last = String::new();
    loop {
        let Some((_, current)) = live(target)? else {
            return Ok("Runtime observation ended".into());
        };
        if current.instance.instance_id != status.instance.instance_id {
            return Ok("Selected instance ended; successor was not followed".into());
        }
        let reports: serde_json::Value = ureq::post(&format!("{url}/v1/reports/recent_actions"))
            .timeout(Duration::from_secs(2))
            .send_json(serde_json::json!({"limit": 64}))
            .map_err(error)?
            .into_json()
            .map_err(error)?;
        let encoded = serde_json::to_string(&reports).map_err(error)?;
        if encoded != last {
            if json {
                println!("{encoded}");
            } else {
                println!(
                    "{}\n{}",
                    current.instance.instance_id,
                    serde_json::to_string_pretty(&reports).map_err(error)?
                );
            }
            std::io::stdout().flush().map_err(error)?;
            last = encoded;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}
