# Theory Durability Symmetry Implementation Design

Date: 2026-08-12
Status: ready for implementation
Parent workstream: [Theory Durability Symmetry Workstream](theory_durability_symmetry_workstream.md)
Initial domain assessment: [Theory Durability Symmetry Assessment By Domain](theory_durability_symmetry_domain_assessment.md)
Contract review: [Theory Durability Symmetry Contract Coherence Assessment](theory_durability_symmetry_contract_coherence_assessment.md)
Scope: Theory Elevation Step 1

## Objective

Implement durable, owner-resolved revisions for every semantic body used by the docs freshness runtime. A runtime composition must activate only a complete installed revision set, use that exact set for its lifetime, and leave durable lineage that remains resolvable after later installations activate newer receipts.

This design converts the accepted workstream criteria into contracts, storage boundaries, dependency order, migration posture, and verification gates. It does not define the generic declaration layer.

## Governing Constraints

- Domain owners validate, install, resolve, and interpret their own semantic bodies.
- Root config, init, CLI, and runtime code remain adapters.
- No semantic state is written beneath the target workspace.
- Canonical domain products travel intact through wrappers until an explicit receiver-owned translation.
- Compatibility readers are thin, characterized, and carry an explicit removal condition.
- Formatter evidence precedes lint and test evidence.
- The runtime never manufactures fallback semantics.
- A partially installed set never replaces the last complete active set.

## Design Summary

Each owning domain gains an append-only revision registry. Root initialization installs the supplied bodies through those registries, resolves every exact result, verifies cross-domain references, and commits one root-owned installation receipt containing references only. The receipt is the cross-store commit barrier.

Runtime assembly selects the latest complete receipt for the configured identity set and resolves every body by exact identity and content hash. It freezes the result into one immutable composition snapshot. No actor reads a current registry head after assembly.

```mermaid
flowchart LR
    SOURCE[Authored theory source]
    LOAD[Root load and validate]
    OWNERS[Domain install commands]
    REVISIONS[Append-only domain revisions]
    VERIFY[Cross-domain reference verification]
    RECEIPT[Complete installation receipt]
    RESOLVE[Exact revision resolution]
    SNAPSHOT[Immutable composition snapshot]
    TURN[Curation through belief turn]

    SOURCE --> LOAD
    LOAD --> OWNERS
    OWNERS --> REVISIONS
    REVISIONS --> VERIFY
    VERIFY --> RECEIPT
    RECEIPT --> RESOLVE
    RESOLVE --> SNAPSHOT
    SNAPSHOT --> TURN
```

The receipt owns no semantic bodies. It is operational metadata proving that one exact set passed complete installation and reference validation.

## Core Decisions

### Per-owner registries

Each revision family has a typed contract and append-only exact store in its owning domain. Registry implementations may repeat the small validation, identity, append, and exact-resolution pattern. This deliberate repetition preserves ownership and avoids a generic registry becoming semantic authority.

The existing `TheoryRevisionRef` remains the reference shape for every world-model registry. Execution and docs define their own exact references because those domains do not share world-model authority or crate dependency direction. Root capability defines no revision reference or semantic registry.

### Root installation receipt

Owner stores cannot participate in one transaction. Installation therefore has two layers:

1. Each owner appends or reuses an exact revision without activating it independently.
2. Root commits a complete installation receipt only after every exact revision resolves and all cross-domain references validate.

Runtime assembly reads receipts, never a loose collection of owner heads. If installation stops halfway, the new domain revisions remain harmless and reusable. The last complete receipt remains active.

The receipt head is the sole activation pointer for a selected package. The existing belief-family head remains temporarily for compatibility with pre-receipt callers and is not consulted by the elevated runtime path.

### Composition-lifetime snapshot

The first slice freezes one exact revision set for one runtime composition. Every curation, Strategy, planning, dispatch, outcome interpretation, and belief tick in that composition uses the same set.

A completed installation becomes visible to a newly assembled runtime. Hot replacement inside an already composed process is deferred. This cut prevents mixed-revision work without introducing mutable executor replacement.

### Authored bodies and installed revisions

Authored JSON is a compatibility source. It is validated and installed, then ceases to be runtime authority. Installed revision records contain the canonical validated body plus installation metadata.

