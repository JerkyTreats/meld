# Runtime Supervisor Domain Plan

Date: 2026-06-16
Status: historical supervisor design evidence
Scope: root `meld` runtime supervisor domain for lifecycle, leases, health, and operator visibility

Current runtime-completion authority is [Runtime Completion Ground Map](runtime_completion_ground_map.md), followed by [Runtime Completion Implementation Workstreams](runtime_completion_implementation_workstreams.md). This document preserves supervisor boundary evidence but does not set the current completion sequence.

## Purpose

Root `meld` needs a real supervisor domain. It owns runtime lifecycle and operator visibility for domain runtimes while leaving semantic progress inside the owning domain stores.

The supervisor starts the flywheel, keeps runtime instances unique, tracks health, handles shutdown, and prevents duplicate work processes. It is focused infrastructure. It is not the cognitive loop and does not own the meaning of events, beliefs, goals, tasks, publications, or satisfaction.

## Design Position

The flywheel remains:

```text
world model runtime
-> execution runtime
-> event spine
-> world model runtime
```

The supervisor sits beside that loop:

```text
root supervisor
-> start domain runtimes
-> monitor lifecycle
-> coordinate shutdown
-> report health
```

Domain runtimes carry momentum through durable handoff ports. The supervisor owns whether a runtime process should exist, not what semantic work that runtime should commit next.

## Supervisor Authority

The supervisor owns:

- runtime identity
- desired runtime set
- runtime instance registration
- process lifecycle
- runtime leases
- heartbeat records
- health snapshots
- restart policy
- graceful shutdown
- store flush ordering
- bounded diagnostic reports
- operator status views

This is legitimate root authority because it protects the product process from lost workers, duplicated workers, stale handles, and unobservable failure.

## Supervisor Non Authority

The supervisor must not own:

- event truth
- event sequence
- graph replay cursors
- belief evidence cursors
- belief views
- agent delivery cursors
- agent curation decisions
- execution goal lifecycle
- planning results
- task network revisions
- task dispatch claims
- task outcome records
- pending publication state
- satisfaction evidence decisions

Each item remains in the domain store that gives it meaning.

## Target Module Shape

Root runtime modules should make supervisor concerns explicit.

```text
src/runtime.rs
src/runtime/assembly.rs
src/runtime/contracts.rs
src/runtime/error.rs
src/runtime/ports.rs
src/runtime/storage.rs
src/runtime/supervisor.rs
src/runtime/supervisor/contracts.rs
src/runtime/supervisor/health.rs
src/runtime/supervisor/leases.rs
src/runtime/supervisor/restart.rs
src/runtime/supervisor/shutdown.rs
src/runtime/supervisor/store.rs
```

This uses `parent.rs` plus `parent/child.rs` module layout. Do not use `mod.rs`.

## Product Storage

The product root should include a supervisor store beside domain stores.

```text
<product_root>/
  supervisor.sled/
  ledger.sled/
  workspace.sled/
  world_model.sled/
  execution/
    goals.sled/
    task_artifacts.sled/
    task_networks/
```

`supervisor.sled` stores lifecycle records only. It does not store semantic progress.

## Core Records

The supervisor should persist stable records with explicit ownership.

```rust
pub struct RuntimeInstance {
    pub instance_id: String,
    pub product_root: PathBuf,
    pub started_at_ms: u64,
    pub status: RuntimeInstanceStatus,
}

pub struct RuntimeDesiredState {
    pub runtime_id: String,
    pub enabled: bool,
    pub restart_policy: RestartPolicy,
}

pub struct RuntimeLease {
    pub runtime_id: String,
    pub lease_id: String,
    pub instance_id: String,
    pub acquired_at_ms: u64,
    pub expires_at_ms: u64,
    pub status: RuntimeLeaseStatus,
}

pub struct RuntimeHeartbeat {
    pub runtime_id: String,
    pub lease_id: String,
    pub observed_at_ms: u64,
    pub health: RuntimeHealth,
    pub last_report: Option<WorkerTickReport>,
}

pub struct RuntimeHealthSnapshot {
    pub runtime_id: String,
    pub lease_id: Option<String>,
    pub status: RuntimeHealthStatus,
    pub last_heartbeat_at_ms: Option<u64>,
    pub retryable_error_count: u64,
    pub fatal_error_count: u64,
}
```

The records are supervisor facts. They can reference domain runtime ids and diagnostic summaries, but they must not duplicate domain state.

## Runtime Ids

Runtime ids should be stable, human readable, and domain scoped.

```text
events.ledger
world_model.graph.reducer
world_model.belief.docs_freshness
world_model.agent.seed.docs_freshness
execution.goals
execution.planning
execution.task_network.network-docs
execution.publication.network-docs
```

The runtime id identifies the supervised process. Domain stores still own their own record ids and cursors.

