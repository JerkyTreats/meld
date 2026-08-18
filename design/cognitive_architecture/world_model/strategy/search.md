# Strategy Search

## Thesis

Strategy Search is the pure domain function that constructs, evaluates, and recommends a grounded theory of action for one proposed Goal.

It searches an immutable problem description. It performs no external reads or writes, admits no evidence, mutates no Goal, compiles no Task, and authorizes no work.

```text
Strategy problem
→ canonical search semantics
→ candidate construction and evaluation
→ recommendation and alternatives
→ independent candidate verification
```

Search owns recommendation under an explicit evaluation policy. The Agent owns the policy selection and final authorization. Execution receives only an authorized candidate and owns its operational realization.

## Mandate

Strategy Search must answer:

```text
For this exact Goal,
under this exact epistemic and action context,
what is the strongest defensible approach found within the search bounds?
```

The search mandate contains five inseparable responsibilities:

- derive settlement obligations without changing the Goal
- construct candidate action graphs from Capability contracts and reusable Methods
- discharge artifact, binding, world-state, authority, and evidence obligations
- evaluate eligible candidates under the supplied policy
- recommend the strongest candidate found and explain the result

Search does not decide whether the Goal should exist or whether its recommended candidate may run.

## Architecture and delivery conformance

This document defines the semantic conformance boundary and the maximal valid design space for Strategy Search. It does not require every conforming engine to implement every search, comparison, proof, or optimization facility described here.

An active implementation plan selects a bounded subset of this space. A minimal engine is conforming when it preserves the canonical problem, successor, candidate, eligibility, boundedness, verification, and authority semantics required by that plan. It may use a simple traversal and comparison policy while leaving richer algorithms and result claims unimplemented.

Features described here become delivery requirements only when an active plan selects them. In particular, Pareto-frontier retention, global optimality, exhaustive search certificates, stochastic exploration, alternate engines, transposition storage, learned policy, parallel search, and durable search state are optional extensions rather than baseline requirements.

## Pure function boundary

The canonical boundary is:

```rust
pub fn search(request: &StrategySearchRequest) -> StrategySearchResult;

pub fn verify_candidate(
    problem: &StrategyProblem,
    candidate: &StrategyCandidate,
) -> CandidateVerification;
```

Every fact that can affect construction, eligibility, or evaluation must be present in `StrategyProblem`. Every fact that can affect traversal, ordering, termination, or replay must be present in `StrategySearchRequest` or in the versioned search engine identity recorded by the result.

Search must not depend on:

- a live world-model query
- a live Capability catalog query
- ambient process configuration
- wall-clock time as semantic input
- hidden learned preferences
- mutable Method storage
- task-network state
- provider or worker availability
- nondeterministic iteration order
- randomness without an explicit seed

Internal mutation is permitted as an optimization. Arenas, queues, indexes, memoization, transposition tables, and parallel workers do not violate purity when they cannot change the semantic result for the same complete input and engine identity.

## Strategy problem

`StrategyProblem` is the complete immutable search universe for one Goal.

```rust
pub struct StrategyProblem {
    pub problem_id: StrategyProblemId,
    pub goal: Goal,
    pub planner_snapshot: PlannerSnapshot,
    pub theory_snapshot: StrategyTheorySnapshot,
    pub capability_snapshot: CapabilityContractSnapshot,
    pub method_snapshot: MethodLibrarySnapshot,
    pub evaluation_policy: StrategyEvaluationPolicy,
}
```

The problem identity is derived from the canonical content identities of every field. A revised Goal, world-model projection, theory declaration, Capability contract, Method, or evaluation policy creates a different problem.

### Goal

The Goal is ground and proposed. Its target is immutable throughout search.

Strategy may derive subordinate settlement obligations and subgoals. Every derivation must preserve an explicit contribution path back to the exact Goal. No candidate may weaken, replace, reinterpret, or opportunistically satisfy a different target.

### Planner snapshot

The planner snapshot is the complete epistemic position Strategy may use. It contains ground propositions, uncertainty and abstention declarations, source identities, projection identity, and expiry meaning required by the problem.

Strategy evaluates this snapshot. It does not reconstruct belief, infer absence from missing belief records, admit evidence, or reach back into graph, belief, causation, or regime internals.

Counterfactual effects create search-local projected states. They never modify the planner snapshot and never become observed truth.

### Theory snapshot

The theory snapshot supplies the semantic bridge from Goal to action. It contains the activated PDS declarations needed to determine:

