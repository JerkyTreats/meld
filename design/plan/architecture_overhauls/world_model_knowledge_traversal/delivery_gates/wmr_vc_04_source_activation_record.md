# WMR-VC-04-R1 Source Activation Record

Date: 2026-09-04

Slice identifier: `WMR-VC-04-R1`

Status: Gate accepted, slice closed, committed, and pushed

Source baseline: `713ca3ee`

Authority: direct program-owner corrective and retirement authority

Frozen gate: [WMR-VC-04-DG revision 4](wmr_vc_04_execution_route_gate.md)

Worker packet: [WMR-VC-04-R1 worker packet](../detailed_design/wmr_vc_04_worker_packet.md)

## Authorized Work

Preserve the verified actual Docs Task route and atomically retire the duplicate Execution planning authority, Execution Goal writer, package-plan handoff route, old runtime identities, exports, exclusive tests, and exclusive fuzz surface.

The authority explicitly excludes PDS changes, Task replacement, a synthetic workspace Task, workspace owner return, commit, push, deployment, and `WMR-VC-05`.

## Persisted Data Tripwire

Before source retirement, the configured Execution storage root was inspected. The legacy Goal database and current Task storage contained no application records requiring a writable compatibility route. No anomaly required a design return.

Historical Task Network records remain supported by a named read-only decoder. Networks containing retired planning lineage reject new commands.

## Activated Write Set

- move Task admission and direct lowering into `meld-execution::task_admission`
- seal admission decisions behind canonical consumer validation
- seal attributed graph insertion behind canonical direct lowering and persist exact full-node hashes
- preserve Agent Task authorization, execution receipts, and terminal milestone acceptance
- preserve Task Network admission attribution, claims, outcomes, and neutral publication
- preserve exact Docs Capability dispatch through the product catalog and registry
- delete Execution Goal ownership and storage
- delete the Execution planner, Method search, realization, world projection, and composition lowering
- delete package-plan handoffs and dispatch wiring
- remove old runtime registrations and storage resources
- remove the retired planner projection adapter, resource requirement, and dead error surface
- rewrite tests and harness topology around direct Task admission
- add historical read-only replay proof
- update program artifacts for gate revision 4

## Deferred Surface

The candidate contains no new WMR `workspace_scan` registration, no workspace candidate publication, no owner Event append, and no owner Graph visibility claim. Existing separately supported workspace behavior outside this candidate remains unchanged.

## Actual Task And Endpoint

The source Task is the exact durable Agent authorization for the five Docs Capability types. The direct root proof reads that authorization, admits it, lowers five nodes into one region, drives durable claims, records a terminal region outcome, publishes the neutral outcome, advances Graph over the appended Event, and returns the terminal outcome identifier to Agent.

The final proof endpoint is the durable Agent milestone owner position equal to the admitted region terminal outcome identifier.

## Limits

The original forty-path estimate caused a bounded design return during logical review. Rename-aware comparison from pre-retirement checkpoint `dae788ae98a546c3eafb8fc094604a86a8bdd1f6` to the exact worktree candidate establishes 49 changed production paths, 1,088 added production lines, and 8,305 removed production lines. The corrected final limit is fifty production paths and twelve hundred added production lines. Production deletions remain uncapped.

The path increase accounts only for the mandatory atomic retirement inventory and named source moves. It adds no responsibility or authority. The combined uncommitted VC-04 candidate is larger because it also contains the preserved canonical Task route implemented before R1.

No new crate, dependency, database, durable store, Event authority, Graph authority, or Task Network was added by R1.

## Assurance Result

Exact candidate `WMR-VC-04-R1::5308f0ba6ce873bd1e00b4ba283d60eefc9df996f9ba261461fc44929c86c430` passed in the required order:

- [fresh bounded logical implementation review](../reviews/wmr_vc_04_corrected_implementation_review_receipt.md)
- [dedicated Style Assurance](../reviews/wmr_vc_04_style_assurance_receipt.md)
- [fresh Gate Acceptance](wmr_vc_04_gate_acceptance_receipt.md)

Any later material source or executable-test edit invalidates this candidate identity and its ordered assurance.

## Authority Boundary

Program-owner close authority ended this activation at the accepted exact `WMR-VC-04-R1` candidate. Separate authority dated 2026-09-04 authorized its commit and branch push. Deployment and another slice remain unauthorized.
