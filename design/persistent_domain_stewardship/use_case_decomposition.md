# Use-Case Decomposition

Date: 2026-07-14  
Status: proposed  
Scope: adversarial qualification and decomposition of candidate Persistent Domain Stewardship applications

## Purpose

Persistent Domain Stewardship is intended to be a reusable application model, not a synonym for every long-running agent.

This document defines how to test a proposed use case, decompose it into stewardship-package modules, and determine whether Meld is appropriate relative to a simpler mechanism.

The output of the process is not an implementation plan. It is a package-oriented domain analysis that can be compiled into design artifacts.

## Stewardship Qualification

A candidate should be treated as a stewardship use case only when most of the following hold.

| Property | Qualification question |
|---|---|
| Bounded domain | Can the subjects, relations, responsibility, and exclusions be declared? |
| Persistent mandate | Does responsibility remain after one execution completes? |
| Independent change | Does the domain change without a direct steward action? |
| Partial observability | Must state be inferred from incomplete, stale, or conflicting evidence? |
| Standing objective | Is there a condition to maintain rather than a one-time deliverable? |
| Observation choice | Can the steward select additional evidence to acquire? |
| Intervention choice | Are several responses, including tolerance, possible? |
| Action economics | Do action, delay, and inaction have variable cost or risk? |
| Feedback | Can later observations evaluate the intervention? |
| Authority boundary | Can permissions, budgets, approvals, and prohibitions be declared? |
| Temporal continuity | Are prior evidence, decisions, and outcomes relevant later? |
| Audit value | Is it useful to reconstruct why the steward acted? |

A use case that lacks several of these properties may still use Meld components, but it should not drive the stewardship package model.

## Mandatory Baseline

Every decomposition must identify the simplest credible alternative:

- ordinary function or tool call
- scheduled job
- fixed workflow
- rule engine
- reconciler or controller
- optimizer
- behavior tree
- finite-state machine
- conventional RAG assistant
- domain-specific planning system

The analysis must state what additional result justifies Meld's event, graph, belief, Agent, planning, and outcome machinery.

The burden of proof belongs to the stewardship design.

## Failure Categories

A candidate is not a stewardship use case merely because it is:

- long-running
- stateful
- agentic
- connected to many tools
- described as a knowledge graph
- personalized
- probabilistic
- capable of taking actions

### Workflow disguised as stewardship

A fixed sequence such as:

```text
receive topic
→ research
→ write script
→ synthesize audio
```

is a workflow unless a standing learner or editorial objective remains active, observations revise the plan over time, and later outcomes influence future behavior.

### Monitor disguised as stewardship

A system that continuously reports drift but cannot acquire additional evidence or intervene is a monitor.

### Reconciler disguised as stewardship

A deterministic controller that reads authoritative state and applies one known correction does not need an epistemic world model.

### General agent disguised as stewardship

An agent with broad tools and memory is not a steward unless its mandate, scope, objectives, authority, and outcome semantics are explicit.

### Knowledge graph disguised as stewardship

Representing entities and relationships does not establish responsibility, decision policy, action authority, or feedback.

## Decomposition Procedure

Each candidate use case is decomposed through the following sections.

## 1. Domain Boundary

Declare:

- the bounded domain
- explicit non-goals and excluded domains
- principal subject types
- stable identity sources
- external systems of record
- the principal that grants the stewardship mandate
- the security and tenancy boundary

Questions:

- What exact thing is being maintained?
- Where does the steward's responsibility stop?
- Can the domain be partitioned into independent assignments?
- Which statements can Meld authoritatively assert, and which remain source claims or beliefs?

Output:

```text
DomainModuleSpec
ScopeTemplateSpec
Principal and authority assumptions
```

## 2. Descriptive Model

Declare:

- object types
- relation types
- attributes and units
- artifact types
- lifecycle events
- bitemporal requirements
- identity resolution and supersession rules

Questions:

- Which entities need stable historical identity?
- Which relations are current-state projections versus immutable facts?
- Which changes are replacement, correction, or contradiction?
- Which values require units or bounded types?

Output:

```text
DomainModuleSpec
```

## 3. Observation Model

For every observation channel, declare:

- raw source
- sensory lane or connector
- promotion rule
- promoted event schema
- event-time semantics
- subject extraction
- fact and graph projection
- provenance and security labels
- freshness and expected cadence
- failure and late-arrival behavior

Questions:

- Is the source authoritative, reported, inferred, or generated?
- Can channels disagree?
- What evidence is absent when the source is silent?
- Which observations are expensive enough to require planning?

Output:

```text
ObservationModuleSpec
```

## 4. Epistemic Model

For each concern, declare:

- belief family id
- question being assessed
- subject type and dimension
- admissible evidence schemas
- source mappings
- comparator or inference method
- prior
- freshness and decay
- contradiction handling
- planner-facing projection
- additional observations capable of reducing uncertainty

Questions:

- What does the posterior mean?
- What constitutes missing evidence rather than negative evidence?
- How can the belief be falsified?
- Which beliefs are shared across perspectives?

Output:

```text
BeliefModuleSpec
```

## 5. Stewardship Charter

Declare:

- perspective and trust policy
- valid assignment scope
- standing concerns
- desired, breach, and restore propositions
- hysteresis and stability window
- uncertainty tolerance
- inaction-cost model
- act, tolerate, observe, and escalate policy
- lifecycle and suspension conditions

Questions:

- What condition remains the steward's responsibility after one goal succeeds?
- When should divergence be tolerated?
- When should the steward gather evidence instead of intervene?
- Which beliefs are relevant to this perspective?

Output:

```text
StewardCharterSpec
StewardConcernBindingSpec
StewardshipObjectiveSpec
```

## 6. Operational Model

For every observation and intervention action, declare:

- action class
- operator preconditions
- expected effects
- required and produced artifacts
- capability requirements
- known methods and compositions
- cost model
- resource claims and conflicts
- compensation or rollback
- idempotency semantics

Questions:

- Can a fixed method solve the problem?
- Which decomposition choices must remain dynamic?
- Which work can be shared across objectives or assignments?
- Can the environment change while execution is active?

Output:

```text
ActionModuleSpec
meld-lang operators and methods
CapabilityRequirementSpec
```

## 7. Outcome Model

For every intervention class, declare:

- verification observations
- evaluation window
- success proposition
- partial-success proposition
- failure proposition
- harmful-outcome proposition
- evaluator independence
- attribution assumptions
- cost and value evidence publication

Questions:

- Does task completion prove domain improvement?
- What later event would disconfirm success?
- Can the proposer evaluate its own work safely?
- How delayed or confounded is the outcome?

Output:

```text
OutcomeContractSpec
```

## 8. Governance Model

Declare:

- requested authority class per action
- principal grant
- approval policy
- financial, compute, time, and intervention budgets
- rate limits
- prohibited actions
- data-access requirements
- audit requirements
- kill switches
- escalation rules

Questions:

- What is the maximum credible blast radius?
- Which actions are reversible?
- Can a package or planner expand its own authority?
- What happens when authority is unavailable?

Output:

```text
GovernanceModuleSpec
```

## 9. Scenario Model

Define scenarios that cover:

- nominal restoration
- tolerance
- insufficient evidence
- contradictory evidence
- stale evidence
- denied authority
- unavailable capability
- action failure
- harmful outcome
- external restoration
- concurrent environmental change
- replay and package upgrade

Output:

```text
ScenarioModuleSpec
```

## Classification Of Decomposed Elements

Every field discovered during decomposition must be classified as exactly one of:

| Class | Meaning |
|---|---|
| Kernel mechanism | Generic runtime behavior shared by domains |
| Package declaration | Declarative, versioned domain semantics |
| Executable plugin | Sensor, comparator, capability, evaluator, or adapter code |
| Runtime state | Observations, beliefs, episodes, goals, tasks, and outcomes |
| External authoritative state | Data retained in source systems of record |