Executable capability contracts are the exception in source form. Their owner publishes them from the capability implementation. Initialization installs the published contract values directly into the execution registry. Executable closures, clients, and physical bindings are never serialized.

### Exact lineage without copied truth

Durable records carry revision references, not duplicated theory fields. Runtime snapshots carry complete owner revision products. Adapter projections may translate typed references at crate boundaries, but they do not restate body fields.

### No new application branches

The docs domain supplies authored bodies and executor constructors. World-model and execution registries remain generic over their existing domain contract types. No generic actor matches on `docs_freshness`.

## Revision Units And Contracts

| Revision unit | Contract owner | Stable identity | Exact identity | Installed body |
| --- | --- | --- | --- | --- |
| Belief family | world-model belief | family id | content hash | `BeliefFamilyConfig` |
| Curation rule | world-model Agent | selected rule id | content hash | `AgentCurationRuleConfig` |
| Outcome mapping | world-model belief | mapping set id | content hash | `OutcomeMappingSetConfig` |
| Strategy theory | world-model Strategy | theory id | content hash | `StrategyTheoryPackage` |
| Executable contract | execution capability | type id plus version | content identity | `CapabilityTypeContract` |
| Claim policy | docs | policy id | content identity | `DocsClaimPolicy` |

### World-model reference

The existing shape remains canonical for world-model registries:

```rust
pub struct TheoryRevisionRef {
    pub registry: String,
    pub id: String,
    pub content_hash: String,
}
```

Add validation for nonempty fields and known registry identity at each consumer boundary. Do not turn the registry string into an application vocabulary enum.

### Execution capability reference

```rust
pub struct CapabilityContractRevisionRef {
    pub selector: meld_lang::CapabilityRef,
    pub content_identity: String,
}
```

The exact identity continues to use `CapabilityTypeContract::content_identity`. The existing `CapabilityRef` travels intact as the type and version selector. Installing identical content under a selector is unchanged. Installing different content under the same type and version is rejected as version drift; semantic changes require a capability version increment.

The installed revision product contains the intact `CapabilityTypeContract`, its content identity, and installation sequence. The existing `CapabilityCatalog` remains the activated in-memory projection built from receipt-selected exact revisions.

### Strategy theory package

Add one world-model-owned installed aggregate that carries the existing Strategy products intact:

```rust
pub struct StrategyTheoryPackage {
    pub snapshot: StrategyTheorySnapshot,
    pub capabilities: Vec<StrategyCapability>,
    pub evaluation_policy: StrategyEvaluationPolicy,
    pub search_bounds: StrategySearchBounds,
    pub requested_dimensions: Vec<String>,
}
```

The stable package identity is `snapshot.theory_id`; the package does not repeat it.

Each existing `StrategyCapability` retains the exact execution contract content identity in `contract_id` and the type and version selector in its operator resolution. Installation validates both against the exact executable revisions returned by execution.

The package contains no Goal, subject, planner snapshot, world state, Method, executor, physical scope, or provider binding. Those values are supplied at composition or turn construction and remain outside theory.

This is the canonical installed Strategy body. The owner registry wraps it only with content hash and installation sequence. A world-model activation function combines it with dynamic Goal and planner state to produce the existing `StrategyProblem` and `AgentStrategyRuntimeConfig` shapes. The delivered search API remains unchanged.

### Curation rule revision

Keep `AgentCurationRuleConfig` unchanged. The registry install command receives the selected rule id beside the canonical body. The current `AgentCurationRuleBinding` remains only as a compatibility read shape for old Agent records.

```rust
pub struct AgentCurationRuleRevision {
    pub content_hash: String,
    pub rule: AgentCurationRuleConfig,
    pub installed_at_seq: u64,
}
```

New Agent records bind a `TheoryRevisionRef` rather than embedding the rule body. Actor construction resolves that exact revision from the immutable composition snapshot.

### Docs claim policy reference

```rust
pub struct DocsClaimPolicyRevisionRef {
    pub policy_id: String,
    pub content_identity: String,
}
```

The reference is derived from the existing `DocsClaimPolicy` fields and `content_identity` method. Its installed revision product contains the intact policy, content identity, and installation sequence.

### Installation receipt

