# Metadata Contract Ready Cleanup

Date: 2026-03-02
Status: active

## Intent

Define one cleanup track that unblocks metadata contract execution with low churn.
This folder contains a focused code review and a synthesis execution spec.

## Related Docs

- [Code Review](code_review.md)
- [Synthesis Technical Specification](technical_spec.md)
- [Boundary Cleanup Foundation Spec](../README.md)
- [Workflow Metadata Contracts Spec](../../metadata_contracts/README.md)

## Problem

Current cleanup tracks establish strong boundaries, yet one readiness gap remains.
Raw prompt payload can still be written into frame metadata and read surfaces do not enforce registry driven visibility policy.
This gap raises migration risk for the metadata contracts phase.

## Scope

- enforce a hard forbidden key gate for raw prompt and raw context metadata payload keys
- establish forward accepted frame keys for `prompt_digest` `context_digest` and `prompt_link_id`
- enforce typed metadata policy failures for unknown key forbidden key and budget overflow
- remove queue local free form metadata map assembly from runtime write paths
- add direct and queue parity coverage for invalid metadata and visibility behavior
- add one no bypass gate that proves all runtime frame writes flow through shared validation

## Out Of Scope

- prompt artifact storage implementation
- workflow thread and turn record implementation
- provider capability expansion

## Entry Criteria

1. domain metadata separation cleanup is complete
2. frame integrity cleanup seams are active
3. generation orchestration cleanup seams are active

## Exit Criteria

1. default frame metadata output can never include raw prompt payload values
2. shared write validation accepts only approved current keys and approved forward keys
3. typed metadata policy failures are deterministic across direct and queue writes
4. runtime write paths cannot bypass shared frame metadata validation
5. metadata contracts phase can begin without additional foundation scope growth