This classification prevents package configuration from absorbing executable runtime logic and prevents the runtime from accumulating domain-specific branches.

## Cross-Domain Extraction Corpus

The package model should be tested against deliberately dissimilar use cases.

## Codebase Quality Steward

### Candidate

Maintain software quality across performance, persistence safety, reliability, security, usability, documentation, and maintainability.

### Strong stewardship properties

- machine-readable observations
- persistent subject identity
- several partially conflicting objectives
- reversible branch-based actions
- measurable tests and benchmarks
- independent repository change during execution

### Simpler baseline

CI, static analysis, dependency bots, scheduled coding agents, and fixed remediation workflows.

### Meld must purchase

- uncertainty-aware combination of heterogeneous evidence
- cross-quality arbitration
- selective validation
- suppression of redundant or low-value work
- persistent decision context
- plan repair during repository change
- outcome-based action cost and value revision

### Verdict

Strong fit. A single documentation-generation workflow is not sufficient proof; a multi-concern long-lived curator is.

## Game Faction Steward

### Candidate

Maintain a faction's survival, territory, resources, relationships, and strategic objectives under partial information.

### Strong stewardship properties

- objective world state with private perspective projections
- rumor, trust, and contradictory reports
- repeated decisions
- cheap simulation and counterfactual replay
- full control of outcome ground truth

### Simpler baseline

Behavior tree, utility AI, GOAP, blackboard, or scripted simulation.

### Meld must purchase

- persistent divergent beliefs
- provenance-sensitive testimony
- active scouting
- strategic plan repair
- long-term identity and memory
- explainable counterfactual decisions

### Boundary

Meld does not own per-frame aiming, steering, collision avoidance, or animation control.

### Verdict

Strong research fit at strategic timescales.

## Service Reliability Steward

### Candidate

Maintain SLOs, capacity margins, dependency health, recovery readiness, and remediation state.

### Strong stewardship properties

- high-rate observations
- uncertain diagnosis
- expensive or risky interventions
- measurable service outcomes
- persistent recurrence history

### Simpler baseline

Alerts, autoscaling, deterministic remediation, incident workflows, and runbooks.

### Meld must purchase

- selective diagnostics
- competing remediation hypotheses
- blast-radius and action-cost reasoning
- persistence across incidents
- recurrence and intervention-effect learning

### Boundary

Hard real-time controls, circuit breakers, and millisecond-scale recovery remain deterministic systems.

### Verdict

Strong fit at the minutes-to-months operational horizon.

## Learner Steward

### Candidate

Maintain learner mastery, prerequisite readiness, retention, misconception correction, and learning progression.

### Strong stewardship properties

- longitudinal subject identity
- partial and noisy evidence
- active diagnostic actions
- standing mastery objectives
- several content and practice capabilities

### Simpler baseline

Spaced repetition, Bayesian knowledge tracing, fixed adaptive sequencing, or an LLM with learner memory.

### Meld must purchase

- evidence provenance across modalities
- active diagnostic selection
- explicit uncertainty
- cross-topic continuity
- reproducible instructional decisions
- later retention and transfer feedback

### False-positive form

Research-to-script-to-voice generation is a workflow unless it closes the learner-model loop.

### Verdict

Conditional but credible. Outcome quality is weaker and more delayed than in software or simulation.

## Portfolio Thesis And Risk Steward

### Candidate

Maintain research theses, mandate constraints, exposure, risk, liquidity, invalidation conditions, and portfolio-level interactions.

### Strong stewardship properties

- uncertain and changing evidence
- explicit priors and counterevidence
- variable action costs
- persistent mandate and risk constraints
- delayed outcome history

### Simpler baseline

Research database, risk engine, optimizer, fixed rebalancing policy, and rule-based alerts.

### Meld must purchase

