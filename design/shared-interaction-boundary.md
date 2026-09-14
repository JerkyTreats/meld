# Shared Interaction Boundary

## Thesis

Meld realizes a shared interaction as ordinary admitted cognition and executable work. An interaction owner supplies one immutable request revision. Persistent Domain Stewardship supplies exact standing meaning and capability requirements. Agent and Strategy construct and authorize complete work. Execution realizes that work through exact capability revisions and waits on durable owner evidence without involving a model.

This boundary does not create an interaction scheduler, a generic timer, a second conversation store, or a second planner.

## Ownership

### Interaction owner

The interaction owner defines the request parameter grammar and retains immutable request revisions. A request revision may include the required motion duration, qualification policy identity, capture policy, synthesis selection, and presentation constraints.

Changing values under the same admitted grammar creates a new request revision. It does not rewrite the PDS package or capability contract. Changing the grammar or semantic contract requires a new exact owner revision.

The interaction owner also owns temporal qualification over admitted source samples. It defines continuous versus cumulative motion, idle-gap behavior, monotonic time interpretation, engagement epochs, disarming rules, and the bounded qualified-evidence product.

### Persistent Domain Stewardship

PDS declares the standing interaction outcome, required owner theory, admissible capability contracts, Strategy knowledge, authority requirements, and activation participants.

PDS selects immutable package and capability revisions. It does not hold request instances, measure elapsed motion, poll for completion, schedule retries, or interpret conversation state.

### Wallpaper owner

Wallpaper owns native simulation and rendering along with source, viewport, and input attribution. It publishes scoped samples or an owner-defined native observation product to the interaction owner.

Wallpaper does not decide whether the complete shared interaction is satisfied. A real rendered canvas is sufficient for the native surface boundary. Full lab chrome is not required.

### T3 owner

T3 owns authenticated conversation position, interaction resource state, presentation within the transcript, cancellation at its boundary, and continuation admission.

T3 transports or references the interaction request selected by the initiating Agent. It does not own the interaction parameter grammar, motion qualification, Meld Task state, or PDS meaning.

### Meld world model

The Agent admits one named reconciliation request under its current activation and admission epoch. The request key identifies one immutable interaction-owner request revision.

Epoch preparation resolves that exact owner revision and freezes its values as Task inputs. Maintained-condition evaluation decides whether work is needed. Strategy may construct several native planning operations and one or more complete Tasks from the exact admitted capabilities and frozen inputs. Agent authority admits or rejects each finished product.

The world model does not perform sensory timing or operational waiting.

### Meld Execution

Execution validates and admits Agent-authorized Tasks, preserves exact capability and input identity, claims ready work, persists artifacts and outcomes, and gates downstream work through artifact data flow.

Execution does not infer missing semantic steps or reinterpret qualified evidence. It treats a long-lived interaction as owner work that is unresolved until durable owner evidence can reconstruct a result.

### Events

Events preserves neutral envelopes, provenance, idempotent record identity, replay, durable cursors, and watermark wake. Domain payload meaning remains with the producer and accepting owner.

Raw high-rate input is not generic Event truth. Meld receives bounded owner products and evidence references after interaction-owner admission.

### Runtime Lifecycle and Core Composition

Runtime Lifecycle realizes exact activation generations, participant incarnations, admission fences, durable waits, structural wake paths, recovery, and retirement. Core Composition connects exact owner contributions and physical bindings without absorbing their semantics.

The supervisor may maintain operational health and wake on durable Events. It never uses a model to poll a quiescent interaction and never owns the three-second condition.

## Request Admission

The public request key is an idempotent identity, not an untyped parameter container. Before Strategy construction, an interaction-owned preparation boundary resolves the key to one immutable request revision and proves the relationship among:

- initiating request identity
- interaction-owner revision identity
- current Agent activation and admission epoch
- intended conversation position reference
- exact input schema revision
- frozen Task input values

Missing, mutable, stale, or mismatched owner data fails closed. The request remains unresolved rather than being reconstructed from current defaults.

The transport used to create or reference the owner revision is outside Meld semantic ownership. Transport adaptation must preserve the owner product intact.

## Capability Admission

Every executable interaction operation enters through the canonical exact capability path:

1. An owning domain publishes a capability contract and implementation offer.
2. The PDS package selects the exact contract content identity.
3. Activation preparation selects one exact implementation and required physical bindings.
4. Strategy sees the admitted semantic capability view.
5. Agent authorizes a complete Task using frozen request inputs.
6. Execution independently validates the Task against the prepared catalog before dispatch.

A new script or native adapter is not executable merely because a package names it. It needs an owner-published contract, an exact content revision, an implementation offer, activation preparation, authority, and Task admission.

Request value changes remain Task input changes when the accepted schema and capability contract are unchanged.

## Deterministic Interaction Flow

A no-model and no-summary interaction graph may contain several native operations and is not limited to one planning action.

```text
immutable interaction request
  -> admitted Agent epoch inputs
  -> T3 interaction creation
  -> native Wallpaper presentation
  -> qualified-motion owner wait
  -> retained capture
  -> T3 continuation request
```

Each arrow is either an explicit Task dependency or an owner acceptance boundary. A capability output becomes a typed artifact. Downstream work remains pending until its required artifact is durable.

The qualified-motion result must identify the request revision, interaction resource, activation generation, admission epoch, engagement epoch, source sequence range, monotonic timing basis, qualification policy revision, and evidence reference. It must not claim more than the owner admitted.

## Waiting and Recovery

Human latency must not occupy a supervisor step. The interaction capability begins one idempotent owner operation, records enough owner state to recover it, and returns unresolved while completion is absent.

A later dispatch recovery attempt may complete only from durable owner evidence. It must not repeat the external effect, infer completion from process memory, or convert absence into failure. An Event watermark may wake maintenance early, while the owner remains responsible for validating the returned evidence.

Recovery distinguishes:

- operation identity from attempt identity
- activation generation from participant incarnation
- interaction resource lifetime from presentation lifetime
- engagement epoch from provider or conversation turn lifetime
- qualified evidence from raw samples
- successful capture from successful continuation admission

Late, duplicated, stale, cancelled, or foreign evidence remains visible and cannot complete current work under a new identity.

## Capture, Synthesis, and Continuation

A synthesis-free graph captures retained evidence and requests one T3 continuation without a model operation.

When synthesis is selected, that selection comes from the immutable request before Task authorization. Strategy constructs the exact graph that either includes or omits the synthesis operation. Execution does not choose a semantic branch at runtime.

Capture must finish before normal teardown. Continuation consumes a bounded result and provenance references rather than raw input. T3 admits or rejects continuation against its current authenticated conversation state and preserves the actual actor provenance.

## Cancellation and Termination

Cancellation closes new effects for the addressed admission while preserving already durable owner history. The interaction owner, Wallpaper owner, T3 owner, and Execution each apply cancellation only to state they own.

Loss of trustworthy presentation or input ownership disarms the engagement epoch. Re-engagement creates a fresh epoch. Stale samples cannot extend later progress.

Terminal interaction evidence does not imply that capture succeeded, that continuation was accepted, or that the maintained condition has been restored. Each owner publishes its own result, and Agent reconciliation judges the complete outcome from admitted evidence.

## Architectural Rejections

- Do not put request parameter grammar in T3 transport or generic PDS machinery.
- Do not make PDS a scheduler, timer, observation store, or execution runtime.
- Do not place the three-second timer in generic Runtime Lifecycle or Execution.
- Do not let Events validate interaction semantics.
- Do not keep a model turn open or ask a model to poll while waiting.
- Do not execute a script from package text without exact capability revision admission.
- Do not rewrite a PDS package merely because request values changed.
- Do not use raw pointer traffic as generic Event truth or retained transcript content.
- Do not treat a detached native window, synthetic input, or screenshot-only result as the native causal proof.
- Do not make Execution infer synthesis selection from a runtime condition.

## Acceptance Boundary

Acceptance requires one immutable owner request, one exact admitted capability set, one real native rendered canvas, deliberate scoped input, owner-qualified temporal evidence, retained capture, one admitted T3 continuation, and truthful cleanup.

Acceptance also requires idempotent initiation, no model calls while waiting, restart recovery without repeated external effects, rejection of stale completion evidence, and request value changes without selected package revision changes.

This document defines ownership, intended behavior, and acceptance boundaries.
