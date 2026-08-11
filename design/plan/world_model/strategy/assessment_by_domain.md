# Strategy Assessment By Domain

Date: 2026-08-07
Status: active
Evidence basis: current `implementation/minimal-strategy-search` checkout and Strategy minimal-slice implementation
Scope: cross-domain impact of the first runnable world-model Strategy slice

## Concern definition

The Strategy minimal slice turns one Agent-curated docs-freshness Goal draft into one evidence-backed concrete candidate by closing declared Capability contracts, records the exact Agent authorization, admits the Goal with that candidate, and lets Execution realize only the authorized meaning.

The concern crosses many domains at runtime. This assessment distinguishes domains that own new behavior from domains that participate unchanged in the product path.

## In scope

- one ground docs-freshness Goal
- one exact planner frame
- one activated theory snapshot
- one bounded Capability contract snapshot
- optional configured Methods
- one prospective evidence route
- one deterministic bounded construction walk
- at most one retained candidate
- durable Agent authorization
- Strategy-gated Goal admission
- authorized Execution realization
- existing outcome, evidence, belief, and satisfaction path
- replay of settled Agent judgment

## Out of scope

- Pareto-frontier retention, global optimality, and advanced search certificates
- alternate engines, stochastic exploration, learned policy, and parallel search
- transposition tables and durable search state
- Strategy-specific storage
- new crates, services, actors, or dependencies
- generalized lifecycle or coordination machinery

## Domain snapshot

The snapshot was regenerated from the current checkout with:

```sh
find src -maxdepth 1 -type f -name '*.rs' -printf '%f\n' | sed 's/\.rs$//' | sort
```

```text
agent
api
branches
capability
cli
compat
concurrency
config
context
control
error
events
execution
harness
heads
ignore
init
lib
logging
merkle_traversal
metadata
prompt_context
provider
runtime
serve
session
store
task
telemetry
tree
types
views
workflow
workspace
world_state
```

Workspace domain crates were also assessed because Strategy crosses their explicit contracts:

```text
meld-world-model
meld-execution
meld-lang
meld-events
```

## Top-level domain assessment

