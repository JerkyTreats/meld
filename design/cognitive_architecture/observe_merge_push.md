# Observe, Merge, Push

Date: 2026-04-11
Status: active
Scope: foundational cognitive loop, world model design, and multi-process architecture direction

## Prompt

Full world state will never be known. Even the parts we do know can change rapidly and at any time.

This is a declarative statement about the fundamental design direction of this system — not a
limitation to work around, but the foundational assumption everything is built from.

The core loop of all conscious entities:

```
Observe → Merge → Push
```

Raw sensory input. Integration of that input into an internally coherent model of the world.
Actions taken based on that model.

The system has built a strong Push layer. That is no small feat. But it is only one part of the
whole. The next problem is the World Model — a mix of motivations, goals, and assumptions about
the world, with the discovered need to sense and validate those assumptions as they relate to
active motivations.

Sensory data is raw. It is fast, streaming, dense. It is unstorable because there is too much,
too fast. Semi-independent sense organs convert raw sensory data into something appropriate for
World Knowledge. This is inherently diff-based — it is the changes from the prior that matter.
There is a Bayesian connection here.

World Knowledge is a Knowledge Graph. This KG is temporally based, with ever-evolving knowledge
of the world. Not just filesystem, but thesis about the world, evidence maps of those thesis.
The existing docs writer already has this structure: reflection, reflection, reflection.

The vision is more ambitious than code assessment and improvement. That is a thin vein of the
total possibility.

Sensory subsystem: multi-modal and parallelized. Independent Knowledge Graph integrators that
curate the KG from sensory inputs, research task completion, and task execution. Parallelized
execution agents that construct plans and execute various goals.

The event spine is the core, with a web of knowledge articulating around each observed point
in time. The larger architectural jump is multi-process: Sensory, Curation, and Execution agents
as independent entities whose core touchpoint is the Knowledge Graph.

---

## Response

The vision is coherent and the architecture already built points directly at it.

### The Loop Is a Cognitive Architecture

Observe → Merge → Push is a **sense-model-act loop**. Every agent framework, every robotics
system, every planning-under-uncertainty system has this structure at its core. The insight that
full world state is never known is not a limitation — it is the foundational assumption the whole
thing is designed from.

### How the Existing Design Maps

**The event spine is already the right center.**

Not because it was designed to be a cognitive bus — it was designed as a task execution log.
But the multi-domain spine document already anticipated the extension: `domain_id`,
`content_hash`, `DomainObjectRef`, the `knowledge_graph` domain stub, the cross-domain temporal
query model. The spine is a shared temporal ledger — the one thing every process can write to
and read from without needing to call each other.

```
Sensory agents   →  emit to spine  (domain: workspace_fs, git_events, ...)
Curation agents  →  read spine, write KG projections
Execution agents →  read KG, plan, dispatch tasks, emit task events to spine
```

No process calls another. The spine is the bus.

**The Knowledge Graph is the accumulated prior.**

This is the Bayesian connection made architectural. Every belief in the KG has a probability.
Thesis nodes are beliefs. Evidence maps are likelihood terms. Sensory inputs are observations
that shift posteriors. The prior reducer that maintains Bayesian priors per
`(node_id, decision_type)` is a small, early instance of a KG integrator — it reads events,
maintains a belief, makes it available for inference.

At scale: the KG is the world model. It is not a data store you query — it is the system's
current best estimate of what is true, with uncertainty, with temporal provenance, with evidence
lineage. The spine is the diff record. The KG is the integrated projection.

**The frame chain is already a belief revision history.**

Every frame written for a node is a belief snapshot — at this point in time, the system believed
this about this node. The frame chain is belief over time. The Bayesian prior calibration from
frame history is the system learning which of its past beliefs turned out to be accurate. That
is reflection in the precise technical sense.

### What Is Implied That Does Not Exist Yet

**Sensory agent framework.** The system's only current sensor is workspace scan. Sensory agents
need to be:

- Always-on, not task-triggered
- Modality-specific (filesystem, git, network, code analysis, external feeds)
- Diff-native — they emit changes, not state snapshots
- Fast enough that the spine does not become a bottleneck

Each sense organ converts raw input into a typed artifact and emits it to the spine. The
`workspace_scan_batch` event design is already the right shape for this.

**KG integrator agents (Curation).** These are the reducers that maintain the KG from spine
events. They are the merge layer. They:

- Subscribe to spine events across domains
- Update thesis nodes, evidence maps, belief probabilities
- Resolve conflicts between observations
- Maintain temporal provenance on every KG edge

The prior reducer is the first embryonic instance. Full curation means integrating evidence from
multiple sensory domains into coherent beliefs — not just whether a file has changed, but what
the system currently believes about a component's stability, quality, test coverage, and
documentation accuracy.

**KG data model beyond the stub.** The multi-domain spine names `knowledge_graph` as a domain
but does not define its object model. Thesis nodes, evidence edges, belief probabilities,
temporal versioning — these need a concrete design.

### The Key Architectural Tension

The spine's current sequencing model assumes deliberate writers — tasks completing, artifacts
emitting, discrete events. Sensory agents are the opposite: high-frequency, streaming,
potentially many writers concurrently.

The moment the system goes multi-process, it has a distributed sequencing problem. The current
`seq: u64` in the spine envelope assumes a single writer or a coordination point. With
independent sensory processes and curation processes all writing to the spine simultaneously,
the sequencing mechanism must become explicit:

- A spine coordinator process that assigns global sequence (single writer, others submit)
- Per-process sequences with a vector clock or logical clock merge layer
- An append-only log with leader election

This is the one infrastructure decision that must be made before multi-process becomes real.
The spine design is correct. The sequencing mechanism needs to be resolved.

### The Ambition, Stated Precisely

An **open-world agent** with:

- Streaming sensory input converted to typed observations by modality-specific sense organs
- A KG as the world model, updated continuously from observations and task outcomes
- Goals expressed as desired KG states, not fixed task sequences
- A planner that reads the current KG, forms hypotheses about what actions will move the KG
  toward goal states, and dispatches tasks
- Execution agents that run those tasks, emit results back to the spine, which flow back into
  the KG
- The loop closing: execution changes W, sensory agents observe those changes, curation updates
  K(W), planner re-evaluates goals against the new K(W)

The docs writer is not the goal. It is the first thin slice that exercises all three layers
simultaneously — sense (git diff, AST analysis), model (Bayesian evaluation, prior
calibration), act (generate documentation). The full loop, in miniature.

The synthesis capability is what makes this open-world rather than closed: when the planner
cannot satisfy a goal from the current capability catalog, it synthesizes a new capability.
The system can grow its own sense organs.

## Read With

- [Multi-Domain Spine](events/multi_domain_spine.md)
- [Await Observation Semantics](execution/control/program/await_observation_semantics.md)
- [Bayesian Evaluation Example](execution/examples/bayesian_evaluation.md)
- [Synthesis Overview](execution/control/synthesis/README.md)
- [Goals](../goals/README.md)
- [HTN Turing Plan](../htn_turing.md)
