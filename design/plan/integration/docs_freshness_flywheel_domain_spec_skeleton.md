# Docs Freshness Flywheel Domain Spec Skeleton

Date: 2026-06-22
Status: proposed
Scope: build-facing domain skeleton for one executable `docs_freshness` flywheel

## Purpose

This document decomposes the next `docs_freshness` flywheel iteration into domain-owned spec skeletons. It is intended as the handoff plan for implementation orchestration.

The source report describes the goal and scope. This document turns that scope into buildable contracts, ordered dependencies, and verification targets.

## Build Principle

Each domain owns its own durable nouns and runtime verbs.

Root assembly and supervisor wire and run. They do not decide semantic truth, goal value, belief meaning, or task meaning.

The first implementation may use narrow physical configuration, but records and APIs should use the intended durable boundaries.

Workspace scan is goal-driven execution work in this slice. Activation and bootstrap prepare durable intent and subscriptions, then low docs freshness belief creates an active goal. Planning may include `workspace_scan` work only after that active goal exists.

Planning owns decomposition into task network structure. It may emit one ordered graph or later graph deltas, but docs writer work must depend on scan receipt and snapshot availability whenever workspace state is missing or stale.

This slice follows the current codebase shape: world model agents curate goals, execution planning proposes task network graph commands, task network state is the executable plan, dispatch owns side effects, and publication feeds the event spine back into belief.

## Delivery Waves

Wave zero defines shared contracts that later domains consume.

- activation config shape
- directive record shape
- workspace scan capability output shape
- docs writer package trigger request shape
- runtime handle tick report shape
- integration proof fixture identity map

Wave one wires activation, bootstrap, and first belief projection.

- activation config loads
- directive and seed agent bootstrap is idempotent
- missing docs freshness evidence can project low confidence without fixture constants

Wave two wires planning to execution.

- low belief creates an active goal
- docs freshness method can decompose into `workspace_scan` work upstream of docs writer work when workspace state is missing or stale
- task dispatch can execute scan and then execute the package with provider access

Wave three wires publication, evidence, and satisfaction.

- task outcome produces a pending publication
- publication appends canonical events
- evidence ingestion revises belief
- satisfaction closes the goal
- reopen checkpoints prove durable recovery

## Product Assembly And Activation Config

Owner: root product assembly

Build item: single docs freshness activation file

Contract skeleton:

- `DocsFreshnessActivationConfig`
- source format is parsed only by product assembly
- supported initial format is chosen by implementation and maps into this DTO
- `subject_ref` as full `DomainObjectRef`
- `workspace_root`
- `target_selector`
- `belief_family_config_ref`
- `branch_scope`
- `perspective_policy`
- `directive_id`
- `directive_text`
- `seed_agent_id`
- `observation_scope`
- `curation_rule_id`
- curation threshold rule config or ref
- required artifact type
- publication event mapping config ref
- `method_id`
- `task_package_id`
- `task_network_id`
- `provider_binding_ref`
- `frame_type`
- `force_policy`
- enabled runtime ids

Inputs:

- workspace root
- repository config
- activation config file
- CLI overrides applied before validation
- provider availability config

Outputs:

- validated activation DTO
- runtime factory inputs for owning domains
- owner-scoped input package for world model belief
- owner-scoped input package for world model agent bootstrap and curation
- owner-scoped input package for execution planning, task package, task network, and publication
- diagnostics for invalid configuration

Durable records:

- none written by assembly

Idempotency:

- loading the same config is pure
- validation must not query semantic stores
- parsing the same activation file and overrides yields the same validated DTO
- downstream runtimes receive typed input values, not the raw activation document
- config drift is detected by owning domains when durable records already exist

Acceptance tests:

- valid activation file loads and resolves relative paths
- missing directive id fails validation
- missing subject ref fails validation
- missing semantic derivation inputs fail validation
- unsupported activation format fails validation before stores open
- invalid provider binding or required provider config absence fails before supervisor start
- assembly does not write directive, agent, goal, belief, task, or event records
- runtime factory inputs do not expose the raw activation document

Non goals:

- no belief evaluation
- no seed agent writes
- no goal creation
- no event append
- no runtime parses the activation file