- decision-time thesis reconstruction
- active research selection
- belief decay and contradiction
- separation of research belief from execution authority
- risk-aware act/tolerate policy
- prospective outcome and attribution history

### Boundary

Bayesian representation does not create predictive edge. Live financial execution requires strict external controls and prospective validation.

### Verdict

Good fit for thesis and risk stewardship; weak initial fit for autonomous live trading.

## Cross-Domain Common Shape

The concrete vocabulary differs, but the decomposition shape remains stable:

```text
subjects and relationships
    ↓
promoted observations
    ↓
facts, graph projection, and evidence
    ↓
belief dimensions
    ↓
perspective and standing objectives
    ↓
observe / tolerate / act / escalate
    ↓
operators, methods, and capabilities
    ↓
verification observations and outcomes
    ↓
revised beliefs and future policy
```

A comparative view:

| Element | Code quality | Game faction | Reliability | Learner | Portfolio |
|---|---|---|---|---|---|
| Subjects | modules, APIs, tests | actors, factions, territory | services, deployments | learner, concepts | portfolio, positions |
| Observations | diffs, tests, benchmarks | sightings, reports | metrics, traces, incidents | responses, projects | prices, filings |
| Beliefs | quality and risk | threat, intent, trust | fault and capacity risk | mastery and misconception | thesis and exposure risk |
| Objective | quality bounds | strategic conditions | SLO and recovery | durable mastery | mandate and risk limits |
| Observation actions | tests, profiles | scout, interrogate | diagnostics | diagnostic question | research retrieval |
| Interventions | patch, refactor | allocate, negotiate | scale, rollback | teach, practice | rebalance, hedge |
| Outcomes | merge, tests, incidents | territory, casualties | SLO, recurrence | retention, transfer | return, drawdown |

This common shape justifies a package meta-model. It does not justify a universal domain ontology.

## Falsification Criteria

Persistent Domain Stewardship should be rejected or narrowed if any of the following become dominant.

### Packages restate workflows

If package files contain long procedural sequences, branches, retries, and control flow, the system has created a workflow language with additional terminology.

### New domains change runtime grammar

If every domain adds proposition kinds, planner branches, event-loop logic, or task states, the runtime is not domain-independent.

### Domain code dominates the package

Custom sensors, comparators, and capabilities are expected. If each domain also requires a bespoke world model, planner, and lifecycle engine, only a superficial schema is shared.

### Standing objectives remain prose

If desired state and breach/restore semantics remain inside prompts, stewardship cannot be replayed, tested, or audited.

### Outcome verification is unavailable

Without observable results, the system cannot distinguish successful execution from successful stewardship.

### Authority is entangled with capability

If importing or discovering a capability implicitly permits its execution, the model is unsafe for consequential domains.

### Simpler controllers perform equivalently

If a rule, workflow, optimizer, behavior tree, or reconciler achieves equivalent precision, cost, safety, and explainability, use the simpler mechanism.

### Package grammar changes for every case

The package model has not stabilized until several dissimilar domains can be represented without structural schema changes.

## Acceptance Criteria For A New Package

A candidate stewardship package should not enter implementation until it has:

- a bounded domain and explicit exclusions
- a named principal and authority source
- typed subjects, relations, dimensions, and artifacts
- at least one promoted observation route
- at least one falsifiable belief family
- a standing objective with breach and restore semantics
- an explicit simpler baseline
- at least one observation or intervention choice that requires adaptive reasoning
- an outcome contract independent of task completion
- a denied-authority scenario
- a harmful-outcome scenario
- a statement of what Meld measurably improves

## Recommended Design Exercise

Create complete decompositions for:

1. software performance and reliability
2. game faction strategy
3. learner mastery
4. service reliability
5. portfolio thesis and risk

For every discovered field, classify it as kernel mechanism, package declaration, executable plugin, runtime state, or external authoritative state.

Only then should the source schema in `package_model.md` be frozen into implementation types.
