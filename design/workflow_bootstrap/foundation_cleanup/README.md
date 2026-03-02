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
