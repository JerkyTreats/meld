# PDS Authorized Implementation Findings

Date: 2026-08-17
Status: closed predecessor register with carried findings
Scope: confirmed findings from review and live verification of the authorized PDS implementation workstreams through `W06`

Closeout disposition is authoritative in [PDS Authorized Implementation Closeout](pds_authorized_implementation_closeout.md). Findings remain technically open where stated, but this register grants no active predecessor implementation authority.

## Purpose

This register preserves failures and unresolved observations discovered while verifying routed documentation freshness against `gmail-operator`.

The runtime and Strategy findings remain valid independently of defects in the live verification harness. Harness defects are recorded separately so they cannot explain away runtime or cognitive-runtime behavior.

Recording a finding does not expand implementation authorization. Runtime findings require disposition against the authorized `W06` contracts. Strategy findings require an explicit scope decision before implementation if their correction would exceed the current authorization boundary.

## Live Run Specimen

The triggering specimen used the routed documentation-freshness PDS package against `gmail-operator` with `qwen3-coder-next` served through the verified local `lmserver` path.

Observed facts:

- elapsed runtime was `1037.64` seconds
- the provider completed `50` requests
- `docs.inspect_scope.v1` completed
- `docs.draft_patch_set.v1` completed
- `docs.validate_patch_set.v1` completed
- `docs.publish_patch_set.v1` did not run
- `docs.assess_published_scope.v1` did not run
- two task outcome publications were observed before shutdown
- shutdown failed on an expired lease for `execution.planning`
- generated README bytes and the temporary product store were not retained

The terminal error was:

```text
shutdown failed after startup: supervisor store error: stale lease owner for runtime id 'execution.planning'
```

## Findings Summary

| ID | Domain | Finding | State | Disposition boundary |
| --- | --- | --- | --- | --- |
| `PDS-IF01` | runtime supervisor | one synchronous actor step can expire unrelated actor leases | confirmed open | requires `W06` conformance disposition |
| `PDS-IF02` | runtime supervisor | shutdown can replace completed work with a terminal lease error | confirmed open | requires `W06` lifecycle disposition |
| `PDS-IF03` | runtime and cognition | operational staleness has no semantic reassessment handoff | confirmed open | requires owner and cognitive-runtime disposition |
| `PDS-IF04` | execution and runtime | provider work occupies a supervisor step instead of a durable operation | confirmed open | conflicts with `W06-T05` and `W06-T06` |
| `PDS-IF05` | Strategy | declared candidate cost materially understates realized work | confirmed open | requires Strategy scope disposition |
| `PDS-IF06` | docs and Strategy | atomic capability contracts hide a procedural mini-strategy | confirmed open | requires capability-boundary disposition |
| `PDS-IF07` | Strategy | the live specimen offered no meaningful alternative strategy | confirmed open | evidence gap and design question |
| `PDS-IF08` | Strategy | observed execution cost does not inform later Strategy judgment | confirmed open | requires Strategy scope disposition |
| `PDS-IF09` | verification | failure cleanup destroys the evidence needed to assess output | confirmed open | current verification closure |
| `PDS-IF10` | verification | generated README correctness remains unknown | open pending retained rerun | current verification closure |
| `PDS-IF11` | runtime lifecycle | one zero-work pass is projected as quiescence without owner waits | confirmed open | requires `W06` lifecycle disposition |
| `PDS-IF12` | PDS and Strategy | installed Strategy theory conflates settlement meaning with runtime policy and exact affordances | architecture disposition accepted | runtime refactor requires authorization |
| `PDS-IF13` | docs and belief | final correctness probability crosses the evidence boundary before belief assessment | architecture disposition accepted | docs evidence rework requires authorization |
| `PDS-IF14` | PDS and cognitive runtime | state-free owner routing does not by itself prevent precomputed cognition | architecture disposition accepted | owner contract enforcement requires authorization |

## Runtime Findings

