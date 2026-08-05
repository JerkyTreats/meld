# Strategy Assessment By Domain

Date: 2026-08-05
Status: active
Evidence basis: current `runtime-completion` checkout and Strategy minimal-slice assessment
Scope: cross-domain impact of the first runnable world-model Strategy slice

## Concern definition

The Strategy minimal slice turns one Agent-curated docs-freshness Goal draft into one evidence-backed concrete candidate, records the exact Agent authorization, admits the Goal with that candidate, and lets Execution realize only the authorized meaning.

The concern crosses many domains at runtime. This assessment distinguishes domains that own new behavior from domains that participate unchanged in the product path.

## In scope

- one ground docs-freshness Goal
- one exact planner frame
- one configured Method and available action
- one prospective evidence route
- one bounded candidate
- durable Agent authorization
- Strategy-gated Goal admission
- authorized Execution realization
- existing outcome, evidence, belief, and satisfaction path
- replay of settled Agent judgment

## Out of scope

- construction beyond one configured Method and action
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
| World-model Strategy | `own` | No runtime module | not started | No `strategy` module exists | Add one bounded constructor and contracts |
| World-model Agent | `own` and `consume` | Goal drafting, durable decisions, and named goal port exist | partial | `crates/meld-world-model/src/agent` | Persist exact authorization before sink submission |
| World-model belief | `publish` | Observationality, evidence schemas, mappings, comparison, and reconciliation exist | complete | `crates/meld-world-model/src/belief` | Reuse unchanged |
| World-model planner | `publish` | Exact ground planner frame exists | complete | `crates/meld-world-model/src/planner` | Reuse unchanged |
| Shared language | `consume` | Goal, Method, Composition, unification, substitution, evaluation, and validation exist | complete | `crates/meld-lang/src` | Reuse unchanged |
| Execution goals | `consume` | Producer-neutral acceptance and durable lifecycle exist | partial | `crates/meld-execution/src/goals` | Enforce and store authorized admission |
| Execution planning | `consume` | Method verification, realization, lowering, and actor path exist | partial | `crates/meld-execution/src/planning` | Add exact authorized-candidate realization path |
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
| Contracts | absent | Define bounded input, candidate, prospective evidence route, authorization, and typed rejection |
| Construction | absent | Bind one configured Method to one Goal and exact frame |
| Evidence-route validation | absent | Validate action outcome to admitted evidence meaning |
| Identity and replay | absent | Derive exact payload identity and preserve settled judgment |
| Persistence | absent | no Strategy store |

### World-model Agent

| Component | Current state | Required change |
| --- | --- | --- |
| Curation contracts | partial | Add configured Strategy policy and authorization payload |
| Threshold Goal drafting | complete | reuse unchanged |
| Goal-curation runtime | partial | Invoke Strategy after draft creation and before decision persistence |
| Goal-curation actor | partial | Receive bounded Strategy inputs from assembly |
| Curation decision | partial | Persist exact authorization |
| Goal command | partial | Carry authorization through the named port |
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
| Acceptance request | partial | Carry authorized candidate payload |
| Acceptance validation | partial | Reject missing, empty, mismatched, or invalid authorization |
| Goal record | partial | Retain accepted operational copy |
| In-memory and persistent stores | partial | Persist and replay extended record |
| Goal query | complete | Return authorization with record |
| Lifecycle mutations | complete | reuse unchanged |

### Execution planning

| Component | Current state | Required change |
| --- | --- | --- |
| Actor request and result | partial | Carry Strategy and candidate identity |
| Candidate search | first applicable Method | Bypass for Strategy-gated Goals |
| Method library | complete | Resolve and reverify exact authorized Method |
| Mechanical validation | complete | Reuse precondition, binding, cost, and structure checks |
| Action and realization | complete | Reuse exact association and package route |
| Lowering and task-network mutation | complete | reuse unchanged |

### Root runtime

| Component | Current state | Required change |
| --- | --- | --- |
| Stewardship theory binding | planning and outcome theory are composed | Expose a neutral Strategy input to the Agent factory |
| Agent factory and handle | no Strategy input | Supply Strategy during goal-curation ticks |
| Curation Goal port | direct Agent-to-Execution mapping | Preserve and map authorization |
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
- authored theory owns the configured Method, action, outcome, and evidence relationship

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

### Gap one: authorization payload

Define the smallest payload that lets Agent persist its exact judgment and lets Execution recover and realize the accepted candidate without querying Agent storage.

The default posture is the same complete payload in the Agent decision and Execution Goal record, joined by one content identity.

### Gap two: neutral Strategy theory input

World-model must not depend on Execution types. Root assembly must map current Method, action, realization, outcome, and evidence declarations into a world-model-owned input without deciding semantic validity.

### Gap three: observational Method correction

The shipped Method currently asserts a docs-freshness value. The first slice must remove that assertion and authorize the action through its prospective evidence route.

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
