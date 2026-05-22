# Further Research Prompts

Date: 2026-04-11
Status: active
Scope: prompt pack for ChatGPT Deep Research to test, refine, and challenge the Observe → Merge → Push architecture direction

## Use

- Run each prompt as a separate Deep Research job
- Prefer primary sources over summaries and commentary
- Ask for concrete examples from robotics, planning, cognitive architecture, distributed systems, and software agents
- Require a clear split between established consensus, plausible extension, and speculative analogy
- Require direct citations for every major claim

## Prompt 1

Goal: validate the high-level thesis and find where it breaks

```text
Evaluate this thesis with evidence:

"Observe → Merge → Push is a useful abstraction for the core loop of open-world agents. It maps onto observation, belief update, and action in planning under uncertainty, robotics, and several cognitive architectures. The fact that full world state is never known is not a limitation to patch over. It is the foundational design assumption."

Research questions:
- In which formal traditions is this equivalence strong, exact, or standard
- In which traditions is it only a loose analogy
- What are the clearest counterexamples or failure cases
- Which source traditions should anchor the claim if I want to state it rigorously

Requirements:
- Use primary sources where possible
- Include planning under uncertainty, robotics, Soar, ACT-R, blackboard systems, and at least one modern agent framework or agent paper
- Separate formal equivalence from metaphor
- End with a rewritten version of the thesis that is as strong as the evidence supports, but no stronger
```

## Prompt 2

Goal: test whether a temporal knowledge graph can play the role of belief state

```text
Research whether a temporal knowledge graph with uncertainty and provenance can serve as the world model for an open-world software agent.

Context:
- The proposed architecture has sensory agents that emit typed observations
- A merge layer integrates those observations into a world model
- Execution agents read that model and choose actions
- The world model is imagined as a temporal knowledge graph with thesis nodes, evidence links, uncertainty, and revision over time

Questions:
- What are the closest precedents in AI, robotics, databases, and knowledge representation
- How does this compare with belief states in POMDPs, dynamic Bayes models, truth maintenance systems, probabilistic databases, factor graphs, and blackboard architectures
- What information must such a model carry to support action selection under uncertainty
- What are the main failure modes, such as stale beliefs, contradictory evidence, unbounded growth, or expensive inference

Deliverable:
- A recommended minimum viable world model schema
- A list of capabilities that should be deferred until later
- A judgment on whether "knowledge graph as belief state" is rigorous, useful engineering shorthand, or misleading
```

## Prompt 3

Goal: research the sensory side as a first-class subsystem

```text
Research architectural patterns for modality-specific observation pipelines in open-world systems.

Target problem:
- Multiple sense organs observe filesystem change, git change, code structure, test results, external services, and runtime behavior
- They should emit observations or diffs rather than full snapshots
- They should be always on, parallel, and cheap enough to run continuously

Questions:
- What do robotics perception stacks, stream processing systems, change data capture systems, and event-sourced architectures teach here
- How should an observation be typed, timestamped, deduplicated, and linked to provenance
- When is a diff-native model clearly superior to periodic snapshotting
- What are the best practices for loss handling, backpressure, batching, and replay

Deliverable:
- A concrete observation contract for a software agent
- A comparison of three viable architectures
- A recommendation for what the first two sensors should be in a code workspace agent
```

## Prompt 4

Goal: study the merge layer as belief revision rather than simple aggregation

```text
Research merge and belief revision systems for agents that receive conflicting, partial, and time-varying observations.

Context:
- The proposed architecture uses a merge layer that turns observations into beliefs
- The system needs uncertainty, temporal provenance, and evidence lineage
- It should support later planning and reflection

Questions:
- What are the strongest precedents in Bayesian data fusion, truth maintenance, probabilistic logic, probabilistic soft logic, Dempster-Shafer style fusion, and multi-sensor robotics
- How do practical systems resolve contradiction, confidence decay, source reliability, and retraction
- Which approaches support incremental updates well
- Which approaches are too expensive or too brittle for a code workspace agent

Deliverable:
- A comparison table of candidate merge models
- A recommended merge strategy for version one
- Clear reasons for rejecting at least two alternatives
```

