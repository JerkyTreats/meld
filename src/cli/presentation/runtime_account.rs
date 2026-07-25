//! Foreground runtime tick-account rendering.
//!
//! Owner: CLI presentation. Renders the structured per-tick account the
//! foreground runtime emits for every maintenance pass — one line per pass,
//! text by default and one JSON object per line under `--format json`.
//! Rendering is pure: it derives entirely from the account DTO, which the
//! runtime tooling derives from durable per-tick reports. Presentation
//! never becomes semantic truth and never gates runtime work.

use crate::runtime::tooling::{RuntimeTickAccount, RuntimeTickActorAccount};

/// Render one tick account as a single machine-readable JSON line.
pub fn render_runtime_tick_account_json(account: &RuntimeTickAccount) -> String {
    serde_json::to_string(account).unwrap_or_else(|error| {
        format!(
            r#"{{"type":"runtime_tick_account_error","tick":{},"error":"{}"}}"#,
            account.tick, error
        )
    })
}

/// Render one tick account as a single human-readable text line.
///
/// A quiescent pass renders `pass=quiescent`, active work renders
/// `pass=active` with per-actor segments, so working, idle, and dead
/// (no line at all) states stay distinguishable from the account alone.
pub fn render_runtime_tick_account_text(account: &RuntimeTickAccount) -> String {
    let mut line = format!(
        "tick={} at_ms={} instance={} pass={}",
        account.tick,
        account.at_ms,
        account.instance_id,
        if account.quiescent {
            "quiescent"
        } else {
            "active"
        }
    );
    for actor in &account.actors {
        line.push_str(" | ");
        line.push_str(&render_actor_segment(actor));
    }
    line
}

fn render_actor_segment(actor: &RuntimeTickActorAccount) -> String {
    let checkpoint = actor
        .checkpoint
        .as_ref()
        .map(|checkpoint| {
            format!(
                " checkpoint={}:{}->{}",
                checkpoint.name, checkpoint.input, checkpoint.output
            )
        })
        .unwrap_or_default();
    let issues = if actor.issues.is_empty() {
        format!(" issues={}", actor.issues.len())
    } else {
        format!(
            " issues={} [{}]",
            actor.issues.len(),
            actor
                .issues
                .iter()
                .map(|issue| format!("{}:{}", issue.severity, issue.code))
                .collect::<Vec<_>>()
                .join(",")
        )
    };
    format!(
        "actor={} lifecycle={} outcome={} attempted={} committed={}{}{}{}",
        actor.runtime_id,
        actor.lifecycle,
        actor.outcome,
        actor.items_attempted,
        actor.items_committed,
        checkpoint,
        issues,
        if actor.budget_exhausted {
            " budget_exhausted=true"
        } else {
            ""
        }
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::tooling::{RuntimeTickCheckpointAccount, RuntimeTickIssueAccount};

    fn account(quiescent: bool, actors: Vec<RuntimeTickActorAccount>) -> RuntimeTickAccount {
        RuntimeTickAccount {
            kind: "runtime_tick_account".to_string(),
            tick: 3,
            at_ms: 1_000,
            instance_id: "instance-a".to_string(),
            quiescent,
            actors,
        }
    }

    fn working_actor() -> RuntimeTickActorAccount {
        RuntimeTickActorAccount {
            runtime_id: "world_model.graph_replay".to_string(),
            lifecycle: "active_working".to_string(),
            outcome: "succeeded".to_string(),
            items_attempted: 3,
            items_committed: 3,
            checkpoint: Some(RuntimeTickCheckpointAccount {
                name: "event_commit_watermark".to_string(),
                input: 0,
                output: 3,
            }),
            issues: vec![RuntimeTickIssueAccount {
                severity: "retryable".to_string(),
                code: "slow_reduce".to_string(),
                message: "reduction lagged".to_string(),
            }],
            budget_exhausted: false,
        }
    }

    #[test]
    fn text_line_distinguishes_quiescent_from_active_passes() {
        let quiescent = render_runtime_tick_account_text(&account(true, Vec::new()));
        let active = render_runtime_tick_account_text(&account(false, vec![working_actor()]));

        assert!(quiescent.contains("pass=quiescent"));
        assert!(active.contains("pass=active"));
        assert!(active.contains("actor=world_model.graph_replay"));
        assert!(active.contains("lifecycle=active_working"));
        assert!(active.contains("checkpoint=event_commit_watermark:0->3"));
        assert!(active.contains("issues=1 [retryable:slow_reduce]"));
    }

    #[test]
    fn json_line_is_one_valid_object_per_pass() {
        let line = render_runtime_tick_account_json(&account(false, vec![working_actor()]));

        let parsed: serde_json::Value = serde_json::from_str(&line).unwrap();
        assert_eq!(parsed["type"], "runtime_tick_account");
        assert_eq!(parsed["tick"], 3);
        assert_eq!(parsed["quiescent"], false);
        assert_eq!(
            parsed["actors"][0]["runtime_id"],
            "world_model.graph_replay"
        );
        assert_eq!(parsed["actors"][0]["checkpoint"]["output"], 3);
        assert!(!line.contains('\n'));
    }
}
