//! Passive runtime status resolution and presentation.

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::config::ConfigLoader;
use crate::error::ApiError;
use crate::runtime::assembly::{
    DesiredRuntimeState, ProductRuntimeAssembly, ProductRuntimeDescription, RuntimeFactoryRegistry,
};
use crate::runtime::contracts::{
    RuntimeActionRecord, RuntimeHandleKind, RuntimeImplementationState, RuntimeRoleClass,
    RuntimeStatusCacheLayout, RuntimeStatusCacheState, RuntimeStatusCacheWarning,
    RuntimeStatusHealthCounts, RuntimeStatusHealthSummary, RuntimeStatusInstanceSummary,
    RuntimeStatusLedgerSummary, RuntimeStatusProcessSummary, RuntimeStatusReadRequest,
    RuntimeStatusReadResult, RuntimeStatusReader, RuntimeStatusRuntimeRow,
    RuntimeStatusShutdownSummary, RUNTIME_STATUS_RECENT_ACTION_MAX_COUNT,
};
use crate::runtime::status_cache::FilesystemRuntimeStatusReader;

/// Passive runtime status assembled from configuration and cache files only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PassiveRuntimeStatusReport {
    /// Product root derived from passive configuration.
    pub product_root: PathBuf,
    /// Supervisor store path shown for operator context only.
    pub supervisor_store_path: PathBuf,
    /// Cache directory read by this command.
    pub status_cache_path: PathBuf,
    /// Missing, fresh, stale, unreadable, or unsupported cache state.
    pub cache_state: RuntimeStatusCacheState,
    /// Supervisor instance copied from a supported snapshot.
    pub instance: Option<RuntimeStatusInstanceSummary>,
    /// Process projection copied from a supported snapshot.
    pub process: Option<RuntimeStatusProcessSummary>,
    /// Shutdown projection copied from a supported snapshot.
    pub shutdown: Option<RuntimeStatusShutdownSummary>,
    /// Filtered runtime rows from cache or passive desired-state fallback.
    pub runtimes: Vec<RuntimeStatusRuntimeRow>,
    /// Health counts recomputed for the filtered runtime rows.
    pub health_counts: RuntimeStatusHealthCounts,
    /// Event ledger projection copied from a supported snapshot.
    pub ledger: Option<RuntimeStatusLedgerSummary>,
    /// Bounded newest action records returned by the cache reader.
    pub recent_actions: Vec<RuntimeActionRecord>,
    /// Writer and reader warnings.
    pub warnings: Vec<RuntimeStatusCacheWarning>,
}

/// Resolve runtime paths and desired state without opening runtime stores.
pub fn describe_for_workspace(
    workspace_root: &Path,
    config_path: Option<&Path>,
) -> Result<ProductRuntimeDescription, ApiError> {
    let config = match config_path {
        Some(path) => ConfigLoader::load_from_file(path)?,
        None => ConfigLoader::load(workspace_root)?,
    };
    ProductRuntimeAssembly::describe_for_workspace(workspace_root, &config)
        .map_err(passive_status_error)
}

/// Execute the early CLI status route without constructing `RunContext`.
pub fn handle_cli_status(
    workspace_root: &Path,
    config_path: Option<&Path>,
    format: &str,
    runtime_ids: &[String],
) -> Result<String, ApiError> {
    let description = describe_for_workspace(workspace_root, config_path)?;
    let reader = FilesystemRuntimeStatusReader::new(&description.product_root);
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(passive_status_error)?
        .as_millis()
        .try_into()
        .map_err(passive_status_error)?;
    let stale_after_ms = description.lifecycle_config.lease_duration_ms;
    read_and_format(
        description,
        &reader,
        RuntimeStatusReadRequest {
            now_ms,
            stale_after_ms,
            recent_action_limit: RUNTIME_STATUS_RECENT_ACTION_MAX_COUNT,
        },
        format,
        runtime_ids,
    )
}

/// Read and format passive runtime status through one injected cache reader.
pub fn read_and_format<R>(
    description: ProductRuntimeDescription,
    reader: &R,
    read_request: RuntimeStatusReadRequest,
    format: &str,
    runtime_ids: &[String],
) -> Result<String, ApiError>
where
    R: RuntimeStatusReader,
    R::Error: ToString,
{
    validate_format(format)?;
    let read = reader
        .read_status(read_request)
        .map_err(passive_status_error)?;
    let report = PassiveRuntimeStatusReport::from_read(description, read, runtime_ids)?;
    format_report(&report, format)
}