## Lease Contract

The lease contract prevents duplicate runtime ownership.

- acquire a lease before starting a runtime handle
- permit one active lease per runtime id
- renew the lease while the runtime is healthy
- record heartbeats against the active lease
- release the lease during graceful shutdown
- mark expired leases during supervisor startup recovery
- never use the lease as a domain cursor

If a runtime cannot acquire its lease, it must not start.

## Startup Flow

1. Resolve product root.
2. Open product stores.
3. Open supervisor store.
4. Register runtime instance.
5. Load desired runtime set.
6. Mark expired leases from older instances.
7. Build flywheel ports.
8. Start enabled runtimes after lease acquisition.
9. Begin heartbeat monitoring.
10. Surface health through supervisor query APIs.

This sequence starts infrastructure and then lets each domain runtime drive its own work.

## Shutdown Flow

1. Stop accepting new start requests.
2. Signal cancellation to supervised runtime handles.
3. Let each runtime reach its domain safe point.
4. Flush domain stores through owning adapters.
5. Write final heartbeat or stopped record.
6. Release runtime leases.
7. Mark the runtime instance stopped.

Shutdown order should protect durability before process exit.

## Restart Policy

The first implementation should support conservative restart policy values.

- `never`
- `on_retryable_failure`
- `on_heartbeat_expiry`

Restart policy must not reinterpret domain failure semantics. A domain runtime reports failure. The supervisor decides whether to restart the handle, then the domain runtime resumes from its own store.

## Event Spine Relationship

Supervisor lifecycle records should not be appended to the canonical event spine by default. Heartbeats and leases are operational lifecycle detail, not product world facts.

Supervisor summaries may become event facts only through an explicit policy that elevates an operational condition into product meaning.

## Domain Handle Boundary

Each supervised runtime handle should expose a narrow lifecycle contract.

```rust
pub trait SupervisedRuntime {
    fn runtime_id(&self) -> RuntimeId;
    fn start(&mut self, context: RuntimeSupervisorContext) -> Result<(), RuntimeSupervisorError>;
    fn stop(&mut self) -> Result<(), RuntimeSupervisorError>;
    fn heartbeat(&self) -> RuntimeHeartbeat;
}
```

The handle is a lifecycle wrapper. It does not expose domain internals to root.

## Implementation Phases

### Phase S1 - Supervisor Store

Add `supervisor.sled` to the product storage layout. Define tables or trees for instances, desired states, leases, heartbeats, health snapshots, and lifecycle events.

Exit criteria:

- product assembly opens supervisor storage
- supervisor storage flushes independently
- no domain cursor is stored in supervisor records

### Phase S2 - Supervisor Contracts

Define runtime ids, desired state, leases, heartbeats, health snapshots, restart policy, and lifecycle errors under root runtime supervisor contracts.

Exit criteria:

- records serialize and round trip
- invalid ids are rejected
- lease status changes are explicit

### Phase S3 - Lease Manager

Implement acquire, renew, release, and expiry recovery.

Exit criteria:

- duplicate active lease acquisition fails
- expired leases are recovered at startup
- stale lease owners cannot renew after replacement

### Phase S4 - Runtime Registry

Create a registry of available runtime factories and desired runtime state.

Exit criteria:

- enabled runtime ids map to concrete factories
- disabled runtime ids do not start
- missing factories produce supervisor diagnostics

### Phase S5 - Supervised Handles

Wrap event, world model, and execution runtimes behind lifecycle handles.

Exit criteria:

- supervisor starts domain runtimes after lease acquisition
- each runtime reports heartbeat
- domain work still flows through direct handoff ports

### Phase S6 - Shutdown And Flush

Add coordinated cancellation, domain safe point waits, store flushes, lease release, and stopped instance records.

Exit criteria:

- shutdown can run twice without corrupting state
- stores flush before final lease release
- stopped instance is visible in supervisor status

### Phase S7 - Health And Status

Expose a supervisor query API and later a thin CLI status command.

Exit criteria:

- operator can see runtime ids, lease owners, heartbeat ages, restart counts, and last diagnostic outcome
- status output does not read semantic domain internals

### Phase S8 - Restart Policy

Add conservative restart behavior for retryable failure and heartbeat expiry.

Exit criteria:

- restarted runtime reacquires a new lease
- old lease owner cannot commit heartbeat
- resumed work comes from domain stores

## Verification

```sh
cargo test runtime::supervisor
cargo test runtime::storage
cargo test runtime::contracts
rg -n "root-owned|root owned|root cursor|shared cursor table" src design/plan/integration
```

## Acceptance

The supervisor domain is ready when root `meld` can start, observe, stop, and restart domain runtimes without owning their semantic progress.

The durable foundation is preserved when domain momentum still flows through direct ports:

```text
world model -> execution -> events -> world model
```
