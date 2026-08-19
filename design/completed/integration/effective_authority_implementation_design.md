# Effective Authority Implementation Design

Date: 2026-08-13
Status: implemented
Parent program: [Theory Elevation Program](theory_elevation_program.md)
Domain assessment: [Effective Authority Assessment By Domain](effective_authority_domain_assessment.md)
Scope: Theory Elevation Step 4

Completion evidence: [Effective Authority Completion Evidence](effective_authority_completion_evidence.md)

## Objective

Prevent a stewardship Agent from invoking a capability merely because the capability is installed. Extend the exact Strategy authorization path with a compact effective-authority decision and enforce it independently at Goal admission, planning, and dispatch.

## Reused Machinery

- Strategy theory remains the package-owned semantic body and gains an explicit requested action set.
- `StrategyAuthorization` remains the single Agent authorization record.
- Composition operators remain the source of required capability type identities.
- Capability contracts remain the source of executable availability and exact content identity.
- Guarded Goal admission remains the persistence boundary.
- `ExecutionComposition` and `TaskLineage` remain the planning-to-dispatch lineage path.
- Complete receipts remain the exact activation boundary.

No parallel role engine, policy service, authorization ledger, workflow, or capability wrapper is introduced.

## Minimal Policy Contract

One selected policy body carries:

```text
policy identity
principal identity
exact subject scope
principal granted action ids
runtime allowed action ids
restricted action ids
```

For this product maturity, action ids are capability type ids. They are stable enough to prove separation from availability but remain open to refinement after the CVE expression.

## Effective Authority

For every capability type required by the verified candidate:

```text
required by candidate
and requested by Strategy package
and granted to principal
and allowed by runtime policy
and not restricted
and exact subject scope matches
```

The resulting decision carries policy identity and content hash, principal, subject scope, requested actions, and authorized actions. It is embedded in the existing Strategy authorization and therefore covered by authorization identity.

## Enforcement

Agent curation evaluates authority after candidate verification and before attaching Strategy authorization. Denial becomes an honest Agent abstention with no Goal command.

Execution admission validates structural agreement between the candidate composition and authority decision. Planning resolves the active exact policy and re-evaluates the decision before lowering. Task lineage carries the compact decision. Dispatch validates the exact active policy and the task capability before invocation.

Compatibility records without authority decisions remain valid only when no exact authority policy is activated. Exact receipt activation always requires authority lineage.

## Exact Installation

The selected declaration gains `principal_id` and `authority_policy_id`. Theory source provisioning requires `authority_policy.<id>.json`. Stage 2 installs the execution-owned policy revision before committing the complete receipt. Runtime resolution rejects identity, principal, or scope disagreement. Assembly passes one immutable binding to Agent curation, planning, and dispatch.

## Acceptance Criteria

### AC1 Capability is not authority

An installed and resolvable capability cannot be authorized unless its action id passes every policy intersection input.

### AC2 Exact durable policy

Unchanged policy content reuses its original revision. Changed content creates a new append-only revision. Historical receipts resolve exact policy content.

### AC3 Complete receipt

Every elevated stewardship receipt cites the exact authority policy revision. Missing or inconsistent policy revisions fail closed during resolution.

### AC4 Single Agent authorization lineage

Effective authority is embedded in existing Strategy authorization and its deterministic identity. No second Agent decision or authorization store is created.

### AC5 Admission validation

Execution rejects authority lineage that does not cover every capability type in the authorized composition.

### AC6 Planning revalidation

Planning rejects absent, stale, out-of-scope, restricted, or incomplete authority before task-network mutation.

### AC7 Dispatch revalidation

Dispatch independently rejects a task whose authority decision does not match the active policy or task capability type.

### AC8 Honest denial

Denied authority produces no Goal command, no task-network mutation, and a stable diagnostic reason.

### AC9 Compatibility

Older direct and fixture paths retain behavior when no exact authority policy is active. Exact docs activation never uses that fallback.

### AC10 Quality gates

All changed Rust passes:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
git diff --check
```

## Explicit Deferrals

- no approvals
- no expiring grants
- no dynamic revocation
- no budgets or rate limits
- no delegation
- no organization model
- no generic action taxonomy beyond capability type identity
- no policy hot replacement inside a running assembly