```rust
pub struct TheoryInstallationReceipt {
    pub receipt_id: String,
    pub selection: SelectedStewardshipPackage,
    pub belief_family: TheoryRevisionRef,
    pub curation_rule: TheoryRevisionRef,
    pub outcome_mapping: TheoryRevisionRef,
    pub strategy_theory: TheoryRevisionRef,
    pub executable_contracts: Vec<CapabilityContractRevisionRef>,
    pub claim_policy: DocsClaimPolicyRevisionRef,
    pub installed_at_seq: u64,
}
```

`receipt_id` is a content hash over the intact selection and semantic reference fields in deterministic order. `TheoryInstallationReceipt::selection_key` derives a content hash from the configured stable identities in `selection`. The installation store keeps append-only receipts and one current receipt head per derived selection key.

The receipt does not contain physical scope, provider configuration, subject state, Agent state, or theory bodies.

### Resolved composition snapshot

```rust
pub struct ResolvedStewardshipTheory {
    pub receipt: TheoryInstallationReceipt,
    pub belief_family: BeliefFamilyRevision,
    pub curation_rule: AgentCurationRuleRevision,
    pub outcome_mapping: OutcomeMappingRevision,
    pub strategy_theory: StrategyTheoryRevision,
    pub executable_contracts: Vec<CapabilityContractRevision>,
    pub claim_policy: DocsClaimPolicyRevision,
}
```

This root adapter value carries canonical owner products intact. It validates reference equality once, then supplies typed bodies to actor and executor factories.

## Registry Behavior

Every registry follows the same observable contract:

- Validate the stable identity and semantic body before writes.
- Derive the exact identity through canonical JSON serialization and BLAKE3.
- Preserve the first installation sequence for an existing exact revision.
- Append a missing exact revision without activating it independently.
- Flush before reporting installation success.
- Recompute and verify the content identity when reading a revision.
- Never repair corrupt state silently during a read or install.
- Return exact historical revisions independently of receipt activation state.

The existing belief-family registry keeps its current-head behavior for compatibility. Its exact resolution is tightened to this contract, and the elevated runtime stops consulting its current head.

### Store tree pattern

Each new registry owns one append-only tree with a domain-specific name:

```text
<kind>_registry_revisions
```

Revision keys use an unambiguous structured encoding or a length-safe tuple encoding. New code must not introduce delimiter-sensitive composite identity.

The revision value contains the stable identity, exact identity, full canonical body, and first installation sequence. The receipt store separately owns append-only receipts and the active receipt head per selected package.

## Product Storage Layout

All new databases live beneath the external product storage root:

```text
<product-root>/world_model.sled
<product-root>/theory.sled
```

World-model registries share the existing world-model database through distinct owner trees. Docs claim policy, execution contract, and root installation receipt stores share `theory.sled` through distinct owner APIs and trees. Sharing a physical database does not transfer semantic authority.

`ProductStorageLayout` gains one theory database path. `StoreScope` gains one theory flag. A stewardship composition requests that group directly. Runtime role descriptors do not become a theory schema.

`OpenProductStores` exposes typed root-owned stores and passes a cloned database handle to execution-owned and docs-owned constructors. It does not expose semantic trees across owners.

Storage tests must prove every new path remains outside the target workspace and scoped opening creates only requested store groups.

## Authored Theory Layout

The docs compatibility package gains two bodies:

```text
theory/docs_freshness/strategy_theory.docs_freshness.json
theory/docs_freshness/claim_policy.docs-claims-strict-v1.json
```

The existing belief family, curation rule, and outcome interpretation bodies remain. The stale Method, available-action, and realization files remain untouched until Step 2.

`TheorySelection` and `SelectedStewardshipPackage` gain these identities:

```text
strategy_theory_id
claim_policy_id
```

Executable contract identities are derived from the `StrategyCapability` entries in the installed Strategy theory package. They are not copied into config.

Source provisioning performs a full read and validation pass before writing any destination file. Provisioning remains byte-idempotent and does not activate theory.

## Installation Flow

### Load phase

The CLI adapter resolves the physical binding and loads all five authored bodies into one root `WorldInitTheoryBundle`. It also asks the docs capability publisher for the five `CapabilityTypeContract` values without constructing executors.

No owner store is written until every body has passed its owner validation and every selected id matches its loaded body.

### Owner install phase

