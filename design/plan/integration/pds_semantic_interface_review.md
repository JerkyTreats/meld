# PDS Semantic Interface Review

Date: 2026-08-18
Status: accepted architecture decision and implementation finding
Scope: the public semantic interface between PDS domain theory and the Meld cognitive flywheel, grounded in documentation freshness and tested against the bounded dependency-security truth slice

## Purpose

The authorized PDS implementation proves package routing, exact owner installation, activation, lifecycle, and a second-domain attachment. Live documentation-freshness verification also exposed a deeper question: what meaning may PDS supply without precomputing the cognition Meld is meant to perform.

This review records the implementation evidence behind the canonical boundary now fixed in [Persistent Domain Stewardship](../../cognitive_architecture/persistent_domain_stewardship.md). The architecture decision is accepted. Runtime redesign, dependency-security expansion, Strategy replay, remediation, and implementation beyond `W06` remain separately authorized work.

## Canonical Conclusion

Meld presents PDS with domain-owned semantic extension points over the cognitive flywheel.

PDS may supply stable domain meanings and norms. It must not supply situated cognitive conclusions, commitments, plans, or executions.

```text
PDS supplies

domain vocabulary
event and fact relevance
evidence admissibility and proof meaning
domain propositions
maintained conditions
settlement criteria
governance meaning

Meld produces

facts for a current subject and time
admitted evidence
belief revisions
Goals
Strategy candidates
authorizations
tasks
outcome admission and reassessment
```

Capability providers independently publish the exact affordances available to an activation. Domain theory does not author the active toolbox or the action chain.

## The Flywheel As Interface

The flywheel gives each owner a concrete artifact boundary:

```text
Event
  ↓
Fact
  ↓
Evidence
  ↓
Belief
  ↓
Goal
  ↓
Strategy and tasks
  ↓
Capability realization
  ↓
Event
```

The public theory seam is not a horizontal cut that forbids every belief-level concept. Evidence is evidence only in relation to a proposition, and a maintained condition must identify what should remain true. PDS therefore must be able to declare belief questions, proof obligations, and desired propositions.

The cut is between type-level meaning and runtime-produced state:

| PDS may declare | PDS may not declare |
| --- | --- |
| proposition vocabulary | current belief revision |
| relevant event and fact kinds | current truth for a subject |
| evidence schemas and admissibility | Goal instance |
| support, contradiction, coverage, and freshness meaning | selected Strategy candidate |
| maintained propositions | action sequence or task graph |
| settlement proof obligations | exact capability choice |
| domain policy | successful settlement result |

This preserves both directions of cognition. Bottom-up evidence constrains what can be believed. Top-down maintained intent determines which questions matter. Meld closes the gap between them.

## Interface Enforcement

The boundary cannot depend on author discipline alone. It needs structural enforcement.

### State-free theory

Installed theory may contain stable vocabulary, rules, policy, and proof requirements. It may not contain current observations, active findings, current confidence, live Goals, selected candidates, task state, or outcomes.

State-free is necessary but insufficient. A state-free declaration can still encode an exact workflow, fixed toolbox, or precomputed inference procedure.

### Owner-exclusive runtime construction

Each flywheel owner exclusively creates its own live artifacts.

| Artifact | Runtime owner |
| --- | --- |
| canonical event | events |
| graph fact and evidence admission | world model |
| belief revision | belief |
| Goal | Agent |
| Strategy candidate | Strategy |
| task and operation | execution |
| canonical domain outcome | result owner plus events admission |

A theory route installs and resolves owner meaning. It receives no command path that can construct live artifacts in a downstream owner store.

### Proof-carrying evidence

Evidence that bears directly on a maintained proposition must preserve enough lower-level ground for owner admission and later explanation. A naked scalar named after the target conclusion is not sufficient by itself.

Relevant ground includes exact subject, source facts, source revisions, policy revision, coverage, contradictions, currency, calculation identity, and provenance as applicable to the domain.

An external or domain-specific assessment may be admitted as evidence. It does not become belief merely because its producer labels the subject clean, fresh, safe, or correct.

### Evidence non-substitution

An operational success cannot substitute for semantic proof.

Examples:

- task completion does not prove README correctness
- successful scanner execution does not prove advisory coverage
- no findings does not prove bounded clean posture
- a commit does not prove vulnerability resolution
- publication does not prove that the published bytes match current source truth

### Affordance independence

The active capability catalog is supplied by physical activation, not by domain correctness theory. Authority remains a separate candidate and dispatch constraint rather than changing what the catalog says is available.

The same installed domain theory must remain meaningful when a provider changes, a new capability appears, an implementation becomes unavailable, or authority context changes. Strategy receives the current exact catalog and constructs candidates against the current world state while preserving authority as a separate input.

### Counterfactual conformance

The boundary is healthy when these variations remain possible:

```text
same theory plus different evidence
    may produce different beliefs

same belief plus different maintained intent
    may produce different Goals

same Goal plus different capability catalog
    may produce different Strategies

same Strategy plus different authority
    may produce different execution admission
```