- which settlement obligations follow from the Goal
- which action outcomes can contribute to those obligations
- which world-state preconditions govern each action
- which evidence routes can support later reconciliation
- which governance classifications and domain constraints apply
- which outcome meanings distinguish success, partial success, failure, and harm

PDS authors these declarations. Strategy interprets them for the current problem. Search cannot invent missing action, causal, evidence, or authority meaning.

The theory snapshot does not contain the exact capability snapshot, Method snapshot, evaluation policy, requested projection dimensions, search bounds, or effective authority. Those are independently owned inputs to problem assembly or the search request.

### Capability contract snapshot

The Capability snapshot is the complete atomic action vocabulary available to search. Every entry has exact identity and version and declares the contract fields needed for construction:

- required artifact inputs
- produced artifact outputs
- required bindings
- supported scope
- operational effect declarations
- execution behavior relevant to eligibility
- declared cost and resource meaning
- semantic action and outcome references supplied by theory

Strategy may select and wire exact Capability contracts from this snapshot. It does not invoke implementations or inspect providers. Execution later revalidates the selected contracts and compiles the authorized graph without substituting different semantic work.

The snapshot is activation-local. The same PDS theory revision may be combined with different capability snapshots without changing domain meaning.

### Method library snapshot

The Method snapshot contains reusable Strategy solution templates. A Method is a candidate generator, not a search result and not a compiled Task.

Methods may guide expansion and provide known action shapes. Search must remain capable of constructing candidates directly from Capability contracts when no Method applies.

Search never mutates the Method snapshot. Any reusable Method proposal derived from a successful episode is separate from the pure search result and requires its own authorial admission.

Methods are Strategy-owned reusable cognition. They are not installed as PDS stewardship mandates.

### Evaluation policy

The evaluation policy defines what stronger means for this Agent and Goal class. It contains:

- hard eligibility constraints
- candidate evaluation dimensions
- comparison and dominance rules
- deterministic tie breaking
- tolerated assumptions and uncertainty
- cost and risk ceilings
- alternative retention rules

The policy may consume authoritative estimates supplied by the snapshots. It may not cause Strategy to invent probabilities, causal effects, evidence strength, or authority.

The Agent selects or authors the policy. Strategy applies it and owns the resulting recommendation. The Agent remains free to reject that recommendation.

## Search request

`StrategySearchRequest` binds one semantic problem to explicit traversal controls.

```rust
pub struct StrategySearchRequest {
    pub request_id: StrategySearchRequestId,
    pub problem: StrategyProblem,
    pub engine: StrategySearchEngineIdentity,
    pub search_bounds: StrategySearchBounds,
    pub search_seed: Option<SearchSeed>,
}
```

The request identity includes the problem, bounds, seed, and versioned engine identity. Different algorithms and bounds may search the same `StrategyProblem`. Their results remain directly comparable because candidate identity and evaluation depend on the problem rather than the traversal budget.

### Search bounds

Search bounds are explicit structural limits over the search space. They may constrain:

- expanded positions
- generated successors
- construction depth
- open frontier size
- retained alternatives
- repeated-state expansion
- observation branch width

Structural bounds preserve reproducibility across machines. Wall-clock cancellation is an operational stop signal rather than semantic proof that search completed.

A bounded result never claims that an unexplored candidate does not exist.

### Search seed

Search is deterministic unless the selected search policy explicitly permits stochastic exploration. Any stochastic choice uses the supplied seed. The seed participates in request identity and replay.

## Initial settlement obligations

Search does not begin by looking for arbitrary runnable actions. It begins by deriving the obligations that would make action defensible for the exact Goal.

```text
Goal target
+ subject and scope
+ outcome meaning
+ prospective evidence requirements
+ authority and constraints
→ initial settlement obligations
```

The initial obligation set is the root of the search space. It is derived solely from the Goal and theory snapshot.

For an observational Goal, the root obligation is not to assert the desired observation. It is to construct a route by which action can produce a substantive outcome that may become admissible evidence for later belief reconciliation and Agent satisfaction.

## Search position

A search position is one immutable partial theory of action.

```rust
pub struct StrategySearchPosition {
    pub problem_id: StrategyProblemId,
    pub projected_world: ProjectedWorldState,
    pub open_obligations: ObligationSet,
    pub candidate_graph: CandidateActionGraph,
    pub bindings: Bindings,
    pub evidence_routes: Vec<ProspectiveEvidenceRoute>,
    pub assumptions: Vec<StrategyAssumption>,
    pub accumulated_evaluation: CandidateEvaluation,
    pub lineage: StrategyLineage,
}
```

The position contains only information that can affect a future continuation, candidate eligibility, or evaluation. Traversal history is excluded unless the evaluation policy explicitly makes it semantically relevant.

