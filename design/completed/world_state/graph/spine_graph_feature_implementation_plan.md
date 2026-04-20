# Spine Graph Feature Implementation Plan

Date: 2026-04-18
Status: completed
Scope: phased delivery tracker for spine hardening, graph completion, branch federation, and first consumer integration

## Overview

Objective:

- fully land the first event spine plus graph traversal feature with mature federation and one real production consumer

Outcome:

- canonical spine history is durable and replay safe
- required publisher domains publish graph usable facts
- `world_state/graph` persists explicit derived facts
- branch federation preserves branch provenance and branch presence
- workflow task execution consumes traversal resolved artifacts

## Checkpoint 1

Goal:

- harden the spine substrate so canonical history survives session cleanup and supports idempotent derived fact append

Status:

- completed

Tasks:

- add optional `record_id` to canonical event contracts
- add idempotent append support in the spine store
- expose idempotent emit helpers through runtime compatibility surfaces
- preserve canonical spine history during session cleanup
- add verification for legacy compatibility and idempotent append

Verification gates:

- canonical spine history remains readable after session prune
- idempotent append reuses an existing record without duplicate spine rows
- legacy records without `record_id` still deserialize and replay
- staged diff review before commit

Verification evidence:

- Gate pass via `cargo test event_spine --tests`
- Gate pass via manual diff review of checkpoint 1 files before commit

## Checkpoint 2

Goal:

- publish canonical workspace facts from promoted watch batches

Status:

- completed

Verification evidence:

- Gate pass via `cargo test workspace_traversal --tests`
- Gate pass via `cargo test watch_batch --lib`
- Gate pass via manual diff review of checkpoint 2 files before commit

## Checkpoint 3

Goal:

- persist derived traversal facts through idempotent spine append

Status:

- completed

Verification evidence:

- Gate pass via `cargo test traversal_graph --tests`
- Gate pass via `cargo test world_state::graph --lib`
- Gate pass via `cargo test event_spine --tests`
- Gate pass via manual diff review of checkpoint 3 files before commit

## Checkpoint 4

Goal:

- annotate federated traversal results with branch provenance

Status:

- completed

Verification evidence:

- Gate pass via `cargo test branches_query --tests`
- Gate pass via `cargo test branches_runtime --tests`
- Gate pass via manual diff review of checkpoint 4 files before commit

## Checkpoint 5

Goal:

- resolve workflow task path outputs through traversal artifact anchors

Status:

- completed

Verification evidence:

- Gate pass via `cargo test workflow_task_compatibility --tests`
- Gate pass via `cargo test traversal_graph --tests`
- Gate pass via manual diff review of checkpoint 5 files before commit

## Checkpoint 6

Goal:

- close the branch with full verification and documented gate evidence

Status:

- completed

Verification evidence:

- Gate pass via `cargo test event_spine --tests`
- Gate pass via `cargo test workspace_traversal --tests`
- Gate pass via `cargo test watch_batch --lib`
- Gate pass via `cargo test traversal_graph --tests`
- Gate pass via `cargo test branches_query --tests`
- Gate pass via `cargo test branches_runtime --tests`
- Gate pass via `cargo test workflow_task_compatibility --tests`
- Gate pass via `cargo test workflow_cli --tests`
- Gate pass via `cargo test context_generate_recursive_completes_levels_bottom_up --test integration_tests`
- Gate pass via `cargo test --tests`
- Gate pass via manual diff review of checkpoint 6 files before commit

Closeout notes:

- broader verification exposed completed task path reuse regressions in workflow CLI coverage
- the branch now persists durable workflow thread `final_frame_id` state for task package runs
- completed task path reuse is gated by traversal `frame_ref` anchor presence plus durable thread state
- workflow CLI reuse tests now seed canonical task path state through one real execution before asserting reuse

## Related Documents

- [Spine Graph Completion Review](spine_graph_completion_plan.md)
- [Branch Feature Implementation Plan](branch_feature_implementation_plan.md)

## Exception List

- full branch federation remains in scope
- belief and curation remain out of scope
- no push without explicit user confirmation
