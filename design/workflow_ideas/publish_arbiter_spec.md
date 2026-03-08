# Publish Arbiter Workflow

Date: 2026-03-08
Status: draft

## Intent

Define a workflow architecture for materializing workflow outputs into repository files without creating self triggered regeneration loops or unnecessary upstream documentation churn.

## Problem

Meld workflows read from the same repository that future publish steps may write back into.

That creates two related hazards.

- A workflow writes a managed file such as `README.md`
- the write changes the Merkle tree
- the watch path or recursive generate path sees a new dirty branch
- the workflow re reads its own output as source input
- the workflow writes again
- the loop repeats

The same problem appears at larger scope.

- a narrow source edit changes one leaf node
- Merkle identity changes for every ancestor up to root
- naive dirty propagation marks every ancestor as requiring regeneration
- parent and root documentation can be rewritten even when their reader facing meaning did not change

The repository needs an arbiter layer that reasons about write intent, source scope, and publish impact above raw Merkle dirtiness.

## Design Goals

- prevent self caused workflow loops
- limit regeneration blast radius
- preserve deterministic bottom up execution where it is still useful
- separate source freshness from publish freshness
- let workflows publish to repository files when explicitly allowed
- avoid rewriting parent or root docs when source meaning did not change
- keep adapters thin and keep orchestration inside workflow domain

## Non Goals

- redesign the Merkle tree hashing model
- make semantic impact perfect in v1
- allow arbitrary workflow side effects with no declared write scope
- remove current frame based workflow outputs

## Current Behavior Summary

Current code already provides strong building blocks.

- tree ids are recomputed bottom up from filesystem content in `src/tree/builder.rs`
- recursive generate already executes by depth level in `src/context/generation/run.rs`
- workflow outputs are stored as frames in `src/workflow/executor.rs`
- prompt and context lineage already produce `prompt_digest` and `context_digest` in `src/prompt_context/orchestration.rs`
- watch mode expands a path change to affected ancestors in `src/workspace/watch/runtime.rs`

Current code also has one critical gap for safe publish.

- workflow completion reuse is keyed by workflow and node identity, not by source freshness for a stable logical publish target

That means a self caused file write can look like a brand new target because the Merkle node id changed.

## Proposed Model

Introduce a new workflow domain component named publish arbiter.

The publish arbiter sits between workflow artifact generation and repository file materialization.

The publish arbiter owns four concerns.

1. stable logical target identity
2. source scope freshness
3. publish decision and write suppression
4. upward impact propagation

## Core Principle

Raw Merkle dirtiness is necessary for storage correctness but insufficient for documentation publish decisions.

The publish arbiter must decide whether a change is:

- source meaningful for this target
- publishable for this target
- upward relevant for any parent target

## Layered Heads

Use three distinct head concepts.

- content head
- artifact head
- publish head

### Content Head

The content head is the current repository state as observed by Merkle nodes and node records.

This head changes for any file content or structure change.

### Artifact Head

The artifact head is the latest workflow output frame for a logical target.

This head changes when workflow generation produces a new artifact candidate.

### Publish Head

The publish head is the latest artifact that has been approved for materialization into repository files.

This head changes only after arbiter approval and successful file write.

## Stable Logical Target

The arbiter must not key publish state by Merkle `node_id` alone.

Each publishable target should have a stable logical target id derived from declarative workflow routing data.

Suggested fields:

- `workflow_id`
- `target_kind`
- `target_path`
- `target_agent_id`

Example target kinds:

- `directory_readme`
- `crate_readme`
- `root_readme`
- `api_summary`

Suggested logical target id formula:

- hash of `workflow_id + target_kind + canonical_target_path + target_agent_id`

This keeps publish identity stable even if `node_id` changes after a managed write.

## Workflow Publish Contract

Any workflow that may materialize files must declare a publish contract.

Suggested contract fields:

- `logical_target_kind`
- `read_scope`
- `write_scope`
- `managed_output_paths`
- `upward_policy`
- `publish_mode`
- `normalization_policy`

### Read Scope

`read_scope` defines which source files and frame types are valid inputs for freshness decisions.

Example values:

- one node path
- one subtree rooted at path
- explicit include glob set
- explicit source frame types

