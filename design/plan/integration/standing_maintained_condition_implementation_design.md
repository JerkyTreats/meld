# Standing Maintained Condition Implementation Design

Date: 2026-08-13
Status: implemented
Parent program: [Theory Elevation Program](theory_elevation_program.md)
Domain assessment: [Standing Maintained Condition Assessment By Domain](standing_maintained_condition_domain_assessment.md)
Completion evidence: [Standing Maintained Condition Completion Evidence](standing_maintained_condition_completion_evidence.md)
Scope: Theory Elevation Step 3

## Objective

Make standing responsibility a durable Agent-owned contract whose exact revision causes zero or more transient Goals over time. Preserve the current docs convergence behavior while moving desired condition, priority, and Goal provenance out of the curation rule.

## Minimal Contract

One maintained-condition body carries:

```text
condition identity
belief dimension identity
desired condition
Goal priority
desired-state summary
```

The subject remains physical grounding from the Agent record and never enters the reusable theory body. The desired condition reuses the existing `meld-lang` condition contract. Step 3 supports one selected condition per Agent.

## Owner And Durability

Meld world model Agent owns the condition body and an append-only exact revision registry. Installation follows the existing curation-rule registry pattern. The complete stewardship receipt pins the exact condition revision, and runtime assembly resolves that revision before creating semantic actors.

The Agent record may bind either an exact maintained-condition revision or a compatibility threshold rule. Exact runtime composition injects the receipt-selected condition binding. Compatibility fixtures lower their existing threshold rule into an ephemeral condition and retain characterized parity.

## Curation

Agent curation grounds the installed condition against the Agent subject and evaluates the resulting proposition against the existing planner projection.

```text
condition satisfied or no newer delivery
  -> zero Goal commands

condition breached with no matching open Goal
  -> one proposed transient Goal

matching open Goal exists
  -> no duplicate Goal

condition restored
  -> existing satisfaction mutation

later breach after satisfaction
  -> existing lifecycle reopen mutation
```

The Goal target is the grounded maintained condition. Priority and desired summary come from that condition. The curation rule supplies trigger policy and references the condition identity but does not remain authoritative for desired-state fields on the exact path.

The threshold, priority, desired summary, and source fields still present in the curation-rule body are compatibility inputs only. Exact receipt activation derives desired state and Goal lineage from the installed maintained condition.

## Goal Lineage

Meld lang extends Goal provenance with a maintained-condition breach variant carrying condition identity, dimension, observed summary, and desired summary. Agent dedupe identity includes the maintained-condition identity on the exact path. Execution persists and mutates the Goal without interpreting that identity.

## Exact Installation

The selected package gains `maintained_condition_id`. Theory source provisioning requires `maintained_condition.<id>.json`. Stage 2 installs the Agent-owned revision before committing the complete receipt. Stage 3 binds the exact revision to the Agent record. Runtime resolution rejects any disagreement among condition dimension, curation reference, belief family dimension, and requested Strategy dimensions.

## Compatibility

Threshold-only curation records remain readable and executable. The Agent owner lowers their existing dimension, threshold, priority, desired summary, and source kind into an ephemeral compatibility maintained condition. Exact receipt activation never uses this fallback.

Compatibility may be removed only after all fixtures and stored Agent records bind exact maintained-condition revisions and parity coverage remains green.

## Acceptance Criteria

### AC1 First-class owner contract

Meld world model owns a validated maintained-condition body with stable identity, dimension, desired condition, priority, and desired summary.

### AC2 Durable exact revision

Installing unchanged content reuses the original revision. Changed content creates a new append-only revision. Historical resolution verifies content identity.

### AC3 Complete receipt

Every elevated stewardship receipt includes the exact maintained-condition revision. Runtime resolution fails closed when that revision is absent, corrupt, or inconsistent with the selected package.

### AC4 Owner curation

Agent curation grounds and evaluates the installed condition. Root code never constructs the Goal target or compares threshold values.

### AC5 Causal Goal provenance

Every Goal emitted by the elevated path names the maintained condition that caused it. Goal identity and dedupe remain deterministic for equal condition, subject, branch, and world state.

### AC6 Zero or more transient Goals

A held condition emits zero Goals. A breached condition emits one Goal. Repeated unchanged delivery emits no duplicate. Restoration satisfies the Goal. Later drift reopens its lifecycle without deleting the standing condition.

### AC7 Compatibility parity

Threshold-only curation fixtures retain their current behavior through an explicit compatibility lowering path. The elevated docs path uses only the exact maintained-condition binding.

### AC8 Runtime parity

The canonical `documentation_maintenance` declaration preserves receipt revision lineage, dynamic Strategy closure, publication evidence, satisfaction, later drift, and quiet convergence.

### AC9 Quality gates

All changed Rust passes:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
git diff --check
```

## Explicit Deferrals

- no multiple maintained conditions per Agent
- no temporal stability window
- no hysteresis policy
- no authority model
- no CVE package
- no settled Strategy replay
- no final declaration syntax
- no hot theory replacement