Install in this dependency order:

1. Belief family, curation rule, and outcome mapping through world-model registries.
2. Executable capability contracts through the execution capability registry.
3. Strategy theory through its world-model registry after root verifies its capability references against the exact execution results.
4. Claim policy through the docs registry.

The Strategy owner validates the package's internal semantic consistency, including prospective routes against its capability set. Root performs only cross-domain reference validation, checking each capability contract content identity and type and version selector against the exact execution results.

### Receipt commit phase

After installation, root resolves every returned reference by exact identity. It then performs cross-domain validation over the resolved products:

- Every Strategy capability selects one exact executable contract revision.
- Every requested projection dimension is available from the belief family.
- The curation rule dimension matches the selected belief family dimension.
- The claim policy id matches the selected policy identity.

Only after all checks pass does root append and activate the installation receipt. Receipt installation is idempotent by its content-derived id.

### Partial failure

If any owner install or cross-domain check fails, no receipt head changes. Re-running the same command reuses already installed exact revisions and continues safely. A later corrected body creates a new revision where required.

Stage reporting lists every owner revision reference and the final receipt id. An applied disposition means at least one revision or the receipt head changed. An unchanged disposition means the complete receipt was already current.

## Runtime Resolution And Assembly

### Receipt selection

Runtime assembly computes the selection key from the configured stable identities and resolves the current complete receipt. These outcomes are distinct:

- No receipt for the selection produces `theory_image_not_installed`.
- A receipt with a missing exact revision produces `theory_revision_missing`.
- A stored body with an invalid hash produces `theory_revision_corrupt`.
- A cross-reference mismatch produces `theory_image_inconsistent`.

All are unresolved required bindings. None causes semantic actor construction.

### Exact body resolution

The resolver calls each owner store with the receipt's exact reference. It never calls `current` for semantic work. The returned products form `ResolvedStewardshipTheory`.

### Actor and executor composition

The immutable snapshot drives composition:

- Belief assessment receives the exact belief-family revision.
- Evidence ingestion receives the exact mapping revision and family revision.
- Goal curation receives the exact curation revision and Strategy theory revision.
- Strategy receives the exact snapshot whose capabilities were validated against the exact execution contracts.
- Planning receives a catalog built from the exact execution contract revisions.
- Docs executor construction receives the exact claim-policy revision.
- Dispatch receives executors whose published contract identities match the exact catalog.

Executor registration fails if any constructed executor publishes a contract identity different from the receipt revision. This guards code drift between installation and runtime composition.

### Compatibility injection

Test and harness callers may inject a complete `ResolvedStewardshipTheory` snapshot. They may not inject loose optional bodies. The current `StewardshipTheoryBindings` compatibility surface becomes a thin complete-snapshot adapter.

Add a `TODO compat-shim:` removal note stating that the adapter remains for harness fixtures and may be removed after every caller builds through installation receipts and parity coverage stays green.

Production CLI assembly stops loading XDG theory bodies and stops calling the docs PDS composer directly.

## Durable Lineage Design

### Agent curation

`AgentRecord` gains an exact curation-rule reference. New registration writes the reference and no embedded rule body.

`AgentCurationDecision` gains `curation_rule_revision` with a compatibility default. New Goal-producing decisions require the field. Satisfaction decisions may omit it when no curation rule participates.

`StrategyAuthorization` gains the exact Strategy theory reference. Its content-derived authorization id includes that reference. The candidate already carries the exact executable capability content identities selected by Strategy.

### Execution admission

The Agent-to-execution adapter translates the world-model authorization into `ExecutionStrategyAuthorization`. The execution copy gains the Strategy theory content identity. It retains its existing `capability_contract_ids` as the exact executable capability content identities.

Execution remains unable to resolve world-model bodies. It uses the Strategy identity for lineage and validates the existing capability identity list against its own exact registry-backed catalog.

### Realization

Planning revalidates authorized content identities against the immutable exact catalog. It never looks up a current capability contract head.

Task lineage already links task nodes to Goal, composition, operator, capability type, and capability version. Correlation to exact contract content follows task to Goal authorization. No duplicate theory fields are added to every task node in this slice.

### Claim policy

Docs validation and publication continue to emit `policy_identity`. The value must equal the installed claim-policy revision content identity. Executor construction records the exact policy revision in its process-local activation object and refuses mismatches.