| Domain | Needed integration | Current integration | Completeness | Evidence | Non-integration rationale | Follow-up |
| --- | --- | --- | --- | --- | --- | --- |
| `agent` | `none` | Legacy root Agent surface | not needed | World-model Agent owns curation | Root Agent is not the cognitive Agent authority | none |
| `api` | `none` | Product facade | not needed | No operator surface is required | First proof runs through product assembly | none |
| `branches` | `none` | Branch identity exists | not needed | Planner frame already carries branch scope | No Strategy-specific branch state | none |
| `capability` | `none` | Capability contracts exist | not needed | Execution planning already resolves actions | Strategy must not bind physical implementations | none |
| `cli` | `adapter` | Loads and composes stewardship theory | partial | `src/cli/runtime_assembly.rs` | Presentation remains uninvolved | Reuse existing theory composition and change only constructor mapping if required |
| `compat` | `none` | Compatibility support exists | not needed | No supported Strategy data exists | A new compatibility system is unjustified | none |
| `concurrency` | `none` | Shared limits exist | not needed | One synchronous bounded attempt | No new worker or coordinator | none |
| `config` | `consume` | Stewardship selection and physical binding exist | complete | `src/config/stewardship` | none | Reuse existing selection contracts |
| `context` | `none` | Context frames exist | not needed | Planner frame is the Strategy state input | Prompt hydration is downstream execution behavior | none |
| `control` | `none` | Orchestration plans exist | not needed | Strategy produces a Composition, not a control plan | No ownership transfer | none |
| `error` | `none` | Shared error mapping exists | not needed | Domain-local typed rejections suffice | No public error surface required | none |
| `events` | `none` | Canonical outcome ledger exists | not needed | Existing publication and replay path is reused | No Strategy event required for the first proof | none |
| `execution` | `none` | Root compatibility surface | not needed | Workspace execution crate owns the change | Avoid duplicate execution authority | none |
| `harness` | `observe` | Product boot and durable projections exist | complete | `src/harness` | none | Use for observation only if the product test benefits |
| `heads` | `none` | Legacy head support exists | not needed | No Strategy head semantics | none | none |
| `ignore` | `none` | Path-selection infrastructure exists | not needed | Docs package retains its current selection behavior | none | none |
| `init` | `adapter` | Theory provisioning exists | complete | `src/init/world/theory.rs` | none | Reuse generic JSON loading |
| `lib` | `adapter` | Crate export surfaces exist | partial | Workspace crate roots | none | Export only stable Strategy and admission contracts |
| `logging` | `none` | Logging exists | not needed | Existing runtime issues are sufficient | No Strategy truth in logs | none |
| `merkle_traversal` | `none` | Docs traversal exists | not needed | Existing package route remains unchanged | Strategy does not own traversal mechanics | none |
| `metadata` | `none` | Product metadata exists | not needed | Strategy lineage uses domain contracts | No hidden metadata registry | none |
| `prompt_context` | `none` | Prompt lineage exists | not needed | Provider context is downstream realization | No Strategy-specific prompt input | none |
| `provider` | `none` | Provider bindings exist | not needed | Execution route owns physical binding | Strategy selects meaning, not provider | none |
| `runtime` | `adapter` | Agent and Execution factories are composed | partial | `src/runtime/assembly.rs` and `src/runtime/ports.rs` | none | Wire Strategy at the existing curation-to-goal-set seam |
| `serve` | `none` | Served surfaces exist | not needed | No first-slice API requirement | none | none |
| `session` | `none` | Command lifecycle exists | not needed | Strategy lifecycle is not session lifecycle | none | none |
| `store` | `none` | Persistence substrate exists | not needed | Existing Agent and Goal stores are sufficient | Store must not interpret Strategy meaning | none |
| `task` | `none` | Docs package execution exists | not needed | Existing route is reused unchanged | Strategy does not execute tasks | none |
| `telemetry` | `observe` | Worker reports exist | complete | `src/runtime/contracts.rs` and `src/telemetry` | none | Reuse existing committed and error reporting |
| `tree` | `none` | Tree primitives exist | not needed | Downstream package owns tree mechanics | none | none |
| `types` | `none` | Root shared types exist | not needed | Contracts stay in owning crates | Avoid a generic Strategy type layer | none |
| `views` | `none` | Presentation models exist | not needed | Durable test evidence is sufficient | none | none |
| `workflow` | `none` | Docs-writer workflow exists | not needed | Existing action realization names it | Strategy does not own workflow procedure | none |
| `workspace` | `none` | Source and publication authority exist | not needed | Existing docs route reads and writes workspace state | No direct Strategy contract | none |
| `world_state` | `none` | Root compatibility projection exists | not needed | World-model planner owns the relevant projection | Avoid duplicate projection semantics | none |

## Workspace domain assessment

| Domain | Needed integration | Current integration | Completeness | Evidence | Follow-up |
| --- | --- | --- | --- | --- | --- |
| World-model Strategy | `own` | Pure bounded constructor and independent verifier exist | partial | `crates/meld-world-model/src/strategy` | Inspect semantics and extend only after the minimal engine is accepted |
| World-model Agent | `own` and `consume` | Goal drafting, durable decisions, Strategy authorization, and named goal port exist | partial | `crates/meld-world-model/src/agent` | Activate only when root supplies installed Strategy theory |
| World-model belief | `publish` | Observationality, evidence schemas, mappings, comparison, and reconciliation exist | complete | `crates/meld-world-model/src/belief` | Reuse unchanged |
| World-model planner | `publish` | Exact ground planner frame exists | complete | `crates/meld-world-model/src/planner` | Reuse unchanged |
| Shared language | `consume` | Goal, Method, Composition, unification, substitution, evaluation, and validation exist | complete | `crates/meld-lang/src` | Reuse unchanged |
| Execution goals | `consume` | Guarded acceptance validates and durably retains exact authorization | complete | `crates/meld-execution/src/goals` | Inspect compatibility posture before requiring authorization universally |
| Execution planning | `consume` | Exact authorized-candidate path bypasses Method search and revalidates current mechanics | partial | `crates/meld-execution/src/planning` | Prove the production package realization path under installed Strategy theory |
| Execution task network | `publish` and `consume` | Durable mutation and execution path exists | complete | `crates/meld-execution/src/task_network` | Reuse unchanged |
| Execution publication | `publish` | Package aggregate outcomes exist | complete | `crates/meld-execution/src/task_network` | Reuse unchanged |
| Events | `publish` and `consume` | Canonical append and replay exist | complete | `crates/meld-events/src` | Reuse unchanged |

