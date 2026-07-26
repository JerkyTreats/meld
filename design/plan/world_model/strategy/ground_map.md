# Strategy Ground Map

Date: 2026-07-24
Status: active
Scope: implementation readiness for world-model Strategy construction, mapping each Strategy concept to verified existing runtime primitives and naming the remaining construction delta

## Purpose

The canonical Strategy design lives under [World Model Strategy](../../../cognitive_architecture/world_model/strategy/README.md). Canonical files state intent, boundaries, and contracts. This document states ground: which primitives already exist in code, which Strategy concepts are direct reuses or generalizations of those primitives, and which require new code.

The headline verdict, scoped to Strategy construction and admission: the pure planning operations, candidate evaluation, typed rejection reporting, provenance lineage, data-defined belief families, and generic curation already exist and are exercised by tests. The first-slice construction delta is small relative to the design surface. The deferred normative machinery listed at the end of the delta is not small and is explicitly tiered out.

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

`WorldState::gap` is the seed of the Strategy obligation graph. `apply` plus `evaluate` is the seed of candidate regression.

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

These limits scope every readiness claim above.

- Method selection returns on the first applicable entry at `crates/meld-execution/src/planning/runtime.rs:648`. There is no candidate set output, no backtracking, and no depth bound because there is no recursion.
- Recursive lowering of `StepKind::Goal` steps is explicitly deferred with the `GoalStepDeferred` diagnostic at `crates/meld-execution/src/planning/lowering.rs:169`. Subgoal structure is representable but never expanded.
- Task-network mutation is inject-only at `crates/meld-execution/src/task_network/mutation.rs:106`. Alternative comparison must complete before lowering.
- `execution.task_dispatch` is disabled by default in the first-proof roster at `src/runtime/assembly.rs:314`.
- No code names an affordance, obligation, strategy record, directive schema, or admission verdict. The directive on an agent record is an opaque string.
- The planner projection emits no `Related` propositions, so graph relations are absent from every `WorldState` frame.
- There is no committed product belief-family file or method directory. The authored meaning surface exists as loader schemas and one test fixture. Correction 2026-07-25: a committed belief-family body now exists at `theory/docs_freshness/belief_family.docs_freshness.json` with observationality authored, but nothing on the product load path resolves it — the loader reads the XDG theory root, and the committed selection id `docs-freshness-family` does not match the file's `docs_freshness` family id. The method-directory half still holds. Line numbers throughout this document are advisory; `BeliefFamilyConfig` gained the observationality field after this map was written.
- Producer discovery exists only as trigger-to-goal unification. Nothing unifies an intermediate obligation against `Effect::Assert` payloads, `Effect::Update` has no proposition form to unify against, and no effect-to-producer index exists.
- `WorldState::gap` returns the whole query as one opaque gap for indeterminate evaluation, returns whole disjunctions for `Any`, and returns whole negations for `Not`. Per-conjunct decomposition with typed indeterminate obligations does not exist.
- Proposition-level substitution is private inside `meld-lang`; Execution works around it by wrapping propositions in one-step compositions.

## Two execution paths

The multi-node docs fan-out runs through the task package expansion path: `TaskPackageSpec`, repeated regions, stage chains, and an opaque traversal capability that computes ordered node batches. The composition path that Strategy rides is single-method and has not executed a multi-node fan-out end to end.

The package path is the prior architecture. The decided direction is that the sense-model-act runtime subsumes it, with the existing workflow preserved as a characterized compatibility Method per [Docs Freshness Strategy](../../../cognitive_architecture/world_model/strategy/docs_freshness.md). Composition-path parity for sibling fan-out and child-before-parent dependencies is therefore the gating workstream for Strategy end-to-end execution: a constructor that emits compositions the lowering path cannot realize proves nothing. That work is chartered as Workstream Eight in [Runtime Completion Implementation Workstreams](../../integration/runtime_completion_implementation_workstreams.md), consuming the bounded package-step contract as its second consumer.

## Workflow migration classification

The docs writer YAML mixes domain meaning, one chosen theory of action, and physical execution policy. Migration classifies each concern rather than copying the file into a stewardship package.

