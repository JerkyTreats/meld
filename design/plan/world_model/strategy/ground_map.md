# Strategy Ground Map

Date: 2026-08-05
Status: delivered 2026-08-12, retained as ground evidence
Scope: implementation readiness for world-model Strategy construction, mapping each Strategy concept to verified existing runtime primitives and naming the remaining construction delta

The construction delta below was delivered in full by commits b48c09d, 4894b73, and 5056b46. Two sections are superseded by that landing and are annotated in place: the known limits of the planning path, and the existing execution route.

## Purpose

Evergreen Strategy meaning and authority live under [World Model Strategy](../../../cognitive_architecture/world_model/strategy/README.md). The active delivery scope lives in [Strategy Minimal Slice Requirements](minimal_slice_requirements.md), and cross-domain ownership lives in [Strategy Assessment By Domain](assessment_by_domain.md). This document states ground: which primitives already exist in code, which Strategy concepts reuse them, and which first-slice behavior remains absent.

The headline verdict is that Strategy crosses a large product path but changes a small ownership set. Shared-language operations, Agent judgment persistence, planner frames, Method verification, action realization, Goal storage, task-network execution, outcome publication, evidence ingestion, belief reconciliation, and satisfaction already exist. The missing slice is candidate construction, durable Agent authorization, guarded Goal admission, and exact authorized realization.

## Verified primitive inventory

### Shared language

`meld-lang` is a working planning intermediate representation, not a type stub.

| Primitive | Location | Note |
|---|---|---|
| Proposition language | `crates/meld-lang/src/proposition.rs` | `Holds`, `Exists`, `Accessible`, `Related`, `All`, `Any`, `Not` with pattern variables and ground checks |
| Term vocabulary | `crates/meld-lang/src/term.rs` | object refs, dimensions, artifact types, literals, variables, step-derived values |
| Operator | `crates/meld-lang/src/operator.rs` | propositional preconditions and effects plus artifact-typed capability resolution constraints |
| Method | `crates/meld-lang/src/method.rs` | trigger, preconditions, composition template, net effects, cost, preference |
| Composition | `crates/meld-lang/src/composition.rs` | `StepKind::Op` and `StepKind::Goal` steps; `Ordering`, `DataFlow`, `Conditional` edges |
| Unification | `crates/meld-lang/src/unify.rs:60` | pattern matching with mergeable bindings |
| Substitution | `crates/meld-lang/src/substitute.rs:36` | binding application over propositions and compositions |
| Evaluation | `crates/meld-lang/src/evaluate.rs:36` | satisfied, unsatisfied with gap, indeterminate |
| Gap extraction | `crates/meld-lang/src/world_state.rs:73` | `gap` returns the unsatisfied propositions of a query |
| Effect application | `crates/meld-lang/src/world_state.rs:82` | counterfactual state projection via `apply` |
| Structural validation | `crates/meld-lang/src/validate.rs:92` | cycles, dangling edges, artifact source mismatch, unbound variables |

### Execution

| Primitive | Location | Note |
|---|---|---|
| Method library | `crates/meld-execution/src/planning/method_library.rs:46` | JSON directory loading, verification, deterministic ordering |
| Method verification | `crates/meld-execution/src/planning/method_library.rs:214` | trigger variable coverage, structural validation, operator resolution |
| Candidate evaluation | `crates/meld-execution/src/planning/runtime.rs:718` | trigger unification, precondition evaluation, cost ceiling, net-effect projection, projected-effect goal satisfaction check |
| Typed rejection grounds | `crates/meld-execution/src/planning/contracts.rs:229` | `CandidateStatus` covering trigger miss, unsatisfied or indeterminate preconditions, cost, effect miss |
| Mechanical no-method result | `crates/meld-execution/src/planning/runtime.rs:659` | `NoApplicableMethod` carrying the full candidate report set |
| Composition lowering | `crates/meld-execution/src/planning/lowering.rs:204` | operator steps to task nodes, dataflow and ordering edges to dependency edges, init sources |
| Task lineage | `crates/meld-execution/src/task_network/state.rs:190` | goal, method, composition, step, operator, frame, and capability identity on every committed node |
| Capability contracts | `crates/meld-execution/src/capability/contracts.rs:153` | typed input and output artifact slots, effect kinds, execution contract |
| Operator resolution | `crates/meld-execution/src/planning/method_library.rs:300` | contract matching over scope kind and artifact slot constraints |
| Goal lifecycle | `crates/meld-execution/src/goals/contracts.rs` | idempotent add, modify, satisfy, suspend, resume, remove commands |