### Position identity

Every position has a canonical content identity suitable for equality, memoization, and transposition reuse.

Two positions are equivalent only when they have the same legal continuations and the same evaluation under the containing problem. Position identity therefore includes every future-relevant projected proposition, open obligation, binding, selected action dependency, evidence route, assumption, accumulated cost, and theory reference.

Search order, diagnostic history, allocation identity, worker identity, and arrival path do not participate in semantic position identity.

An optimization may merge equivalent positions. It may not merge positions merely because their artifact graph looks similar.

## Obligations

An obligation is a typed condition that must be discharged before a partial position can become an eligible candidate.

### Settlement obligation

Establishes how the candidate contributes toward settlement of the exact Goal without asserting that settlement already occurred.

### Artifact obligation

Requires an admissible artifact of an exact type and schema. It may be discharged from problem context, an existing artifact declaration, or a selected upstream Capability output.

### Binding obligation

Requires a ground value for a Capability or Method binding. Every value preserves its authoritative source and identity contribution.

### World-state obligation

Requires a proposition to be supported by the planner snapshot or by a valid counterfactual predecessor within the candidate.

Satisfied conditions discharge the obligation. Unsatisfied conditions reject that branch unless declared action meaning supports constructing an upstream change. Indeterminate conditions remain open or produce an observation branch. They never become assumed truth.

### Action-contribution obligation

Requires every selected Capability to have a declared role in the Goal settlement path. Input and output compatibility alone cannot discharge this obligation.

### Evidence obligation

Requires a prospective route from substantive action outcome through governed outcome meaning to an evidence schema relevant to the Goal subject and dimension.

### Authority obligation

Requires the candidate to remain within declared authority and constraint boundaries. Agent authorization of the finished candidate is a later act and is not discharged inside search.

## Successor semantics

Successor generation defines the canonical search space independently of any traversal algorithm.

For one selected open obligation, search may:

- apply a reusable Method whose trigger and semantic meaning match
- select a Capability whose declared output or effect contributes
- bind an obligation from authoritative problem context
- add upstream Capability inputs as new obligations
- introduce an observation action for an indeterminate condition
- expand a finite declared outcome branch
- reject a transition whose meaning, bindings, evidence route, or constraints are invalid

Every transition returns a new immutable position plus typed derivation grounds.

Matching a Capability output is necessary but not sufficient. Expansion must also preserve input closure, action contribution, world-state applicability, authority, evidence meaning, and the original Goal.

Methods and direct Capability construction use the same successor semantics. A Method accelerates the walk by proposing a reusable subgraph. It does not bypass obligation discharge or candidate verification.

## Candidate terminal condition

A search position becomes an eligible `StrategyCandidate` only when:

- the exact Goal and planner snapshot remain anchored
- every required obligation is discharged
- every Capability identity, version, binding, and dependency is ground
- every artifact input is supplied or produced exactly once under its contract
- every semantic precondition is supported
- every selected action has a declared contribution path
- every required authority constraint is met
- every observational obligation has a valid prospective evidence route
- the candidate is within hard policy bounds
- the candidate graph is structurally valid

Explicitly tolerated assumptions may remain only when the evaluation policy permits them. They are carried in the candidate and affect evaluation. They are not silently treated as discharged facts.

Candidate terminality does not mean the Goal is satisfied. It means the proposed approach is complete enough for independent verification and Agent judgment.

## Strategy candidate

`StrategyCandidate` is the sole canonical semantic product of successful search.

```rust
pub struct StrategyCandidate {
    pub candidate_id: StrategyCandidateId,
    pub problem_id: StrategyProblemId,
    pub goal_ref: GoalRef,
    pub planner_snapshot_ref: PlannerSnapshotRef,
    pub action_graph: CandidateActionGraph,
    pub method_lineage: Vec<MethodInstanceRef>,
    pub bindings: Bindings,
    pub obligation_proof: ObligationProof,
    pub prospective_evidence_routes: Vec<ProspectiveEvidenceRoute>,
    pub assumptions: Vec<StrategyAssumption>,
    pub evaluation: CandidateEvaluation,
    pub theory_refs: Vec<StrategyTheoryRef>,
    pub capability_refs: Vec<CapabilityContractRef>,
}
```

The candidate action graph is a ground Capability dependency blueprint with semantic edges and exact contract identities. It is neither a compiled Task nor task-network state.

The candidate does not contain:

- a Goal mutation
- Agent authorization
- an execution schedule
- a task instance identity
- a provider or worker selection
- an inferred substantive outcome
- admitted evidence
- a belief revision
- a claim of Goal satisfaction