## Prompt 5

Goal: resolve the event spine ordering problem with evidence, not intuition

```text
Research sequencing and ordering models for a shared event spine used by many sensory, merge, and execution processes.

Target problem:
- Many writers emit observations and task outcomes concurrently
- The current design assumes a shared temporal spine
- The open question is whether ordering should use a single coordinator, per-process clocks with a merge layer, or a replicated log with leader-based sequencing

Questions:
- What correctness properties matter if the spine feeds a belief revision system
- When is total order required, and when is causal or partial order enough
- What do event sourcing, distributed logs, Lamport clocks, vector clocks, CRDT work, and robotics blackboard systems imply here
- What are the operational tradeoffs for throughput, simplicity, replay, and failure recovery

Deliverable:
- A decision memo with one recommended sequencing model
- The assumptions under which that recommendation holds
- A migration path from single-process to multi-process
```

## Prompt 6

Goal: determine how planning should sit on top of a changing world model

```text
Research planning and action selection methods for agents that plan over uncertain and changing beliefs rather than over a fixed known state.

Context:
- The architecture uses Observe → Merge → Push
- The planner reads the current world model and picks actions
- Actions may include both task execution and information gathering
- The world may change while planning and acting

Compare:
- POMDP and related belief-space planning
- Contingent planning and planning under uncertainty
- FOND planning where relevant
- HTN acting and hierarchical planning and acting
- Receding horizon or model predictive approaches
- Any strong modern software-agent planning approaches with explicit uncertainty handling

Questions:
- Which approaches are realistic for a code workspace agent
- Which approaches degrade gracefully under stale or partial beliefs
- How should planning interact with execution monitoring and replanning
- What minimal planning stack would preserve rigor without becoming research theater

Deliverable:
- A recommended planning stack for version one
- A map of what can be borrowed from HTN and what cannot
- A list of state assumptions the planner must never silently make
```

## Prompt 7

Goal: determine whether observation should be an explicit planned action

```text
Research active perception, information gathering, and value-of-information methods that decide when an agent should sense before it acts.

Target question:
- In an open-world software agent, should observation tasks such as re-scan, run tests, inspect a file, query git, or call an external service be modeled as first-class actions in the planner

Questions:
- What do active perception, dual control, belief-space planning, and metareasoning say about this
- Under what conditions is it rational to spend time sensing before committing to an action
- How do practical systems estimate the value of an observation when uncertainty, cost, and latency all matter
- What simple decision rules work well without requiring full optimal control

Deliverable:
- A practical policy for when the system should schedule observation actions
- A ranking of low-cost versus high-cost sensing actions
- A recommendation for how to encode uncertainty thresholds in the architecture
```

## Prompt 8

Goal: anchor reflection and self-calibration in established work

```text
Research methods for belief calibration and self-correction in systems that repeatedly make claims about the world and later observe outcomes.

Context:
- The architecture already has a primitive prior reducer
- The broader goal is for the system to learn which beliefs and predictors are reliable over time
- The system should revise confidence based on outcomes rather than only on local heuristics

Questions:
- What does the literature on calibration, probabilistic forecasting, Brier score, reliability analysis, online learning, and model monitoring contribute here
- How should a system track source reliability, predictor reliability, and domain-specific accuracy
- What history should be kept for later calibration
- What are the risks of feedback loops and self-reinforcing bias

Deliverable:
- A concrete design for calibration records and update rules
- Metrics for whether the merge layer is becoming better calibrated over time
- Failure cases that should be tested early
```

## Prompt 9

Goal: research capability growth without losing control of provenance and safety