### World model

| Primitive | Location | Note |
|---|---|---|
| Belief families as data | `crates/meld-world-model/src/belief/contracts.rs:115` | `BeliefFamilyConfig` holds dimension, predicate, evidence schemas, source mappings, comparator, planner projection |
| Derived belief keys | `crates/meld-world-model/src/belief/ingestion.rs:157` | keys bind a family config to subjects at ingestion rather than by enumeration |
| Belief status vocabulary | `crates/meld-world-model/src/belief/contracts.rs:317` | settled, stale, needs observation, needs assessment, pending, invalid |
| Config-driven comparator | `crates/meld-world-model/src/belief/comparator.rs:65` | one weighted Bayesian engine; factors, weights, polarity, prior all configured |
| Generic curation rule | `crates/meld-world-model/src/agent/curation.rs:246` | threshold rule over any dimension; emits a ground `meld-lang` Goal with dedupe key |
| Subscriptions with cursors | `crates/meld-world-model/src/agent/subscription.rs` | per belief key delivery with monotonic cursor |
| Generic planner projection | `crates/meld-world-model/src/planner/projection.rs:16` | emits confidence, staleness, and observation-needed `Holds` plus `Accessible` from configured field names |
| Graph identity and walks | `crates/meld-world-model/src/world_state/graph/contracts.rs` | `DomainObjectRef` subjects, perspective anchors, scoped relation walks |

The family, curation, and projection layers are structurally domain-agnostic. Within the world-model crate the `docs_freshness` name appears only in fixtures and examples, never in match arms or types. One product-level binding remains: `BELIEF_CONTEXT_FAMILY_ID` at `src/context/belief_context.rs:26` hardcodes the family name in belief-context assembly.

### Product runtime

| Primitive | Location | Note |
|---|---|---|
| Runtime roster | `src/runtime/assembly.rs:625` | twelve role descriptors from event append through publication; most have no bound actor per [Runtime Completion Ground Map](../../integration/runtime_completion_ground_map.md); the source declaration is dependency-ordered while the registry iterates by id |
| Supervisor | `src/runtime/supervisor/entrypoint.rs` | operational lifecycle only; tick-driven bounded work budgets; bounded restart policy |
| Self observation | `src/runtime/self_observation.rs` | runtime health promoted back into the ledger as facts |
| Generic traversal | `src/merkle_traversal.rs:37` | depth ordering is generic over the node graph; the directory filter is the only filesystem-specific part |

## Known limits of the current planning path

Superseded 2026-08-12: the authorized-candidate path landed in b48c09d and 4894b73 bypasses Method selection entirely, dispatch flows through the product capability runtime, and the docs theory activation question resolved into the compiled stewardship image whose elevation proceeds under the [Theory Elevation Program](../../integration/theory_elevation_program.md). The limits below scoped the readiness claims as written and stand as dated evidence.

These limits scope every readiness claim above.

- Method selection returns on the first applicable entry at `crates/meld-execution/src/planning/runtime.rs:648`. There is no candidate set output, no backtracking, and no depth bound because there is no recursion.
- Task-network mutation is inject-only at `crates/meld-execution/src/task_network/mutation.rs:106`. Alternative comparison must complete before lowering.
- `execution.task_dispatch` is disabled by default in the first-proof roster at `src/runtime/assembly.rs:314`.
- There is no committed product belief-family file or method directory. The authored meaning surface exists as loader schemas and one test fixture. Correction 2026-07-25: a committed belief-family body now exists at `theory/docs_freshness/belief_family.docs_freshness.json` with observationality authored, but nothing on the product load path resolves it — the loader reads the XDG theory root, and the committed selection id `docs-freshness-family` does not match the file's `docs_freshness` family id. The method-directory half still holds. Line numbers throughout this document are advisory; `BeliefFamilyConfig` gained the observationality field after this map was written.

