# WMR-VC-05 Source Activation Record

Date: 2026-09-05

Slice identifier: `WMR-VC-05`

Status: Gate accepted; slice closed; cleanup, commit, and push authorized

Source baseline: `a7032bbf`

Authority: direct program-owner authorization for `SI-VC-05`

Frozen gate: [WMR-VC-05-DG](wmr_vc_05_product_compilation_agent_genesis_gate.md)

Worker packet: [WMR-VC-05 worker packet](../detailed_design/wmr_vc_05_worker_packet.md)

Source assessment: [WMR-VC-05 source assessment](../product_compilation_and_agent_genesis_source_assessment.md)

## Activated Write Set

- add complete PDS product declarations, compilation receipts, heads, assignment, topology, preparation, closure, and prepared-head persistence
- extend stewardship assignment and activation identities to exact product lineage
- add Agent-owned genesis intents, subscription requests, canonical Event-backed publication operations, receipts, and assignment lineage fences
- add Belief-owned exact subscription acceptance
- add Capability-owned exact inert preparation receipts
- add a structural participant plan to the prepared closure
- add `prepare-activation` as an explicit world-init stage
- migrate world init, tooling, runtime assembly, and product fixtures to the successor route
- rehydrate runtime Capability bindings only from an exact prepared product closure
- add Docs Freshness idempotency and reopen proof
- add dissimilar Dependency Security compilation proof

## Retired Authority

The production world-init route no longer directly calls Event append, seed Agent registration, or Agent subscription mutation for Agent genesis. `AgentGenesis` retains the injected canonical append capability and owns publication. Runtime assembly no longer invents compatibility assignment and activation identifiers. World initialization has no reduced compatibility genesis body and fails closed without complete product inputs. A lifecycle label no longer proves complete product genesis. Descriptor inference no longer produces the prepared product evidence.

The lower `AgentRegistration` and `AgentSubscription` facades are no longer exported. Their modules and raw Agent and subscription writers are crate-private, and the storage integration fixture now creates Agent state through `AgentGenesis`.

The user authorized a focused retirement correction after reviewing the remaining source surface. The correction removes the harness-owned initialization route and its public request payload, the alternate world-init theory installers, caller-supplied semantic world-init content, loose runtime theory injection, the unused startup activation writer and store, the public Event append proof minting surface, and the dead Agent subscription command. The harness retains only an empty historical manifest field for schema reads. The canonical lifecycle service remains unchanged and later lifecycle behavior is still outside this slice.

The fourth correction seals the remaining ownership escapes. Belief now returns an opaque acceptance proof only after its authority validates and durably persists the exact request. Agent genesis consumes that proof by request identity and cannot construct or persist the Belief decision. The raw acceptance writer is crate-private.

Live runtime theory resolution now begins at the prepared product head, resolves its exact closure, declaration, compilation, and every package receipt named by the compilation, reconstructs the compilation for equality, and only then resolves owner bodies. Capability activation and Agent authority fencing consume that same resolved closure and compilation. The legacy root theory receipt store is read-only, has no current head, and resolves historical records only by explicit receipt identity.

## Persisted Data Decision

Existing immutable package and native owner revisions remain valid inputs. New product and genesis records are additive content-addressed aggregates in existing databases. No evidence required a compatibility writer or data rewrite. Conflicting immutable Agent identity or lineage now rejects instead of silently reusing the old identifier.

## Scope

The implementation stays within the existing crates, services, and database roots. It adds no dependency, Event authority, Graph authority, Task Network, runtime coordinator, or lifecycle generation.

No numeric change budget was frozen before the initial implementation. The fourth corrected worktree changes thirty-seven production Rust paths with 4,470 added and 2,147 removed lines, including focused tests collocated with owner modules. The missing initial budget is retained as an evidence limitation rather than retroactively invented. The correction tripwire is structural and permits no new crate, store, service, writer, compatibility path, or later-slice behavior.

## Logical Review History

Logical review failed three times on 2026-09-05. Each failure returned the slice to implementation correctness and invalidated later assurance. The first four findings were classified as implementation defects:

- Agent completion accepted a ledger position without proving that the position contains the canonical genesis Event record
- prepared-head installation could combine individually valid records from different product lineages
- product genesis could run without the declared compilation installed as the current product head
- Belief acceptance returned before its owner store established the flush boundary

The second review confirmed those four defects were substantially corrected and identified two remaining authority defects:

- Agent completion accepted an exact Event proof from a foreign ledger because root supplied the proof instead of Agent owning publication through the runtime capability
- world initialization retained a reduced production-reachable genesis route that manufactured assignment and product compilation identities without a complete product aggregate

The third review confirmed both prior blockers were closed and identified two remaining ownership escapes:

- a public acceptance constructor and raw writer allowed callers to bypass Belief-owned durable acceptance
- runtime theory and Agent authority policy still selected live semantics through the legacy root receipt head or an unrelated package head instead of the exact prepared product compilation

The frozen affected-domain set for the fourth correction is Belief acceptance, Agent genesis proof consumption, PDS product aggregation, runtime theory resolution, runtime assembly, and the Agent authority port. Capability remains a publisher of an exact preparation receipt and a consumer of the same resolved closure. Events, Graph, Task Network, lifecycle, package compilation, and product semantics are explicit non-integrations for this correction.

The prior bounded corrections changed Agent-owned Event publication, root and harness genesis routing, retirement of the reduced compatibility body, lower Agent mutation exports, retired construction surfaces, focused adversarial tests, and delivery evidence. The fourth correction is limited to producer-owned Belief proof issuance, raw writer confinement, prepared-compilation runtime hydration, removal of the legacy live selector and writer, Agent authority-port rewiring, bypass-test inversion, obsolete characterization removal, and this evidence. A new crate, store, service, writer, compatibility path, lifecycle behavior, or unrelated cleanup remains a stop condition requiring renewed design authority.

## Assurance State

The prior green checks remain historical evidence only because they preserved both new bypasses. The fourth corrected candidate requires Belief authority proof for every Agent completion fixture and proves durable acceptance survives reopen. Runtime reopening after newer loose owner revisions now retains the exact prepared compilation and its owner bodies. Historical root receipts remain readable only by explicit identity. The candidate also retains all earlier Event, product-coherence, stage-order, Capability-selection, and durability proofs.

Exact candidate `WMR-VC-05::2a8c32ed802b48275b711c916b4e7a410874c99f7a2323dfcfc0a265391dc850` passed fresh logical review, but its Style Assurance evidence was later found contradictory. Strict rustdoc reported eleven diagnostics rather than the recorded three. Ten were baseline diagnostics and one was introduced when public documentation linked to the crate-private `WorldInitPipeline`. This reopened Style Assurance and invalidated its dependent gate projection.

The bounded correction replaces that public link with accurate non-linking language. Exact successor candidate `WMR-VC-05::5c866f0fe0ad82033deba8191aad80972703cdff13d8f3e75e016de7bef25186` passed [narrow logical confirmation](../reviews/wmr_vc_05_style_successor_logical_confirmation.md) and then [corrected Style Assurance](../reviews/wmr_vc_05_style_assurance_receipt.md). Strict rustdoc now reports exactly the ten baseline diagnostics and no successor diagnostic. Formatting, workspace compilation, warning-clean Clippy with the documented repository allowance, focused world-init tests, the complete serialized workspace suite, exact-manifest verification, and diff hygiene pass. The final [Gate Acceptance receipt](wmr_vc_05_gate_acceptance_receipt.md) accepts the exact successor with no violation or exception.

## Closeout

The accepted product behavior begins with exact principal-selected package receipts and ends at one durable inert prepared product closure and prepared head. Product aggregation, Agent genesis, Belief acceptance, Capability preparation, and runtime hydration each have one canonical owner path. Superseded writers, selectors, compatibility initialization, startup activation storage, and exclusive characterization fixtures are retired.

The successor manifest contains fifty-four source and executable-test paths. Thirty-seven production Rust paths changed with 4,470 additions and 2,147 removals under the recorded program account. The slice adds no crate, dependency, database, service, Event authority, Graph authority, Task Network, runtime coordinator, or lifecycle generation.

Program-owner authority closes `WMR-VC-05` at the accepted successor. Separate authority authorizes cleanup, commit, and branch push. Deployment, `WMR-VC-06`, and every later slice remain unauthorized.

## Commit Effect

If applied, this commit makes exact product compilation, owner-bound Agent genesis, durable Belief acceptance, complete topology, inert Capability preparation, and prepared-product runtime hydration the canonical route while removing the superseded compatibility writers, selectors, initialization path, startup activation store, and exclusive fixtures.

## Authority Boundary

The completed authorization ends at an inert prepared activation closure. `WMR-VC-06` and every later source slice remain unauthorized.
