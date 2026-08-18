# PDS Isolation And Runtime Portability

Date: 2026-08-15
Status: proposed discovery model
Scope: isolation semantics for PDS package installation, assignment, activation, external runtimes, capability invocation, result admission, and replay

## Purpose

PDS may attach domain behavior implemented as a linked library, owned subprocess, shared sidecar, remote service, or persistent external controller. Those placements have different containment properties, but they must preserve the same Meld authority boundaries and domain result contracts.

This document separates the isolation guarantees that remain stable across placements from the enforcement mechanisms that remain implementation choices. It complements the [PDS Router Detailed Design Specification](../plan/integration/pds_router_design_spec.md) and the [canonical runtime lifecycle and quiescence design](../cognitive_architecture/runtime_lifecycle_and_quiescence.md) without turning `theory::router` into a plugin host or process supervisor.

## Working Model

The strongest current model is:

```text
package receipt
    pins state-free semantic meaning

assignment
    binds principal, authority context, perspective, and subject scope

activation
    binds physical resources and an isolation requirement set

activation generation
    fences one assignment-wide set of live authority and exact activation inputs

participant incarnation
    realizes one expected participant through one operational ownership lifetime

operation attempt or passive delivery
    performs one fenced capability attempt or carries one authenticated source notification

admitted result
    becomes domain truth only after owner validation and lineage checks
```

These identities answer different questions. A package receipt says what meaning was selected. An assignment says whose standing responsibility and authority apply. An activation says which physical bindings may realize it. An activation generation says which assignment-wide authority fence governed it. A participant incarnation says which operational ownership lifetime handled the work. An operation and attempt say which authorized capability work produced a result. A passive delivery instead names the admitted subscription, delivery, and source cursor.

No one identity substitutes for another.

## Isolation Is A Vector

PDS isolation is not a boolean and is not equivalent to process placement.

| Axis | Protected boundary | Required property |
| --- | --- | --- |
| semantic | package and owner theory revisions | one assignment cannot silently observe another revision |
| normative | principal, perspective, scope, and authority | shared machinery does not merge mandates or grants |
| binding | credentials, endpoints, workspaces, and executables | contributors receive only declared owner bindings |
| state | Meld stores, owner stores, and external operational stores | state ownership and namespace remain explicit |
| failure | process, service, queue, and retry fate | shared failure is declared rather than inferred from placement |
| resource | compute, memory, concurrency, rate, and time | one activation cannot consume an undeclared global budget |
| effect | files, branches, services, and other mutation targets | execution arbitrates conflicts using exact effect identity |
| admission | callbacks, scanner output, model output, and observations | only owner-validated results enter canonical domain products |
| replay | historical package, assignment, activation, and source lineage | current heads or bindings never reinterpret old work |

A deployment can be strong on one axis and weak on another. A dedicated process may still use a shared credential or database. A remote multi-tenant service may provide excellent process containment while offering only logical state separation. A linked library may be acceptable for trusted deterministic code when binding, effect, admission, and replay isolation remain strong.

## Stable Isolation Invariants

### Installation Is Not Code Execution

Package parsing, structural linking, owner validation, owner installation, and exact resolution operate over inert data. They do not load package-selected native libraries, run commands, open package-named endpoints, or expose credentials.

`theory::router` owns this installation boundary. It does not supervise live adapters.

### Package Identity Is Placement Neutral

Equivalent physical realizations may activate the same package receipt. Process ids, hostnames, credentials, endpoints, resource limits, and adapter placement do not participate in package identity.

Placement neutrality preserves semantic identity. It does not promise byte-identical observations from two changing external sources. Result equivalence means the same domain schema, validation rules, authority boundaries, evidence classes, and lineage obligations apply.

### Assignment Authority Never Flows Through Sharing

Two assignments may share a package receipt, process, service, cache, or product runtime. Sharing does not merge principals, authority grants, scopes, perspectives, budgets, or current maintained-condition state.

Every invocation remains attributable to exactly one assignment and one effective-authority decision.

Passive source notifications are not capability invocations. They remain attributable to one admitted assignment subscription, activation generation, adapter delivery identity, and source cursor or revision without inventing an execution claim.

### Bindings Are Owner Scoped

An activation contributor receives only the binding ids it declared. A shared runtime adapter must not gain access to foreign owner bindings merely because root assembly can resolve them.

Credential values remain behind binding resolvers. Receipts and diagnostics carry stable refs or redacted identities, never secret material.

### Prepared Closure Is Not Live Authority

Owner preparation, binding closure, and draft executor construction remain inert. One aggregate prepared receipt proves only that the activation can proceed to runtime start.