## Candidate evaluation and recommendation

Eligibility is a hard semantic gate. Evaluation happens only after eligibility is established.

The evaluation policy maps a candidate into a comparison value. The value may be multi-dimensional and may preserve a Pareto frontier before deterministic recommendation. A minimal conforming engine may instead use deterministic lexicographic ordering over a small declared evaluation vector. Evaluation dimensions may include supported Goal contribution, expected evidence quality, uncertainty, assumption load, cost, risk, reversibility, and policy preference.

Every evaluation component must be derived from explicit problem inputs or candidate structure. Search may aggregate and compare authoritative values. It may not manufacture them.

The recommended candidate is the strongest eligible candidate found under the evaluation policy. Ordered alternatives preserve distinct eligible approaches according to the policy's retention and diversity rules.

The recommendation is Strategy authorship, not Agent authorization.

## Search result

`StrategySearchResult` reports both semantic output and the limits of the search claim.

```rust
pub struct StrategySearchResult {
    pub request_id: StrategySearchRequestId,
    pub problem_id: StrategyProblemId,
    pub engine: StrategySearchEngineIdentity,
    pub completion: StrategySearchCompletion,
    pub recommendation: Option<StrategyCandidate>,
    pub alternatives: Vec<StrategyCandidate>,
    pub rejections: Vec<StrategyRejectionSummary>,
    pub frontier: StrategyFrontierSummary,
    pub certificate: StrategySearchCertificate,
    pub statistics: StrategySearchStatistics,
}

pub enum StrategySearchCompletion {
    Exhaustive,
    Bounded,
}
```

`Exhaustive` means the canonical successor space for the finite problem was exhausted or a valid proof bound established that no unexplored position can improve the result.

`Bounded` means an explicit search bound stopped exploration while positions remained. A bounded result may contain a recommendation, but it may claim only that this was the strongest candidate found. It may not claim global optimality or absence of an eligible candidate.

Only an exhaustive result with no recommendation may state that the problem has no eligible candidate.

Rejections summarize typed semantic grounds. They do not need to retain every duplicate traversal rejection. Frontier summaries preserve the kinds and evaluation bounds of unresolved positions without turning live search memory into durable domain state.

Statistics contain deterministic structural work counts for the exact engine and request. They do not participate in candidate identity or authorization. Wall-clock and environment measurements remain outside the pure result.

## Independent verification

Candidate validity must be checkable without trusting the algorithm that discovered the candidate.

The candidate verifier consumes only the immutable problem and candidate. It recomputes:

- Goal and snapshot anchoring
- canonical identities and ground bindings
- Capability input and output closure
- action dependency validity
- world-state precondition support
- action contribution to settlement obligations
- authority and hard policy constraints
- prospective evidence-route validity
- candidate evaluation under the supplied policy

The verifier does not replay search order and does not accept search diagnostics as proof.

```rust
pub enum CandidateVerification {
    Valid {
        evaluation: CandidateEvaluation,
        proof: ObligationProof,
    },
    Invalid {
        grounds: Vec<StrategyRejectionGround>,
    },
}
```

Candidate soundness is distinct from search completeness and optimality. A valid candidate can be verified locally. A claim that no better candidate exists requires an exhaustive search result or a separately checkable search-space bound in `StrategySearchCertificate`.

Optimized search and reference verification must share canonical proposition, binding, effect, and contract semantics. They must not share an opaque validity verdict.

## Search certificate

The search certificate makes the scope of the result explicit. It records:

- exact problem and engine identity
- exact request, policy, bounds, and seed identity
- root obligation identity
- recommendation and alternative identities
- completion claim
- explored and unresolved evaluation bounds
- pruning rule identities used by the engine
- any proof bound supporting exhaustive or optimal claims

The certificate need not expose every internal node. It must contain enough information to reproduce the search under the same engine or independently challenge any completeness or optimality claim.

A minimal bounded engine need only record request identity, engine identity, explicit bounds, completion posture, candidate identities, and deterministic work counts. Proof bounds, pruning identities, unresolved evaluation bounds, and optimality claims are required only when the engine makes the corresponding stronger claim.

## Algorithm conformance

Search algorithms are replaceable implementations of this contract. Depth-first, best-first, branch-and-bound, AND-OR, hierarchical, or hybrid designs may be used when they preserve the same canonical problem, successor, candidate, evaluation, and verification semantics.

An algorithm may change:

- traversal order
- frontier representation
- move or obligation ordering
- memoization strategy
- transposition storage
- admissible pruning
- incremental evaluation
- parallel exploration

