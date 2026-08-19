# Theory Durability Symmetry Workstream

Date: 2026-08-12
Status: implemented
Parent program: [Theory Elevation Program](theory_elevation_program.md)
Initial assessment: [Theory Durability Symmetry Assessment By Domain](theory_durability_symmetry_domain_assessment.md)
Contract assessment: [Theory Durability Symmetry Contract Coherence Assessment](theory_durability_symmetry_contract_coherence_assessment.md)
Implementation design: [Theory Durability Symmetry Implementation Design](theory_durability_symmetry_implementation_design.md)
Completion evidence: [Theory Durability Symmetry Completion Evidence](theory_durability_symmetry_completion_evidence.md)
Scope: Theory Elevation Step 1 only

## Objective

Replace every compiled or process-injected semantic body used by docs freshness with a durable, content-identified revision owned and resolved by the responsible domain. A completed runtime turn must remain semantically inspectable after any installed theory head advances.

The expected outcome is a docs freshness runtime whose semantic inputs are installed data, whose turns use one immutable revision set, and whose durable records carry enough lineage to resolve every body that influenced curation, Strategy construction, realization, outcome interpretation, and belief reconciliation.

## Initiation Gate

The workstream may begin because all gate statements now hold:

- The affected-domain assessment is complete and its affected set is frozen.
- The complete first-slice theory inventory is named below.
- Every theory unit has one authoritative owner.
- Adapter and domain responsibilities are separated.
- Step 2 declaration lowering and later elevation steps are explicit non-goals.
- Observable acceptance criteria and verification gates are defined.
- No unresolved ownership decision blocks the first implementation task.

Implementation begins at the owner contracts and registry behavior. Root assembly changes do not begin until the owning-domain install and resolution contracts exist.

## Frozen Theory Inventory

| Revision unit | Semantic contents | Authority | Current compatibility source |
| --- | --- | --- | --- |
| Belief-family revision | evidence schemas, comparator, priors, projection, observationality, anchor requirement | `meld-world-model` belief | `theory/docs_freshness/belief_family.docs_freshness.json` |
| Curation-rule revision | divergence threshold, priority, desired condition, source kind | `meld-world-model` Agent | `theory/docs_freshness/curation_rule.docs_freshness.json` and Agent record binding |
| Outcome-mapping revision | event matching, subject extraction, evidence field extraction | `meld-world-model` belief | `theory/docs_freshness/outcome_interpretation.docs_freshness.json` |
| Strategy-theory revision | settlement rules, prospective evidence routes, evaluation policy, search bounds, requested projection dimensions, Strategy capabilities, artifact dependencies, effects, costs, and outcome bindings | `meld-world-model` Strategy | `src/docs/pds.rs` |
| Executable-contract revision | full `CapabilityTypeContract` body and content identity | `meld-execution` capability | docs capability publication during process composition |
| Claim-policy revision | validation thresholds, weights, revision limit, stable policy identity | `docs` | `src/docs/pds.rs` |

Dynamic Goals, world state, planner snapshots, candidates, Agent judgments, executor instances, observations, evidence, beliefs, and outcomes are runtime state. They are not installed theory.

The stale planning Method, available-action, and realization files are not part of the live Strategy path. Their retirement remains in Step 2 and they must not be revived to satisfy this workstream.

## Required Revision Contract

Every revision unit must expose an owner-specific typed contract with these behaviors:

- A stable semantic identity selects a revision family.
- A canonical content hash identifies exact semantic content.
- Installation validates the body before appending an exact revision.
- Installing identical content is an idempotent no-op.
- Installing changed content appends a revision without deleting or rewriting an older revision.
- Exact resolution by semantic identity and content hash returns the historical body.
- A missing or corrupt exact body returns a typed failure.
- The complete installation receipt is the sole activation head for a selected package.

The existing belief-family current head remains a compatibility surface while its callers move to receipt-selected exact resolution. New owner registries do not introduce independent activation heads.

Executable capability contracts retain their existing type and version uniqueness. Different content under the same type and version is version drift and fails installation rather than creating a hidden revision. A semantic capability change increments the capability version.

Each owner may choose its physical store. Shared hashing or persistence mechanics may be reused, but no generic registry may become authoritative for foreign semantics.

## Acceptance Criteria

### AC1 Complete semantic elevation

Every row in the frozen theory inventory is either proven already durable with exact lineage or migrated to an owner-specific registry. `src/docs/pds.rs` contains no claim-policy values, settlement bodies, evidence-route bodies, evaluation-policy values, search bounds, projection-dimension lists, Strategy capability graphs, costs, or outcome bindings as compiled semantic constants.

Stable protocol identifiers and compatibility loader names may remain in code when they identify a contract rather than define its semantic body.

### AC2 Installation through domain authority

World initialization validates the complete selected image before activation, invokes typed owner install commands, and reports every installed revision reference. A partial install is safe to retry and cannot become an active runtime image until all selected identities resolve successfully.

Authored source provisioning remains XDG configuration placement only. Durable installation remains in the owning domain stores.

### AC3 Immutable turn resolution

