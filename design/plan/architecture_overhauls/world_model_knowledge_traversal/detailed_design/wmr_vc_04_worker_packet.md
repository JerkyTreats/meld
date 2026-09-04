# WMR-VC-04-R1 Worker Packet

Date: 2026-09-04

Slice: `WMR-VC-04-R1`

Mode: active delivery

Status: closed, committed, and pushed for exact candidate `WMR-VC-04-R1::5308f0ba6ce873bd1e00b4ba283d60eefc9df996f9ba261461fc44929c86c430`

Source baseline: `713ca3ee`

Authority: direct program-owner corrective and retirement authority

Review order: logical implementation review, Style Assurance, Gate Acceptance

## Product Increment

Preserve the verified route for the actual accepted five-Capability Docs Task while atomically deleting the superseded Execution planner, Goal writer, package-plan handoff, runtime identities, public exports, and exclusive support surface.

The Task contains exactly:

- `docs.inspect_scope`
- `docs.draft_patch_set`
- `docs.validate_patch_set`
- `docs.publish_patch_set`
- `docs.assess_published_scope`

## Required Behavior

- Agent rechecks its current Planner cut and activation fence before Task authorization
- Agent persists a fresh Task authorization with the exact authority decision
- Execution independently validates the offered Task and alone may durably record its admission decision
- lowering consumes the admitted Task body directly
- only canonical lowering may insert an attributed region
- every operational node retains complete admission attribution and a durable canonical full-node hash
- mutation and dispatch reject duplicate steps or substituted executable content
- two accepted admissions create distinct regions in one Task Network
- dispatch revalidates live authority and generation before claim and invocation
- task artifacts persist before the terminal outcome command
- crash replay converges without duplicate outputs
- Execution publication remains neutral
- Graph catchup occurs only after an Event is appended
- Agent accepts only the Task-declared `ExecutionTerminal` milestone

## Required Retirement

- delete the `meld-execution` planning module
- delete Execution Goal command, query, store, and database opening
- delete world projection, Method search, realization, and composition lowering
- delete package-plan handoffs and dispatch branches
- delete package-plan preparation and invocation adapters used only by that branch
- delete old runtime identities `execution.planning` and `execution.goal_set`
- delete old public exports, exclusive tests, and exclusive fuzz targets
- delete the retired planner projection port, resource requirement, and dead error surface
- rename surviving Task admission ownership away from planning and Goal modules
- retain historical planning decode only as a named read-only path

## Required Deferral

Do not add or retain a WMR `workspace_scan` realization, `workspace_event_candidates` publication, workspace owner Event append, or workspace owner Graph claim.

Do not change PDS selection, synthesize a workspace Task, replace the accepted Task, or treat a separately constructed workspace candidate as Agent output.

## Exact Boundary

The proof begins at the actual `AgentAuthorizedProduct::Task` stored for the accepted Plan. It ends at the durable Agent milestone whose owner position is the admitted Task region terminal outcome identifier.

The proof records Event publication and Graph catchup as separate positions. Neither is the Agent milestone owner position in this slice.

No operational result may be interpreted as Docs correctness, Belief settlement, Goal satisfaction, or owner truth.

## Compatibility Rule

Retained historical decoding must have:

- one named owning module
- one real store-open caller
- one historical replay fixture
- no public current writer
- a write fence after legacy lineage is detected
- a clear future removal condition

Generic Task and explicit Workflow persistence remain supported. Explicit Workflow execution cannot import or assert Agent Task authority without a durable admission attribution.

## Verification

- `cargo fmt --all --check`
- `cargo check --workspace`
- `cargo test --workspace --no-run`
- `cargo test --workspace`
- root Task route proof
- historical Task Network replay proof
- explicit Workflow regression suite
- `cargo clippy --workspace --all-targets --all-features -- -A clippy::result_large_err -D warnings`
- repository diff and retired-symbol scans

The unmodified strict Clippy command is also recorded. Its only failures are the existing repository-wide `result_large_err` debt outside this retirement change.

## Stop Conditions

Stop and return to design if implementation requires a compatibility writer, a forwarding runtime, another Task Network, a new database, a new dependency, a PDS change, Task replacement, or an owner-return artifact not produced by the accepted Task.

## Authority Boundary

This packet authorizes only `WMR-VC-04-R1`. It does not authorize commit, push, deployment, `WMR-VC-05`, or any later slice.

## Completion

The exact candidate passed fresh logical review, dedicated Style Assurance, and Gate Acceptance in order on 2026-09-04. Program-owner close authority then closed the slice, and separate authority committed and pushed it to the branch. No deployment or later-slice authority was created.
