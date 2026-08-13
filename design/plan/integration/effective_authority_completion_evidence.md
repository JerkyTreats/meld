# Effective Authority Completion Evidence

Date: 2026-08-13
Status: complete
Scope: Theory Elevation Step 4
Design: [Effective Authority Implementation Design](effective_authority_implementation_design.md)

## Delivered Contract

Effective authority is now a deterministic intersection over the verified candidate, package request, principal grant, runtime allowance, explicit restrictions, and exact subject scope. Action identities use existing capability type identities at the current product maturity.

The implementation extends existing contracts. Strategy authorization carries the authority decision. Execution goal admission retains it. Planning revalidates it before lowering. Task lineage carries it. Dispatch independently checks it before invoking the existing claimed-task port.

No role engine, approval flow, authorization service, authorization ledger, workflow, or capability wrapper was added.

## Durable Activation

The stewardship declaration selects `principal_id` and `authority_policy_id`. The execution domain owns append-only exact policy revisions. Complete theory receipts pin the exact revision alongside existing theory and executable contracts.

Theory source provisioning validates selected policy identity, principal, and subject before writing. Exact runtime resolution rejects absent revisions and selection disagreement. Assembly freezes one policy binding and supplies it to Agent judgment, planning, and dispatch.

## Compatibility

Absent authority fields deserialize as the compatibility state and remain omitted during serialization. Legacy task-network fixtures retain their prior hashes. Compatibility paths remain usable only when no exact authority policy is active.

The elevated docs path always activates the shipped `docs_workspace_local` policy and requests the five capability actions already present in its Strategy package.

## Acceptance Evidence

The pure language contract proves full intersection and explicit restriction. The policy registry proves idempotent exact reuse and historical revision resolution. Agent tests prove restriction becomes an indeterminate decision with no Goal command and allowance embeds the decision in existing Strategy authorization.

Execution tests prove planning permits a matching decision and rejects a newly restricted policy. Dispatch tests prove an active policy rejects missing lineage before invocation and permits matching lineage. Existing task-network round-trip fixtures prove absent authority does not change legacy identities.

The runtime CLI flywheel test passes through the receipt-hydrated authority path from Agent authorization through planning and task dispatch.

## Quality Gate

The delivery gate requires these commands to pass from the repository root:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
git diff --check
```

The final command results are recorded in the Step 4 commit handoff.