## Existing execution route

Superseded 2026-08-12: the landed slice went further than this section planned. Commit 4894b73 composes the docs course of action from atomic capability contracts with no Method and no workflow route, so the package is no longer the realization of the authorized action. The package route remains only as legacy characterization.

The docs writer already runs through the task package expansion path. The first Strategy slice treats that package as the realization of one configured semantic action.

This slice does not migrate the package workflow or require general Composition parity. Strategy authorizes the action meaning and Execution uses the existing realization route.

## Construction delta

The first slice contains seven bounded changes.

1. Add a world-model Strategy constructor for one ground Goal, one exact planner frame, activated theory, one bounded Capability contract snapshot, optional configured Methods, and one prospective evidence route. It performs a deterministic bounded backward walk from settlement obligations, closes required Capability inputs, reuses current shared-language operations, and retains at most one eligible candidate. A Method may seed this walk but direct Capability construction must work without one.
2. Extend Agent curation contracts and runtime so the Agent authorizes the exact candidate after Goal drafting and before durable decision persistence. Recovery reuses that settled payload without constructing or choosing again.
3. Carry the authorization through the existing `CurationGoalSetPort`. Extend Execution Goal acceptance and the existing Goal record to validate and retain the accepted operational copy.
4. Add an authorized-candidate path to Execution planning. It bypasses semantic search, revalidates the exact Capability contracts, optional Method lineage, bindings, Composition, actions, and realizations, then uses existing lowering.
5. Map current planning and outcome theory into a neutral world-model Strategy input in root runtime assembly. Root copies contracts and wires owners without deciding Strategy validity.
6. Correct docs-freshness theory. The Method must stop asserting an observational docs-freshness value. The curation rule identifies the one Strategy policy and the prospective route from action outcome to required evidence.
7. Add focused Strategy, Agent, Goal admission, and authorized-planning tests plus one assembled docs-freshness product proof.

No first-slice change is required in `meld-lang`, planner projection, belief assessment, task-network mutation, dispatch, publication, events, evidence ingestion, satisfaction curation, harness, telemetry, workflow, workspace, or persistence infrastructure.

### Delta sequencing

Freeze two contracts first: the authorization payload shared by Agent and Execution, and the immutable Strategy problem input produced by root assembly.

After those contracts settle, world-model construction and Execution admission may proceed independently. Root wiring follows both. The authored-theory correction and focused tests may proceed with their owning components. The assembled product proof closes the slice.

## Authored surface today

Running docs freshness end to end requires eight human-authored artifact kinds: workflow package YAML, workflow thread profile YAML, turn prompts, belief family JSON, planning method JSON, agent curation rule data, agent seed TOML, and provider binding TOML. The curation rule is authored data carried on the agent registration rather than a standalone file with its own loader. Domain meaning concentrates in three of them: the belief family config, the method definitions, and the curation rule. Procedure concentrates in the package and profile. Physical binding concentrates in the TOML surfaces. This separation is the empirical seed of the stewardship-package split proposed under [Persistent Domain Stewardship](../../../persistent_domain_stewardship/README.md): the meaning artifacts already look like domain-theory fragments, and the package and profile rows are the ones Strategy-derived construction replaces.

## Read with

- [World Model Strategy](../../../cognitive_architecture/world_model/strategy/README.md)
- [Strategy Minimal Slice Requirements](minimal_slice_requirements.md)
- [Strategy Assessment By Domain](assessment_by_domain.md)
- [Strategy Boundary Contracts](../../../cognitive_architecture/world_model/strategy/contracts.md)
- [Docs Freshness Strategy](../../../cognitive_architecture/world_model/strategy/docs_freshness.md)
- [CVE Freshness Strategy](../../../cognitive_architecture/world_model/strategy/cve_freshness.md)
- [Use Case Catalog](../../../use_cases/README.md)
- [Runtime Completion Ground Map](../../integration/runtime_completion_ground_map.md)
- [Runtime Harness Plan](../../integration/runtime_harness_plan.md) — the frozen debugger register names Strategy first-slice development as the harness's first customer; the Goal-and-Belief prototype session lands alongside this construction
- [Cognitive Architecture Implementation Plan](../../README.md)
