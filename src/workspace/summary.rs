use serde_json::json;

use crate::telemetry::summary::TypedSummaryEvent;

pub fn status(
    format: &str,
    breakdown: bool,
    ok: bool,
    duration_ms: u128,
    error: Option<&str>,
) -> TypedSummaryEvent {
    TypedSummaryEvent::new(
        "status_summary",
        json!({
            "scope": "workspace",
            "format": format,
            "breakdown": breakdown,
            "ok": ok,
            "duration_ms": duration_ms,
            "error": error,
        }),
    )
}

pub fn validate(
    format: &str,
    ok: bool,
    duration_ms: u128,
    error: Option<&str>,
) -> TypedSummaryEvent {
    TypedSummaryEvent::new(
        "validate_summary",
        json!({
            "scope": "workspace",
            "format": format,
            "ok": ok,
            "duration_ms": duration_ms,
            "error": error,
        }),
    )
}

pub fn delete(
    target_path: bool,
    target_node: bool,
    dry_run: bool,
    no_ignore: bool,
    ok: bool,
    duration_ms: u128,
    error: Option<&str>,
) -> TypedSummaryEvent {
    TypedSummaryEvent::new(
        "workspace_mutation_summary",
        json!({
            "operation": "delete",
            "target": summary_target(target_path, target_node),
            "dry_run": dry_run,
            "no_ignore": no_ignore,
            "ok": ok,
            "duration_ms": duration_ms,
            "error": error,
        }),
    )
}

pub fn restore(
    target_path: bool,
    target_node: bool,
    dry_run: bool,
    ok: bool,
    duration_ms: u128,
    error: Option<&str>,
) -> TypedSummaryEvent {
    TypedSummaryEvent::new(
        "workspace_mutation_summary",
        json!({
            "operation": "restore",
            "target": summary_target(target_path, target_node),
            "dry_run": dry_run,
            "ok": ok,
            "duration_ms": duration_ms,
            "error": error,
        }),
    )
}

pub fn compact(
    ttl_days: Option<u64>,
    all: bool,
    keep_frames: bool,
    dry_run: bool,
    ok: bool,
    duration_ms: u128,
    error: Option<&str>,
) -> TypedSummaryEvent {
    TypedSummaryEvent::new(
        "workspace_maintenance_summary",
        json!({
            "operation": "compact",
            "ttl_days": ttl_days,
            "all": all,
            "keep_frames": keep_frames,
            "dry_run": dry_run,
            "ok": ok,
            "duration_ms": duration_ms,
            "error": error,
        }),
    )
}

pub fn list_deleted(
    older_than_days: Option<u64>,
    format: &str,
    ok: bool,
    duration_ms: u128,
    error: Option<&str>,
) -> TypedSummaryEvent {
    TypedSummaryEvent::new(
        "list_summary",
        json!({
            "scope": "workspace_deleted",
            "older_than_days": older_than_days,
            "format": format,
            "ok": ok,
            "duration_ms": duration_ms,
            "error": error,
        }),
    )
}

pub fn ignore(
    has_path: bool,
    dry_run: bool,
    format: &str,
    ok: bool,
    duration_ms: u128,
    error: Option<&str>,
) -> TypedSummaryEvent {
    TypedSummaryEvent::new(
        "config_mutation_summary",
        json!({
            "scope": "workspace_ignore",
            "action": if has_path { "add" } else { "list" },
            "dry_run": dry_run,
            "format": format,
            "ok": ok,
            "duration_ms": duration_ms,
            "error": error,
        }),
    )
}

pub fn unified_status(
    format: &str,
    include_workspace: bool,
    include_agents: bool,
    include_providers: bool,
    breakdown: bool,
    test_connectivity: bool,
    ok: bool,
    duration_ms: u128,
    error: Option<&str>,
) -> TypedSummaryEvent {
    TypedSummaryEvent::new(
        "status_summary",
        json!({
            "scope": "unified",
            "format": format,
            "include_workspace": include_workspace,
            "include_agents": include_agents,
            "include_providers": include_providers,
            "breakdown": breakdown,
            "test_connectivity": test_connectivity,
            "ok": ok,
            "duration_ms": duration_ms,
            "error": error,
        }),
    )
}

pub fn validate_workspace(ok: bool, duration_ms: u128, error: Option<&str>) -> TypedSummaryEvent {
    validate("text", ok, duration_ms, error)
}

fn summary_target(target_path: bool, target_node: bool) -> &'static str {
    if target_path {
        "path"
    } else if target_node {
        "node"
    } else {
        "unknown"
    }
}
