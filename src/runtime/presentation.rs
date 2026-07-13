//! Runtime CLI presentation.

use crate::error::ApiError;
use crate::runtime::activation::PassiveActivationDescription;
use crate::runtime::contracts::{RuntimeImplementationState, RuntimeRoleClass};
use crate::runtime::supervisor::{RuntimeHealthStatus, RuntimeInstanceStatus};
use crate::runtime::tooling::{RuntimeCliRunResult, RuntimeCliStatus};

/// Format a passively validated product activation for CLI output.
pub fn format_runtime_activation_description(
    description: &PassiveActivationDescription,
    format: &str,
) -> Result<String, ApiError> {
    match format {
        "json" => serde_json::to_string_pretty(description).map_err(|error| {
            ApiError::ConfigError(format!(
                "Runtime activation failed: failed to encode JSON: {error}"
            ))
        }),
        "text" => Ok([
            if description.validation_scope == "source_owner_packages_and_execution_assets" {
                "Activation source, owner packages, and execution assets validated".to_string()
            } else {
                "Activation source and owner packages validated".to_string()
            },
            format!("Activation: {}", description.activation_id),
            format!("Hash: {}", description.activation_hash.as_str()),
            format!("Source: {}", description.source_path.display()),
            format!("Source bytes: {}", description.source_bytes),
            format!(
                "Workspace: {}",
                description.canonical_workspace_root.display()
            ),
            format!("Target: {}", description.resolved_target.display()),
            format!(
                "Enabled runtimes: {}",
                description.enabled_runtime_ids.join(", ")
            ),
            format!("Validation scope: {}", description.validation_scope),
            format!(
                "Application ready: {}",
                if description.application_ready {
                    "yes"
                } else {
                    "no"
                }
            ),
        ]
        .join("\n")),
        other => Err(ApiError::ConfigError(format!(
            "Runtime activation failed: invalid format '{other}', expected 'text' or 'json'"
        ))),
    }
}

/// Format runtime status for CLI output.
pub fn format_runtime_status(status: &RuntimeCliStatus, format: &str) -> Result<String, ApiError> {
    match format {
        "json" => serde_json::to_string_pretty(status).map_err(|err| {
            ApiError::ConfigError(format!(
                "Runtime command failed: failed to encode JSON: {err}"
            ))
        }),
        "text" => Ok(format_runtime_status_text(status)),
        other => Err(ApiError::ConfigError(format!(
            "Runtime command failed: invalid format '{other}', expected 'text' or 'json'"
        ))),
    }
}

/// Format a completed foreground runtime run for CLI output.
pub fn format_runtime_run_result(
    result: &RuntimeCliRunResult,
    format: &str,
) -> Result<String, ApiError> {
    match format {
        "json" => serde_json::to_string_pretty(result).map_err(|err| {
            ApiError::ConfigError(format!(
                "Runtime command failed: failed to encode JSON: {err}"
            ))
        }),
        "text" => Ok(format_runtime_run_result_text(result)),
        other => Err(ApiError::ConfigError(format!(
            "Runtime command failed: invalid format '{other}', expected 'text' or 'json'"
        ))),
    }
}

fn format_runtime_status_text(status: &RuntimeCliStatus) -> String {
    let mut lines = vec![
        "Runtime supervisor".to_string(),
        format!("Product root: {}", status.product_root.display()),
        format!(
            "Supervisor store: {}",
            status.supervisor_store_path.display()
        ),
        match &status.instance {
            Some(instance) => format!(
                "Instance: {} status={} started={} stopped={}",
                instance.instance_id,
                instance_status(instance.status),
                instance.started_at_ms,
                instance
                    .stopped_at_ms
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "<none>".to_string())
            ),
            None => "Instance: <none>".to_string(),
        },
        String::new(),
        "Runtimes:".to_string(),
    ];

    for runtime in &status.runtimes {
        lines.push(format!(
            "- {} {} role={} implementation={} factory={} health={} lease={} owner={}",
            runtime.runtime_id,
            if runtime.desired_enabled {
                "enabled"
            } else {
                "disabled"
            },
            role_class(runtime.role_class),
            implementation_state(runtime.implementation_state),
            if runtime.factory_available {
                "available"
            } else {
                "unavailable"
            },
            health_status(runtime.health_status),
            runtime.active_lease_id.as_deref().unwrap_or("<none>"),
            runtime
                .active_owner_instance_id
                .as_deref()
                .unwrap_or("<none>")
        ));
    }

    lines.join("\n")
}

fn format_runtime_run_result_text(result: &RuntimeCliRunResult) -> String {
    [
        "Runtime supervisor stopped".to_string(),
        format!("Instance: {}", result.instance_id),
        format!("Product root: {}", result.product_root.display()),
        format!("Started runtimes: {}", result.started_runtime_count),
        format!("Stopped runtimes: {}", result.stopped_runtime_count),
        format!("Ticks: {}", result.tick_count),
        format!("Shutdown: {}", result.shutdown_id),
    ]
    .join("\n")
}

fn instance_status(status: RuntimeInstanceStatus) -> &'static str {
    match status {
        RuntimeInstanceStatus::Starting => "starting",
        RuntimeInstanceStatus::Running => "running",
        RuntimeInstanceStatus::Stopping => "stopping",
        RuntimeInstanceStatus::Stopped => "stopped",
        RuntimeInstanceStatus::Stale => "stale",
        RuntimeInstanceStatus::Failed => "failed",
    }
}

fn health_status(status: RuntimeHealthStatus) -> &'static str {
    match status {
        RuntimeHealthStatus::Unknown => "unknown",
        RuntimeHealthStatus::Starting => "starting",
        RuntimeHealthStatus::Healthy => "healthy",
        RuntimeHealthStatus::Degraded => "degraded",
        RuntimeHealthStatus::Unhealthy => "unhealthy",
        RuntimeHealthStatus::Stopped => "stopped",
    }
}

fn role_class(role: RuntimeRoleClass) -> &'static str {
    match role {
        RuntimeRoleClass::Unknown => "unknown",
        RuntimeRoleClass::Actor => "actor",
        RuntimeRoleClass::PassiveService => "passive-service",
        RuntimeRoleClass::PortOnly => "port-only",
    }
}

fn implementation_state(state: RuntimeImplementationState) -> &'static str {
    match state {
        RuntimeImplementationState::Unknown => "unknown",
        RuntimeImplementationState::Concrete => "concrete",
        RuntimeImplementationState::Inert => "inert",
        RuntimeImplementationState::Unavailable => "unavailable",
    }
}
