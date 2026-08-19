# PDS W00 Docs Characterization Protocol

Date: 2026-08-16
Status: accepted for authorized implementation
Mapped workstream: `W00`
Scope: freeze the current docs package, activation, flywheel, lifecycle, and reopen behavior as a machine-readable parity oracle

## Objective

Define the exact characterization fixtures and comparison rules that later routed docs implementation must satisfy.

This packet does not change docs semantics or introduce router behavior. It specifies evidence production only.

## Current Oracle

The current oracle is the fixed docs path formed by:

```text
StewardshipConfig
→ PhysicalBinding
→ WorldInitTheoryBundle
→ TheoryInstallationReceipt
→ ResolvedStewardshipTheory
→ docs exact capability registration
→ current runtime flywheel
```

The oracle is exercised through current owner registries and current runtime assembly. `src/docs/pds.rs` remains characterization-only.

## Fixture Layout

Implementation creates this versioned fixture root:

```text
tests/fixtures/pds/docs_characterization/v1/
├── provider_free/
│   ├── config.toml
│   └── expected_semantic_snapshot.json
├── scripted_provider/
│   ├── config.toml
│   ├── provider_transcript.json
│   └── expected_semantic_snapshot.json
├── reopen/
│   └── expected_semantic_snapshot.json
└── historical_receipt/
    ├── fixed_receipt.json
    └── expected_resolution.json
```

Fixtures are copied to a temporary workspace before execution. No fixture run writes generated state beneath the fixture source tree or target workspace.

## Semantic Snapshot Contract

The machine-readable output is `DocsParitySnapshotV1`.

```rust
struct DocsParitySnapshotV1 {
    schema_version: u32,
    input: DocsInputSnapshot,
    installation: DocsInstallationSnapshot,
    activation: DocsActivationSnapshot,
    flywheel: DocsFlywheelSnapshot,
    lifecycle: DocsLifecycleSnapshot,
    reopen: DocsReopenSnapshot,
}

struct DocsInputSnapshot {
    expression: String,
    subject: DomainObjectRef,
    principal_id: String,
    agent_id: String,
    selected_theory_ids: BTreeMap<String, String>,
}

struct DocsInstallationSnapshot {
    receipt_id: String,
    owner_revision_refs: BTreeMap<String, String>,
    capability_contract_refs: Vec<String>,
    claim_policy_ref: String,
    strategy_theory_ref: String,
}

struct DocsActivationSnapshot {
    selected_capabilities: Vec<ExactCapabilitySnapshot>,
    registered_invokers: Vec<String>,
    provider_required: bool,
    runtime_ids: Vec<String>,
}

struct DocsFlywheelSnapshot {
    canonical_events: Vec<SemanticEventSnapshot>,
    admitted_evidence: Vec<SemanticEvidenceSnapshot>,
    belief_transitions: Vec<BeliefTransitionSnapshot>,
    maintained_condition_transitions: Vec<String>,
    goal_transitions: Vec<String>,
    task_transitions: Vec<String>,
    outcome_transitions: Vec<String>,
    final_artifact_hashes: BTreeMap<String, String>,
}

struct DocsLifecycleSnapshot {
    participant_ids: Vec<String>,
    ordered_states: Vec<String>,
    final_wait_reasons: Vec<String>,
    wake_sources: Vec<String>,
}

struct DocsReopenSnapshot {
    resolved_receipt_id: String,
    resolved_owner_refs: BTreeMap<String, String>,
    belief_state_hash: String,
    goal_state_hash: String,
    task_state_hash: String,
}
```

The implementation may split these types into domain-owned test support modules. The serialized schema and comparison meaning remain as specified.

## Normalization Rules

The snapshot preserves exactly:

- package selection identities
- installation receipt identity
- every exact owner revision ref
- capability type, version, and content identity
- claim-policy and Strategy revision identity
- canonical event type, domain, subject, causation, and payload hash
- evidence class, source event, theory ref, and semantic value
- maintained-condition, Goal, task, and outcome transition meaning
- final generated artifact bytes through content hash
- reopen resolution and durable state hashes

The snapshot normalizes or omits:

- temporary filesystem roots
- process ids and thread ids
- wall-clock timestamps
- log text and log order
- sled allocation details
- nondeterministic map iteration order
- provider request ids not used as canonical lineage

All retained collections use semantic keys and deterministic sort order. A comparison may not discard an exact identity merely to make legacy and routed results match.

## Required Scenarios

