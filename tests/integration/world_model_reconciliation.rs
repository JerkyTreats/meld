use meld::cli::{BranchesCommands, Commands, RunContext};
use meld::workspace::events::source_ref;
use meld::world_state::graph::contracts::OWNER_PUBLICATION_EVENT_TYPE;
use meld_events::{LedgerCursor, ReplayRequest, MAX_REPLAY_LIMIT};
use serde_json::Value;
use tempfile::TempDir;

use crate::integration::with_xdg_env;

#[test]
fn real_scan_retries_one_owner_operation_and_returns_through_branch_owner_walk() {
    let temp = TempDir::new().unwrap();
    with_xdg_env(&temp, || {
        let workspace = temp.path().join("workspace");
        std::fs::create_dir_all(workspace.join("docs")).unwrap();
        std::fs::write(workspace.join("README.md"), "root\n").unwrap();
        std::fs::write(workspace.join("docs/guide.md"), "guide\n").unwrap();
        let source = source_ref(&workspace).unwrap();

        let first = RunContext::new(workspace.clone(), None).unwrap();
        first.execute(&Commands::Scan { force: false }).unwrap();
        first.catch_up_graph_projection().unwrap();
        let first_output = owner_walk(&first, &source.object_id, &source.object_id);
        assert_eq!(
            first_output["branches"][0]["cut"]["status"], "complete",
            "{first_output:#}"
        );
        assert_eq!(
            first_output["branches"][0]["cut"]["currentness"],
            "latest_complete"
        );
        assert!(
            first_output["branches"][0]["result"]["objects"]
                .as_array()
                .unwrap()
                .len()
                >= 5
        );
        let source_event_seq = first_output["branches"][0]["cut"]["receipts"][0]["source_event"]
            ["seq"]
            .as_u64()
            .unwrap();
        assert_eq!(owner_event_count(&first), 1);

        let structural = structural_walk(&first, &source.object_id);
        let facts = structural["walk"]["visited_facts"].as_array().unwrap();
        assert!(facts.iter().all(|row| {
            row["fact"]["event_type"].as_str() != Some(OWNER_PUBLICATION_EVENT_TYPE)
        }));
        drop(first);

        let restarted = RunContext::new(workspace.clone(), None).unwrap();
        let scan = restarted.execute(&Commands::Scan { force: false }).unwrap();
        assert!(scan.contains("already exists"));
        assert_eq!(owner_event_count(&restarted), 1);
        restarted.catch_up_graph_projection().unwrap();
        let retried = owner_walk(&restarted, &source.object_id, &source.object_id);
        assert_eq!(
            retried["branches"][0]["cut"]["receipts"][0]["source_event"]["seq"],
            source_event_seq
        );
        assert_eq!(retried["branches"][0]["cut"]["status"], "complete");
    });
}

fn owner_walk(context: &RunContext, scope_id: &str, object_id: &str) -> Value {
    let output = context
        .execute(&Commands::Branches {
            command: BranchesCommands::GraphOwnerWalk {
                scope: "active".to_string(),
                branch_ids: Vec::new(),
                owner_id: "workspace_fs".to_string(),
                owner_scope_id: scope_id.to_string(),
                domain: "workspace_fs".to_string(),
                object_kind: "source".to_string(),
                object_id: object_id.to_string(),
                direction: "both".to_string(),
                relation_types: Vec::new(),
                max_depth: 4,
                max_objects: 64,
                max_occurrences: 128,
                max_paths: 128,
                format: "json".to_string(),
            },
        })
        .unwrap();
    serde_json::from_str(&output).unwrap()
}

fn structural_walk(context: &RunContext, object_id: &str) -> Value {
    let output = context
        .execute(&Commands::Branches {
            command: BranchesCommands::GraphWalk {
                scope: "active".to_string(),
                branch_ids: Vec::new(),
                domain: "workspace_fs".to_string(),
                object_kind: "source".to_string(),
                object_id: object_id.to_string(),
                direction: "both".to_string(),
                relation_types: Vec::new(),
                max_depth: 4,
                current_only: false,
                include_facts: true,
                format: "json".to_string(),
            },
        })
        .unwrap();
    serde_json::from_str(&output).unwrap()
}

fn owner_event_count(context: &RunContext) -> usize {
    let replay = context.event_replay_capability();
    let page = replay
        .replay(ReplayRequest {
            cursor: LedgerCursor {
                ledger_id: replay.ledger_identity(),
                after_seq: 0,
            },
            limit: MAX_REPLAY_LIMIT,
        })
        .unwrap();
    page.records
        .iter()
        .filter(|record| record.event_type == OWNER_PUBLICATION_EVENT_TYPE)
        .count()
}