```text
Research how an open-world agent can extend its own sensing and acting capabilities while preserving provenance, auditability, and safety.

Context:
- The long-term design includes capability synthesis
- If the current tool catalog cannot satisfy a goal, the system may need to compose, generate, or install a new capability
- This could include new sensors, new transforms, or new execution skills

Questions:
- What relevant precedents exist in program synthesis, tool learning, planner domain learning, robotic skill acquisition, and agent tool-use research
- What contracts or guardrails are needed before a synthesized capability can be trusted
- How should new capabilities be evaluated, versioned, sandboxed, and rolled back
- What parts of this are mature engineering versus speculative frontier work

Deliverable:
- A staged maturity model for capability growth
- A safety and provenance checklist
- A recommendation for the earliest useful and lowest-risk form of capability synthesis
```

## Prompt 10

Goal: turn the architecture into a research program with measurable success criteria

```text
Design an evaluation plan for an Observe → Merge → Push architecture in the setting of a code workspace agent.

Requirements:
- The plan should test sensing quality, merge quality, planning quality, execution quality, and full-loop adaptation
- It should include both offline replay and live online tests
- It should include ablations that remove or weaken the merge layer
- It should measure recovery from stale beliefs, contradictory observations, and environmental change

Questions:
- What benchmarks from planning, robotics, information fusion, and software engineering are relevant
- Which synthetic tasks should be created if no benchmark fits well
- What metrics matter most, such as task success, calibration, latency, unnecessary action rate, information gain per cost, and robustness under change
- How should one compare this architecture against a simpler baseline

Deliverable:
- A staged benchmark plan
- A baseline matrix
- A small first experiment that could be run in this repo
```

## Prompt 11

Goal: find the strongest historical architectural precedents

```text
Research historical system architectures that most closely resemble this proposed split:

- sensory agents that emit observations
- a shared temporal spine or common substrate
- merge or curation processes that integrate beliefs
- execution processes that act based on the integrated world model

Candidate traditions to examine:
- blackboard systems
- global workspace inspired systems
- robotic architectures
- distributed AI
- control systems
- event-sourced software systems where relevant

Questions:
- Which precedents are genuinely close in structure rather than only in vocabulary
- What did those systems get right
- What failed in practice
- What terminology from those traditions would sharpen the current design work

Deliverable:
- A shortlist of the three closest precedents
- A mapping from each precedent to the proposed architecture
- Lessons that should change the design now
```

## Prompt 12

Goal: force a skeptical synthesis before implementation momentum outruns evidence

```text
Act as a skeptical research reviewer for this proposed architecture:

"An open-world software agent should be built around Observe → Merge → Push, with always-on modality-specific sensors, a temporal uncertainty-aware knowledge graph as world model, and execution plus planning agents that act against that model through a shared event spine."

Task:
- Build the strongest possible case against this architecture
- Identify where the design is elegant but unnecessary
- Identify where established theory does not support the proposed implementation choices
- Identify simpler architectures that would likely capture most of the value

Then switch sides and provide the strongest evidence-based defense.

Deliverable:
- A steelman critique
- A steelman defense
- A final recommendation that states which parts deserve immediate implementation, which parts need more research, and which parts should be dropped
```

## Prompt 13

Goal: determine the correct multi rate bus hierarchy for sensing, integration, and canonical fact commit

```text
Research architectures for systems that must handle multiple temporal lanes rather than one uniform event bus.

Context:
- The proposed architecture now distinguishes several classes of flow:
  - raw sensory lanes with high rate and possible loss
  - intermediate integration lanes that perform temporal smoothing, tracking, or semantic extraction
  - a canonical temporal spine for durable semantic facts
  - downstream projections such as knowledge graph, task network, and context views
- The concern is that a single bus model may be too blunt and may either overload the canonical spine or force every domain into the wrong execution semantics

Questions:
- What do robotics perception stacks, stream processors, CEP systems, telemetry pipelines, and event-sourced systems teach about layered temporal channels
- How should one decide what remains in a fast lane versus what is promoted into a durable shared ledger
- What latency, loss, replay, and ordering guarantees are appropriate at each lane
- Which historical architectures most clearly separate raw signals, integrated observations, and canonical facts

Deliverable:
- A lane hierarchy model with clear boundaries
- Promotion criteria for moving data from faster lanes into the canonical spine
- Recommended guarantees for each lane such as loss tolerance, batching, replay posture, and ordering strength
- Failure modes when one tries to collapse all lanes into one bus
```

