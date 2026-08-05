# Strategy Minimal Slice Requirements

Date: 2026-08-05
Status: active
Scope: first runnable world-model Strategy proof through the docs-freshness product path

## Authority

This document defines implementation scope for the first Strategy slice.

The cognitive architecture [World Model Strategy](../../../cognitive_architecture/world_model/strategy/README.md) and [Strategy Boundary Contracts](../../../cognitive_architecture/world_model/strategy/contracts.md) define evergreen meaning and authority boundaries. This document supplies delivery-specific scope and acceptance requirements for the first slice.

## Objective

Strategy must turn one Agent-curated Goal draft into one evidence-backed concrete course of action before that Goal enters Execution.

The slice proves the authority boundary and the complete product path:

```text
belief divergence
→ Agent Goal draft
→ Strategy candidate
→ Agent judgment
→ Goal admission
→ authorized realization
→ substantive outcome
→ evidence ingestion
→ belief reconciliation
→ Agent satisfaction
```

## Product proof

The proof uses one ground docs-freshness Goal, one exact planner frame, one configured Method, one available action, and one prospective evidence route.

The configured action runs through the existing docs-writer package route and produces a real `README.md`. A substantive package outcome enters the existing evidence and belief path. Only Agent satisfaction over reconciled belief closes the Goal.

Test code may install theory, inject the initial observation, and drive bounded runtime ticks. It must not manually perform semantic handoffs after product assembly starts.

## Universal invariants

### SMSR-001 Agent authority

Strategy constructs candidates. The Agent authorizes the selected candidate. Construction, persistence, and successful execution do not confer Agent authority.

### SMSR-002 Admission gate

An Agent-curated Goal must not enter Execution without a nonempty Agent-authorized candidate.

### SMSR-003 Epistemic isolation

Strategy consumes exact world-model, belief, and theory inputs. It must not create a second evidence-admission engine, comparator, causal model, or satisfaction decision.

### SMSR-004 Execution isolation

Execution may revalidate and realize authorized meaning. It must not select another Method, add semantic work, alter claimed outcome meaning, or satisfy the Goal.

### SMSR-005 Observational outcome separation

An observational Goal must be pursued through a prospective evidence route. A Method effect must not assert that the observational Goal condition became true.

Task completion, artifact creation, projected effects, and planning commitment are not Goal satisfaction.

### SMSR-006 Settled replay

Candidate consideration and Agent epistemic choice may be nondeterministic. Once the Agent judgment is durably settled, replay must reuse the exact judgment and selected candidate payload without constructing or choosing again.

Operational actions derived from that judgment must remain reproducible and idempotent under Execution contracts.

### SMSR-007 Failure closure

Missing, indeterminate, stale, invalid, unavailable, ambiguous, or unauthorized inputs must produce no admitted Goal and no task-network mutation.

## First-slice behavior

### SMSR-010 Bounded input

The Strategy attempt accepts only:

```text
one ground proposed Goal
one exact planner frame
one configured Method
one Method-to-action realization
one action outcome contract
one prospective evidence route
one configured Agent judgment policy
```

The attempt produces at most one candidate and performs no recursive Goal expansion.

### SMSR-011 Candidate construction

Strategy must reuse existing shared-language unification, substitution, evaluation, and Composition validation.

The candidate is eligible only when:

- the Goal is ground and proposed
- the frame has exact identity and matches the Goal scope
- the Method trigger binds the Goal target
- every precondition is satisfied
- every binding is ground
- the concrete Composition is structurally valid
- the Method resolves to exactly one available action
- the action has a valid existing realization route
- the prospective evidence route is valid for the Goal dimension

### SMSR-012 Prospective evidence route

For the configured observational dimension, the candidate must identify:

```text
Method identity
action identity
outcome contract identity
outcome mapping identity
substantive outcome rule identity
required evidence schema identity
Goal dimension and subject
```

The route is a prediction that execution can produce admissible evidence. It is not an assertion that the Goal threshold will be crossed.

### SMSR-013 No candidate

When the configured Method cannot produce one eligible candidate, Strategy returns a typed no-candidate result. The Agent decision may be persisted, but no Goal command is submitted.

### SMSR-014 Agent judgment payload

The Agent-authorized payload contains only:

```text
authorization identity
Agent decision identity
Goal identity and target digest
planner frame identity and source references
Method identity and content identity
ground bindings
concrete Composition and content identity
action identity and outcome contract
prospective evidence route
policy identity and version
```

The payload must not contain an execution schedule, task identity, provider binding, inferred outcome, or state unrelated to this settled judgment.

### SMSR-015 Durable judgment before submission

The existing Agent curation decision must persist the exact authorization payload before the Goal command crosses the curation-to-goal-set port.

A retry after persistence must submit the stored payload. It must not rerun Strategy construction or Agent choice.

### SMSR-016 Goal admission

The existing Agent Goal command and Execution Goal acceptance request carry the authorization payload through the named curation-to-goal-set boundary.

Execution validates that the authorization matches the Agent, Goal, target, Method, Composition, and frame identities. It stores the accepted operational copy with the Goal using the existing Goal store.

No Strategy store or admission service is introduced.

### SMSR-017 Authorized planning

For a Strategy-gated Goal, Execution reads the admitted candidate and bypasses Method search.

Execution revalidates Method content, bindings, current action availability, realization uniqueness, Composition structure, and lifecycle eligibility before lowering the exact Composition.

### SMSR-018 Existing downstream path

Composition lowering, task-network mutation, dispatch, docs-writer execution, publication, outcome interpretation, evidence ingestion, belief reconciliation, satisfaction curation, and Goal lifecycle mutation remain owned by their existing domains.

The Strategy slice may extend lineage with the authorization identity where required to prove continuity. It must not redesign those domains.

### SMSR-019 Architectural limits

The first slice adds no:

- crate
- workspace dependency
- Strategy-specific durable store
- standalone Strategy actor or service
- compatibility system
- generalized Strategy framework

## Acceptance evidence

### SMSR-020 Construction proof

Focused world-model tests prove one eligible candidate and typed rejection for trigger miss, unsatisfied or indeterminate preconditions, unbound substitution, invalid Composition, ambiguous realization, stale frame, and invalid prospective evidence route.

### SMSR-021 Judgment and replay proof

Agent tests prove that the exact authorization persists before sink submission and that recovery submits the stored payload without reconstruction.

### SMSR-022 Admission proof

Execution Goal tests prove rejection of missing, empty, mismatched, and content-invalid authorization. Duplicate admission returns the existing Goal and byte-equivalent authorization.

### SMSR-023 Realization proof

Execution planning tests prove that only the admitted Method, bindings, Composition, and action can run. Altered or unavailable inputs fail before task-network mutation.

### SMSR-024 Outcome separation proof

Tests prove that projected effects, planning success, task completion, and artifact creation do not satisfy the Goal.

### SMSR-025 Product proof

One product integration test drives the assembled docs-freshness runtime through substantive README publication, evidence ingestion, belief reconciliation, and Agent satisfaction.

The test proves one authorization identity connects the Agent decision, accepted Goal, realized Composition, and resulting work.

## Read with

- [Strategy Assessment By Domain](assessment_by_domain.md)
- [Strategy Ground Map](ground_map.md)
- [World Model Strategy](../../../cognitive_architecture/world_model/strategy/README.md)
- [Strategy Boundary Contracts](../../../cognitive_architecture/world_model/strategy/contracts.md)