Dependencies:

- directive record contract from world model agent
- belief key derivation contract from world model belief
- evidence mapping config contract from world model belief
- runtime id list from supervisor registry
- provider availability surface

## Workspace Scan Capability

Owner: workspace domain plus execution capability catalog

Build item: `workspace_scan` capability contract and adapter

Contract skeleton:

- capability type id `workspace_scan`
- capability version `1`
- owning domain `workspace`
- execution class `local_io`
- scope kind `workspace`
- scope ref kind `workspace_root`
- `WorkspaceScanReceipt`

Inputs:

- workspace root
- optional target selector
- scan policy
- session id
- ignore policy

Outputs:

- scan summary artifact
- workspace snapshot ref
- root node ref
- observed node refs
- workspace scan receipt

Effects:

- read filesystem
- update workspace node store
- flush workspace node store

Durable records:

- workspace node records
- workspace scan receipt

Forbidden effects:

- provider calls
- docs writing
- belief mutation
- goal mutation
- canonical event append

Idempotency:

- same root hash and no force returns existing snapshot refs
- force rebuild writes equivalent node records when content is unchanged
- scan receipt id is distinct from snapshot id
- same scan receipt id and same request content replays
- same scan receipt id with divergent request content fails

Acceptance tests:

- active docs freshness goal plus missing workspace state produces scan task work
- current workspace state does not produce duplicate node writes
- stale workspace state produces a new snapshot ref
- capability output includes full object refs
- capability output includes a durable workspace scan receipt
- no event envelope is appended directly by capability execution

Dependencies:

- current workspace scan service split into effect core and task artifact builder
- execution capability catalog registration
- task network artifact shape for scan summary
- durable `WorkspaceScanReceipt` shape

## World Model Agent Bootstrap

Owner: `meld-world-model` agent domain

Build item: directive, seed agent, curation rule, and subscription bootstrap runtime

Contract skeleton:

- `DirectiveRecord`
- `SeedAgentRegistration` with `directive_id`
- `AgentCurationRuleRecord`
- `AgentSubscriptionRecord`
- `AgentActivationRecord`
- bootstrap command
- bootstrap report

Inputs:

- activation config subset owned by world model agent
- subject ref
- branch scope
- perspective
- belief key derivation inputs
- bootstrap sequence

Outputs:

- directive record
- seed agent record
- curation rule record
- belief subscription record
- activation record
- operational agent status update
- bootstrap report

Durable records:

- directive record
- seed agent record
- curation rule record
- subscription record
- activation record
- bootstrap receipt

Idempotency:

- same directive id and same text replays
- same directive id and different text fails with config conflict
- same seed agent id and different subject fails with config conflict
- same curation rule id and different threshold fails with config conflict
- same subscription natural key replays

Acceptance tests:

- first bootstrap writes directive, agent, rule, and subscription
- second bootstrap with same config writes nothing new
- bootstrap conflict is fatal and names the conflicting field
- agent references directive id rather than embedding directive text as authority
- operational `AgentRecord.status` requires an active subscription and an activation record

Non goals:

- no belief confidence decision
- no goal command submission
- no satisfaction decision
- no event append

Dependencies:

- directive record schema
- conflict semantics in registration APIs
- activation config DTO

## World Model Belief Runtime

Owner: `meld-world-model` belief domain

Build item: docs freshness family snapshot loading and evidence revision path

Contract skeleton:

- `BeliefFamilyConfig` snapshot
- belief assessment request
- promoted evidence ingestion request
- belief view projection
- dirty key runtime tick report

Inputs:

- family config JSON or config ref
- graph anchor evidence
- content written evidence
- content review evidence
- event replay cursor

Outputs:

- config snapshot hash
- belief revision
- belief view
- planner projection view
- dirty key cursor updates

Durable records:

- belief config snapshot
- evidence item
- evidence assignment
- belief revision
- belief view
- runtime cursor
- evidence replay no-op receipt

Idempotency:

- same config hash reuses snapshot
- same evidence id replays only when content matches
- conflicting evidence id fails
- event replay cursor advances only after durable ingestion

Acceptance tests:

- missing evidence produces low confidence or stale state as configured
- docs writer success maps to support evidence
- docs writer failure maps to a durable no-op receipt and no evidence assignment
- projection exposes full subject refs and configured dimension
- reopen recovers view from durable records

Non goals:

- no goal creation
- no task execution
- no provider calls

Dependencies:

- event type contract from publication runtime
- activation config family ref
- world state graph subject refs

## World Model Agent Curation And Satisfaction

Owner: `meld-world-model` agent domain

Build item: bounded curation and satisfaction runtime handles

Contract skeleton:

- subscription delivery
- curation decision command
- goal command sink submission
- satisfaction review
- goal mutation command sink submission

Inputs:

- active subscriptions
- belief views
- planner projection
- active goals for agent
- curation rule records

Outputs:

- proposed goal command
- sink submission receipt
- satisfaction decision
- goal mutation command

Durable records:

- curation decision
- sink submission
- sink receipt
- satisfaction review
- satisfaction decision

Idempotency:

- delivery cursor advances only after sink acceptance or durable no-op
- curation dedupe key prevents duplicate goals
- satisfaction dedupe key prevents duplicate goal mutation
- sink receipts replay without resubmission

Acceptance tests:

- low docs freshness belief submits one goal command
- repeated delivery does not submit a duplicate goal
- unsatisfied world state keeps goal active
- satisfied world state submits one satisfy mutation
- reopen after satisfaction preserves decision and goal lifecycle

Non goals:

- no execution goal storage writes except through sink
- no task network writes
- no provider calls

Dependencies:

- execution goal command port
- execution goal mutation port
- belief query port
- planner projection port

## Execution Goals

Owner: `meld-execution` goals domain

Build item: active goal persistence and mutation boundary for agent commands

Contract skeleton:

- agent goal command acceptance
- goal mutation acceptance
- active goal query
- lifecycle query

Inputs:

- agent goal command
- agent goal mutation command
- sequence or review position

Outputs:

- accepted goal record
- goal lifecycle mutation result
- active goal list

Durable records:

- goal record
- goal command receipt
- goal mutation receipt

Idempotency:

- same command id replays
- command id with divergent content fails
- active goal query returns only active lifecycle records

Acceptance tests:

- curation command creates active goal
- duplicate command replays
- satisfy mutation closes goal
- foreign agent mutation is rejected
- reopen returns active and satisfied states correctly

Non goals:

- no belief reads
- no planning
- no task network writes

Dependencies:

- world model agent sink command shape

## Execution Planning And Method Package Bridge

Owner: `meld-execution` planning and task package adapter

Build item: docs freshness method bound to real docs writer package trigger

Contract skeleton:

- method config `refresh_docs_v1`
- method trigger over docs freshness target
- package trigger bridge
- task network command proposal
- lowering diagnostics

Inputs:

- active goal
- planner world state
- workspace readiness view
- latest workspace scan receipt
- workspace snapshot ref
- task network completion state
- method library
- activation execution config
- task package registry
- provider binding ref

Outputs:

- execution composition
- docs writer package trigger request
- task network mutation command
- `workspace_scan` task network mutation command when workspace state is missing or stale
- lowering diagnostics

Durable records:

- planning receipt or command diagnostic

Idempotency:

- same active goal and same projection frame produce stable lowering request
- scan command identity includes active goal id and workspace readiness state
- package task identity includes package fields and either scan receipt plus snapshot ref or upstream scan task artifact selectors
- same command id replays
- changed package trigger fields produce a new deterministic command identity
- task network commands and journals are durable only after task network acceptance

Acceptance tests:

- docs freshness goal matches `refresh_docs_v1`
- active goal with missing or stale workspace state lowers to task network structure where `workspace_scan` is upstream of docs writer work
- docs writer task is not ready until scan receipt and snapshot ref are available
- planning may emit one ordered graph or later graph deltas without changing that readiness invariant
- repeated planning does not duplicate accepted equivalent work
- planner preserves full `DomainObjectRef` in bindings
- lowerer builds a docs writer package task node, not a generic synthetic docs write task
- required package runtime fields are present
- missing provider binding produces a blocking diagnostic

