//! Read-only decoding for Task Network records written by the retired planner.
//!
//! The store calls this codec only while opening existing journals and
//! snapshots. A network containing decoded planning lineage remains readable
//! but cannot accept new commands. This codec can be removed after the oldest
//! supported persisted Task Network schema no longer carries planning fields.

use serde::de::DeserializeOwned;

use super::{codec::to_decode, error::TaskNetworkStoreError};

pub(super) fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, TaskNetworkStoreError> {
    serde_json::from_slice(bytes).map_err(to_decode)
}

#[cfg(test)]
mod tests {
    use crate::task::{CompiledTaskRecord, TaskRunContext};
    use crate::task_network::{
        command::{Command, Request},
        journal::JournalRecord,
        mutation::{CommitRecord, Inject, Mutation, Set},
        state::{NetworkState, TaskLineage, TaskNode, TaskStatus},
        store::{
            records::{revision_key, StoredJournalRecord, TREE_JOURNAL_BY_REVISION},
            SledTaskNetworkStore,
        },
    };

    #[test]
    fn historical_planning_record_replays_through_named_read_only_decoder() {
        let node = TaskNode {
            task_instance_id: "legacy-task".to_string(),
            lifecycle_epoch: 0,
            compiled_task: CompiledTaskRecord {
                task_id: "legacy-compiled-task".to_string(),
                task_version: 1,
                init_slots: vec![],
                capability_instances: vec![],
                dependency_edges: vec![],
            },
            init_sources: vec![],
            task_run_context: TaskRunContext {
                task_run_id: "legacy-run".to_string(),
                session_id: None,
                trigger: "historical_fixture".to_string(),
            },
            lineage: TaskLineage {
                composition_id: "legacy-composition".to_string(),
                goal_id: "legacy-goal".to_string(),
                method_id: "legacy-method".to_string(),
                step_id: "legacy-step".to_string(),
                operator_id: "legacy-operator".to_string(),
                world_state_frame_id: "legacy-frame".to_string(),
                capability_type_id: "docs.write".to_string(),
                capability_version: 1,
                authority_decision: None,
                admission: None,
            },
        };
        let set = Set::new(
            "legacy-network",
            "legacy-composition",
            "legacy-once",
            vec![Mutation::Inject(Inject::new(node.clone(), vec![]))],
        );
        let mut next = NetworkState::empty("legacy-network");
        next.tasks.insert(node.task_instance_id.clone(), node);
        next.statuses
            .insert("legacy-task".to_string(), TaskStatus::Pending);
        next.set_revision_and_hash(1);
        let commit = CommitRecord::new(
            "legacy-command",
            "legacy-network",
            1,
            NetworkState::empty("legacy-network").state_hash,
            next.state_hash.clone(),
            set,
        );
        let mut wire =
            serde_json::to_value(StoredJournalRecord::new(JournalRecord::Commit(commit))).unwrap();
        let mutation_set = wire
            .pointer_mut("/record/Commit/mutation_set")
            .unwrap()
            .as_object_mut()
            .unwrap();
        let source = mutation_set.remove("source_task_id").unwrap();
        mutation_set.insert("source_composition_id".to_string(), source);
        mutation_set.insert(
            "diagnostics".to_string(),
            serde_json::json!([{"code": "legacy_fixture"}]),
        );

        let db = sled::Config::new().temporary(true).open().unwrap();
        db.open_tree(TREE_JOURNAL_BY_REVISION)
            .unwrap()
            .insert(revision_key(1), serde_json::to_vec(&wire).unwrap())
            .unwrap();
        db.flush().unwrap();

        let mut reopened = SledTaskNetworkStore::open(db, "legacy-network").unwrap();
        assert_eq!(reopened.state().revision, 1);
        assert!(reopened.state().contains_legacy_planning());
        let state = reopened.state();
        let error = reopened
            .submit(Request {
                command_id: "forbidden-current-write".to_string(),
                network_id: state.network_id.clone(),
                base_revision: state.revision,
                base_state_hash: state.state_hash.clone(),
                read_preconditions: vec![],
                command: Command::ApplyMutationSet(Set::empty(
                    "legacy-network",
                    "current-task",
                    "current-once",
                )),
            })
            .unwrap_err();
        assert!(error.to_string().contains("read-only"));
    }

    #[test]
    fn current_mutation_and_lineage_writes_omit_retired_planning_fields() {
        let set = Set::empty("current-network", "current-task", "current-once");
        let set_wire = serde_json::to_value(&set).unwrap();
        assert_eq!(set_wire["source_task_id"], "current-task");
        assert!(set_wire.get("source_composition_id").is_none());
        assert!(set_wire.get("diagnostics").is_none());

        let lineage = TaskLineage::unattributed(
            "current-step".to_string(),
            "current-operator".to_string(),
            "docs.write".to_string(),
            1,
        );
        let lineage_wire = serde_json::to_value(lineage).unwrap();
        for retired in [
            "composition_id",
            "goal_id",
            "method_id",
            "world_state_frame_id",
        ] {
            assert!(lineage_wire.get(retired).is_none());
        }
    }
}