## Frozen affected-domain set

The first pass freezes these affected domains before internal decomposition.

Changed behavior or adapter work:

- world-model Strategy
- world-model Agent
- Execution goals
- Execution planning
- root runtime
- CLI initialization and public export adapters
- authored docs-freshness theory

Participating through existing contracts:

- configuration
- world-model belief
- world-model planner
- shared language
- Execution task network
- Execution publication
- events
- harness and telemetry

Every other assessed domain remains explicitly `none`.

## Affected-domain decomposition

This second pass maps one level of major concerns inside each affected domain. It distinguishes changed behavior from adapters and reuse unchanged.

### World-model Strategy

| Component | Current state | Required change |
| --- | --- | --- |
| Contracts | implemented | Inspect immutable problem input, bounds, candidate, prospective evidence route, authorization, and typed rejection |
| Construction | implemented minimum | Inspect deterministic backward closure from settlement obligations through Capability contracts and optional Method seeding |
| Candidate verification | implemented minimum | Inspect independent closure, applicability, Goal contribution, evidence route, identity, and evaluation checks |
| Evidence-route validation | implemented minimum | Inspect exact outcome-contract match to prospective evidence meaning |
| Identity and replay | implemented minimum | Inspect content-derived candidate and authorization identities plus settled payload reuse |
| Persistence | absent | no Strategy store |

### World-model Agent

| Component | Current state | Required change |
| --- | --- | --- |
| Curation contracts | extended | Inspect configured Strategy input and authorization payload |
| Threshold Goal drafting | complete | reuse unchanged |
| Goal-curation runtime | implemented behind activation seam | Inspect Strategy invocation after draft creation and before decision persistence |
| Goal-curation actor | implemented behind activation seam | Inspect bounded Strategy input supplied by assembly |
| Curation decision | implemented | Inspect exact authorization persistence and no-candidate decision posture |
| Goal command | implemented | Inspect authorization carried through the named port |
| Store and query | complete substrate | Persist and inspect the extended decision |
| Registration and subscription | complete | reuse unchanged |
| Satisfaction curation | complete | reuse unchanged |

### World-model belief and planner

| Component | Current state | Required change |
| --- | --- | --- |
| Family contracts and config | complete | reuse observationality and evidence schemas |
| Family registry and query | complete | resolve exact installed theory |
| Outcome interpretation | complete | reuse substantive aggregate mapping |
| Evidence ingestion and comparator | complete | reuse unchanged |
| Planner contracts, projection, and query | complete | reuse exact frame and lineage |

### Execution goals

| Component | Current state | Required change |
| --- | --- | --- |
| Acceptance request | implemented | Inspect producer-neutral authorized candidate payload |
| Acceptance validation | implemented for guarded requests | Inspect empty, mismatched, and invalid authorization rejection |
| Goal record | implemented | Inspect retained operational authorization copy |
| In-memory and persistent stores | implemented | Inspect persistence and replay of the extended record |
| Goal query | complete | Return authorization with record |
| Lifecycle mutations | complete | reuse unchanged |

### Execution planning

| Component | Current state | Required change |
| --- | --- | --- |
| Actor request and result | extended | Inspect Strategy authorization recovered from the Goal record |
| Candidate search | first applicable Method compatibility path | Authorized Goals bypass semantic search |
| Method library | complete | Reuse only for compatibility Goals and optional lineage |
| Mechanical validation | extended | Inspect exact frame, precondition, Capability resolution, and structure revalidation |
| Action and realization | complete | Reuse exact association and package route |
| Lowering and task-network mutation | complete | reuse unchanged |

### Root runtime

| Component | Current state | Required change |
| --- | --- | --- |
| Stewardship theory binding | optional Strategy injection exists | Install a truthful authored Strategy snapshot before production activation |
| Agent factory and handle | optional Strategy input is wired | Inspect activated and compatibility construction paths |
| Curation Goal port | authorization mapping implemented | Inspect lossless world-model to Execution mapping |
| Planning factory | complete | reuse current Method and action theory |
| Registration | complete | no new runtime role |
| Storage | complete | no new store |
| Supervisor and dispatch | complete | reuse unchanged |