## Prompt 14

Goal: define the lowering contract for temporal distillation across lanes

```text
Research the design of a lowering component that converts high rate uncertain observations into slower semantic facts.

Context:
- The current working metaphor is a "temporal transistor"
- A lowering stage may integrate a raw stream over a short window, resolve jitter, track continuity, aggregate evidence, and emit semantically meaningful events to slower channels
- Example cases may include raw video to object tracks, dense filesystem watch churn to stable workspace deltas, or noisy test and runtime signals to service health facts

Questions:
- What are the closest precedents in sensor fusion, object tracking, temporal aggregation, online estimation, and stream summarization
- What fields must a lowered observation carry such as observed time, integration window, confidence, provenance, supersession links, and uncertainty
- How should lowering handle hysteresis, duplicate suppression, late arrivals, contradiction, and confidence threshold crossing
- What is the right split between lossy integration and durable provenance preservation

Deliverable:
- A minimum viable lowering contract
- A state machine for promotion from raw observation to integrated observation to committed fact
- A list of domain-independent rules and domain-specific knobs
- A recommendation for one first lowering path to prototype in a code workspace agent
```

## Prompt 15

Goal: generalize context attachment beyond filesystem nodes

```text
Research how context artifacts, annotations, beliefs, and decision records should attach to heterogeneous domain objects rather than only filesystem nodes.

Context:
- Current Context Frames are attached to filesystem nodes or prior frames
- The architecture is expanding toward multiple domains such as workspace objects, task runs, capability invocations, knowledge graph entities, evidence records, and possibly tracked temporal objects
- The current multi-domain spine design already introduces DomainObjectRef as a candidate universal anchor

Questions:
- What are the strongest precedents in annotation systems, provenance models, named graphs, entity linking, and temporal databases for generalized attachment
- Should one context artifact attach to exactly one anchor, to a typed set of anchors, or to a time interval
- How should attachment work for events, objects, beliefs, and evolving tracks that change over time
- When should context remain one concept versus splitting into evidence frames, belief frames, decision frames, or projection-specific artifacts

Deliverable:
- A recommended generalized anchoring model grounded in DomainObjectRef or a comparable abstraction
- A migration path from node-based frame basis to domain-agnostic anchors
- A judgment on whether "Context Frame" remains the right name once attachment is generalized
- Failure modes that would make the generalized model too abstract or too hard to query
```

## Prompt 16

Goal: determine execution substrate by domain rather than forcing one runtime model everywhere

```text
Research how to select the natural execution substrate for each domain in an open-world agent architecture.

Context:
- The current system has strong task and capability abstractions for deliberate work
- New sensory and curation domains may not fit task and capability well because they are continuous, stream-like, estimate-driven, or reducer-centric
- The emerging policy idea is "To each domain be true": each domain should use the execution model natural to its workload while still publishing canonical facts into the shared temporal spine

Questions:
- What domains are best served by workflow engines, task graphs, actor systems, stream processors, reducer loops, blackboard systems, or dataflow runtimes
- What decision criteria separate deliberate side-effecting execution from continuous sensing and belief revision
- Which historical systems succeeded or failed because they over-unified runtime semantics across incompatible workloads
- What common contract must every domain expose so that shared provenance, replay, and cross-domain reasoning still work

Deliverable:
- A domain-to-runtime classification matrix
- A policy memo for execution substrate selection by domain
- A minimum common lowering and publication contract that all domains must satisfy
- A recommendation for which current domains in a code workspace agent should stay on task and capability and which should not
```

## Prompt 17

Goal: formalize temporal semantics so the system distinguishes observation time from commit order

