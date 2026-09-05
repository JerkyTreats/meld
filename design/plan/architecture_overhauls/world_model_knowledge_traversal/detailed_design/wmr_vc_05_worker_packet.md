# WMR-VC-05 Worker Packet

Date: 2026-09-05

Slice: `WMR-VC-05`

Mode: active delivery

Status: exact style-successor candidate Gate accepted and slice closed

Source baseline: `a7032bbf`

Authority: direct program-owner authorization for `SI-VC-05`

Review order: logical implementation review, Style Assurance, Gate Acceptance

## Product Increment

Create one complete PDS product compilation and one Agent-owned genesis route, then persist an inert prepared activation closure over exact assignment, topology, Capability preparation, owner revision, authority input, and participant plan identities.

## Required Behavior

- compile only the exact selected package receipt set
- reject missing, extra, mismatched, ambiguous, or corrupt inputs
- declare a finite Agent topology with exact directives, observation components, required routes, subscription contracts, and structural participant references
- bind assignment to product revision, compilation receipt, principal, subject, perspective, branch, topology, requested authority, and grant lineage
- let Agent own genesis intent validation, immutable identity conflict detection, request durability, canonical Event-backed publication, and completion receipt bound to the exact ledger position
- let each source owner accept its exact requested relationship
- require every declared position and subscription acceptance before topology completion
- let Capability prepare exact inert realization inputs without claiming availability
- persist and reopen the complete prepared product position
- make rerun content-idempotent
- rebuild topology from durable Agent receipts after restart

## Required Retirement

- remove direct root seed registration as product genesis authority
- remove direct root Belief subscription mutation as product genesis authority
- remove direct root Event publication as Agent genesis authority
- remove reduced world-init genesis and every synthetic assignment or compilation identity
- fail closed when a genesis caller supplies no complete product aggregate
- remove synthesized compatibility assignment and activation identifiers from runtime assembly
- stop using lifecycle labels as proof of complete Agent genesis
- stop using descriptor-derived registration inventory as prepared product evidence
- prevent runtime Capability rehydration without one exact prepared closure
- remove harness-owned product initialization and its public request payload
- remove alternate world-init theory installers and caller-supplied semantic initialization content
- remove loose runtime theory injection that can survive outside exact prepared closure hydration
- delete the unused startup activation writer while retaining the canonical lifecycle service
- narrow append-proof minting and dead lower Agent commands to their owning modules
- require an opaque Belief proof minted only after durable source-owner acceptance
- confine the raw Belief acceptance writer to its crate
- resolve live owner semantics from the prepared head and its exact product compilation
- verify every package receipt named by that compilation before owner-body hydration
- remove the legacy root receipt writer and live current-head selector
- bind Agent authority observation to the same prepared compilation used by assembly

Lower Agent registration and subscription mutations remain private implementation mechanisms behind Agent genesis. Raw Agent and subscription writers are crate-private. Existing package installation and native owner registries remain canonical and reusable.

## Exact Boundary

The proof begins with the configured principal selection, physical binding, and exact installed package receipt. It ends with a durable inert prepared closure and prepared product head.

The endpoint proves no participant is running, ready, current, admitted, or retired. Those claims require `WMR-VC-06`.

## Direct Product Proof

- Docs Freshness completes install, compile, assignment, Agent genesis, Belief acceptance, topology, Capability preparation, prepared closure, unchanged rerun, and close plus reopen reconstruction
- Dependency Security compiles its own dissimilar package, topology, observation component, subscription contract, directive, and participant plan through the same structural PDS compiler

## Verification

- `cargo fmt --all -- --check`
- `cargo check --workspace --all-features`
- `cargo test --workspace --all-features -- --test-threads=1`
- `cargo clippy --workspace --all-targets --all-features -- -A clippy::result_large_err -D warnings`
- Agent genesis identity and conflict tests
- Belief source acceptance identity tests
- product compilation completeness and reopen tests
- Docs root runtime and unchanged rerun proof
- Dependency Security structural product proof
- canonical route and retired-symbol scans
- `git diff --check`

The unmodified strict Clippy command may still expose the repository-wide `result_large_err` baseline. That baseline is not relaxed by this packet.

## Stop Conditions

Return to design if implementation requires another Event authority, Graph authority, Agent store, Capability registry, product store outside the existing theory database, runtime coordinator, lifecycle generation, compatibility writer, or semantic interpretation inside PDS.

## Authority Boundary

This packet authorizes only the `WMR-VC-05` implementation candidate. It does not authorize independent acceptance claims, commit, push, deployment, `WMR-VC-06`, Startup proof, or either product migration.