### Write Scope

`write_scope` defines which repository paths the workflow may materialize.

The arbiter must reject writes outside declared scope.

### Managed Output Paths

`managed_output_paths` defines the exact paths whose writes are considered self managed for loop suppression.

Examples:

- `src/tree/README.md`
- `README.md`

### Upward Policy

`upward_policy` declares whether a child target can dirty any parent target.

Suggested values:

- `none`
- `local_summary`
- `parent_summary`
- `root_summary`

### Publish Mode

Suggested values:

- `frame_only`
- `stage_only`
- `materialize`

`frame_only` stores output frames and does not write repo files.

`stage_only` writes artifacts into a staging area outside the observed repository tree.

`materialize` allows arbiter controlled repository writes.

### Normalization Policy

This policy ensures two semantically identical outputs compare equal before publish.

Typical rules:

- normalize final newline
- normalize line endings
- trim trailing whitespace when allowed

## Publish Arbiter State

The arbiter persists state by logical target id.

Suggested record fields:

- `logical_target_id`
- `workflow_id`
- `target_path`
- `managed_output_paths`
- `observed_scope_digest`
- `published_output_digest`
- `last_artifact_frame_id`
- `last_publish_frame_id`
- `impact_class`
- `last_source_event_id`
- `last_publish_at_ms`

### Observed Scope Digest

The arbiter computes `observed_scope_digest` from the declared read scope after excluding managed output paths.

This is the key loop breaker.

If `README.md` is managed output for the same logical target, `README.md` must not contribute to the source freshness digest for that target.

### Published Output Digest

The arbiter computes `published_output_digest` from normalized output bytes.

If a candidate artifact normalizes to the same digest, the arbiter skips the repository write.

### Impact Class

`impact_class` is a coarse summary used for upward propagation.

Suggested values:

- `none`
- `local_text_only`
- `child_inventory_changed`
- `public_api_changed`
- `structural_summary_changed`

## Digest Strategy

The arbiter needs more than raw node ids.

For each logical target compute three digest families.

- source inventory digest
- doc semantic digest
- publish output digest

### Source Inventory Digest

This digest answers whether the source set that matters to the target changed.

Inputs may include:

- canonical source file paths
- source file content hashes
- selected frame digests by frame type

Managed output paths are excluded.

### Doc Semantic Digest

This digest answers whether the summary that parents care about changed.

It should be derived from reduced structured facts, not full prose.

Examples:

- module inventory
- public identifiers
- exported commands
- section titles
- declared caveats

This digest can be produced by a narrow summarizer step or by deterministic extraction from workflow turn artifacts.

### Publish Output Digest

This digest answers whether the target file bytes would change after normalization.

It does not decide parent invalidation by itself.

## Decision Flow

For any workflow artifact candidate, the arbiter runs this decision flow.

1. resolve logical target id
2. load publish contract
3. compute current `observed_scope_digest`
4. compare against stored `observed_scope_digest`
5. if unchanged, skip generation or skip publish based on available artifact freshness
6. if changed, accept artifact candidate evaluation
7. normalize candidate output and compute `publish_output_digest`
8. compare against current file bytes if file exists
9. if output digest is unchanged, advance source freshness state but do not write file
10. compute reduced `impact_class`
11. publish file only if within write scope and allowed by mode
12. update parent dirtiness only if `upward_policy` allows it and `impact_class` requires it

## Loop Suppression Rules

The arbiter must suppress self triggered reruns when all conditions hold.

- the last filesystem event touched only managed output paths for the same logical target
- the current `observed_scope_digest` is unchanged
- the current candidate output digest is unchanged or already published

When those conditions hold, the system should record a no op publish outcome and stop propagation.

## Parent Propagation Rules

Parent invalidation must be based on reduced impact, not raw ancestor dirtiness.

Suggested rules for v1:

- `local_text_only` does not dirty parent targets
- `child_inventory_changed` may dirty the direct parent target
- `public_api_changed` may dirty direct parent and any summary target that explicitly depends on API surface
- `structural_summary_changed` may dirty up to root if the root target opted into `root_summary`

This means a guard clause change in `src/tree/builder.rs` can change raw Merkle ancestors while still producing `impact_class = local_text_only`, which stops doc propagation.