| Scenario | Input and execution | Required proof |
| --- | --- | --- |
| `C00-fixed-selection` | canonical fixed declaration | exact lowered selection and physical binding |
| `C01-legacy-lowering` | legacy docs table | same semantic selection as `C00` |
| `C02-installation` | complete current theory bundle | exact owner refs, sorted capability refs, and fixed receipt |
| `C03-provider-free` | exact deterministic capability subset | activation succeeds without provider binding |
| `C04-scripted-flywheel` | scripted provider transcript and README fixture | event through satisfied maintained-condition parity |
| `C05-terminal-failure` | deterministic docs validation failure | terminal classification and no publication |
| `C06-provider-retry` | retryable provider transport fixture | bounded retry without semantic duplication |
| `C07-reopen` | close and reopen product stores | exact receipt, owner refs, and durable state preserved |
| `C08-historical-receipt` | install a later owner revision, then resolve prior fixed receipt | prior exact image remains resolvable |
| `C09-quiescence` | drive current loop until its existing quiet projection | current participant and wait evidence captured without redefining lifecycle meaning |

The scripted flywheel reuses the existing README parity and generation parity fixtures. Live provider runs are supplemental evidence and never replace `C04`.

## Existing Evidence Reused

The implementation builds on these current tests:

- `src/config/stewardship/selection.rs` legacy and canonical lowering tests
- `src/config/stewardship/binding.rs` physical binding and workspace-purity tests
- `src/runtime/theory.rs` historical receipt and missing owner revision tests
- `src/runtime/assembly.rs` reopen, provider binding, genesis, dispatch, and quiet-publication tests
- `src/docs/pds.rs` five-capability Strategy closure tests
- `src/docs/capability.rs` exact activation, identity drift, publication, and failure-classification tests
- `tests/integration/docs_freshness_fixture_contract.rs`
- `tests/integration/docs_freshness_reopen_contract.rs`
- `tests/integration/parity_fixture_contract.rs`

Existing assertions remain. The characterization harness aggregates their semantic products into the new versioned snapshot.

## Compatibility Seam Register

| Seam | Current owner | Removal gate |
| --- | --- | --- |
| `StewardshipConfig.docs_freshness` | config compatibility | legacy config deployment window closes and `C01` remains green through canonical lowering |
| fixed `TheorySelection` fields | config compatibility | routed package config is canonical and `D03` parity passes |
| `SelectedStewardshipPackage` fixed owner ids | config compatibility | assignments cite exact package receipts and historical decoding is proven |
| `WorldInitTheoryBundle` fixed body fields | init compatibility | all current bodies install through owner routes |
| fixed `TheoryInstallationReceipt` | runtime compatibility | new writes use generic receipts and old receipts resolve read-only |
| `ResolvedStewardshipTheory` decoded docs policy | runtime compatibility | owner-scoped generic package resolution is canonical |
| `published_product_contracts` docs-only return | capability compatibility | contributor aggregation passes exact identity parity |
| direct docs `register_exact_contracts` call | runtime compatibility | routed capability activation passes `C03` and `C04` |
| `src/docs/pds.rs` direct composition | test compatibility | routed characterization covers the five-capability chain |

No new caller may target a seam in this table.

## Verification Design

Implementation adds `tests/integration/pds_w00_docs_characterization.rs` with one test per required scenario.

Focused commands:

```sh
cargo test --test pds_w00_docs_characterization
cargo test --test docs_freshness_fixture_contract
cargo test --test docs_freshness_reopen_contract
cargo test --test parity_fixture_contract
cargo test runtime::theory::tests
cargo test runtime::assembly::tests
cargo test docs::capability::tests
```

The harness must support an explicit `UPDATE_PDS_CHARACTERIZATION=1` mode that writes candidate snapshots only under a temporary output directory. Updating checked-in expectations requires an intentional fixture review and may not happen during an ordinary test run.

## Acceptance Criteria

- every required scenario emits a deterministic `DocsParitySnapshotV1`
- two runs over copied identical inputs produce byte-identical snapshots
- live provider evidence is clearly separated from deterministic acceptance
- receipt and owner revision identities survive reopen and historical resolution
- every compatibility seam has a test-visible removal gate
- later routed comparison can identify the exact semantic field that diverged

## Rejection Criteria

Reject implementation if it:

- compares log text instead of semantic products
- removes exact identities from comparison
- updates expected snapshots automatically
- depends on wall-clock ordering
- writes test state beneath the target workspace
- treats one quiet tick as new canonical quiescence meaning
- changes docs behavior while establishing the oracle

## Decision Ledger

- Characterization output is one versioned semantic snapshot, not a prose report.
- Scripted deterministic provider behavior is the parity authority for model-backed paths.
- Live provider evidence is supplemental.
- Current quiet behavior is recorded without promoting it to the `W06` lifecycle contract.
- Compatibility deletion gates are part of the oracle.

## Completion Condition

`D00` is delivered. `W00` may implement this protocol without further cross-domain design decisions.
