# Phase 0 Baseline

Date: 2026-04-25
Status: active baseline
Scope: characterization and gate targets for the crate split start point

## Characterization Coverage

| Behavior | Evidence tests |
| --- | --- |
| event append and replay | `integration::event_spine::mixed_spine_events_replay_with_object_refs`, `integration::event_spine::read_events_after_pruned_session_still_reads_canonical_history`, `integration::event_spine::idempotent_append_reuses_existing_record_id` |
| world model catch up | `integration::traversal_graph::graph_runtime_repeated_catch_up_is_idempotent`, `integration::traversal_graph::derived_anchor_events_are_readable_from_spine_after_restart`, `integration::workflow_task_compatibility::workflow_task_path_live_and_replay_artifact_resolution_match` |
| workflow execution | `integration::workflow_task_compatibility::workflow_execute_routes_docs_writer_through_task_path` |
| task execution | `integration::task_executor::task_executor_assembles_payload_and_records_events`, `integration::task_executor::task_executor_publishes_canonical_events` |
| provider backed generation | `integration::generation_parity::generation_parity_file_success_matches_fixture`, `integration::generation_parity::generation_parity_directory_success_matches_fixture`, `integration::generation_parity::generation_parity_retryable_failure_matches_fixture`, `integration::generation_parity::generation_parity_non_retryable_failure_matches_fixture` |

## Focused Verification Command

```sh
cargo test --test integration_tests event_spine:: -- --nocapture
cargo test --test integration_tests traversal_graph::graph_runtime_repeated_catch_up_is_idempotent -- --nocapture
cargo test --test integration_tests traversal_graph::derived_anchor_events_are_readable_from_spine_after_restart -- --nocapture
cargo test --test integration_tests workflow_task_compatibility:: -- --nocapture
cargo test --test integration_tests task_executor:: -- --nocapture
cargo test --test integration_tests generation_parity:: -- --nocapture
```

## Gate Inputs

- `F0` formatter gate uses `cargo fmt --check`
- `F1` compile gate uses `cargo check`
- `F2` uses the focused command set in this document
- `F3` uses [Crate Split Dependency Checklist](dependency_checklist.md)

## Read With

- [PLAN](PLAN.md)
- [Crate Split Dependency Checklist](dependency_checklist.md)
- [Public Surface Inventory](public_surface_inventory.md)
- [Initial Port Inventory](core/port_inventory.md)