| Current configuration | Target owner | Strategy meaning |
|---|---|---|
| accepted target fields | assignment scope and Execution trigger | identifies the requested workspace subject |
| target Agent identity | stewardship assignment and Agent perspective | identifies responsibility and judgment authority |
| provider and frame bindings | activation | supplies physical provider and context bindings |
| force posture | Agent and Strategy episode posture | changes reuse and intervention preference without changing domain theory |
| canonical `README.md` filename | docs correctness domain theory | identifies the maintained artifact |
| `directories_bottom_up` | compatibility Method only | candidate topology must be derived from obligations and constraints |
| overwrite on new head | publication affordance plus governance | declares allowed publication effect and conflict posture |
| repeated node region | compatibility Method and lowering machinery | Strategy grounds concrete folder obligations and independent branches |
| fixed prepare, execute, finalize stages | capability implementation and compatibility Method | semantic affordances remain implementation-neutral |
| four named turns and prompt refs | compatibility Method assets | Strategy may select actions that use these assets but the domain theory does not order them |
| child `frame_ref` wiring | compatibility dataflow | cannot discharge subtree coverage without typed admission |
| schema and no-drift gates | capability and artifact validation | do not establish exact-byte correctness |
| retries, timeouts, fail-fast, and resume | Execution policy | bounded operational realization rather than domain meaning |
| artifact persistence | Execution durability policy | preserves replay inputs and outputs |

During migration the existing workflow remains a characterized compatibility Method until Strategy-derived execution demonstrates parity for:

- accepted targets and start conditions
- traversal and node coverage
- sibling concurrency
- child artifact propagation
- force and reuse semantics
- provider and prompt asset selection
- retry and timeout precedence
- nonblocking gates
- persistence and replay
- publication and failure behavior

No current precedence rule should be inferred or silently changed during that migration.

## Concept-to-ground map

| Strategy concept | Ground | Posture |
|---|---|---|
| Obligation graph | `evaluate` gap results exist; `gap` collapses indeterminate, disjunctive, and negated queries | new decomposition semantics over existing evaluation, not a wrapper |
| Settlement transform | `evaluate` and `EvalResult` exist; transform absent | new shared-language vocabulary and transform plus a projection emission contract |
| Producer discovery | `unify` against Method triggers only | new effect-indexed discovery covering `Assert` payloads and an `Update` proposition form |
| Candidate regression | `evaluate_candidate` net-effect projection | generalize regression target to the settlement transform |
| Eligibility typing | `CandidateStatus` and `MethodCandidateReport` | reuse shape for `StrategyAlternative` eligibility |
| `NoMethodAvailable` | `NoApplicableMethod` report shape | new world-model admission error lifted from that shape |
| Semantic action affordance | `Operator` joined to `CapabilityTypeContract` through resolution | new catalog record promoting the operator to a standalone published verb |
| Bounded construction | `gap`, `unify`, `substitute`, `apply` | new bounded means-end loop over existing operations |
| Multi-candidate output | first-match `plan_goal` | generalize first-return to bounded collect |
| Subgoal closure | `StepKind::Goal` representable; lowering closed | expansion happens at construction; lowering stays closed |
| Goal draft gate | goal lifecycle commands | new admission gate before the add command |
| Admission bundle | none | new small cross-domain record |
| Known Strategy catalog | none | new persisted record and goal-pattern lookup |
| Evidence admission verdict | `EvidencePolicyId` slots exist; verdict record absent | new record plus task-network reducer input |
| Relation projection | `Proposition::Related` exists; never emitted | extend planner projection with declared relation vocabulary |
| Capacity facts | none | project capacity as propositions so construction stays pure |
| Outcome association | `TaskLineage` provenance | extend lineage with Strategy decision identity |
| Directive | opaque string on the agent record | new structured maintained-condition record |
| Agent strategy judgment | `curate_threshold_rule` pattern exists for Goal drafting | new configured deterministic judgment policy on the same pattern |
| Coverage admission rule | config-driven comparator exists for evidence confidence | new configured admission rule for discharge verdicts |
| Candidate projection | none | new `StrategyProjection` production; deferred past the first slice with mechanical eligibility only |

## Construction delta

The new code required for the first Strategy slice, in dependency order.