An algorithm may not change:

- which transitions are semantically legal
- what discharges an obligation
- what makes a candidate eligible
- how the evaluation policy compares completed candidates
- the meaning of bounded and exhaustive results
- the authority boundary among Goal, Strategy, Agent, and Execution

Any pruning rule that can affect the recommended candidate must have a declared semantic justification. A heuristic may order exploration freely. It may discard a position only when the result contract permits the corresponding loss of completeness or a valid bound proves the position cannot improve the recommendation.

## Verification properties

Every conforming search engine is judged against domain properties rather than exact traversal traces.

### Goal fidelity

Every returned candidate preserves the exact Goal and carries a complete contribution path from every selected action to its settlement obligations.

### Candidate soundness

Every returned candidate passes independent verification.

### Closure

No eligible candidate contains an unresolved required artifact, binding, precondition, authority, action-contribution, or evidence obligation.

### Epistemic integrity

Counterfactual effects, artifact production, and task completion meaning never become observation, admitted evidence, belief revision, or Goal satisfaction.

### Recommendation correctness

No returned alternative outranks the recommendation under the exact evaluation policy.

### Exhaustive completeness

An exhaustive result contains every policy-relevant eligible alternative or proves why omitted alternatives cannot affect the recommendation or retained frontier.

### Bounded honesty

A bounded result exposes its unresolved frontier and makes no claim beyond the candidates actually found and verified.

### Replay

The same problem, engine identity, bounds, and seed produce the same semantic result and candidate ordering.

### Irrelevant-extension stability

Adding a Capability, Method, or proposition that cannot contribute to any reachable obligation does not change the semantic recommendation.

### Dominance

A candidate that is strictly worse on every policy dimension cannot displace the candidate that dominates it.

## Performance vector

Performance optimization is evaluated without weakening semantic verification.

Search strength is measured by:

- evaluation quality of the recommendation at equal structural budget
- eligible alternatives discovered at equal structural budget
- search depth and obligation closure reached before bounds
- convergence of bounded recommendations toward an exhaustive reference result

Search efficiency is measured by:

- positions expanded
- successors generated
- duplicate positions avoided
- transposition reuse
- verifier work
- peak frontier size
- peak retained position state
- time to first valid candidate
- time to improved recommendation
- wall-clock throughput under a fixed benchmark environment

Structural work metrics are authoritative for cross-engine semantic comparison. Wall-clock and memory measurements describe an implementation on a benchmark environment and do not change search meaning.

Optimizations must be compared against a reference successor generator and independent candidate verifier. Faster production of an unverifiable or semantically different candidate is not a Strategy improvement.

## Research evaluation frame

Research into candidate search algorithms must evaluate each design against the same concerns:

- clarity of the position and successor model
- sound handling of AND obligations and OR alternatives
- support for three-valued world-state preconditions
- ability to stop cleanly at observation boundaries
- completeness and optimality claims that can be stated precisely
- usefulness under explicit bounds
- independent candidate verification
- canonical transposition identity
- admissible pruning and understandable failure modes
- incremental evaluation and memoization opportunities
- deterministic replay
- performance on both shallow wide and deep narrow problems
- ease of maintaining one semantic reference implementation beside optimized engines

Algorithm simplicity is a positive design property. Additional sophistication is justified only when it improves search strength or efficiency under this contract without obscuring the validity argument.

## Scope boundary

Strategy Search owns:

- derivation of settlement obligations
- canonical search positions and successors
- Method-guided and direct Capability construction
- candidate eligibility
- candidate evaluation and ordering
- recommendation and alternatives
- bounded and exhaustive completion semantics
- search certificates and statistics
- independent candidate verification contracts

Strategy Search does not own:

- Goal creation, mutation, admission, or satisfaction
- world-model projection or epistemic source truth
- PDS theory authoring or Method admission
- live Capability catalog mutation
- Task compilation or task-network mutation
- execution scheduling, dispatch, retry, or repair
- Agent authorization
- outcome interpretation, evidence admission, or belief reconciliation

## Read with

- [World Model Strategy](README.md)
- [Strategy Boundary Contracts](contracts.md)
- [Goals and Methods](../../meld-lang/goals_and_methods.md)
- [Operators and Resolution](../../meld-lang/operators.md)
- [Compositions](../../meld-lang/compositions.md)
- [World State and Evaluation](../../meld-lang/world_state.md)
- [Execution Planning](../../execution/planning/README.md)
- [Persistent Domain Stewardship](../../../persistent_domain_stewardship/README.md)