Non goals:

- no provider calls
- no task execution
- no event append

Dependencies:

- activation execution config
- workspace readiness and scan receipt query contracts
- task network command and completion query contracts
- task package trigger contract
- `DomainObjectRef` preserving lowering
- task network store

## Execution Task Network And Dispatch

Owner: `meld-execution` task network and task runtime domains

Build item: run workspace scan and docs writer package tasks from the planned task network

Contract skeleton:

- task claim request
- workspace scan task dispatch
- task init hydration
- task execution report
- artifact commit
- outcome commit

Inputs:

- task network with ready task node
- workspace scan task init payload
- task package init payload
- provider access for package tasks
- context storage
- prompt storage
- workspace scan adapter

Outputs:

- scan receipt
- scan artifacts
- task artifacts
- final frame ref
- task outcome
- pending publication

Durable records:

- task claim
- scan receipt
- scan artifact records
- task artifact records
- task outcome
- pending publication
- task network journal

Idempotency:

- task claim is lease guarded
- scan task does not require provider access
- artifact ids are stable from task run and slots
- repeated successful outcome replays without duplicate publication

Acceptance tests:

- ready `workspace_scan` task executes to completion
- scan dispatch records one scan receipt and stable artifacts
- ready docs writer package task executes to success with test provider
- invalid provider binding is rejected before runtime dispatch
- configured provider unavailable at dispatch produces retryable dispatch report without task claim or lease
- package traversal expands expected workspace nodes
- final frame artifact is committed
- successful outcome creates one pending publication

Non goals:

- no belief mutation
- no satisfaction mutation
- no direct canonical event append

Dependencies:

- provider runtime port
- context and prompt stores
- workspace scan state
- publication outbox contract

## Execution Publication And Event Spine

Owner: `meld-execution` publication runtime plus events domain

Build item: pending task outcome publication to canonical event append

Contract skeleton:

- publication record
- event envelope builder
- event append sink
- publication mark command
- publication runtime report

Inputs:

- pending publication
- task outcome
- event append port

Outputs:

- canonical event envelope
- event sequence
- published publication mark
- publication runtime report

Durable records:

- publication record
- event envelope
- event append receipt
- task network publication mark

Idempotency:

- publication id derives from outcome
- event record id derives from publication id
- repeated append replays same sequence when event store supports idempotent append
- publication mark validates immutable outcome fields

Acceptance tests:

- pending docs writer success appends one canonical event
- repeated runtime tick does not append duplicate event
- publication mark survives reopen
- failure publication is appendable as an execution fact and belief maps it to a durable no-op receipt

Non goals:

- no belief interpretation
- no goal lifecycle mutation
- no task execution

Dependencies:

- event envelope contract
- task network publication store
- evidence source mapping contract

## Supervisor And Runtime Handles

Owner: runtime supervisor and domain runtime handles

Build item: concrete bounded semantic runtime handles under `meld runtime run`

Contract skeleton:

- runtime handle trait or enum
- bounded tick request
- heartbeat report
- health report
- safe point report
- shutdown report

Inputs:

- desired runtime state
- opened product stores
- runtime factory inputs
- work budget
- cancellation signal

Outputs:

- runtime heartbeat
- runtime diagnostics
- semantic tick report
- safe point acknowledgement

Runtime handles:

- world model agent bootstrap
- world model belief assessment
- world model evidence ingestion
- world model agent goal curation
- execution goal set
- execution planning
- execution task network command
- execution task dispatch
- execution publication
- world model satisfaction curation

Durable records:

- supervisor lease
- heartbeat
- health snapshot
- lifecycle event
- semantic records owned by each domain runtime

Idempotency:

- supervisor restart does not imply semantic replay by itself
- each domain runtime resumes from its own durable cursor
- heartbeat and health do not replace domain receipts

Acceptance tests:

- runtime run starts enabled concrete handles
- each handle can tick with bounded budget
- provider-dependent dispatch is disabled unless provider is available
- shutdown reaches safe point and flushes product stores
- status reports semantic runtime ids and health

Non goals:

- no semantic writes by supervisor directly
- no directive construction by supervisor
- no hidden execution inside assembly construction