1. Directive maintained-condition record. Structured subject pattern, dimension, condition, and posture replacing the opaque directive string, consumed by grounding.
2. Settlement transform. A shared-language vocabulary addition and transform, not a small pure function: observationality must be declared per dimension by the owning belief family, the settlement proposition shape must be defined in `meld-lang`, and the planner projection contract must emit that vocabulary for every grounded question in scope.
3. Obligation decomposition. Per-conjunct evaluation producing typed obligations for unsatisfied and indeterminate propositions with subject and rule provenance. New evaluation semantics; `WorldState::gap` collapses indeterminate, disjunctive, and negated queries and cannot be wrapped.
4. Affordance catalog. Standalone operator records with artifact meaning and outcome contract references, loaded and verified like the method library.
5. Planner projection enrichment. Declared relation vocabulary projected as `Related` propositions; per-subject per-path-shape context-fit verdicts projected as typed propositions by the owning projection.
6. Bounded means-end constructor. For ground positive conjunctive obligations the loop composes existing pure operations. New machinery it also requires: effect-indexed producer discovery including an `Update` proposition form, disjunction decomposition and choice, a declared policy for negated and indeterminate obligations, a public proposition-level substitute, and the prospective-gate assumption rule.
7. Configured judgment policies. One deterministic Agent strategy-judgment policy on the curation-rule pattern, and one configured coverage-admission rule producing discharge verdicts. Without these the judgment and admission records have schemas but no producers.
8. Goal draft gate and admission bundle. World-model curation state before the Execution goal set, admission requiring a nonempty authorized candidate inventory.
9. Known Strategy catalog persistence and goal-pattern lookup over the untransformed target.
10. Evidence admission verdict record and its task-network ingestion path.
11. Selected-Strategy to outcome association extending existing task lineage.

Composition-path parity for multi-node fan-out is prerequisite operational work chartered as Workstream Eight under the runtime completion authority, not part of this delta.

### Delta sequencing

The delta does not serialize behind runtime completion as a block. Items 2, 3, and the pure core of item 6 live in `crates/meld-lang`, share no write scope with any runtime completion packet, and may proceed at any time, in that order. Items 1 and 9 are new record schemas draftable after the coordinated contract gate. Item 4 unblocks when the affordance-shaped available-action binding freezes. Items 5 and 7 coordinate with world-model lanes on write scope but depend on no completed workstream. Item 8 unblocks when the named curation-to-goal-set port lands. Items 10 and 11 need the task-network and lineage contracts from the dispatch workstream. Only end-to-end Strategy execution waits for composition-path parity.

### Deferred normative machinery

The contracts corpus requires machinery that is intentionally not in the first slice: the three epoch fences with a linearizable authority-preserving validation contract, atomic wake registration with durable cursor replay, content-addressed proposal closure hashing, idempotent judgment and planning reducers with publication outboxes, and `StrategyProjection` production with threshold, critical-path, and information-gain projections. First-slice ranking uses static cost and preference only. Learned ranking, generalized Strategy promotion, catalog governance, distributed consensus, and a learned efficacy model remain replaceable until the basic cognitive path proves useful. This tier is real work and is deliberately sequenced behind the first slice rather than counted inside it.

## Authored surface today

Running docs freshness end to end requires eight human-authored artifact kinds: workflow package YAML, workflow thread profile YAML, turn prompts, belief family JSON, planning method JSON, agent curation rule data, agent seed TOML, and provider binding TOML. The curation rule is authored data carried on the agent registration rather than a standalone file with its own loader. Domain meaning concentrates in three of them: the belief family config, the method definitions, and the curation rule. Procedure concentrates in the package and profile. Physical binding concentrates in the TOML surfaces. This separation is the empirical seed of the stewardship-package split proposed under [Persistent Domain Stewardship](../../../persistent_domain_stewardship/README.md): the meaning artifacts already look like domain-theory fragments, and the package and profile rows are the ones Strategy-derived construction replaces.

## Read with

- [World Model Strategy](../../../cognitive_architecture/world_model/strategy/README.md)
- [Strategy Requirements](../../../cognitive_architecture/world_model/strategy/requirements.md)
- [Strategy Contracts](../../../cognitive_architecture/world_model/strategy/contracts.md)
- [Docs Freshness Strategy](../../../cognitive_architecture/world_model/strategy/docs_freshness.md)
- [CVE Freshness Strategy](../../../cognitive_architecture/world_model/strategy/cve_freshness.md)
- [Use Case Catalog](../../../use_cases/README.md)
- [Runtime Completion Ground Map](../../integration/runtime_completion_ground_map.md)
- [Runtime Harness Plan](../../integration/runtime_harness_plan.md) — the frozen debugger register names Strategy first-slice development as the harness's first customer; the Goal-and-Belief prototype session lands alongside this construction
- [Cognitive Architecture Implementation Plan](../../README.md)