Runtime objects then start with admission closed and return generation-scoped readiness evidence. An expected-prior-generation fence publishes the ready generation as current. Only that publication opens new work admission.

This separation prevents a successful constructor or partial process start from becoming live authority and lets a failed neighboring activation remain isolated.

### Activation Generations And Participant Incarnations Fence Live Realizations

Credential rotation, endpoint change, selected implementation change, isolation requirement change, and activation replacement create a new activation generation even when package and assignment identity remain unchanged.

An equivalent mechanical restart may create a new participant incarnation under the same activation generation only after recovery reconciliation proves the exact activation inputs and authority fence remain valid. Changed inputs require a new activation generation.

A stale activation generation or participant incarnation cannot silently publish a result as though it came from the current realization. The owning domain must classify a late result as rejected, historical-only, or eligible for current admission under an explicit result policy.

### Invocation Is Fenced Separately From Runtime Lifetime

One participant incarnation can perform many attempts and deliveries. External work keeps four identities separate:

```text
durable operation key
    stable for one semantic task unit and lifecycle epoch across retries

attempt id
    unique for each dispatch attempt

activation generation
    identifies the assignment-wide authority fence used by that attempt

participant incarnation
    identifies the operational ownership lifetime used by that attempt
```

The current execution task lifecycle epoch is the closest implemented primitive for a durable operation fence. A dispatch claim includes worker identity and is attempt lineage, so it cannot automatically serve as a cross-worker external idempotency key. A remote request id or process-local attempt id is also not a replacement for the durable operation key.

### External Output Is Untrusted Input

Successful transport proves only that bytes arrived. Process exit zero, HTTP success, webhook delivery, and model completion do not prove domain truth.

The destination domain validates scope, source revision, completeness, schema, evidence class, adapter identity, and either operation plus attempt or passive-delivery lineage before admitting a canonical product. The canonical event layer then preserves that admitted product and provenance.

### Side Effects And Evidence Remain Distinct

An external runtime may both mutate an environment and report an assessment. The mutation attempt, mutation evidence, subsequent observation, and domain verdict remain separate records.

Retry after an ambiguous transport failure must reuse the durable operation key under a new attempt id and current activation generation plus participant incarnation, or perform capability-specific reconciliation first. It must not assume that lack of a response means lack of an effect.

### Shared State Is Declared

External advisory databases, model caches, scanner indexes, and service queues may be shared. Their sharing scope and source revision obligations belong to activation and adapter contracts.

Shared operational state does not become Meld truth. Any shared state that affects a canonical result is named through source identity, revision, observation time, or an explicit bounded uncertainty state.

### Deactivation Stops New Authority

Deactivation prevents new invocations and invalidates the current activation generation. It does not erase durable facts, execution attempts, or results already admitted under valid lineage.

Whether an already authorized in-flight observation may arrive after deactivation is an owner policy question. No late result may silently enter the current projection without its retired generation and participant incarnation being visible.

## Candidate Portable Activation Contract

The following shapes are illustrative. They express information that every placement must settle without selecting one sandbox technology.

```rust
struct RuntimeIsolationRequirements {
    sharing: RuntimeSharingRequirements,
    state_scope: StateIsolationScope,
    failure_scope: FailureIsolationScope,
    secret_binding_ids: Vec<String>,
    filesystem_grants: Vec<FilesystemGrantRef>,
    network_grants: Vec<NetworkGrantRef>,
    resource_budget_ref: Option<String>,
    late_result_policy: LateResultPolicy,
}

struct RuntimeSharingRequirements {
    dedicated_activation: bool,
    equivalence_keys: Vec<RuntimeSharingEquivalenceKey>,
    shared_service_allowed: bool,
}

enum RuntimeSharingEquivalenceKey {
    SameAssignment,
    SamePrincipal,
    SamePackageReceipt,
}

struct ParticipantIncarnationReceipt {
    participant_id: String,
    participant_incarnation_id: String,
    activation_id: String,
    activation_generation_id: String,
    owner_domain: String,
    adapter_identity: String,
    adapter_version: String,
    placement_claim: AdapterPlacement,
    isolation_claim: IsolationRealizationClaim,
    binding_set_hash: String,
    started_at_seq: u64,
}

struct AdapterInvocationContext {
    assignment_id: String,
    package_receipt_id: String,
    activation_id: String,
    participant_id: String,
    participant_incarnation_id: String,
    activation_generation_id: String,
    capability_contract_ref: String,
    operation_key: String,
    execution_claim_id: String,
    attempt_id: String,
}
```

