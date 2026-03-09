# Boundary Cleanup Foundation Spec

Date: 2026-03-01
Status: active

## Intent

Define cleanup work that must land before metadata contract rollout and turned workflow feature delivery.
This cleanup reduces blast radius by isolating boundaries and removing cross domain coupling.

## Why First

- reduces churn during R1 and R2 implementation
- centralizes enforcement points so later features can build on stable contracts
- lowers risk of regressions from broad refactors during feature work

## Related Specs

1. [Domain Metadata Cleanup](domain_metadata/README.md)
2. [Frame Integrity Boundary Cleanup](frame_integrity/README.md)
3. [Generation Orchestration Boundary Cleanup](generation_orchestration/README.md)
4. [Metadata Contract Ready Cleanup](metadata_contract_ready/README.md)

## Scope

- isolate frame metadata contracts from other metadata surfaces
- establish one shared write boundary for frame metadata validation
- remove integrity check dependence on free form metadata lookup
- split large generation orchestration flow into focused units
- close metadata contract readiness gaps before metadata contracts execution

## Out Of Scope

- workflow feature behavior changes
- provider capability expansion
- cross workspace orchestration

## Cleanup Order

1. domain metadata separation
2. frame integrity boundary cleanup
3. generation orchestration split
4. metadata contract readiness hardening

## Resolution Decisions

- frame metadata validation ownership is unified in `src/metadata/frame_write_contract.rs`
- `ContextApi::put_frame` remains the single write entry and delegates validation only
- compatibility wrapper migration tracks are excluded from this cleanup set
- module layout changes must follow project rule and avoid `mod.rs` targets

## Cohesive Ordered Set

1. normalize module layout where cleanup targets still use `mod.rs`
2. implement domain metadata type separation and cross domain adapters
3. activate shared frame write contract at the write entry boundary
4. complete frame integrity structural hash decoupling and typed policy errors
5. split generation orchestration units with parity gates and keep queue lifecycle stable
6. harden metadata policy readiness for digest key migration and read visibility gates

## Exit Criteria

- frame metadata validation is centralized and deterministic
- storage integrity checks are independent from arbitrary metadata keys
- generation orchestration units have clear ownership and characterization coverage
- metadata contracts phase start does not require new foundation cleanup scope

## Phase 4 Readiness Gate Status

Date: 2026-03-04
Status: complete

Phase 4 integrated parity and readiness gates are complete.
Verification summary:

- impacted integration gates passed for context api frame queue store config context cli node deletion and generation parity
- generation parity gates P1 P2 and P3 passed with committed artifacts in `tests/fixtures/generation_parity/`
- direct and queue metadata write parity passed for unknown key forbidden key and budget failure classes
- storage integrity determinism gates passed for non structural metadata mutation and structural corruption
- CLI exception list remains bounded and now includes `context regenerate` alongside existing non default path selectors

## Phase 5 Metadata Contract Readiness Status

Date: 2026-03-04
Status: complete

Phase 5 metadata contract readiness hardening gates are complete.
Verification summary:

- shared write boundary rejects raw prompt payload keys including `prompt` `raw_prompt` and `raw_context`
- generated frame metadata writes now emit digest based references and avoid raw prompt payload storage
- default read projection uses metadata key registry visibility policy
- no bypass runtime write guard test confirms queue and adapter runtime writes route through shared `put_frame` boundary
- direct and queue parity suites pass for unknown forbidden and budget failure classes