impl PassiveRuntimeStatusReport {
    /// Build one report from passive description and tolerant cache result.
    pub fn from_read(
        description: ProductRuntimeDescription,
        read: RuntimeStatusReadResult,
        runtime_ids: &[String],
    ) -> Result<Self, ApiError> {
        let cache_layout = RuntimeStatusCacheLayout::from_product_root(&description.product_root);
        let RuntimeStatusReadResult {
            latest,
            recent_actions,
            cache_state,
            mut warnings,
            ..
        } = read;
        let snapshot = latest.map(|record| record.snapshot);
        let mut rows = match snapshot.as_ref() {
            Some(snapshot) => snapshot.runtimes.clone(),
            None => description
                .desired_runtime_state
                .iter()
                .map(fallback_runtime_row)
                .collect(),
        };
        rows = filter_runtime_rows(rows, runtime_ids)?;
        let health_counts = health_counts(&rows);
        if let Some(snapshot) = snapshot.as_ref() {
            warnings.extend(snapshot.warnings.iter().cloned());
        }

        Ok(Self {
            product_root: description.product_root,
            supervisor_store_path: description.supervisor_store_path,
            status_cache_path: cache_layout.root,
            cache_state,
            instance: snapshot.as_ref().and_then(|value| value.instance.clone()),
            process: snapshot.as_ref().and_then(|value| value.process.clone()),
            shutdown: snapshot.as_ref().and_then(|value| value.shutdown.clone()),
            runtimes: rows,
            health_counts,
            ledger: snapshot.and_then(|value| value.ledger),
            recent_actions,
            warnings,
        })
    }
}

fn fallback_runtime_row(desired: &DesiredRuntimeState) -> RuntimeStatusRuntimeRow {
    let health = fallback_health(desired);
    RuntimeStatusRuntimeRow {
        runtime_id: desired.runtime_id.clone(),
        desired_enabled: desired.enabled,
        factory_available: desired.factory_available,
        role_class: desired.role_class,
        implementation_state: desired.implementation_state,
        handle_kind: match desired.implementation_state {
            RuntimeImplementationState::Concrete if desired.factory_available => {
                RuntimeHandleKind::Concrete
            }
            RuntimeImplementationState::Inert => RuntimeHandleKind::Inert,
            RuntimeImplementationState::Unknown => RuntimeHandleKind::Unknown,
            RuntimeImplementationState::Unavailable | RuntimeImplementationState::Concrete => {
                RuntimeHandleKind::Unavailable
            }
        },
        lease: None,
        heartbeat: None,
        health: RuntimeStatusHealthSummary {
            status: health.to_string(),
            retryable_error_count: 0,
            fatal_error_count: 0,
            budget_exhausted: false,
        },
        restart_count: 0,
        last_restart_cause: None,
        last_lifecycle_event: None,
        last_action: None,
        last_progress: None,
    }
}

fn fallback_health(desired: &DesiredRuntimeState) -> &'static str {
    if !desired.enabled {
        "stopped"
    } else if !desired.factory_available
        || matches!(
            desired.implementation_state,
            RuntimeImplementationState::Inert | RuntimeImplementationState::Unavailable
        )
    {
        "unhealthy"
    } else {
        "unknown"
    }
}

fn filter_runtime_rows(
    rows: Vec<RuntimeStatusRuntimeRow>,
    runtime_ids: &[String],
) -> Result<Vec<RuntimeStatusRuntimeRow>, ApiError> {
    if runtime_ids.is_empty() {
        return Ok(rows);
    }
    let registry = RuntimeFactoryRegistry::first_proof_registry().map_err(passive_status_error)?;
    let mut canonical_ids = runtime_ids
        .iter()
        .map(|runtime_id| {
            registry
                .canonical_id(runtime_id)
                .unwrap_or(runtime_id)
                .to_string()
        })
        .collect::<std::collections::BTreeSet<_>>();
    let filtered = rows
        .into_iter()
        .filter(|row| canonical_ids.remove(&row.runtime_id))
        .collect::<Vec<_>>();
    if let Some(unknown) = canonical_ids.into_iter().next() {
        return Err(passive_status_error(format!(
            "unknown runtime id filter '{unknown}'"
        )));
    }
    Ok(filtered)
}

fn health_counts(rows: &[RuntimeStatusRuntimeRow]) -> RuntimeStatusHealthCounts {
    let mut counts = RuntimeStatusHealthCounts {
        unknown: 0,
        starting: 0,
        healthy: 0,
        degraded: 0,
        unhealthy: 0,
        stopped: 0,
    };
    for row in rows {
        match row.health.status.as_str() {
            "starting" => counts.starting += 1,
            "healthy" => counts.healthy += 1,
            "degraded" => counts.degraded += 1,
            "unhealthy" => counts.unhealthy += 1,
            "stopped" => counts.stopped += 1,
            _ => counts.unknown += 1,
        }
    }
    counts
}