Sharing equivalence keys are conjunctive rather than ordered. `SamePrincipal` and `SamePackageReceipt` are independent constraints, and a valid sharing group must satisfy both when both are selected. A dedicated-activation requirement overrides shared-service permission.

An isolation realization claim records what the chosen placement says it enforces. For each axis it distinguishes required property, realization mode, enforcing authority, verification evidence, and activation decision. Realization modes include hard enforcement, cooperative enforcement, external attestation, and unsupported. A self-asserted adapter claim cannot satisfy a required hard boundary.

Filesystem grants are physical access contracts rather than semantic execution effects. They name root identity, access mode, writable subpaths, and symlink policy. Network grants separately name egress, authenticated callback ingress, endpoint identity, and rate budget. Resource policy distinguishes logical work budgets from verified hard quotas.

The binding-set hash includes canonical non-secret refs and visible secret-version identities. It never includes secret values.

The activation boundary should fail closed when the selected placement cannot satisfy required binding, sharing, or containment properties. PDS need not understand how a domain adapter constructs a container, subprocess, client, or library handle.

Prepared-activation closure and current-generation publication are separate receipts. The first closes owner preparation, the exact participant plan, and draft invokers. The second cites exact readiness receipts and conditionally advances the assignment from an expected prior generation.

Every expected participant declares its readiness reference, structural wake references, owner safe-point reference, stop reference, and lifecycle dependencies. A participant may report active idle only with a durable owner reason and resolvable wake path. Quiescence is an assignment-wide conclusion over the complete participant plan, not a status returned by one process.

## Placement Models Under Pressure

| Placement | Useful first fit | Isolation that needs explicit proof | Main trap |
| --- | --- | --- | --- |
| linked in-process adapter | trusted deterministic parser, resolver, or scanner library | binding scope, panic containment, resource budget, and result admission | treating trusted code as semantically authoritative |
| owned subprocess | local scanner or converter with a stable command contract | executable identity, environment allowlist, filesystem grants, termination, and retry | assuming a process boundary also isolates credentials or external state |
| shared sidecar | expensive local index or service reused by assignments | tenant namespace, request lineage, source revision, queue fairness, and crash sharing | cross-assignment cache or callback confusion |
| remote request service | hosted model, scanner, or resolver | endpoint identity, transport authentication, timeout, rate budget, and result validation | treating HTTP success as a domain verdict |
| persistent external controller | portfolio monitor, dependency service, or source watcher | cursor ownership, webhook authenticity, assignment correlation, source advance, and replay | allowing hidden external decisions to replace Meld cognition |

No placement is the canonical PDS runtime. A domain may support only a subset. The package states semantic needs, activation chooses an available realization, and owner adapters preserve the domain contract.

Implementation discovery is independent of placement. Product or administrator composition may publish zero or more implementation offers for one exact capability contract. Activation selects exactly one offer into its local executor view. Registration changes create a new catalog generation for later activation and never mutate an active or historical view. A PDS package declares action-class meaning and constraints. It does not select the exact capability contract, native library, executable, or endpoint placement.

## Runtime Sharing And Cardinality

Four cardinalities must remain independent:

```text
one package receipt
→ many assignments

one assignment
→ one current activation and historical activations

one activation
→ one current activation generation and historical generations

one activation generation
→ many expected participants

one expected participant
→ historical participant incarnations

one participant incarnation
→ many fenced attempts and passive deliveries
```

A product may later allow several simultaneous participant incarnations for one participant role to support scale or redundancy. That does not change semantic identity, but it requires deterministic operation ownership, attempt routing, duplicate suppression, callback correlation, and bounded result ordering.

Sharing must satisfy the conjunction of every selected owner constraint. Incomparable constraints accumulate instead of overwriting one another. Declaration order never chooses the result.

## Failure And Recovery Semantics

### Failure Before Effect

The operation remains retryable under the same lifecycle epoch and durable operation key when the adapter can prove that no external effect began. A new dispatch records a new claim and attempt id.

### Ambiguous Effect

If transport fails after dispatch, Meld records an unresolved attempt. Retry preserves the durable operation key while recording a new attempt id, activation generation, and participant incarnation, or first performs a capability-defined reconciliation observation.

### Result After Generation Or Incarnation Change

The adapter preserves the old activation generation and participant incarnation on the result. The owner applies the declared late-result policy and never relabels it as current output.

### Participant Crash

Meld reconstructs Goals, task state, claims, and canonical evidence from Meld stores. External operational state may help resume acquisition, but hidden external Agent or planner state is not required to reconstruct why Meld acted. Admission remains closed until durable operations, deliveries, wake registrations, and owner cursors reconcile. An equivalent restart creates a new participant incarnation under the same activation generation. Any activation input or authority change creates a new generation.

