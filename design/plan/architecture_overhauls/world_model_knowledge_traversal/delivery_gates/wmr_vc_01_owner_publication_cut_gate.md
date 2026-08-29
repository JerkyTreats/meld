# WMR-VC-01 Owner Publication Cut Delivery Gate

Date: 2026-08-26

Gate identifier: `WMR-VC-01-DG`

Revision: frozen revision 1

Status: accepted

Acceptance receipt: [WMR-VC-01 Gate Acceptance](wmr_vc_01_gate_acceptance_receipt.md)

Intended handoff: one canonical workspace publication and exact branch owner-inspection route

Gate owner: primary integrated acceptance lane

Exception authority: user

## Coherence Horizon

The horizon begins with one exact workspace tree revision. It closes when a production branch owner-inspection entrypoint returns one occurrence-rich bounded result tied to an immutable cut after durable Event carriage and the existing Graph runtime projection.

The horizon includes workspace reconstruction, Event identity, Graph admission, owner projection persistence, projection cursor order, cut selection, bounds, lag, replay, restart, branch consumption, and superseded workspace route retirement.

It excludes downstream Curation, Belief acceptance, Planner assembly, Strategy, Agent progression, Execution, PDS activation, lifecycle, Startup, and product migration.

## Required Deliverables

| Deliverable | Owner | Required product |
| --- | --- | --- |
| exact source revision | workspace | deterministic revision, scope, ordered observations, exclusions, failures, and terminal completeness |
| retryable publication | workspace | deterministic operation and Event record identity with restart reconstruction |
| durable carriage | Events | unchanged producer payload, append receipt, replay, and ledger identity |
| graph admission | Graph | `world_state.owner_publication.v1` validation with producer `domain_id` ownership and neutral hint parity |
| graph projection | Graph | intact owner operation, source Event position, and truthful cursor order |
| immutable cut | Graph and Traversal | exact owner revision, receipt, scope, policy, position, status, and identity |
| bounded result | Traversal | deterministic objects, occurrences, paths, provenance, frontier, truncation, and hydration references |
| real consumer | branches | exact owner inspection through production `graph-owner-walk` |
| retirement | workspace and Graph | no best-effort owner append, unrecoverable unchanged scan, or raw workspace knowledge authority remains for new revisions |

## Criteria

| Criterion | Required claim | Blocking condition |
| --- | --- | --- |
| `WMR-VC-01-DG-C01` | workspace derives one stable operation from an exact source revision and versioned complete enumeration rule | process memory or timestamp enters identity |
| `WMR-VC-01-DG-C02` | fresh and unchanged CLI scans reconstruct the current operation, use durable idempotent append, and report failure | best-effort append or an empty unchanged-scan retry remains on the owner path |
| `WMR-VC-01-DG-C03` | watch startup reconstructs and retries the current exact revision | restart can silently lose an unpublished revision |
| `WMR-VC-01-DG-C04` | Events remains unchanged neutral carriage | Event-owned workspace or Graph meaning |
| `WMR-VC-01-DG-C05` | envelope hints match the intact typed owner payload but do not define its meaning | hint-only workspace admission remains authoritative |
| `WMR-VC-01-DG-C06` | equal endpoints with distinct occurrence identities survive Event, Graph, cut, and result | endpoint deduplication collapses owner occurrences |
| `WMR-VC-01-DG-C07` | Graph persists owner material before advancing its identity-bearing cursor | cursor advancement can precede owned effects |
| `WMR-VC-01-DG-C08` | cut identity binds explicit owner requirements, owner revision, completeness receipt, projection position, scope, `latest_complete` policy, and status | implicit or global currentness remains |
| `WMR-VC-01-DG-C09` | equal normalized requests over equal cuts return equal result identity | process order changes identity |
| `WMR-VC-01-DG-C10` | frontier and resource truncation remain explicit | empty or truncated reads imply absence |
| `WMR-VC-01-DG-C11` | hydration and material qualifications remain owner-correct | Graph parses addresses or flattens owner meaning |
| `WMR-VC-01-DG-C12` | Event lag, Graph lag, missing owner revision, replay, and restart remain independently visible | one cursor or status substitutes for another |
| `WMR-VC-01-DG-C13` | production `graph-owner-walk` consumes the exact cut-backed route | exact traversal is reachable only from tests or fixtures |
| `WMR-VC-01-DG-C14` | new workspace revisions cannot enter owner knowledge through raw attachments | incumbent and successor workspace knowledge routes coexist |
| `WMR-VC-01-DG-C15` | structural inspection excludes typed owner publications, historical rows remain nonauthoritative, and exact queries report missing owner state until a typed revision exists | compatibility manufactures completeness, reads typed owner material, or preserves a writer |
| `WMR-VC-01-DG-C16` | docs and dependency-security specimens prove owner neutrality without product migration | substrate imports docs or security workflow meaning |
| `WMR-VC-01-DG-C17` | the candidate passes direct proof, implementation review, Style Assurance, and this gate | any required receipt is absent |
| `WMR-VC-01-DG-C18` | source changes stay inside the approved cut and tripwires | unauthorized expansion or later behavior enters the candidate |

## Direct Product Proof

The direct proof must run through `ProductRuntimeAssembly`, the real Event authority, the real Graph runtime, and the branch owner-query route.

It must demonstrate:

- one fresh and one unchanged workspace scan publication
- idempotent retry of the same revision
- restart reconstruction
- Event-ahead-of-Graph lag
- a missing required owner revision
- equal-endpoint distinct occurrences
- explicit object, occurrence, depth, or path truncation
- equal request and cut identity determinism
- historical raw workspace Events that cannot satisfy an exact cut
- docs and dependency-security owner-neutral specimens
- absence of a new workspace raw-attachment authority
- structural branch inspection that cannot satisfy or expose the typed owner cut

## Forbidden Substitutions

Append success cannot substitute for Graph visibility. Graph visibility cannot substitute for cut inclusion. Cut completion cannot substitute for downstream epistemic acceptance. A test fixture cannot substitute for the real workspace producer or branch consumer. Historical raw facts cannot substitute for owner completeness. Passing tests cannot substitute for removal of the superseded route.

## Blocking Standard

Any failed criterion blocks Gate Acceptance. The gate has no waiver authority. A user-authorized exception must name its scope, rationale, and retirement condition.

## Acceptance Budget

One initial Gate Acceptance pass and one verification pass after one bounded remediation cycle.

The gate remains `not eligible` until source implementation, direct proof, integrated implementation review, and a satisfied Style Assurance Receipt exist for one exact candidate.