## Staging Recommendation

Preferred architecture uses a staging area outside the observed workspace tree.

Suggested flow:

1. workflow writes final artifact bytes to frame storage
2. arbiter writes candidate bytes to staging storage
3. arbiter compares candidate against current repo file
4. arbiter materializes only approved changes into the repo

Benefits:

- no immediate watch feedback from candidate generation
- easier diff review
- easier replay and audit
- simpler loop suppression

## Direct Materialization Rules

If direct repo writes are allowed, require these safeguards.

- managed output paths must be declared
- source digest must exclude managed output paths
- logical target identity must be stable across Merkle changes
- watch handling must classify managed writes as self events
- parent invalidation must use reduced impact classes

Without all of these safeguards, direct repo writes should remain disabled.

## Watch Integration

Watch mode should stop treating every ancestor of a changed file as equally dirty for workflow publish.

Suggested flow:

1. watch rebuilds content state as it does today
2. watch emits raw affected paths
3. arbiter classifies event origin as source event or managed publish event
4. arbiter resolves impacted logical targets
5. arbiter schedules only targets whose source digest or accepted parent impact changed

This keeps tree correctness in one layer and publish blast radius in another layer.

## Runtime Identity Update

Workflow thread reuse should evolve from node keyed identity to target keyed freshness.

Suggested runtime identity fields:

- `workflow_id`
- `logical_target_id`
- `observed_scope_digest`
- `target_frame_type`

This matches the current design intent more closely than the current node keyed thread id and fixes the self write case.

## Domain Ownership

Ownership proposal:

- `src/workflow` owns publish contract parsing, arbiter state, publish decisions, and target dirtiness propagation
- `src/context` continues to own frame retrieval and generation inputs
- `src/tree` continues to own content addressing and path to node relationships
- `tooling` and `api` remain thin adapters that delegate to workflow services

The arbiter should consume public contracts only and should not reach into tree or context internals beyond their owned interfaces.

## Suggested Data Types

Suggested new types under `src/workflow`:

- `publish_contract.rs`
- `publish_arbiter.rs`
- `publish_state.rs`
- `impact.rs`

Suggested structs:

- `LogicalTargetId`
- `PublishContract`
- `PublishDecision`
- `PublishStateRecord`
- `TargetImpact`
- `ManagedWriteEvent`

## Implementation Sequence

### Phase 1 Stable Target Identity

- add logical target id contracts
- add workflow publish contract schema
- persist publish state records keyed by logical target id
- keep workflows in `frame_only` mode

### Phase 2 Source Freshness And No Op Publish

- compute `observed_scope_digest`
- compute normalized output digests
- add no op publish decisions
- update thread reuse to use target freshness rather than only node identity

### Phase 3 Managed Write Suppression

- classify managed output paths in watch mode
- suppress self caused reruns
- record publish telemetry for skipped and materialized outcomes

### Phase 4 Reduced Parent Impact

- add `impact_class`
- add parent invalidation rules based on `upward_policy`
- stop naive parent doc rewrites for local source edits

### Phase 5 Optional Materialization

- enable `stage_only` mode first
- validate parity between staged artifact and repo write result
- enable `materialize` mode behind explicit configuration

## Verification Strategy

Add characterization and parity tests for these scenarios.

- managed `README.md` write does not retrigger its own target when source digest is unchanged
- leaf code edit with `impact_class = local_text_only` does not dirty parent doc target
- child inventory change dirties direct parent target and not unrelated siblings
- root summary target updates only when configured `upward_policy` allows it
- normalized output equality skips file write and still advances freshness state
- staged output and materialized output produce identical final bytes

## Open Questions

- where should reduced semantic digests be extracted, from workflow artifacts or from deterministic code analyzers
- whether `impact_class` should be single value or a small tag set
- whether publish state should live beside workflow state store or in a dedicated store
- whether direct materialization should ever be default in watch mode

## Recommendation

Adopt the publish arbiter as a first class workflow domain service.

Short term, keep docs workflows frame first and stage first.

Medium term, move thread reuse and freshness to stable logical targets with managed output exclusion.

Long term, let parent propagation follow reduced semantic impact rather than raw Merkle ancestry.
