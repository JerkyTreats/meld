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
        } | RuntimeCommands::List { .. }
            | RuntimeCommands::Shutdown { .. }
            | RuntimeCommands::Stop { .. }
            | RuntimeCommands::Restart { .. }
            | RuntimeCommands::Follow { .. }
            | RuntimeCommands::Actions { .. }
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
            RuntimeCommands::List { json } => {
                let live = discover_live(&target)?;
                let instances = if let Some((url, _)) = &live {
                    ureq::get(&format!("{url}/v1/runtime/instances"))
                        .timeout(Duration::from_secs(2))
                        .call()
                        .map_err(error)?
                        .into_json::<Vec<super::supervisor::RuntimeInstance>>()
                        .map_err(error)?
                } else if target.supervisor_store_path.exists() {
                    SupervisorStore::open(&target.supervisor_store_path)
                        .map_err(error)?
                        .list_runtime_instances()
                        .map_err(error)?
                } else {
                    Vec::new()
                };
                let value = serde_json::json!({"scope": "selected assignment", "product_root": target.product_root,
                    "live_verified": live.is_some(), "live_instance_id": live.as_ref().map(|(_, status)| status.instance.instance_id.clone()), "coverage": "retained instances in the selected assignment", "instances": instances});
                render(&value, *json, || {
                    if instances.is_empty() {
                        return "Selected assignment has not started.".into();
                    }
                    instances
                        .iter()
                        .map(|instance| {
                            format!(
                                "{} {:?} {}",
                                instance.instance_id,
                                instance.status,
                                if live
                                    .as_ref()
                                    .is_some_and(|(_, status)| status.instance.instance_id
                                        == instance.instance_id)
                                {
                                    "live verified"
                                } else {
                                    "retained"
                                }
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n")
                })
            }
            RuntimeCommands::Shutdown { instance, json } => {
                let shutdown = if let Some((url, _)) = discover_live(&target)? {
                    ureq::post(&format!("{url}/v1/runtime/shutdown"))
                        .timeout(Duration::from_secs(2))
                        .send_json(instance)
                        .map_err(error)?
                        .into_json::<Option<super::supervisor::RuntimeShutdownState>>()
                        .map_err(error)?
                } else if target.supervisor_store_path.exists() {
                    let store =
                        SupervisorStore::open(&target.supervisor_store_path).map_err(error)?;
                    let record = match instance {
                        Some(id) => store.get_runtime_instance(id),
                        None => store.latest_runtime_instance(),
                    }
                    .map_err(error)?
                    .ok_or_else(|| error("runtime instance not found"))?;
                    store
                        .get_shutdown_state(&shutdown_id(&record))
                        .map_err(error)?
                } else {
                    None
                };
                render(&shutdown, *json, || match &shutdown {
                    Some(record) => format!("{} {:?}", record.shutdown_id, record.status),
                    None => "No native shutdown record; no stop was requested by this read.".into(),
                })
            }
            RuntimeCommands::Start { tick_ms, json, .. } => start(cli, &target, *tick_ms, *json),
            RuntimeCommands::Stop { instance, json } => stop(&target, instance.as_deref(), *json),
            RuntimeCommands::Restart { tick_ms, json } => {
                stop(&target, None, true)?;
                start(cli, &target, *tick_ms, *json)
            }
            RuntimeCommands::Follow {
                instance,
                after,
                json,
            } => follow(&target, instance.as_deref(), *after, *json),
            RuntimeCommands::Actions { after, limit, json } => {
                let page = if let Some((url, _)) = discover_live(&target)? {
                    action_page(&url, *after, *limit)?
                } else {
                    if !target.supervisor_store_path.exists() {
                        return Err(error("no retained runtime reports"));
                    }
                    let store =
                        SupervisorStore::open(&target.supervisor_store_path).map_err(error)?;
                    super::supervisor::SupervisorReportStore::open(&store)
                        .map_err(error)?
                        .read_action_page(0, *after, *limit)
                        .map_err(error)?
                };
                render(&page, *json, || format_actions(&page))
            }
            _ => unreachable!(),
        }
    })())
}

/// An advertisement is only a route hint. The live owner must confirm its root
/// and process identity before callers issue an exact-instance operation.
pub fn discover_live(
    target: &ProductRuntimeDescription,
) -> Result<Option<(String, ControlStatus)>, ApiError> {
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
    if let Some((_, status)) = discover_live(target)? {
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
        if let Some((_, status)) = discover_live(target)? {
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
    let current = discover_live(target)?;
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

fn action_page(
    url: &str,
    after: Option<u64>,
    limit: usize,
) -> Result<super::supervisor::reports::ActionPage, ApiError> {
    ureq::post(&format!("{url}/v1/reports/actions"))
        .timeout(Duration::from_secs(2))
        .send_json(&crate::serve::routes::ActionPageRequest { after, limit })
        .map_err(error)?
        .into_json()
        .map_err(error)
}

fn format_actions(page: &super::supervisor::reports::ActionPage) -> String {
    let mut lines = Vec::new();
    if page.history_gap {
        lines.push("Earlier reports are no longer retained; observation has a history gap.".into());
    }
    for row in &page.actions {
        let action = &row.action;
        lines.push(format!(
            "{} {} {:?}: {} committed",
            row.sequence, action.runtime_id, action.outcome, action.metrics.committed
        ));
        for issue in &action.issues {
            lines.push(format!("  {}: {}", issue.code, issue.message));
        }
        for wait in &action.waiting_on {
            lines.push(format!("  waiting: {}", wait.condition));
        }
    }
    if page.more {
        lines.push(format!(
            "More reports: --after {}",
            page.next_after.unwrap_or_default()
        ));
    }
    lines.join("\n")
}

fn follow(
    target: &ProductRuntimeDescription,
    expected: Option<&str>,
    mut after: Option<u64>,
    json: bool,
) -> Result<String, ApiError> {
    let (url, status) = discover_live(target)?.ok_or_else(|| error("no live runtime to follow"))?;
    if expected.is_some_and(|id| id != status.instance.instance_id) {
        return Err(error("selected instance is not live"));
    }
    loop {
        let Some((_, current)) = discover_live(target)? else {
            return Ok(String::new());
        };
        if current.instance.instance_id != status.instance.instance_id {
            return Ok(String::new());
        }
        let page = action_page(&url, after, 100)?;
        if !page.actions.is_empty() || page.history_gap {
            if json {
                println!("{}", serde_json::to_string(&page).map_err(error)?);
            } else {
                println!("{}", format_actions(&page));
            }
            std::io::stdout().flush().map_err(error)?;
            after = page.next_after;
        }
        if !page.more {
            std::thread::sleep(Duration::from_millis(250));
        }
    }
}
