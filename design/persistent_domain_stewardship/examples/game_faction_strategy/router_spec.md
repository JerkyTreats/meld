# Game Faction Strategy Router Spec

Date: 2026-08-18
Status: discovery router exercise aligned to canonical PDS boundary
Scope: candidate lowering of faction strategy stewardship through `theory::router`

## Purpose

This exercise asks whether one persistent strategic controller can install and activate through the common router while simulation truth, faction policy, belief, planning, execution, and outcome meaning remain domain-owned.

The route map is illustrative. Perspective projection and counterfactual-result admission are unresolved owner bridges, not settled router contracts.

## Semantic Package And Runtime Input Map

| Component id | Candidate route | Owner | Meaning and required links |
| --- | --- | --- | --- |
| `faction-threat` | `world-model.belief-family.v1` | belief | Evidence-grounded threat likelihood and severity |
| `faction-intent` | `world-model.belief-family.v1` | belief | Perspective-bound assessment of another actor's intent |
| `faction-trust` | `world-model.belief-family.v1` | belief | Provenance-sensitive trust and testimony reliability |
| `faction-resource-security` | `world-model.belief-family.v1` | belief | Security and adequacy of protected resources |
| `faction-plan-viability` | `world-model.belief-family.v1` | belief | Current support that an active strategic plan can still work |
| `faction-maintained-conditions` | `world-model.agent-maintained-condition.v1` | Agent | Viability, resource floors, awareness, commitments, and plan health |
| `faction-curation` | `world-model.agent-curation-rule.v1` | Agent | Tolerate, observe, plan, repair, act, or escalate |
| `faction-strategy` | `world-model.strategy-theory.v1` | Strategy | Settlement, prospective evidence, strategic action, outcome, and constraint meaning |
| `faction-capabilities` | activation capability contribution | capability and execution | Exact contracts for observations and strategic intents |
| `faction-authority` | assignment governance selection | Agent and execution | Resource scope, treaty constraints, escalation, and irreversible action classes |
| `faction-outcomes` | `world-model.outcome-mapping.v1` | outcome and belief | Maps admitted world outcomes into maintained-condition and belief evidence |
| `faction-policy` | `faction.policy.v1` | faction or simulation domain | Faction scope, strategic vocabulary, treaty semantics, risk posture, and protected assets |
| `faction-evidence-routes` | candidate `world-model.evidence-mapping.v1` | belief | Maps admitted perspective products into exact belief families |

The semantic package links strategic action classes and outcomes to world-owner result meaning. Current Strategy candidates cite exact activation capability contracts and derive authority requirements from selected actions. The faction owner validates strategic and treaty meaning. Theory validates only exact semantic component closure.

Current sightings, beliefs, resources, treaties, plans, operations, and territory remain runtime state outside the package.

## Owner Semantics And Cross Links

The simulation owner holds objective world state, action legality, mechanical resolution, and authoritative world outcomes. The faction owner holds faction vocabulary, protected assets, treaty meaning, and strategic policy. Belief owns epistemic revision over admitted faction evidence. Agent owns maintained-condition decisions and construction policy. Strategy owns Method admission, composition, and candidates. Execution owns claims, effects, retries, and authority enforcement.

```text
world event or source report
→ simulation and faction admission
→ perspective-safe observation
→ evidence mapping
→ belief revision
→ maintained-condition decision
→ observation or strategic method
→ exact capability invocation
→ world-owner result
→ outcome admission and verification
```

The linker must reject an operation method without a capability, effect class, and authority policy. It must reject a trust or threat mapping that can consume omniscient evaluator state under the faction perspective.

Perspective projection could later be installed through a graph or simulation owner route, but this exercise leaves that route unresolved until an owner publishes an exact state-free contract.

## Assignment And Activation

One assignment binds one exact package receipt to one principal, faction subject, faction perspective, simulation branch, requested authority, and principal grant. Rival factions are separate assignments even if they use the same package and simulation host.

Activation may bind a simulation endpoint, scenario ref, faction control channel, counterfactual runner, local policy adapter, credentials, or an owned subprocess. Package identity does not include any of those bindings.

Preparation builds one assignment-local observation surface, capability catalog, executor registry, and authority context with admission closed. Current publication waits for the world event source, perspective projection, action channel, result admission, and any selected counterfactual service to prove generation-scoped readiness.

## Capability And Authority Closure

Candidate exact capabilities include:

```text
faction.inspect_controlled_state
faction.scout_region
faction.query_ally
faction.run_counterfactual
faction.allocate_resources
faction.propose_treaty
faction.move_strategic_asset
faction.initiate_operation
faction.request_principal_decision
```

