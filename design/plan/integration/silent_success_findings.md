# Silent Success Findings

Date: 2026-07-31
Status: triage
Scope: confirmed findings from the 2026-07-31 bug-class hunt over the flywheel path, with dispositions into the [Flywheel Parity Workstream](flywheel_parity_workstream.md)

## Method

The hunt swept for four signatures of one failure class — independent focused implementations missing cross-cutting nuance a sibling path carries:

1. Dual-path drift between sibling implementations of the same behavior.
2. Evaluated-but-discarded verdicts.
3. Silent empty-tolerance where absence is load-bearing.
4. Vacuous-truth aggregation that cannot distinguish all-N-passed from N-equals-zero.

The known five-link hollow-README chain was excluded; everything below is additional. All findings are confirmed by tracing actual behavior unless marked suspect. The exercise definition is reusable as a review pass whenever a new route or parallel path lands.

## Findings

### F1 — package path ignores `input_refs`

`TurnSpec` at `crates/meld-execution/src/task/package/region.rs:45-62` has no `input_refs` field; lowering never reads the profile's declared inputs, and expansion wires only the adjacent previous turn's output at `src/merkle_traversal/expansion.rs:151-161`. The workflow sibling hard-errors on a missing declared input at `crates/meld-execution/src/workflow/resolver.rs:32-42`. Two authored YAML sources of truth are never cross-checked; a reorder fails loudly on one path and silently starves context on the other. Disposition: parity slice one. Progress 2026-07-31: the enforcement half landed through F8 — readiness now validates declared artifact type and schema per source, which immediately exposed the docs_writer_v2 prerequisite declaring `frame_ref` while the consumed artifact is `readme_final`; both package copies corrected. Remaining: lowering-time cross-validation of package turns against profile `input_refs` so mismatches fail at load rather than at run.

### F2 — `style_gate` passes vacuously through three routes

`evaluate_no_semantic_drift` builds its required-section list dynamically and silently degrades to a pass when the list is empty: an absent input key via `unwrap_or_default` at `gates.rs:141-145`; a fence-naive JSON parse at `gates.rs:157` that rejects the fenced model output its fence-aware sibling `src/context/capability.rs:840-878` accepts; non-object JSON at `gates.rs:160-162`. A key-space mismatch — workflow path keys gate inputs by input ref, capability path by artifact type id — only coincidentally lines up today. A README that dropped every section can pass the semantic-drift gate. Disposition: [Gate Signal First Slice](gate_signal_first_slice.md).

### F3 — staleness digests computed and discarded

`collect_existing_frame_refs` at `src/merkle_traversal/expansion.rs:416-450` treats head-frame presence as done and never compares the stored context digest against the freshly computed one; no call site in the repo performs that comparison — digests are loaded only to populate validation event fields. Emptied batches are dropped without flagging at `expansion.rs:66` and no guard exists for an empty active-batch set. Source changes under a documented directory produce zero work without `--force`. The re-fire half of docs_freshness is currently dead. Disposition: parity slice three, new.

### F4 — evidence replay requires an artifact type nothing produces, and advances its cursor past skips

`fresh_content` at `src/runtime/ports.rs:655-666` filters on artifact type `docs_patch`, which has zero production producers. Skips are counted neither promoted nor rejected, so a fully skipped replay reads as healthy. The cursor advances before the skip at `ports.rs:654`, making skipped events permanently unrecoverable. Every event in a batch is also attributed to one caller-supplied subject rather than its own. Disposition: liveness triage first — if the path is legacy, kill it; if live, the cursor advance is the urgent fix.

### F5 — completion predicates pass on zero of zero

Bare equality at `crates/meld-execution/src/task/executor.rs:294-296`, `task_network/dispatch_actor.rs:1159-1161`, and `task_network/package_step.rs:107-110`, while the sibling `aggregate_publication.rs:205-207` carries the correct `known_units > 0` guard. Combined with F3, a zero-work run records a Succeeded task outcome and only the aggregate layer later disagrees with a misleading `PrematureCompletion`. Disposition: parity slice three, new.

### F6 — capability-path gate handling drops three sibling behaviors

Relative to `workflow/executor/attempt.rs:374-441`, the capability path at `src/context/capability.rs:1071-1080` does not persist gate records, does not retry on gate failure, and ignores `stop_on_gate_fail`. Already the core of the gate slice; the `stop_on_gate_fail` omission is newly confirmed and joins it. Disposition: [Gate Signal First Slice](gate_signal_first_slice.md).

### F7 — force-tombstone ordering drift, corrected on trace

Corrected 2026-07-31 during slice one: orchestration also assembles the prompt before tombstoning — `orchestration.rs:48` builds messages, `:101` tombstones — so both paths condition a forced prompt on the old head and the original failure scenario does not distinguish them. The remaining drift is the previous-metadata snapshot: orchestration snapshots after the tombstone, the capability path snapshots in prepare before it. The capability path's late tombstone at finalize is the safer ordering since the old head survives a failed run. Disposition: no code change in slice one; the metadata-snapshot difference is recorded as a residual for whoever consumes previous-digest fields.

### F8 — intra-task readiness discards schema checks and diagnostics

`task/readiness.rs:23-61` destructures away the declared `schema_version`, returns bare strings where the task-network sibling returns typed diagnostics, and evaluates `.all()` over possibly empty wiring — the direct consumer that makes the known empty-wiring filter dangerous. Disposition: parity slice one. CLOSED 2026-07-31: readiness validates artifact type and schema version per declared source and blocked tasks narrate per-instance reasons through the run error. The empty-wiring `.all()` vacuity remains inert only while the expansion filter exists; revisit with slice-three truthfulness work.

### F9 — publish filter records `missing_head` and nothing reads it

Written at `src/workspace/capability.rs:394` and `:636`, read nowhere. A run can report success having written zero README files. Secondary: `already_published` compares frame ids only, never file content, at `src/workspace/publish.rs:285-291`, so an externally clobbered README is skipped as current. Disposition: primary to parity slice four; secondary recorded as residual.

### F10 — `no_semantic_drift` unconditional early-pass is indistinguishable from a real pass

`gates.rs:123-125` returns a pass with empty reasons when no sections are configured; the durable record cannot distinguish checked-everything from checked-nothing. Disposition: [Gate Signal First Slice](gate_signal_first_slice.md) — gate records must carry what was checked.

### F11 — `MissingGraphScope` warning is recorded beside the decision it should block

Projection emits the warning; `projection_blocks_goal` at `crates/meld-world-model/src/agent/curation.rs:836-841` matches only `MissingBelief`. Curation emits goals against world states with no accessibility proposition and durably records the warning next to the decision. Disposition: residual, recorded beside the existing accessibility-from-anchors residual; revisit at the Strategy pause.

## Checked And Clean

Thin root-crate delegations to `meld-execution`, the quiescence contract at `src/runtime/tooling.rs:800-802` where empty-quiescent is documented intent, the `NoFolderWorkUnits` guard and its `known_units > 0` sibling, documented idempotent no-op discards in `task/step.rs` and `task/runtime.rs`, and empty-level rejection in `generation/plan.rs:99`.

## Related Documentation

- [Flywheel Parity Workstream](flywheel_parity_workstream.md) — slice assignments for these findings
- [Gate Signal First Slice](gate_signal_first_slice.md) — receives F2, F6, F10
- [Failure Signal](../../cognitive_architecture/execution/failure_signal.md) — the canonical semantics violated by the gate-family findings