fn format_report(report: &PassiveRuntimeStatusReport, format: &str) -> Result<String, ApiError> {
    match format {
        "json" => serde_json::to_string_pretty(report).map_err(passive_status_error),
        "text" => {
            let mut lines = vec![
                "Runtime status cache".to_string(),
                format!("Product root: {}", report.product_root.display()),
                format!("Cache state: {:?}", report.cache_state),
                format!("Cache path: {}", report.status_cache_path.display()),
                match &report.instance {
                    Some(instance) => format!(
                        "Instance: {} status={}",
                        instance.instance_id, instance.status
                    ),
                    None => "Instance: <none>".to_string(),
                },
                String::new(),
                "Runtimes:".to_string(),
            ];
            for runtime in &report.runtimes {
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
                    runtime.health.status,
                    runtime
                        .lease
                        .as_ref()
                        .map(|lease| lease.lease_id.as_str())
                        .unwrap_or("<none>"),
                    runtime
                        .lease
                        .as_ref()
                        .and_then(|lease| lease.owner_instance_id.as_deref())
                        .unwrap_or("<none>")
                ));
            }
            for warning in &report.warnings {
                lines.push(format!("Warning {}: {}", warning.code, warning.message));
            }
            Ok(lines.join("\n"))
        }
        _ => unreachable!("format was validated"),
    }
}

fn validate_format(format: &str) -> Result<(), ApiError> {
    match format {
        "text" | "json" => Ok(()),
        other => Err(passive_status_error(format!(
            "invalid format '{other}', expected 'text' or 'json'"
        ))),
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

fn passive_status_error(error: impl ToString) -> ApiError {
    ApiError::ConfigError(format!("Runtime command failed: {}", error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::MerkleConfig;
    use crate::runtime::contracts::{
        RuntimeStatusActionsRead, RuntimeStatusSnapshotRead, RUNTIME_STATUS_CACHE_SCHEMA_VERSION,
    };

    struct FixedReader(RuntimeStatusSnapshotRead);

    impl RuntimeStatusReader for FixedReader {
        type Error = std::convert::Infallible;

        fn read_latest_snapshot(&self) -> Result<RuntimeStatusSnapshotRead, Self::Error> {
            Ok(self.0.clone())
        }

        fn read_recent_actions(
            &self,
            _limit: usize,
        ) -> Result<RuntimeStatusActionsRead, Self::Error> {
            Ok(RuntimeStatusActionsRead::default())
        }
    }

    fn description() -> ProductRuntimeDescription {
        ProductRuntimeAssembly::describe(
            crate::runtime::assembly::ProductRuntimeConfig::for_product_root("/tmp/product"),
        )
        .unwrap()
    }

    #[test]
    fn missing_cache_falls_back_to_truthful_desired_state() {
        let output = read_and_format(
            description(),
            &FixedReader(RuntimeStatusSnapshotRead::Missing),
            RuntimeStatusReadRequest {
                now_ms: 10,
                stale_after_ms: 5,
                recent_action_limit: 10,
            },
            "json",
            &[],
        )
        .unwrap();
        let value: serde_json::Value = serde_json::from_str(&output).unwrap();

        assert_eq!(value["cache_state"], "missing");
        assert_eq!(value["runtimes"].as_array().unwrap().len(), 12);
        let graph = value["runtimes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["runtime_id"] == "world_model.graph_replay")
            .unwrap();
        assert_eq!(graph["desired_enabled"], true);
        assert_eq!(graph["health"]["status"], "unknown");
    }

    #[test]
    fn unreadable_cache_preserves_state_and_alias_filter() {
        let output = read_and_format(
            description(),
            &FixedReader(RuntimeStatusSnapshotRead::Unreadable {
                warnings: vec![RuntimeStatusCacheWarning {
                    code: "unreadable".to_string(),
                    message: "partial snapshot".to_string(),
                }],
            }),
            RuntimeStatusReadRequest {
                now_ms: 10,
                stale_after_ms: 5,
                recent_action_limit: 10,
            },
            "text",
            &["world_model.graph.replay".to_string()],
        )
        .unwrap();

        assert!(output.contains("Cache state: Unreadable"));
        assert!(output.contains("world_model.graph_replay enabled"));
        assert!(output.contains("Warning unreadable"));
        assert!(!output.contains("execution.task_dispatch"));
    }

    #[test]
    fn passive_description_does_not_create_product_paths() {
        let temp = tempfile::tempdir().unwrap();
        let workspace = temp.path().join("workspace");
        std::fs::create_dir_all(&workspace).unwrap();
        let mut config = MerkleConfig::default();
        config.system.storage.product_root = Some(PathBuf::from(".meld-runtime"));

        let description =
            ProductRuntimeAssembly::describe_for_workspace(&workspace, &config).unwrap();

        assert!(!description.product_root.exists());
        assert_eq!(
            RuntimeStatusCacheLayout::from_product_root(&description.product_root).root,
            description.product_root.join("runtime/status")
        );
    }

    #[test]
    fn schema_version_constant_remains_available_to_passive_route() {
        assert_eq!(RUNTIME_STATUS_CACHE_SCHEMA_VERSION, 1);
    }
}