### Outcome mapping and belief

`OutcomeMappingInput` carries the exact mapping revision reference rather than a stable id alone. Promoted evidence identity includes the mapping content hash so revision A and revision B cannot dedupe into the same record.

`PromotedEvidenceRecord`, `EvidenceItem`, and `EvidenceRejection` gain the exact mapping revision reference. Belief revisions continue to carry the existing family revision and reach mapping revisions through their existing durable evidence ids. `BeliefProvenanceSummary` does not duplicate those references.

These new JSON fields use defaults for old records. New outcome-ingested evidence requires the mapping reference. Graph-derived evidence may omit it because no outcome mapping participated.

### Inspection path

Owner registries expose exact query methods. Integration tests trace lineage through existing durable records and invoke those owner methods directly through assembled store handles. Step 1 adds no generic lineage query or new harness product surface.

## Compatibility And Migration

### Agent records

Existing Agent records embed `AgentCurationRuleBinding`. Preserve a temporary read shape while adding the exact reference field.

During stage three initialization, an old embedded binding is installed into the curation registry under the selected id, the Agent record is rewritten with the exact reference, and the embedded field is cleared. New runtime code resolves only the reference.

The compatibility field carries a local removal note. Removal requires a migration test over old serialized records and parity proving the exact-reference path.

### Existing world-model records

Belief, evidence, rejection, and decision records use JSON storage. New lineage fields use `serde` defaults so old records remain readable. New records written under an active receipt must carry the required references.

### Existing execution goals

Execution Goal records also use JSON storage. New authorization fields use defaults for historical records. The authorized Strategy path rejects a newly submitted authorization that lacks exact Step 1 lineage. Previously stored records remain inspectable but cannot be treated as proof of the new acceptance criteria.

### Existing theory files

The three existing runtime-loaded XDG files remain provisioning inputs. Production runtime reads are removed. Their loaders remain only in world initialization and are renamed or documented as authored-source loaders.

## Implementation Phases

### Phase 1 Owner contracts and registry conformance

Goal: establish exact revision contracts before root wiring.

Tasks:

- Add validation to world-model revision references.
- Tighten belief-family exact resolution corruption behavior.
- Extend curation-rule, outcome-mapping, and Strategy contracts and add their stores.
- Add execution capability-contract revision contracts and store.
- Add docs claim-policy revision contracts and store.
- Add focused owner-store conformance tests for identical install, changed install, historical resolution, corrupt body, capability version drift, and deterministic ordering.

Exit gate: every revision unit independently passes the required revision contract with no root composition changes.

### Phase 2 Storage assembly and complete receipt

Goal: create the shared physical theory store and the cross-owner commit barrier.

Tasks:

- Extend product storage layout and scoped opening with one theory group.
- Add the root installation receipt contract and store.
- Add selection-key and receipt-id derivation tests.
- Add storage purity and scoped-open tests.
- Add exact receipt resolution and corruption errors.

Exit gate: a receipt can cite exact fixtures from every owner, survive reopen, and remain unchanged after a newer receipt becomes active.

### Phase 3 Authored image and staged installation

Goal: install the complete docs theory image through owner commands.

Tasks:

- Add Strategy theory and claim policy JSON bodies.
- Extend config selection and physical binding identities.
- Extend source provisioning with full preflight validation.
- Publish docs capability contracts without executor construction.
- Replace `WorldInitContent` theory fields with a complete loaded bundle.
- Install owner revisions in dependency order.
- Cross-validate exact products and commit the receipt last.
- Add partial-install retry and unchanged rerun tests.

Exit gate: world init produces one complete receipt and never advances its head after any injected owner failure.

### Phase 4 Exact runtime resolution

Goal: remove process injection and compiled semantics from production assembly.

Tasks:

- Add the exact receipt resolver and `ResolvedStewardshipTheory`.
- Build actor factories from exact world-model revisions.
- Build the planning catalog from exact execution revisions.
- Activate the exact Strategy package into the existing Strategy runtime inputs.
- Build docs executors with the exact claim-policy revision.
- Replace loose theory bindings with the complete-snapshot adapter.
- Remove XDG theory reads from CLI runtime assembly.
- Remove semantic constants and construction from `src/docs/pds.rs`.
- Add stable unresolved diagnostic codes and no-mutation tests.

