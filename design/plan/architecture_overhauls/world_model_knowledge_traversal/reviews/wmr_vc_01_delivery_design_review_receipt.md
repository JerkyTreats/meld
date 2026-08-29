# WMR-VC-01 Delivery Design Review Receipt

Date: 2026-08-26

Review owner: primary integrated architecture review lane

Review mode: initial review plus one bounded verification

Verdict: passed for explicit user approval

Source authority: none

## Problem First

The initial candidate correctly refused to restore the archived additive source. It still left two practical failures in the proposed replacement boundary. An unchanged workspace scan could not reconstruct a lost publication, and replacing all branch graph inspection with owner-cut semantics would either break unrelated operational inspection or preserve an undeclared fallback.

The corrected thesis is sharp: workspace owns one retryable exact publication, Graph admits it through the existing runtime, `graph-owner-walk` consumes one immutable cut, and structural graph inspection remains a separate nonauthoritative product that never receives typed owner publications.

## Candidate

Reviewed artifacts:

- [source runtime groundmap](../source_runtime_groundmap.md)
- [proposed Delivery Gate](../delivery_gates/wmr_vc_01_owner_publication_cut_gate.md)
- [source delivery program ledger](../world_model_reconciliation_source_delivery_program_ledger.md)

Final artifact digests:

- groundmap: `3c00741bbb9ca1c6da0a527ec32e2c29eb3d86fbfbbf83cdba104225be434b60`
- gate: `45f87220210486df654691404246e29abf5f4db989588876fe24552a6165ab17`
- ledger: `0db61377d76b215b5ff93a8e9e30dea891fcd3031113db26ce3b9b869041c4fb`

The review inspected current workspace scan and watch publication, Event identity and replay, Graph reduction and cursor order, root runtime assembly, traversal persistence and query, and branch CLI inspection.

## Implemented Facts

- workspace scan already derives a deterministic tree revision and ordered observations
- CLI scan currently emits publication candidates best effort
- unchanged scans currently return no publication candidates
- watch startup rebuilds workspace storage but does not publish the current revision
- Events already supports opaque producer payload, stable record identity, idempotent durable append, replay, and ledger identity
- one root-composed `GraphRuntime` already owns projection through one `TraversalStore`
- Graph currently admits allowlisted raw attachments and flattens relation occurrences
- branch CLI currently exposes structural status, neighbors, and walk products

## Frozen Findings

| Finding | Severity | Classification | Evidence | Disposition |
| --- | --- | --- | --- | --- |
| `WMR-VC-01-DR-F01` | high | active-slice design defect | `WorkspaceScanStatus::UpToDate` returns no candidates, so rerun cannot repair a lost publication | require fresh and unchanged scans to reconstruct the same operation and append idempotently |
| `WMR-VC-01-DR-F02` | high | active-slice design defect | replacing structural branch inspection would either break context and Execution graph reads or preserve an undeclared fallback | add production `graph-owner-walk`; keep structural inspection separate and exclude typed owner material |
| `WMR-VC-01-DR-F03` | high | active-slice design defect | the initial cut did not bind caller-selected currentness to an exact policy | require explicit owner set and scope under `latest_complete` at one durable Graph position |
| `WMR-VC-01-DR-F04` | medium | evidence correction | the initial candidate did not name the typed Event route or production command | name `world_state.owner_publication.v1` and `graph-owner-walk` in groundmap, gate, and ledger |

Program-owner disposition: all four findings accepted.

## Corrections

The bounded correction changed delivery design only. It added unchanged-scan reconstruction, separated owner-cut inspection from structural inspection, fixed explicit `latest_complete` selection, named the producer-owned Event route, and named the real CLI consumer.

No source file, store, API, runtime actor, or authority was changed during correction.

## Verification

Verification was limited to the four frozen findings and correction-caused regressions.

| Finding | Verification |
| --- | --- |
| `WMR-VC-01-DR-F01` | passed, groundmap and gate now require unchanged-scan reconstruction and direct proof |
| `WMR-VC-01-DR-F02` | passed, exact and structural inspection products are distinct and typed publications cannot enter the structural route |
| `WMR-VC-01-DR-F03` | passed, cut identity now binds explicit owner requirements, scope, `latest_complete`, and Graph position |
| `WMR-VC-01-DR-F04` | passed, all three candidate artifacts name the Event type and production command |

Correction-caused regression set: empty.

Ledger validation passed. Markdown diff validation passed. No source test is eligible before implementation.

## Boundary Judgment

The candidate uses the smallest existing architecture that can prove the requested behavior. Workspace, Events, the existing Graph runtime, the existing traversal store, and branches remain separate owners. The proposed record family stays inside the canonical Graph authority. The producer payload stays workspace-owned. Structural inspection remains available without becoming a second owner-knowledge reader.

The candidate does not add a crate, service, actor, Event authority, Graph authority, standalone store, universal ontology, compatibility writer, Curation behavior, Planner behavior, or downstream acceptance claim.

## Review Outcome

`WMR-VC-01` is coherent, maturity-aligned, and approval-ready. The review establishes design readiness only. It does not activate source implementation, issue Style Assurance, accept `WMR-VC-01-DG`, authorize a commit, or authorize later source cuts.
