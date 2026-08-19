# Product Event Authority Cutover — E5 Detail

Date: 2026-07-09
Revised: 2026-07-12
Status: complete
Scope: E5 direct product and CLI cutover to one event authority

## Outcome

One product identity now resolves to one persisted ledger identity and one canonical sequence space.
Direct CLI publication, `meld event` reads, graph replay, and `meld runtime run` consume capabilities derived from the same `EventAuthority`.
Product runtime assembly no longer opens canonical event storage.

E5 is complete through `9350a90`.
The main cutover is `97cc225`; closure reliability corrections continue through `9350a90`.

This completion unblocks the runtime visibility workstream.
It does not complete runtime status publication, cache persistence, daemon hosting, IPC, console or action publication, promotion policy, or the full semantic flywheel proof.

Source intent:

- [Events Domain](../../cognitive_architecture/events/README.md)
- [Multi-Domain Event Ledger](../../cognitive_architecture/events/multi_domain_spine.md)
- [Product Event Authority Assessment](product_event_authority_domain_assessment.md)
- [Event Foundation Closeout Program](../events/event_foundation_closeout_program.md)

## Authority Binding

Root composition resolves the product authority in [binding.rs](../../../src/events/binding.rs) before constructing CLI or product runtime adapters.
The branch id is the product identity.
The branch external data home contains `event_authority.json` with the branch id, canonical ledger path, ledger identity, generation, source description, and either `Preparing` or `Active` state.
The target ledger atomically claims and then flushes the branch product identity before root writes a binding or migration batch. Reopen validates the same inverse claim. Separate branch-local binding homes therefore cannot alias one physical authority, even when both configurations name the same external path.

Cutover is serialized by an advisory exclusive lock on `event_authority.lock` through `fs2`.
Binding updates use a temporary file, file sync, atomic rename, and parent directory sync.
A crash can leave the lock file present, but the operating system releases the lock itself.

Binding resolution fails closed when the active branch, ledger path, ledger identity, source marker, or source identity disagrees with the persisted binding.
An `Active` binding does not select or seed another authority and does not fall back to legacy event trees.
A deleted or substituted target ledger is rejected.

Evidence:

- `empty_product_binding_is_active_and_stable_after_reopen`
- `persisted_binding_rejects_another_branch_without_fallback`
- `active_binding_rejects_a_deleted_ledger_instead_of_reseeding_identity`
- `preparing_binding_resumes_a_partial_copy_and_promotes_active`
- `active_binding_fails_closed_when_the_frozen_source_is_deleted`
- `missing_binding_rejects_an_existing_cutover_marker`
- `active_binding_rejects_ledger_identity_substitution`
- `separate_branch_bindings_cannot_share_one_target_ledger`

Authority-owned tests also prove the product claim survives reopen, rejects a different product, and becomes durable when a caller retries after an indeterminate first flush.

These tests are in [binding.rs](../../../src/events/binding.rs).

## Recoverable Legacy Migration

The event-owned migration seam in [migration.rs](../../../crates/meld-events/src/events/migration.rs) strictly validates the complete source and target before copying.
It assigns normal target sequences, preserves a pre-existing target prefix, and records durable source-identity and source-sequence mappings to target sequence and a BLAKE3 canonical-envelope hash.
Interrupted work resumes from those mappings.
Completed work is idempotent.

Migration rejects malformed rows, non-monotonic mappings, divergent record-id collisions, dangling indexes, foreign provenance, equal source and target identities, and source-target path equality before copying a valid prefix.
It preserves canonical envelope meaning, including object refs, relations, payloads, record ids, session and domain fields, and structural provenance.

An empty source is still assigned an identity and receives a durable cutover marker.
This closes the fresh-workspace downgrade hole: an older binary cannot create a new semantic history after product binding activation.
After cutover, semantic appends to legacy event trees are rejected.
Compatibility session trees remain writable, and session lifecycle facts publish through the product authority.

The migration contract is exercised by [event_migration.rs](../../../crates/meld-events/tests/event_migration.rs), including:

- `empty_source_completes_with_identity_and_cutover_marker`
- `migration_preserves_target_prefix_and_semantic_envelopes`
- `mixed_trees_and_per_session_sequence_collisions_are_all_migrated`
- `one_record_batches_resume_and_completed_migration_is_idempotent`
- `identical_record_id_collision_reuses_prefix_but_divergent_collision_fails`
- `malformed_source_fails_before_any_target_mutation`
- `source_semantic_appends_are_rejected_after_cutover_while_other_trees_work`
- `forward_and_backward_provenance_translate_through_the_complete_plan`
- `inserted_mapping_with_missing_target_row_blocks_resume_before_copy`
- `dangling_target_record_index_blocks_migration_before_copy`

Transient sled lock-release variants are retried only at the legacy source open boundary.
The retry behavior landed in `f1021a2` and `bdd5119`; its repeated fixture proof landed in `b6e4e55`.

## Product And CLI Composition