### Authored theory

| Component | Current state | Required change |
| --- | --- | --- |
| Belief family | correct | reuse |
| Curation rule | Goal threshold only | Select one Strategy policy, Method, action, and evidence route |
| Method | asserts an observational value | Remove the false docs-freshness update |
| Available action | correct | reuse artifact and outcome contract |
| Method realization | correct | reuse exact association |
| Outcome interpretation | correct | reuse substantive evidence mapping |

### Observation and evidence

| Component | Current state | Required change |
| --- | --- | --- |
| Harness | product boot and projections exist | observe only |
| Worker reports | committed and error counts exist | reuse |
| Telemetry | runtime observation exists | reuse |
| World-model tests | no Strategy tests | add construction and replay coverage |
| Execution tests | goals and planning tests exist | add admission and exact realization coverage |
| Product integration | downstream docs path exists | add one complete Strategy proof |

### Reused supporting domains

| Domain concern | Current ground | Required relationship | Change posture |
| --- | --- | --- | --- |
| Configuration selection | Existing stewardship selection contracts | supply the configured theory and realization | reuse unchanged |
| Shared language operations | Goal, Method, Composition, unification, substitution, evaluation, validation | construct one concrete candidate | reuse unchanged |
| Belief assessment | Existing observationality, mapping, comparator, and reconciliation | supply authoritative input and consume outcomes | reuse unchanged |
| Planner projection | Exact ground frame and source lineage | supply Strategy context | reuse unchanged |
| Task-network execution | Existing lowering, mutation, dispatch, and replay | realize admitted work | reuse unchanged |
| Publication and events | Existing aggregate outcome and canonical append path | publish substantive outcome | reuse unchanged |
| Harness and telemetry | Existing runtime observation surfaces | observe the assembled proof | reuse unchanged |

## Ownership summary

### New behavior owners

- world-model Strategy owns candidate meaning and prospective evidence validation
- world-model Agent owns judgment and durable replay source
- Execution goals own admitted operational custody and lifecycle enforcement
- Execution planning owns exact mechanical realization
- root runtime owns type mapping and wiring only
- authored theory owns action, outcome, evidence, and optional Method meaning

### Reused unchanged

- belief assessment and reconciliation
- planner projection
- shared language operations
- capability resolution
- Composition lowering
- task-network mutation
- dispatch and docs-writer execution
- publication and event append
- evidence ingestion
- Agent satisfaction and Goal mutation
- harness and telemetry

## Gaps and follow-ups

### Closed gap one: authorization payload

Agent persists the exact verified candidate and Execution retains an operational copy on the Goal record without querying Agent storage.

The default posture is the same complete payload in the Agent decision and Execution Goal record, joined by one content identity.

### Closed gap two: neutral Strategy theory input seam

World model owns the neutral Strategy problem types and root assembly accepts an optional activated problem template without importing Execution types into world model.

### Gap three: observational Method correction

The shipped Method currently asserts a docs-freshness value. The first slice must remove that assertion and authorize the action through its prospective evidence route.

### Gap four: production theory activation

The CLI leaves the Strategy injection empty because no installed authored surface yet supplies the complete settlement, Capability, outcome, and evidence snapshot. Inventing that meaning from planning Methods or adapter-local conventions would violate the ownership boundary. Inspection must settle the authored source and operational realization lineage before production activation and the assembled product proof.

## Extracted implementation spine

1. Add bounded world-model Strategy contracts and construction.
2. Add Agent judgment and replay custody before the existing goal port.
3. Add Strategy-gated Goal admission and operational custody.
4. Add exact authorized-candidate realization in Execution planning.
5. Wire neutral inputs and authorization mapping in root runtime.
6. Correct docs-freshness authored theory.
7. Prove the complete product path with focused and integration tests.

## Read with

- [Strategy Minimal Slice Requirements](minimal_slice_requirements.md)
- [Strategy Ground Map](ground_map.md)
- [Assessment By Domain Policy](../../../../governance/assessment_by_domain_policy.md)
- [World Model Strategy](../../../cognitive_architecture/world_model/strategy/README.md)