### PDS-IF01 — A synchronous actor step can expire unrelated leases

The supervisor renews an actor lease and then invokes its bounded step synchronously. Actors are stepped sequentially inside one supervisor tick. No independent heartbeat path continues while one actor waits on provider input and output.

The default lease duration is fifteen minutes. The live tick ran for more than seventeen minutes. By the time the tick returned, the lease for `execution.planning` had expired even though the long-running provider work occurred elsewhere in the same shared tick.

This means actor liveness is coupled to whole-tick latency. One slow actor can make unrelated live actors appear stale.

This finding exists independently of the test duration, temporary directory, and assertion behavior. The harness exposed the condition but did not create the shared synchronous tick or the lease expiry.

Relevant implementation:

- `src/runtime/supervisor/entrypoint.rs`
- `src/runtime/assembly.rs`
- `src/runtime/tooling.rs`

Closure evidence must show that provider work longer than one lease interval does not stale unrelated participants while genuine participant loss remains detectable.

### PDS-IF02 — Shutdown can replace completed work with a terminal lease error

The task-dispatch tick completed useful semantic work and published task outcomes. Runtime shutdown then attempted to write a final stopped heartbeat for `execution.planning`. The heartbeat write rejected the expired lease, and the runtime command returned a terminal shutdown error.

The command result therefore represented shutdown bookkeeping failure instead of the completed task transition. The supervisor did not cancel the provider work. It converted an expired operational ownership record into the terminal result after useful work had completed.

Closure requires separate reporting of semantic task progress, interrupted operational ownership, and shutdown completion. An expired final heartbeat must not erase or misclassify already committed work.

### PDS-IF03 — Operational staleness has no semantic reassessment handoff

Lease expiry currently terminates in supervisor and store error handling. It does not become an owner-admitted operational fact that can enter the event, belief, Agent, Strategy, and execution loop.

The missing bridge matters because an expired participant lease cannot determine whether any of the following remain valid:

- the maintained condition
- the active Goal
- the planner world-state frame
- the Agent authorization
- the selected theory and capability identities
- the activation generation
- the external result

The supervisor has authority to report participant ownership loss and to fence an incarnation. It does not have authority to decide whether semantic work should be retried, resumed, accepted, replanned, or abandoned.

The canonical lifecycle design already states that operational health and program liveness are separate and that a stall does not authorize the supervisor to restart semantic work. The open item is the implementation bridge from operational interruption to ordinary Meld reassessment.

Relevant design:

- [Runtime Lifecycle And Quiescence](../../cognitive_architecture/runtime_lifecycle_and_quiescence.md)
- [PDS W06 Portable Lifecycle And External Admission Design](pds_w06_portable_lifecycle_admission_design.md)

### PDS-IF04 — Provider work is not represented as a durable external operation

The documentation draft and validation capabilities await provider completion inside capability invocation. The task-dispatch actor remains inside one supervisor step for the entire request sequence.

This conflicts with the authorized `W06` contract that long-running operations are persisted before bounded dispatch and completed through a later bounded poll or delivery followed by owner admission.

The missing durable boundary also prevents complete interruption handling. There is no separately visible operation state that can distinguish dispatched, running, ambiguously completed, returned, admitted, rejected, or safe to retry after participant loss.

Relevant implementation:

- `src/docs/capability.rs`
- `src/docs/claim_validation.rs`
- `src/runtime/supervisor/entrypoint.rs`

Relevant authorized tasks:

- `W06-T05` persist long-running operations before bounded dispatch
- `W06-T06` admit completion through a later bounded owner transition
- `W06-T10` implement interrupted recovery
- `W06-T11` reject or classify late results

### PDS-IF11 — One zero-work pass is projected as quiescence

Runtime tooling currently labels a tick account quiescent when every actor action reports `no_work`. It also labels a pass with no bound actors quiescent.

