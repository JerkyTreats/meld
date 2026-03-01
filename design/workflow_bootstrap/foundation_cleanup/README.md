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

1. [Domain Metadata Separation Spec](domain_metadata_separation_spec.md)
2. [Frame Integrity Boundary Spec](frame_integrity_boundary_spec.md)
3. [Generation Orchestration Split Spec](generation_orchestration_split_spec.md)

## Scope

- isolate frame metadata contracts from other metadata surfaces
- establish one shared write boundary for frame metadata validation
- remove integrity check dependence on free form metadata lookup
- split large generation orchestration flow into focused units

## Out Of Scope

- workflow feature behavior changes
- provider capability expansion
- cross workspace orchestration

## Cleanup Order

1. domain metadata separation
2. frame integrity boundary cleanup
3. generation orchestration split

## Exit Criteria

- frame metadata validation is centralized and deterministic
- storage integrity checks are independent from arbitrary metadata keys
- generation orchestration units have clear ownership and characterization coverage
