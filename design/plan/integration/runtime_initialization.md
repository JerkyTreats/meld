# Runtime Initialization

Date: 2026-07-24
Status: active
Scope: the staged initialization contract for Meld runtimes, from empty directory to supervised flywheel, owning theory installation, identity genesis, and epistemic seeding

## Concern

Initialization was smeared across the runtime completion effort without an owner until this contract. Two requirements-gate decisions were initialization decisions in disguise. The ground map's `init` domain row was undecided. The CLI performs hidden graph catch-up around command routing, which is initialization leaking into command handling. Belief family configs are passed per call with no durable registry, so no record answers which theory revision an actor used at a given tick. The curation rule is a call-site parameter owned by nothing. The agent-native debugger's isolate boot re-derives all of the above in miniature.

This document names initialization as one concern with one contract, so that each of those fragments becomes a stage of a pipeline rather than an ad hoc decision.

## Two initializations

**Machine initialization** is physical hydration: resolve configuration, open stores, build ports, bind actors, start the supervisor. It creates no semantic state. It is repeatable at any time and is already well covered by the stewardship-expression and supervision workstreams.

**World initialization** is genesis: the creation of the first durable meaning in an empty world. It is semantic, it must carry provenance, and it happens once per world rather than once per process. Conflating the two is the root of the smear: bootstrap ownership was a hard gate decision only because it fused a genesis question with a hydration question.

[Agent Genesis And Activation](../../cognitive_architecture/world_model/agent/genesis_and_activation.md) already draws this line correctly for one slice of the problem: creation establishes durable identity with recorded provenance, activation is process hydration that never creates. This document generalizes that split to everything initialization touches. That agent document remains canonical for the identity stage.

## The staged pipeline

```text
stage 0  resolve     config and physical binding           pure, no state
stage 1  open        stores, ledger, ports                 machine, idempotent
stage 2  install     theory: belief families, curation
                     rules, evidence mappings, methods,
                     task packages                          idempotent commands,
                                                            content-hashed revisions
stage 3  genesis     identities: seed agent, perspective,
                     subscriptions                          idempotent commands
                                                            with provenance
stage 4  seed        epistemic genesis facts                ledger append,
                                                            idempotent
stage 5  activate    hydrate actors, start supervisor       no semantic writes
```

Stages 0 and 1 are machine initialization. Stages 2 through 4 are world initialization. Stage 5 is machine initialization again and is the only stage a later process start repeats in full.

## Stage contracts

### Stage 0 — resolve

Owned by the stewardship expression and physical binding workstream. Pure loading and validation: XDG-resolved configuration, explicit workspace selection, the minimal stewardship expression, and the physical binding for workspace, subject, agent, provider, and external storage. Produces no state anywhere.

### Stage 1 — open

Owned by root storage and assembly. Opens the product storage layout and the event authority. Idempotent by construction: opening an existing world changes nothing. Resource opening is scoped to the composed registration set per the harness enablement hooks.

### Stage 2 — install theory

The stage with no current home, and the largest gap this document creates work for.

Theory artifacts are the state-free meaning the runtime executes: belief family configs, curation rules, evidence mapping selections, planning methods, and task packages. Today the family config is passed per call, the curation rule is a parameter at every entry point, and no durable record binds an actor tick to the theory revision it used.

The contract:

- Each theory kind has a durable registry in its owning domain, keyed by identity and content-hash revision on the existing `ConfigSnapshot` pattern.
- Installation is an idempotent domain command. Reinstalling the same content hash is a no-op. Changed content is a new revision, never a mutation.
- Actors resolve the current theory revision per tick through their owning domain, and that revision enters the frame lineage their outputs cite. Which theory produced which belief must be answerable from durable records alone.
- The stewardship expression selects theory by identity and revision. It does not embed theory bodies.
- The evidence mapping is selected by id from the stewardship expression, resolving the gate's artifact-semantics decision as a stage 2 row.

### Stage 3 — genesis identities

Canonical authority is [Agent Genesis And Activation](../../cognitive_architecture/world_model/agent/genesis_and_activation.md). Seed agent creation from trusted init with recorded provenance, perspective registration, curation-rule binding, and subscription binding to the belief keys stage 2 made resolvable. Idempotent by registration identity, which the existing `SeedAgentRegistration` path already provides.

The curation rule gains a durable home in this stage: bound to the agent registration as installed theory rather than re-supplied by every caller.

### Stage 4 — seed epistemic facts

The first stale signal is a genesis fact appended to the ledger through the canonical append capability, idempotent by event identity. The empty world learns that its selected scope is unobserved the same way it will learn everything else: through its own sensory channel, with provenance.

The remaining product choice is the fact's content — an unobserved-scope declaration for the selected subtree is the recommended shape — not who owns seeding, which this pipeline settles.

### Stage 5 — activate

Hydration only. Load durable agent records whose lifecycle says active, reconstruct cursors and watchers, bind actors, start the supervisor. Activation never creates: a world with incomplete genesis produces the truthful unresolved-required-binding state from the supervision workstream, not silent manufacture of missing state.

This stage rule retires the hidden CLI graph catch-up. Catch-up is actor work under supervision, not command-routing side effect.

## Cross-cutting rules

1. **Commands for registrations, events for facts.** Theory and identity install through idempotent domain command paths. Epistemic seeding is a ledger append. Nothing in any stage writes a domain store directly from root or CLI code.
2. **Idempotent by content identity.** Every stage may be re-run at any time. Same content hash, no effect. New content, new revision with lineage.
3. **Activation never creates.** Any path that manufactures semantic state during process start is a defect of the same class as false health.
4. **Every stage is Strategy-replaceable.** Stage 4's genesis fact is the compatibility form of a curated observation Goal through the future admission gate. The directive string bound at stage 3 is the compatibility form of the structured Directive record. Stage 2 is the compatibility form of consuming a compiled stewardship image. Staging keeps each an insertion point rather than a rewrite.

## Consequences for the requirements gate

- **Bootstrap ownership dissolves.** Explicit initialization owns stages 2 through 4; the runtime owns stage 5. Both gate options were half right because the decision fused two stages.
- **First stale signal narrows** to the content of the stage 4 fact.
- **Artifact-mapping source resolves** to stage 2 selection by id from the stewardship expression.

## Relationship to the agent-native debugger

An isolate boot is this pipeline scoped to a registration subset: stages 0 through 5 with one domain's theory, one identity when the domain needs one, and synthetic stage 4 facts. Product and harness must share the pipeline — a harness that initializes differently from the product debugs a system that does not exist. This is the structural form of DBG-011 in [Agent-Native Debugger Requirements](agent_native_debugger_requirements.md).

## Work this creates

- A durable belief family registry with revision resolution in `meld-world-model`.
- A durable home for the curation rule on the agent registration.
- Theory revision identity in actor frame lineage.
- The stage 4 genesis fact contract and its idempotent append.
- An explicit initialization command surface over stages 2 through 4.
- Removal of CLI graph catch-up in favor of supervised actor work.

## Not in scope

- A stewardship package compiler, profile language, or activation records.
- Spawned agent creation, which remains the curated Goal path.
- Any presentation surface.

## Read with

- [Runtime Completion Ground Map](runtime_completion_ground_map.md)
- [Runtime Completion Implementation Workstreams](runtime_completion_implementation_workstreams.md)
- [Agent Genesis And Activation](../../cognitive_architecture/world_model/agent/genesis_and_activation.md)
- [Agent-Native Debugger Requirements](agent_native_debugger_requirements.md)
- [Strategy Ground Map](../world_model/strategy/ground_map.md)