The canonical lifecycle contract requires more. Every required participant must be present or durably represented, and each idle owner must declare a waiting condition with a viable wake path. A missing required participant is stalled. A zero-work observation without a waiting declaration is active idle.

This projection does not appear to have caused the live lease failure, but it violates the same supervisor boundary. The supervisor can observe bounded work reports. It cannot infer program quiescence from process silence.

Relevant implementation and design:

- `src/runtime/tooling.rs`
- [Runtime Lifecycle And Quiescence](../../cognitive_architecture/runtime_lifecycle_and_quiescence.md)

Closure requires the runtime account to distinguish active idle, quiescent, stalled, and absent participant closure using owner lifecycle receipts rather than one tick outcome.

## Strategy Findings

### PDS-IF05 — Declared candidate cost materially understates realized work

The installed docs Strategy theory declares the five-step chain at a total estimated cost of:

```text
time_ms: 120300
provider_calls: 2
```

The live specimen observed:

```text
elapsed_seconds: 1037.64
completed_provider_requests: 50
```

Strategy ranks candidates using the declared aggregate operator costs. It does not expand the internal cardinality of a capability or consult measured execution cost. The selected candidate was therefore evaluated against a materially false operational model.

This finding does not establish that Strategy selected the wrong candidate. It establishes that Strategy lacked truthful information with which to make that judgment.

Relevant implementation and theory:

- `theory/docs_freshness/strategy_theory.docs_freshness.json`
- `crates/meld-world-model/src/strategy/search.rs`

### PDS-IF06 — Atomic capability contracts hide a procedural mini-strategy

The docs capabilities expose drafting and validation as two atomic Strategy steps. Their implementations contain substantial tactical procedure.

Drafting:

- visits every meaningful directory
- generates one README per directory
- carries accepted child README content upward
- retries selected provider failures

Validation:

- visits every generated README
- extracts claims deterministically
- divides claims into batches of six
- invokes the provider for every batch
- recursively splits structurally invalid batches
- may revise a README and reassess it
- may prune rejected claims at the configured limit

These are strategy-shaped choices about decomposition, ordering, batching, retry, revision, and fallback, but Strategy sees only one draft action and one validation action.

The live run against five expected README files required fifty provider requests. The retained evidence does not label every request by capability, so the exact draft and validation split is not proven. Code inspection and request timing support an inference of five initial drafting calls followed by roughly forty-five validation calls. The missing exact attribution is also part of `PDS-IF09`.

The open boundary question is which choices belong in docs-owned capability policy and which must become visible action alternatives or cost expansion for Strategy. Recording that question does not authorize a redesign.

### PDS-IF07 — The live specimen did not exercise meaningful strategy choice

The cold docs world exposed one effective artifact chain from inspection through freshness assessment. Strategy located the assessment capability and closed its required inputs backward through the available producers.

The construction was dynamic in form, but the specimen did not contain competing end-to-end candidates with materially different correctness, latency, cost, or provider requirements. The authored artifact contracts largely predetermined the five-step result.

The run therefore verifies structural composition and authorization. It does not verify that Strategy can recognize or choose a technically coherent but suboptimal route.

Future evidence for Strategy quality needs at least two eligible candidates with truthful and distinguishable consequences. That evidence request is separate from authorization to implement new strategy machinery.

### PDS-IF08 — Observed execution cost does not inform later Strategy judgment

Strategy evaluation is derived from static operator declarations. The observed request count, elapsed time, malformed response rate, batch splitting, and revision activity are not admitted as evidence that can affect later candidate evaluation.

Meld can therefore repeat the same underestimated judgment without learning that the capability realization was much more expensive than declared.

The open question is not automatically a new persistence subsystem. The first disposition must determine whether truthful capability expansion, calibrated declarations, ordinary world-model evidence, or another existing contract can carry enough observed consequence for later judgment.

## Relationship To Design-Gated Strategy Work