`RunContext` resolves branch identity, external product storage, product binding, and event authority before assembling adapters.
`ProgressRuntime` receives an append capability and compatibility session runtime.
Direct event status, tail, trace, session, and flow commands receive event authority capabilities directly.
They do not select event history through `ProgressRuntime`.

`ProductRuntimeAssembly` consumes the resolved authority and shared graph runtime.
`OpenProductStores` owns projection and domain stores only; it neither owns nor flushes the event authority.
The product ledger remains an external authority opened by the root resolver.
The product flush boundary covers the projection and domain stores it owns.

Session compatibility remains in the configured legacy CLI database.
Existing configured non-event CLI storage paths for node, frame, prompt, belief, and session data are preserved.
Only canonical semantic events move to the bound product authority.

Relative product roots resolve below the workspace-specific XDG data root.
Absolute product roots are accepted only outside the target workspace.
Lexical, normalized, and symlink-resolved containment checks reject roots inside the workspace and relative escapes outside the workspace XDG root.
The storage policy is implemented in [storage_paths.rs](../../../src/config/workspace/storage_paths.rs).

## Branch Isolation

Dormant branch migration resolves each registered branch's own configuration, legacy source, product root, binding, and authority.
It does not merge dormant history into the active authority.
Distinct branch identities retain distinct ledger identities and canonical paths.

Route evidence is in [branches_runtime.rs](../../../tests/integration/branches_runtime.rs):

- `dormant_branch_migrations_keep_separate_product_authorities`
- `dormant_branch_migration_uses_its_configured_legacy_store`
- `active_branch_graph_status_reuses_the_open_product_projection`
- `binary_active_graph_query_routes_through_run_context`

## Route-Level Proof

[product_event_authority_cutover.rs](../../../tests/integration/product_event_authority_cutover.rs) proves the real product route:

- `real_cli_migrates_and_reuses_one_authority_for_event_and_runtime_routes` migrates legacy history, publishes a workspace fact, replays it through the shared graph adapter, observes the same ledger identity and sequence, runs the real runtime route, verifies no second identity appears, verifies legacy event rows do not change, and proves reopen restores identity, watermark, consumer cursor, and next sequence.
- `binary_direct_commands_preserve_one_identity_across_processes` proves direct event commands preserve one ledger identity across separate process invocations and that an empty-source cutover rejects later legacy semantic writes.
- `real_route_rejects_a_mismatched_active_binding_without_fallback` proves a mismatched active binding fails through the real command route without legacy fallback.

The assembly-level test `event_append_and_replay_ports_are_wired_to_one_authority` in [assembly.rs](../../../src/runtime/assembly.rs) proves append and replay ports share the supplied authority.
The test `supplied_authority_and_graph_runtime_are_shared_across_assembly` proves the direct and supervised paths reuse the same authority and graph runtime.

## Gates And Review

Repeated authority, cursor, concurrency, recovery, migration, and workspace runs also passed.

Fresh migration review covered crash states, malformed rows, mapping idempotency, record-id conflicts, structural provenance, and semantic parity.
Fresh routing review covered one identity across CLI and runtime, constructor sealing, branch isolation, external storage policy, compatibility path preservation, downgrade prevention, and route-test honesty.
All blocker and should-fix findings were resolved and re-reviewed through `9350a90`.

## Breaking And Rollout Notes

- The first command after upgrade creates or resumes cutover under the external advisory lock.
- Relative configured product roots now resolve under the workspace-specific XDG data root.
- Absolute or resolved product roots inside the target workspace fail with a typed error.
- Migration appends to the target and does not delete legacy history.
- `Preparing` state resumes; `Active` state never falls back.
- Malformed source data, divergent record ids, path or identity substitution, and inconsistent markers fail closed.
- Compatibility session data remains in the legacy CLI database, but legacy event trees accept no new semantic appends after cutover.
- Old binaries are not supported for semantic event writes once the cutover marker is active.
- Production raw event store, writer, and graph-store construction seams are sealed; tests use explicit test-support fixtures.

Post-cutover compatibility remains only for deployed persisted formats. Event store open still migrates legacy session rows, slims old full-value session indexes, and backfills the record index. Execution still decodes and upgrades publication state without an identity-bearing receipt. World model preserves a legacy graph cursor as evidence before rebuilding from sequence zero. Each local `TODO compat-shim` names its minimum supported schema removal condition and the exact parity tests that must remain green before deletion. The legacy authority migration adapter itself is a stable upgrade boundary rather than a temporary writable fallback.

## Runtime Handoff

The event foundation now supplies a stable identity-bearing authority, observable reports, direct local capabilities, and a transport-neutral remote contract.
Runtime work can resume with R1 through R4:

- R1 owns `RuntimeStatusPublisher` cadence, status cache persistence, and staleness.
- R2 owns daemon lifecycle and real IPC framing, reconnects, endpoints, and authentication.
- R3 owns console frames, runtime action records, and heartbeat or action mapping.
- R4 owns the complete semantic flywheel and operator-visibility proof.

Those runtime concerns are consumers of the completed event authority and were not E5 closure gates.
