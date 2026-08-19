# Roleplay Character Continuity Router Spec

Date: 2026-08-18
Status: discovery router exercise aligned to canonical PDS boundary
Scope: candidate lowering of the roleplay continuity example through `theory::router`

## Purpose

This exercise asks whether roleplay continuity can install and activate through the common PDS router without teaching theory, root assembly, belief, Agent, planning, execution, or events what a roleplay character is.

The route map is illustrative. Route owners retain component meaning and may revise the component split during design promotion.

## Semantic Package And Runtime Input Map

| Component id | Candidate route | Owner | Meaning and required links |
| --- | --- | --- | --- |
| `continuity-canon-support` | `world-model.belief-family.v1` | belief | Canon support with authority-ranked evidence |
| `continuity-character-knowledge` | `world-model.belief-family.v1` | belief | Perspective-bound knowledge with admissible information paths |
| `continuity-relationship-state` | `world-model.belief-family.v1` | belief | Relationship dimensions and provenance |
| `continuity-commitment-state` | `world-model.belief-family.v1` | belief | Promise, obligation, release, and supersession state |
| `continuity-thread-state` | `world-model.belief-family.v1` | belief | Open, resolved, abandoned, and disputed thread state |
| `continuity-identity-consistency` | `world-model.belief-family.v1` | belief | Compatibility of a proposed response with grounded continuity |
| `continuity-evidence-routes` | candidate `world-model.evidence-mapping.v1` | belief | Maps admitted narrative products into exact belief families |
| `continuity-maintained-conditions` | `world-model.agent-maintained-condition.v1` | Agent | Standing continuity, knowledge, commitment, and disclosure conditions |
| `continuity-curation` | `world-model.agent-curation-rule.v1` | Agent | Tolerate, observe, draft, escalate, or restore decisions |
| `continuity-strategy` | `world-model.strategy-theory.v1` | Strategy | Settlement, prospective evidence, continuity action, outcome, and constraint meaning |
| `continuity-capabilities` | activation capability contribution | capability and execution | Exact contracts for hydrate, respond, inspect provenance, ask, and propose transition |
| `continuity-authority` | assignment governance selection | Agent and execution | Separates response, state mutation, retcon, and disclosure grants |
| `continuity-outcomes` | `world-model.outcome-mapping.v1` | outcome and belief | Maps accepted turns, corrections, and continuity checks into verification evidence |
| `continuity-policy` | `narrative.policy.v1` | narrative domain | Canon ordering, disclosure classes, retcon policy, and source semantics |

The manifest structurally requires evidence routes to cite every selected belief family, curation to cite maintained conditions, Strategy semantic theory to cite continuity action and outcome meaning, and outcome mappings to cite exact admitted narrative products. Separately admitted Methods cite exact activation capabilities. The narrative owner validates canon and disclosure meaning. Theory proves only exact semantic route and reference closure.

The package does not contain current scenes, relationships, character knowledge, secrets, or conversation turns. Those remain runtime state owned by narrative, graph, belief, and event domains.

## Owner Semantics And Cross Links

The narrative route owns the distinction among principal-authored canon, narrator-authoritative events, accepted generated material, character testimony, deception, speculation, and retcon. Belief owns how admitted evidence revises an exact belief family. Agent owns maintained-condition evaluation, construction policy, and normative curation. Strategy owns Method admission, composition, and candidates. Execution owns capability and effect contracts.

Cross-domain links use exact public refs:

```text
narrative evidence class
→ exact evidence mapping
→ exact belief family
→ maintained-condition proposition
→ curation rule
→ Strategy method
→ exact capability contract
→ authority policy
→ admitted narrative result
→ outcome mapping
```

The linker must reject a response method that lacks a disclosure policy, a state-transition capability that lacks a matching authority class, or an outcome mapping that cannot identify the exact accepted narrative product.

## Assignment And Activation

One assignment binds:

```text
exact package receipt
+ principal
+ active character subject
+ character perspective
+ narrative branch
+ requested authority and principal grant
```

Two characters in one world are two assignments even when they share a package and source corpus. Two principals controlling one character also remain distinct assignments. Declaration order grants neither priority.

Activation adds only physical bindings, such as a narrative query endpoint, bounded context source, model provider, credential refs, or a local deterministic resolver. A provider-free activation remains valid when only deterministic inspection capabilities are selected.

Prepared closure builds an assignment-local capability catalog with admission closed. The generation becomes live only after narrative query access, selected response adapters, callback correlation, and every required runtime object return generation-scoped readiness evidence and current publication succeeds against the expected prior generation.

## Capability And Authority Closure

The package may select contracts such as:

```text
narrative.hydrate_continuity_context
narrative.inspect_claim_provenance
narrative.generate_character_response
narrative.propose_state_transition
narrative.request_principal_clarification
narrative.verify_response_continuity
```

Selection does not grant use. Effective authority intersects the exact contract, assignment request, principal grant, runtime policy, current disclosure restrictions, and execution effect policy.

The response capability may return a candidate artifact without any right to update canon. A transition proposal may be admitted as a draft while execution remains approval-gated. Retcon and protected-secret disclosure remain separate action classes even if one adapter can technically perform both.

## Observation And Result Admission

Passive narrative source advances carry an assignment subscription, activation generation, authenticated delivery identity, source revision, branch, and cursor. They do not fabricate execution claims.

Invoked hydration and response work carries the durable operation key, distinct attempt id, activation generation, participant incarnation, exact capability ref, assignment, perspective, and branch. A retry preserves the semantic operation key while receiving a new attempt.

Narrative owner admission checks:

- exact assignment, character, perspective, and branch
- active or explicitly tolerated historical activation generation and participant incarnation
- source and canon revision
- speaker and authority class
- disclosure eligibility for every protected claim used or emitted
- completeness of provenance and continuity evaluation
- operation and attempt lineage or passive delivery lineage

Transport success and model completion are operational evidence only. They never establish canon acceptance, character knowledge, or successful continuity.

## Isolation And Lifecycle Pressure

This example makes semantic and normative isolation stricter than process isolation. A shared model service may be acceptable while any cross-assignment context, secret, perspective, or grant leakage is not.

The activation requires owner-scoped bindings, assignment-local context projections, explicit source revisions, and generation-fenced admission. A sidecar cache must key entries by every semantic input that affects disclosure and grounding. A cache hit that omits perspective, branch, canon revision, or package receipt is unsafe even if the process is dedicated.

A late response from a retired generation remains historical candidate output. It cannot become the current character utterance merely because it is coherent. Replacement closes old admission before new turns are accepted.

Historical interpretation replay must explain why a response was grounded as it was without requiring the original model endpoint. Live regeneration is new work and may produce different prose.

## Interaction With Other Stewards

The continuity steward may consume exact current canon and entity projections from a lore steward. It must not consume the lore steward's private ambiguity state as character knowledge unless an admitted information path exists.

Several character stewards may share world events while maintaining divergent knowledge and disclosure boundaries. A narrator steward may possess broader observation authority but does not silently transfer that authority to a character assignment.

If two stewards propose changes to the same canon object, narrative effect arbitration and principal policy own the conflict. The router does not choose a winner.

## Router Falsification Findings

The router design is weakened or falsified for this case if:

- package installation needs current character state or a live model call
- root assembly branches on character or campaign identity
- one central body must interpret narrative, belief, Agent, Strategy, and execution semantics
- activation shares perspective, disclosure grants, context, or current state across assignments
- response transport success is treated as canon acceptance
- world truth automatically becomes character knowledge
- historical replay requires the original model service to remain online
- a retcon rewrites old evidence instead of selecting a new current interpretation

The current design survives the static fanout on paper. The main unresolved bridge is the owner contract that turns accepted narrative products into perspective-safe evidence and disclosure decisions.

## Theory Set Implications

### Common Candidate Mechanism

- exact owner-routed component installation
- perspective and branch in assignment identity
- maintained conditions distinct from transient goals
- bounded observation and context hydration capabilities
- exact capability and authority closure
- generation-fenced external result admission
- non-destructive supersession and replay lineage

### Owner Specific Meaning

- canon authority ordering
- character knowledge and information paths
- narrative disclosure classes
- relationship and commitment dimensions
- accepted-turn and retcon semantics
- continuity evaluation of expressive artifacts

These meanings belong to narrative and their consuming domain owners. They are not candidates for a universal PDS ontology.

### Unresolved Bridge

- a narrative owner route for canon, disclosure, and accepted-turn semantics
- exact projection contracts from lore state into character perspective
- effect arbitration for concurrent proposed narrative turns
- result policy for model output that is valid but stale against a newer scene
- outcome evidence strong enough to distinguish plausible prose from maintained continuity

## Source Material

- [Use Case Readme](README.md)
- [Original Roleplay Character Example](../roleplay_character.md)
- [Expression Catalog](../pds_expression_catalog.md)
- [Compilation Layer Span](../compilation_layer_span.md)
- [PDS Router Detailed Design](../../../completed/integration/pds_router_design_spec.md)
- [PDS Isolation And Runtime Portability](../../isolation_and_runtime_portability.md)
- [Stewardship Package Model](../../package_model.md)
- [Stewardship Facet Protocol](../../facet_protocol.md)
- [Proposal Status](../../proposal_status.md)