Dependencies:

- concrete runtime handle factories
- activation runtime selection
- provider availability config

## CLI Activation Surface

Owner: CLI adapter

Build item: user command to activate docs freshness for a folder

Contract skeleton:

- activation command
- config file path option
- target selector option
- dry run option
- runtime run handoff option
- output format

Inputs:

- folder path
- optional config path
- optional provider selection
- optional runtime duration

Outputs:

- activation config written or validated
- runtime status summary
- diagnostics for missing provider or invalid target

Durable records:

- none directly unless command calls product activation service

Idempotency:

- repeated activation with same config confirms existing records through bootstrap
- dry run performs no writes
- command output names existing activation state when present

Acceptance tests:

- command validates folder path
- dry run shows planned domain records without writing
- activation followed by runtime run reaches bootstrap complete
- invalid provider binding fails with actionable diagnostic

Non goals:

- no direct agent store writes from CLI
- no direct goal writes from CLI
- no direct event append from CLI

Dependencies:

- product activation service
- runtime tooling
- provider config selection

## Integration Proof

Owner: integration test suite

Build item: end to end durable reopen proof

Contract skeleton:

- product fixture workspace
- activation config
- bounded runtime driver
- reopen checkpoints
- assertion helpers

Inputs:

- temporary workspace folder
- activation config
- deterministic provider stub
- runtime work budget

Outputs:

- completed flywheel trace
- durable records at each checkpoint
- reopen validation report

Checkpoints:

- after activation config load
- after bootstrap
- after goal acceptance
- after scan request
- after scan completion
- after planning command
- after task outcome
- after pending publication
- after event append
- after evidence ingestion
- after satisfaction
- after reopen

Acceptance tests:

- activation config loaded
- bootstrap wrote or confirmed directive, agent, rule, and subscription
- low docs freshness belief created one active goal
- planner emitted `workspace_scan` work after goal acceptance when workspace state was missing or stale
- scan produced full workspace object refs
- planner lowered to docs writer package task network work
- docs writer outcome produced one pending publication
- publication appended one canonical event
- evidence ingestion revised docs freshness belief
- satisfaction closed the goal
- all checkpoints reopen and recover the same state
- every semantic transition after activation is attributed to an enabled runtime handle tick report

Non goals:

- no fixture-only shortcuts for semantic transitions
- no manual cross-domain writes in test setup after activation
- no direct event append outside publication runtime

Dependencies:

- all prior domain contracts
- deterministic provider test harness
- stable identity map

## Static Boundary Scans

The implementation orchestration should keep these scans in the gate set.

```text
rg -n "Term::Object\\(object\\) => object.object_id" crates/meld-execution/src
rg -n "emit_envelope_best_effort" src/workspace | rg "src/workspace/(scan|capability)"
rg -n "register_seed_agent" crates/meld-world-model/src tests/integration
rg -n "ProductRuntimeConfig::for_product_root" src/runtime src/config
rg -n "single_task_mutation_set" tests/integration crates
rg -n "BeliefView \\{" tests/integration/docs_freshness_reopen_contract.rs
rg --files | rg '/mod\\.rs$'
```

Expected outcomes:

- object ref lowering scan must be removed or narrowed to compatibility-only paths
- workspace scan capability path must not emit canonical events directly
- seed agent registration must have conflict semantics for activation-owned fields
- runtime config loading must carry provider and enabled runtime state into assembly
- docs freshness proof must not inject task network mutation sets directly
- docs freshness proof must not inject synthetic belief views
- new work must not expand `mod.rs` layout

## Orchestration Notes

Shared contract work should stay central before parallel implementation begins.

Parallel-safe areas after shared contracts are stable:

- workspace scan capability adapter
- directive bootstrap store work
- docs writer package trigger bridge tests
- publication idempotency tests
- CLI validation shape

Sequential areas:

- activation config DTO before bootstrap and runtime selection
- directive record before seed agent bootstrap
- package trigger bridge before dispatch proof
- publication before evidence ingestion proof
- evidence ingestion before satisfaction proof

Final gate target:

```text
cargo test --test integration_tests docs_freshness
```