```text
Research temporal semantics for architectures that mix noisy sensory input, integration stages, and a canonical append-only fact spine.

Context:
- The current design relies on a monotonic spine sequence for deterministic replay and cross-domain temporal queries
- The newer sensing model introduces events that may be observed earlier than they are integrated or committed
- A single sequence may be insufficient because it can imply false causality or blur the distinction between world time and commit time

Questions:
- What do temporal databases, valid-time and transaction-time models, stream watermarks, causal ordering, and late-arrival handling contribute here
- Which timestamps or temporal fields are minimally necessary for robust reasoning and replay
- When is total order enough, and when must the system preserve causal order, observation intervals, or retroactive correction
- How should the knowledge graph represent belief validity, supersession, and evidence timing when facts arrive late or are revised

Deliverable:
- A minimum temporal schema for observation, integration, and canonical commit
- Rules for handling late evidence, retractions, and supersession without corrupting replay semantics
- A recommendation for how projections and planners should use sequence versus observed time
- Concrete examples of bugs caused by under-specified time semantics
```

## Prompt 18

Goal: test whether object continuity deserves its own domain distinct from events and knowledge

```text
Research whether systems that track persistent entities through time should model continuity as its own domain rather than folding it into either event history or knowledge representation.

Context:
- The architecture may need to represent persistent evolving things such as filesystem nodes across edits, task runs across retries, service health across intervals, or tracked objects in richer sensory settings
- This has been described informally as an "object momentum network" or continuity layer
- The concern is that identity over time, state estimation, and belief about an object are related but not identical concerns

Questions:
- What are the best precedents in object tracking, identity resolution, temporal entity models, digital twins, and process monitoring
- How do strong systems separate identity continuity, event history, and higher-level belief representation
- When should continuity be modeled as first-class state with its own update rules
- Would a dedicated continuity domain improve reasoning, or just add abstraction overhead

Deliverable:
- A recommendation for or against a dedicated continuity domain
- If yes, a minimal schema and its relationship to the event spine and knowledge graph
- If no, the least bad place to encode continuity concerns
- Evaluation criteria for deciding later with evidence rather than taste
```

## Prompt 19

Goal: test whether ECS is the right internal substrate for curation without letting it escape into the whole architecture

```text
Research whether Entity Component System is a good internal implementation substrate for the curation domain of an open-world software agent.

Context:
- The architecture has a shared temporal spine, sensory lanes, a curation layer, and deliberate execution domains
- The world model is a temporal knowledge graph with thesis nodes, evidence links, belief strength, provenance, supersession, and calibration
- The current repo is not starting from a blank slate
- Identity is still rooted in filesystem NodeID and FrameID in core paths
- Context uses immutable frames attached to nodes or prior frames
- Task and capability already provide a strong deliberate execution substrate
- The open question is narrow: whether ECS should power the live mutable internals of curation while persisted facts remain graph-shaped and spine-shaped

Questions:
- What kinds of knowledge graph and belief maintenance problems are a strong fit for ECS
- What kinds are a poor fit and become harder to query, replay, or debug
- What historical systems or modern systems have used ECS-like sparse state plus projector layers for knowledge integration, blackboard work, digital twins, robotics, or simulation
- How should one compare ECS against a purpose-built reducer-owned temporal graph with typed records and indexes
- What are the real migration risks when an existing system already has identity assumptions, event envelopes, and context attachment models
- Under what conditions does a hybrid approach make more sense, where ECS is internal to curation but public facts remain graph-shaped

Deliverable:
- A decision memo that compares pure temporal graph, ECS-backed curation core, and hybrid approaches
- A capability matrix and migration cost matrix
- A recommended boundary between internal curation machinery and public cross-domain contracts
- Explicit stop conditions that would justify rejecting ECS after a prototype
```

## Read With

- [Observe Merge Push](observe_merge_push.md)
- [Cognitive Loop](cognitive_loop.md)
- [Existing Design Mapping](existing_design_mapping.md)
- [Implied Components](implied_components.md)
- [Spine Sequencing Tension](spine_sequencing_tension.md)
- [Open World Agent](open_world_agent.md)
