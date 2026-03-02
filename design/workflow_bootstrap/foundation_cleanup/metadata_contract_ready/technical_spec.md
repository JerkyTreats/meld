# Metadata Contract Ready Technical Specification

Date: 2026-03-02
Status: active

## Intent

Provide one synthesis execution specification for metadata contract readiness cleanup.
This spec maps each required change to outcome and verification gates.

## Source Synthesis

This specification synthesizes:

- [Metadata Contract Ready Cleanup](README.md)
- [Code Review](code_review.md)
- [Workflow Metadata Contracts Spec](../../metadata_contracts/README.md)

## Boundary

Start condition:
- core cleanup tracks are active and shared write boundary exists
- raw prompt metadata write path still exists

End condition:
- forbidden payload keys are blocked at shared write boundary
- forward digest key set is accepted at shared write boundary
- read output defaults enforce registry driven visibility policy
- typed policy errors are deterministic across direct and queue writes

## Change To Outcome Map

### C1 Enforce forbidden payload key gate

Code changes:
- update shared validator in `src/metadata/frame_write_contract.rs` to reject raw prompt and raw context payload keys
- remove any queue path writes that emit forbidden payload keys in `src/context/queue.rs`

Outcome:
- no runtime frame write can persist raw prompt or raw context payload metadata values

Verification:
- direct write and queue write tests fail deterministically for forbidden key input
- default metadata output cannot display forbidden payload values

### C2 Add forward frame key acceptance for digest migration

Code changes:
- update key policy in `src/metadata/frame_write_contract.rs` to allow `prompt_digest` `context_digest` and `prompt_link_id`
- keep policy deterministic for unknown key rejection

Outcome:
- shared validator is ready for R1 digest reference writes

Verification:
- validator accepts full required bootstrap key set
- unknown key tests still fail deterministically

### C3 Promote typed metadata policy failures

Code changes:
- extend `src/error.rs` with dedicated metadata policy variants for unknown key forbidden key and budget overflow
- update shared validator and call sites to emit these variants

Outcome:
- callers can branch on metadata policy failures without parsing message strings

Verification:
- direct and queue paths return matching typed errors for matching invalid input

### C4 Enforce registry driven visibility at read boundaries

Code changes:
- replace key local projection logic in `src/metadata/frame_types.rs` with registry driven visibility decisions
- keep cli text and json surfaces delegated to shared projection policy in `src/cli/presentation/context.rs`

Outcome:
- metadata visibility behavior is centralized and stable

Verification:
- context text and json outputs exclude forbidden and non visible keys by default
- read behavior stays deterministic for stable input

### C5 Add no bypass runtime write guard

Code changes:
- add integration gate that fails when runtime frame writes bypass `ContextApi::put_frame`
- ensure queue runtime paths continue to route writes through shared validation entry

Outcome:
- future runtime write additions cannot skip shared metadata policy enforcement silently

Verification:
- guard test fails on bypass and passes on approved write routes

## File Level Execution Order

1. `src/error.rs`
2. `src/metadata/frame_write_contract.rs`
3. `src/context/queue.rs`
4. `src/metadata/frame_types.rs`
5. `src/cli/presentation/context.rs`
6. `src/api.rs`
7. `tests/integration/context_api.rs`
8. `tests/integration/frame_queue.rs`
9. `tests/integration/context_cli.rs`

## Verification Matrix

Write policy gates:
- raw prompt and raw context payload keys fail on direct and queue writes
- required digest key set passes shared validation
- unknown key and budget failures are typed and deterministic

Read policy gates:
- default metadata output never leaks forbidden payload values
- visibility policy comes from one metadata domain contract

No bypass gates:
- runtime writes route through shared validation entry
- guard coverage fails when new bypass path is introduced

## Completion Criteria

1. all change sets C1 through C5 are implemented
2. verification matrix gates pass
3. metadata contracts phase can start without foundation scope expansion