The design-gated `W08` item is Settled Strategy Replay. Its hypothesis is that an already selected and Agent-authorized candidate might need a new durable settlement record, replay mode, or reuse path.

The live findings concern a different part of Strategy. They concern the truth and breadth of the problem presented before Agent authorization.

| Finding | Caused by unfinished `W08` | Relationship |
| --- | --- | --- |
| `PDS-IF05` | no | replaying the candidate would preserve the same false cost estimate and expensive realization |
| `PDS-IF06` | no | replay does not expose procedure hidden inside an atomic capability contract |
| `PDS-IF07` | no | replay retains a selected candidate and does not create or compare alternative candidates |
| `PDS-IF08` | no | replay may avoid reconstruction cost, but it does not admit observed execution consequence into a later Strategy problem |

The implemented minimal Strategy slice deliberately retains at most one eligible candidate. It explicitly defers Pareto-frontier retention, global optimality, learned evaluation, multiple engines, and richer search machinery. `PDS-IF07` is therefore evidence of a known first-slice limit, not evidence that settled replay is missing.

The canonical Strategy design permits recommendation plus alternatives and requires declared cost and resource meaning in the Capability snapshot. It also requires every evaluation fact to be present in the immutable Strategy problem rather than read secretly from task-network state or provider internals.

That distinction gives the findings three different dispositions:

- `PDS-IF05` is present contract truthfulness work. The current Strategy engine already consumes cost, but the supplied declaration does not describe the realization.
- `PDS-IF06` is a capability-boundary question. Some docs-owned procedure may remain atomic, but its cardinality and consequences cannot remain invisible when they affect Strategy evaluation.
- `PDS-IF07` and the learning portion of `PDS-IF08` are deliberate first-slice omissions. They need a separately justified Strategy evaluation and alternatives design if they are to become implementation work.

The findings therefore do not supply a justification for Settled Strategy Replay. They expose a different potential continuation around evaluation truth, alternative construction, and admitted operational consequence. That continuation is not currently an authorized workstream and must not be smuggled into `W08` under the replay name.

Relevant scope documents:

- [Strategy Minimal Slice Requirements](../world_model/strategy/minimal_slice_requirements.md)
- [Strategy Search](../../cognitive_architecture/world_model/strategy/search.md)
- [PDS Design-Gated Continuations](pds_design_gated_continuations.md)

## Semantic Interface Findings

### PDS-IF12 — Strategy theory conflates meaning, policy, and affordances

The current `StrategyTheoryPackage` contains settlement rules, exact capability contracts, evaluation policy, search bounds, requested belief dimensions, and requested authority.

Only the settlement rule clearly expresses domain meaning needed to connect a Goal to prospective evidence. Exact available capabilities are activation-local affordances. Search bounds and candidate comparison are Strategy invocation policy. Effective authority is established by Agent judgment and independently enforced by execution.

The current docs and dependency-security bodies therefore arrive as hand-authored partial `StrategyProblem` templates. They do not merely teach Strategy what settlement means. They also constrain the action universe and much of the resulting topology.

This is distinct from `PDS-IF06`. That finding concerns procedure hidden inside one atomic capability. This finding concerns procedure and affordance selection embedded above the capability boundary in installed theory.

### PDS-IF13 — Final correctness probability crosses the evidence boundary

Documentation assessment publishes `stale_probability`, and the docs belief family consumes that scalar as its main required evidence. The substantive claim-support, contradiction, coverage, and lineage judgment has already occurred before the belief comparator runs.

The bounded dependency-security fixture has the same shape through `coverage_probability` and `clean_within_coverage_probability`.

An owner-produced assessment may legitimately become evidence, but a target-shaped scalar without independently admissible lower grounds reduces belief formation to arithmetic over a supplied conclusion. The interface needs proof-carrying evidence and explicit non-substitution rules so task or assessment success cannot masquerade as current belief truth.

### PDS-IF14 — State-free routing does not enforce the cognition boundary