If an earlier declaration already fixes the later artifact, cognition has crossed the interface in the wrong direction.

## Documentation Freshness Grounding

The intended domain meaning is that README files are correct. Correctness has domain-owned semantics over claims, source support, contradiction, coverage, and current lineage.

The intended runtime path is:

```text
source and README observations
  ↓
source facts and README claim facts
  ↓
support, contradiction, coverage, and lineage evidence
  ↓
Meld forms a README correctness belief
  ↓
Agent compares the belief with the maintained condition
  ↓
Strategy constructs a route using the active toolbox
```

The current implementation does not fully express this separation.

- `docs.assess_published_scope` produces a final `stale_probability` after performing the substantive correctness assessment.
- The belief family largely weights that supplied conclusion rather than forming correctness from claim-level evidence.
- `StrategyTheoryPackage` includes exact capabilities, evaluation policy, search bounds, requested dimensions, and requested authority beside settlement meaning.
- The installed docs Strategy body exposes the exact five-capability chain and static costs.
- Draft and validation capabilities contain hidden decomposition, ordering, batching, retry, revision, and pruning choices.

The routed migration may preserve these forms for characterization and parity. Preservation does not establish them as the final public semantic interface.

## Dependency-Security Falsification

Dependency security disproves the strongest horizontal cut of no public belief semantics.

The domain must define distinctions that a generic runtime cannot safely invent:

- adequate current inventory and advisory coverage
- bounded clean posture
- unknown, insufficient, stale, violated, and conflicted posture
- applicability under exact package identity and version range
- negative proof under explicit coverage and currency
- evidence classes that cannot substitute for one another

Those are belief questions and proof semantics. They must be public domain meaning even though every actual belief revision remains runtime-owned.

The bounded dependency-security design supports the refined boundary. Its policy is state-free, canonical products retain lower evidence, evidence classes remain distinct, and the domain does not own belief revisions, Agent judgment, Goals, or task state.

The current fixture also repeats the documentation mismatch:

- the coverage family consumes `coverage_probability`
- the posture family consumes `clean_within_coverage_probability`
- the Strategy body embeds four exact capability contracts and their costs

The second domain therefore strengthens the refined claim rather than validating the current package shape.

## Workstream Disposition

| Workstream | Disposition from this review |
| --- | --- |
| `W00` | characterize and retain the current semantic products without treating them as endorsed interface design |
| `W01` | keep the router structural and opaque to owner meaning |
| `W02` | preserve separate theory-route and capability-contribution paths; do not present current Strategy package composition as a settled universal semantic contract |
| `W03` | preserve docs parity as migration evidence; record current conclusion and capability coupling as known debt |
| `W04` | source the exact active catalog from assignment-local physical activation rather than package meaning |
| `W05` | use security as a falsification case for evidence sufficiency and non-substitution; do not generalize its current scalar belief inputs |
| `W06` | preserve owner admission and durable operation boundaries so transport or supervisor state cannot become semantic truth |

The runtime-supervisor failures from the live docs specimen remain directly actionable against existing `W06` lifecycle contracts. The semantic-interface and Strategy corrections now have canonical architecture disposition. Their runtime implementation still requires explicit authorization where it exceeds parity or current task language.

## Resolved Boundary And Remaining Detail

The ownership boundary is settled. PDS supplies stable proof and settlement meaning. Belief performs current inference. Agent supplies Strategy construction policy and judgment. Activation supplies exact capabilities. Strategy supplies construction and separately admitted Methods. Search controls remain request-scoped.

Some schema detail remains owner-local. All admitted README claims must be supported is documentation semantics. A belief comparator may be referenced as an exact belief-owner policy when it expresses stable proof meaning. A target-shaped current conclusion may not be supplied as theory. Strategy comparison policy and search bounds are not PDS theory.

The current body classification and ownership decision are recorded in [PDS Cognition Boundary Assessment](pds_cognition_boundary_domain_assessment.md). Compatibility migration and exact replacement schemas remain implementation design work and must not begin without authorization.

## Related Documents

- [PDS Authorized Implementation Findings](pds_authorized_implementation_findings.md)
- [PDS Cognition Boundary Assessment](pds_cognition_boundary_domain_assessment.md)
- [Canonical Persistent Domain Stewardship](../../cognitive_architecture/persistent_domain_stewardship.md)
- [PDS Authorized Implementation Workstreams](pds_implementation_workstreams.md)
- [PDS Theory Runtime Layer Map](pds_theory_runtime_layer.md)
- [PDS W02 Owner Routes And Contributors Design](pds_w02_owner_routes_and_contributors_design.md)
- [PDS W03 Docs Routed Migration Design](pds_w03_docs_routed_migration_design.md)
- [PDS W05 Dependency-Security Truth-Slice Design](pds_w05_dependency_security_truth_slice_design.md)
- [Runtime Supervisor Goal, Invariants, And Domain Assessment](runtime_supervisor_invariant_domain_assessment.md)