Before a runtime composition performs semantic work, the runtime resolution boundary freezes one complete revision set. Every turn in that composition uses those exact bodies through completion even when another process activates a newer receipt.

A later runtime assembly may use the newer receipt. Hot theory replacement inside an assembled process is not part of Step 1.

### AC4 No semantic fallback

Steady runtime does not read authored XDG theory bodies as semantic authority and does not fall back to compiled docs constants. Any missing, invalid, corrupt, or mutually inconsistent selected revision leaves the dependent runtime binding truthfully unresolved with a stable diagnostic.

No task-network mutation, capability invocation, or evidence admission occurs from an unresolved image.

### AC5 Curation and Strategy lineage

Every durable curation decision cites the exact curation-rule revision used. Every Strategy authorization cites the exact complete Strategy theory revision used to construct and verify its candidate.

The cited Strategy revision retains the exact executable-contract revision identities and outcome contract bindings considered by Strategy through its existing `StrategyCapability` products.

### AC6 Realization lineage

Execution revalidates authorized capability contract identities against exact resolvable contract bodies. It never substitutes a newer contract merely because that revision is current.

Each concrete executor activation can be correlated with the exact claim-policy revision and other semantic dependency revisions used by that executor. Executable closures and provider clients remain process-local and are never serialized as theory.

### AC7 Outcome and belief lineage

Substantive docs outcomes retain the exact claim-policy identity already produced by validation and publication. Evidence ingestion records the exact outcome-mapping revision that interpreted the outcome. The resulting belief revision preserves its existing belief-family revision and reaches mapping lineage through its durable evidence ids.

From one settled or divergent belief revision, an inspector can resolve the exact mapping, family, claim policy, Strategy theory, and executable contracts that participated in the causal path when those units were relevant.

### AC8 Historical resolution after update

An integration test installs revision A for every changed theory kind, runs one docs freshness composition, installs revision B under the same selected package identities, assembles a new runtime, and runs another composition. An executable capability body changed for B uses an incremented capability version.

The test proves:

- current receipt resolution returns the complete revision B set
- every revision A reference still resolves to byte-equivalent semantic content
- records from the first composition cite only revision A
- records from the second composition cite only revision B
- no first-composition record is silently reinterpreted through revision B

This criterion proves recoverability only. Replaying settled execution without candidate regeneration remains Step 6.

### AC9 Compatibility parity

The elevated docs image preserves the landed runtime behavior:

- belief divergence produces the same class of proposed Goal
- bounded Strategy search constructs the five-capability chain with no Method or workflow route
- Agent authorization precedes Goal admission
- Execution realizes the exact authorized composition
- claim-validated publication produces substantive assessment evidence
- belief reconciliation reaches the same quiescent condition

Identifiers may change only where revision lineage requires a new content-derived identity. Semantic outputs and authority boundaries must remain equivalent.

### AC10 Domain boundary preservation

The implementation adds no application-named branch to `meld-world-model`, `meld-execution`, `meld-lang`, or generic runtime actors. Root config, CLI, init, and runtime code only select, route, install, resolve, freeze, and present domain-owned contracts.

No central PDS store owns belief, Strategy, capability, execution, or docs semantics.

### AC11 Failure and recovery behavior

Tests cover at least these failures:

- selected package has no active receipt
- active receipt head cites a missing receipt
- active receipt cites a missing exact owner revision
- stored content does not match its content hash
- Strategy theory cites an unknown executable contract revision
- Strategy theory cites an unknown outcome binding
- claim-policy revision is absent when a docs executor binds
- outcome mapping revision is absent during evidence ingestion
- installation stops after some owners succeeded and then converges safely on retry

Every failure is typed or carries a stable diagnostic code. None manufactures fallback semantics or causes downstream durable mutation.

### AC12 Regression and quality gates

All changed Rust passes:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

Focused tests must include exact owner-store conformance, receipt isolation, lineage integrity, one partial-install recovery case, and assembled docs convergence.

## Rejection Criteria

The workstream is not complete if any statement below is true:

- A new expression would still need to compile one of the Step 1 semantic body kinds into Rust.
- Runtime success depends on an authored XDG body that was never durably installed.
- A durable record cites an identity whose exact historical body cannot be resolved.
- Runtime consumers silently read an owner activation head when a receipt or record cites an older exact revision.
- One central registry interprets several domains' theory kinds.
- A root adapter validates foreign semantics beyond calling the owning contract.
- Step 2 declaration grammar, Step 3 maintained conditions, Step 4 authority, Step 5 CVE behavior, or Step 6 settled replay enters the implementation scope.
- The docs flywheel no longer converges through the dynamic five-capability path.

## Completion Evidence

Closeout must record:

- the final theory inventory with concrete registry types
- revision A and revision B identities from the historical-resolution test
- the durable lineage path from Agent decision through reconciled belief
- unresolved-binding evidence for at least one absent revision
- formatter, lint, workspace test, and assembled convergence results
- any compatibility surface retained for Step 2

The parent [Theory Elevation Program](theory_elevation_program.md) may mark Step 1 complete only when every acceptance criterion and quality gate above has recorded evidence.