The router correctly keeps owner bodies opaque and state-free. Those properties prevent central semantic ownership and live-state injection, but they do not prevent a theory body from encoding a fixed action chain, a final probability, or a preselected inference posture.

The refined boundary is that PDS supplies stable meanings, proof obligations, and maintained norms while Meld produces situated beliefs, Goals, Strategies, authorizations, and tasks. Belief questions may remain public domain vocabulary. Live belief conclusions and higher runtime artifacts may not be supplied by PDS.

The complete review and dependency-security falsification are recorded in [PDS Semantic Interface Review](pds_semantic_interface_review.md).

The accepted ownership boundary and canonical replacement direction are recorded in [Persistent Domain Stewardship](../../cognitive_architecture/persistent_domain_stewardship.md) and [PDS Cognition Boundary Assessment](pds_cognition_boundary_domain_assessment.md). These findings remain open as implementation findings until the compatibility aggregate and evidence path are replaced and verified.

## Verification Findings

### PDS-IF09 — Failure cleanup destroys output evidence

The live integration test creates its workspace and product stores under a temporary directory. The terminal assertion unwinds the test, and temporary-directory cleanup removes generated patches, validation reports, task state, and candidate README bytes.

Meld-controlled execution context does not by itself guarantee retained evidence. The outer verification harness currently owns the filesystem lifetime and can destroy every returned artifact after a runtime error.

Closure requires an evidence root that survives failure and records at least:

- exact package and owner revision identities
- Strategy candidate and authorization
- task-network state and outcomes
- provider request attribution
- generated patch-set bytes
- validation reports
- publication and assessment products
- runtime and shutdown diagnostics

### PDS-IF10 — Generated README correctness remains unknown

Drafting and validation completed, but the generated bytes were not retained and filesystem publication was not reached. The prior run cannot establish whether the five candidate README files were accurate, useful, appropriately scoped, or free of unsupported claims.

This is an unresolved verification result rather than evidence that the generated files were incorrect.

Closure requires a retained live rerun followed by direct content review and reconciliation against the validation reports and admitted source evidence.

## Settled Interpretations

The review has corrected several earlier shorthand explanations:

- the lease duration was fifteen minutes, not sixty seconds
- the supervisor did not actively cancel the provider work
- useful task work completed before the terminal error
- the terminal runtime failure occurred during shutdown bookkeeping
- the harness cleanup destroyed the temporary product after failure
- the run does not prove poor Strategy selection because no meaningful alternative was available
- the run does prove that Strategy evaluated the candidate with materially incomplete cost information

## Disposition Rules

- Runtime findings may not be closed by changing only the live harness.
- Harness findings may not be used to dismiss runtime or Strategy findings.
- A larger lease duration alone does not close synchronous-step starvation or durable-operation findings.
- A corrected static provider-call count alone does not close hidden capability procedure or missing observed-cost feedback.
- Strategy implementation beyond the current authorization requires an explicit scope decision.
- No finding in this register authorizes work from a design-gated continuation.
- Parity preservation of a current body does not promote that body shape into the final PDS public interface.
- State-free validation alone does not close `PDS-IF12` through `PDS-IF14`.

## Related Workstream Documents

- [PDS Authorized Implementation Workstreams](pds_implementation_workstreams.md)
- [Runtime Supervisor Goal, Invariants, And Domain Assessment](runtime_supervisor_invariant_domain_assessment.md)
- [PDS W00 Docs Characterization Protocol](pds_w00_docs_characterization_protocol.md)
- [PDS W03 Docs Routed Migration Design](pds_w03_docs_routed_migration_design.md)
- [PDS W06 Portable Lifecycle And External Admission Design](pds_w06_portable_lifecycle_admission_design.md)
- [PDS Design-Gated Continuations](pds_design_gated_continuations.md)
- [PDS Semantic Interface Review](pds_semantic_interface_review.md)