The package selects only semantic contracts and action-class meaning. Activation chooses exact implementation offers. Effective authority intersects candidate requirements, faction-controlled subject and resources, assignment request, principal grant, treaty restrictions, scenario policy, runtime restrictions, and execution effect arbitration.

Access to a simulation command channel does not grant every operation. A counterfactual capability does not grant live-world mutation. Draft diplomacy remains distinct from committing a treaty.

## Observation And Result Admission

Passive world advances carry assignment subscription, activation generation, participant incarnation, authenticated simulation adapter, scenario and branch identity, world sequence, perspective scope, and delivery cursor.

Invoked scouting, counterfactual, diplomacy, allocation, and operations carry exact capability, durable operation key, attempt id, activation generation, participant incarnation, faction, branch, and declared effect target.

Owner admission validates:

- exact scenario, branch, faction, and perspective
- source authority and disclosure eligibility
- world sequence and source revision
- observation reliability and provenance
- capability, operation, attempt, and effect lineage
- result completeness and simulation-owner authentication
- active generation or explicit historical late-result policy

A scout report is evidence about world state. A counterfactual result is evidence about a modeled possibility. An operation success response is evidence of mechanical completion. None is silently converted into verified restoration of the strategic condition.

## Isolation And Lifecycle Pressure

This example pressures normative, state, and resource isolation within shared simulation infrastructure. A single process may host several factions, yet it must not share hidden observations, private beliefs, grants, plans, or action queues.

Shared counterfactual compute needs assignment-safe input projections, bounded budgets, fair scheduling, and no state mutation in the live branch. A dedicated process alone does not prove perspective isolation if its input contains omniscient state.

A control-channel replacement or policy adapter upgrade creates a new activation generation. An equivalent simulation restart creates a new participant incarnation under the same generation after recovery readiness. Late operation results retain the old generation and incarnation and cannot mutate the current branch without explicit owner admission.

Historical interpretation replay reconstructs the faction's decision from the information available at that time. Evaluation against full hidden ground truth is a separate analysis projection and never rewrites the historical perspective.

## Interaction With Other Stewards

Several faction stewards may observe the same public event and receive different private reports. Shared event identity does not imply shared evidence or belief revisions.

Resource, diplomacy, or world stewards may publish narrow contracts used by faction Strategy. No steward gains command authority over another. When two strategic operations contend for one world effect, execution and simulation owners arbitrate using declared effect identity and world rules.

A narrator or roleplay steward may consume public faction outcomes. Private faction intent and plans remain undisclosed unless an explicit information path admits them.

## Router Falsification Findings

The router design is weakened or falsified for this case if:

- installation reads live world state or starts a simulation controller
- root assembly branches on faction, scenario, or action names
- one package body must interpret belief, faction, simulation, Strategy, and execution meaning
- full evaluator truth enters faction evidence through ambient runtime access
- shared simulation machinery merges faction grants, plans, cursors, or capability catalogs
- transport success becomes verified strategic restoration
- counterfactual output is admitted as live world truth
- concurrent effects are resolved by package declaration order

The static fanout remains viable. The strongest unresolved bridge is a simulation-owned perspective projection and admission contract that exposes enough evidence for strategy without leaking omniscient state.

## Theory Set Implications

### Common Candidate Mechanism

- exact package and owner component closure
- assignment-bound perspective, branch, principal, and authority
- standing conditions distinct from episodic goals
- observation actions and intervention actions in one Strategy vocabulary
- exact capability selection with independent authority
- passive world advance and invoked attempt lineage
- later outcome verification and historical replay

### Owner Specific Meaning

- faction, territory, treaty, resource, and operation vocabulary
- strategic action legality
- observation and disclosure rules
- world transition and combat resolution
- counterfactual model semantics
- survival, casualty, territorial, and relationship outcome meaning

These remain faction and simulation domain products. They are not universal PDS or belief variants.

### Unresolved Bridge

- perspective-safe projection of simulation state
- owner admission of private reports and public world events
- effect arbitration across simultaneous faction assignments
- product contract for bounded counterfactual execution
- delayed verification semantics for long-horizon strategic conditions
- isolation evidence for multi-faction shared simulation runtimes

## Source Material

- [Use Case Readme](README.md)
- [Expression Catalog](../pds_expression_catalog.md)
- [Compilation Layer Span](../compilation_layer_span.md)
- [Use Case Decomposition](../../use_case_decomposition.md)
- [PDS Router Detailed Design](../../../completed/integration/pds_router_design_spec.md)
- [PDS Isolation And Runtime Portability](../../isolation_and_runtime_portability.md)
- [Stewardship Package Model](../../package_model.md)
- [Stewardship Facet Protocol](../../facet_protocol.md)
- [Proposal Status](../../proposal_status.md)