Exit gate: production composition succeeds only from a complete receipt and uses no authored files or compiled docs semantic constants.

### Phase 5 Durable lineage

Goal: make every participating revision recoverable from durable records.

Tasks:

- Migrate Agent rule binding to exact reference.
- Add curation, Strategy authorization, and execution authorization lineage.
- Include exact mapping revision in evidence identity and records.
- Prove belief-to-mapping traversal through existing evidence ids.
- Validate claim-policy identity at executor construction and outcome production.
- Add compatibility characterization and migration tests.

Exit gate: one belief revision can be traced to every participating exact revision without reading a current head.

### Phase 6 Historical update and convergence proof

Goal: prove revision A and revision B isolation while preserving the live flywheel.

Tasks:

- Install revision A and run docs freshness to quiescence.
- Install revision B under the same selected package identities, incrementing any changed capability version.
- Assemble a new runtime and run the second composition.
- Prove both receipts and all exact bodies remain resolvable.
- Prove first-composition records cite only A and second-composition records cite only B.
- Run the existing assembled-runtime bounded convergence proof.
- Record formatter, lint, workspace test, and convergence evidence.

Exit gate: all workstream acceptance criteria pass and no rejection criterion holds.

## Verification Matrix

| Acceptance criterion | Primary evidence |
| --- | --- |
| AC1 | source scan and exact theory inventory test |
| AC2 | complete receipt and partial-install recovery tests |
| AC3 | composition snapshot isolation after newer receipt activation |
| AC4 | missing, corrupt, and inconsistent receipt resolution tests |
| AC5 | persisted Agent decision and Strategy authorization lineage |
| AC6 | exact catalog revalidation and executor contract identity tests |
| AC7 | outcome through evidence through belief provenance test |
| AC8 | revision A and revision B assembled integration test |
| AC9 | docs five-capability convergence proof |
| AC10 | boundary scans and domain contract review |
| AC11 | typed failure matrix and no downstream mutation assertions |
| AC12 | formatter, lint, workspace tests, and assembled convergence |

Each source-changing phase runs gates in this order:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

Focused tests run before the workspace gate. The final convergence proof runs through the existing product initialization and runtime composition path without adding a new harness contract.

## Expected File Map

Likely new owner modules:

```text
crates/meld-world-model/src/agent/curation_registry.rs
crates/meld-world-model/src/belief/outcome_registry.rs
crates/meld-world-model/src/strategy/registry.rs
crates/meld-execution/src/capability/registry.rs
src/docs/claim_validation/registry.rs
src/runtime/theory.rs
```

Likely changed seams:

```text
crates/meld-world-model/src/belief/registry.rs
crates/meld-world-model/src/belief/registry_store.rs
crates/meld-world-model/src/belief/contracts.rs
crates/meld-world-model/src/belief/evidence_ingestion.rs
crates/meld-world-model/src/agent/contracts.rs
crates/meld-world-model/src/agent/runtime.rs
crates/meld-world-model/src/strategy/contracts.rs
crates/meld-execution/src/goals/contracts.rs
crates/meld-execution/src/planning/runtime.rs
src/config/stewardship/selection.rs
src/config/stewardship/binding.rs
src/init/world/source.rs
src/init/world/theory.rs
src/init/world/pipeline.rs
src/init/world/tooling.rs
src/runtime/storage.rs
src/runtime/assembly.rs
src/cli/runtime_assembly.rs
src/docs/pds.rs
```

This map is advisory. Domain contracts and phase exits control implementation scope, not the file list.

## Explicit Deferrals

- No generic stewardship declaration or compiler.
- No root expression-dispatch removal beyond deleting Step 1 semantic constants.
- No maintained-condition contract.
- No effective authority model.
- No CVE expression.
- No settled Strategy replay or promotion.
- No Method or workflow revival.
- No hot theory replacement inside an assembled runtime.
- No generalized cross-domain registry framework.
- No new harness product surface.

## Completion Condition

Implementation is complete when Phase 6 records passing evidence for every acceptance criterion, the worktree contains no compiled docs semantic body from the frozen inventory, and a historical record can resolve its exact semantic dependencies without consulting any current registry head.