### Shared Service Failure

Every affected activation receives a separately attributable operational failure. One service outage does not merge assignment lifecycle or authorize a fallback placement that violates the activation requirement set.

### Package Or Assignment Upgrade

Old in-flight work retains its exact package, assignment, activation, and execution lineage. Upgrade policy decides whether it may finish, is cancelled, or is admitted only as historical evidence. A current package head never rewrites it.

## Use-Case Pressure Set

The isolation model should survive at least these cases before a runtime contract is frozen.

| Case | Pressure applied | Failure exposed |
| --- | --- | --- |
| deterministic docs inspection | trusted local code and workspace reads without a provider | process isolation made universally mandatory |
| model-backed docs drafting | remote provider with bounded workspace write effects | provider credential or output crosses assignments |
| local dependency scan | subprocess reads one workspace and returns structured findings | exit status becomes security truth |
| persistent security monitor | external service advances advisories while the workspace is unchanged | hidden cursor or callback replaces canonical wake lineage |
| same package in two workspaces | shared semantics with distinct bindings and subjects | package identity absorbs environment identity |
| same subject under two principals | overlapping observation with distinct authority | shared service merges grants or decisions |
| concurrent old and new package revisions | historical and current work overlap | current heads reinterpret in-flight work |
| shared sidecar crash | one process serves several assignments | failure fate or retry state becomes cross-assignment state |
| credential rotation | activation changes without semantic package change | package revision is used as an activation generation fence |
| late remote callback | retired activation returns a validly signed result | stale output enters current truth without policy |

Docs freshness and dependency security are useful as a pair because they pressure opposite ends of this set. Docs proves that trusted local composition should remain possible. Dependency security proves that long-lived external state, callbacks, source revision, and multi-operation tools cannot be hidden behind one generic success result.

## Replay Classes

Historical interpretation replay reads admitted events and domain records under exact package, assignment, activation generation, participant incarnation, source, and adapter lineage. It makes no external call and does not require the old runtime placement to remain available.

Deterministic recomputation is optional. It is valid only when exact recorded inputs and an exact deterministic implementation remain available.

Live re-observation is new work. It receives a new attempt or passive-delivery identity and may observe a newer external source revision. It must not be presented as replay merely because the same package initiated it.

This distinction lets Meld explain historical decisions while an external runtime is offline without claiming it can reproduce a changing external database.

## Ownership Boundary

| Concern | Owner |
| --- | --- |
| package structure, routes, exact component closure | `theory::router` |
| package receipt and exact package resolution | theory |
| assignment and activation selection | configuration and PDS control plane candidate |
| participant incarnation lifecycle | runtime composition and owner adapter |
| isolation requirement declaration | owner activation contributor plus product policy |
| effective authority and dispatch fence | Agent and execution authorities |
| effect conflict arbitration | execution |
| transport validation and domain result admission | destination domain adapter |
| canonical event append and replay | events |
| participant operational diagnostics | runtime supervisor and owner adapter |

The router may carry activation requirement refs and exact owner identities. It must not become the owner of process supervision, network clients, scanner sessions, provider sessions, or domain result validation.

## Open Threads

The following remain unresolved:

- whether isolation requirements are stored directly in activation records or returned only by owner contributors
- whether participant incarnation receipts belong to runtime, configuration, or a PDS projection
- which late observation classes may remain admissible after deactivation
- whether shared sidecars need a product-wide tenant contract or only domain-owned namespace contracts
- which resource budgets require durable semantic lineage rather than operational diagnostics
- how package upgrade cancellation interacts with already authorized external effects
- whether a future untrusted executable adapter needs a common sandbox provider contract

These are implementation-shaping questions. They do not weaken the stable separation among semantic identity, assignment authority, activation bindings, activation generation, participant incarnation, operation and attempt lineage, passive delivery, and domain admission.

## Falsification

Reject or narrow this model if any of the following become true:

- every supported adapter requires the same concrete process topology
- placement-neutral contracts cannot preserve domain result meaning across two implementations
- activation generation, participant incarnation, durable operation, attempt, and passive-delivery identity cannot prevent stale or duplicate admission
- a shared service requires assignments to share authority, perspective, or current state
- owner adapters cannot validate external output without exposing foreign stores or bindings
- replay depends on a current endpoint, package head, or hidden external planner decision
- the router must interpret process, credential, or domain result internals to preserve closure

Until those cases are tested, isolation requirements and runtime receipts remain proposed portable contracts rather than frozen PDS schemas.
