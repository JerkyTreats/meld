# WMR-VC-04 Execution Route Delivery Gate

Gate identifier: `WMR-VC-04-DG`

Revision: 4

Date frozen: 2026-09-04

Status: accepted, closed, committed, and pushed for exact candidate `WMR-VC-04-R1::5308f0ba6ce873bd1e00b4ba283d60eefc9df996f9ba261461fc44929c86c430`

Source baseline: `713ca3ee`

## Revision Disposition

Revision 1 is invalid because it assumed `workspace_scan` instead of the accepted Docs Task.

Revision 2 verified the corrected Task route but did not satisfy repository retirement policy. It allowed the superseded Execution planner, Goal writer, package-plan route, runtime identities, exports, and exclusive support surface to remain constructible.

Revision 3 preserves the valid corrected route and adds atomic retirement as a blocking requirement.

Revision 4 reopens the invalid revision 3 acceptance. It seals durable admission creation behind canonical consumer validation, seals admitted graph insertion behind canonical lowering, requires the exact canonical lowered node at graph mutation and every dispatch position, and deletes the surviving planner projection compatibility adapter and resource residue.

## Coherence Horizon

The gate begins with the actual accepted `WMR-VC-03` Task stored in a durable Agent Plan. It ends at durable Agent acceptance of the exact `ExecutionTerminal` milestone declared by that Task.

Inside the horizon are fresh Agent Task authority, durable Execution admission, direct validation and lowering, one attributed region in the single Task Network, live-fenced production dispatch, durable operational terminality, neutral publication, independent Graph progress where the Event route provides it, and Agent milestone acceptance.

Outside the horizon are workspace owner return, Docs correctness, Belief settlement, Goal satisfaction, PDS compilation, lifecycle redesign, Startup, product migration, and later slices.

## Exact Task

The accepted Task contains exactly:

- `docs.inspect_scope`
- `docs.draft_patch_set`
- `docs.validate_patch_set`
- `docs.publish_patch_set`
- `docs.assess_published_scope`

`workspace_scan` is not part of this Task and may not be synthesized or substituted.

## Exact Trace

```text
Agent Plan Task
-> fresh Agent Task authorization
-> durable Execution Task admission
-> direct Task validation
-> direct Task lowering
-> one attributed Task Network region
-> durable fenced claims
-> production Capability invocation
-> durable region terminal Outcome
-> neutral Execution publication
-> independent Graph catchup where an Event exists
-> Agent ExecutionTerminal milestone acceptance
```

The exact proof endpoint is `AgentMilestoneAcceptance.owner_position_id` equal to the terminal `Outcome.outcome_id` selected for that admission region.

## Blocking Criteria

| Identifier | Criterion |
| --- | --- |
| `WMR-VC-04-DG-C01` | root exposes exactly one active Agent reconciliation route and one Execution Task admission route, and the public Task Network command contract cannot create a durable admission decision |
| `WMR-VC-04-DG-C02` | the proof starts from the actual accepted Agent Task and pins all five Docs Capability types |
| `WMR-VC-04-DG-C03` | Agent checks current Planner cut and live activation authority before fresh Task authorization |
| `WMR-VC-04-DG-C04` | the Task offer retains exact Agent, Goal, Plan, authorization, context, scope, policy, generation, binding, contract, action, and idempotency identities |
| `WMR-VC-04-DG-C05` | Execution durably records accepted, rejected, stale-fence, and replayed admission positions |
| `WMR-VC-04-DG-C06` | consumer validation checks closure, exact installed contracts, exact action authority, live generation, and idempotency without repair; only its validated internal write may create the durable decision |
| `WMR-VC-04-DG-C07` | lowering consumes only the admitted Task and performs no world projection, Method search, Strategy reconstruction, or semantic repair |
| `WMR-VC-04-DG-C08` | one accepted admission creates one independently attributed operational region in the single Task Network; public graph mutation cannot insert attributed nodes; canonical lowering seals the exact full node content for every admitted step; duplicate steps and substituted executable content are rejected |
| `WMR-VC-04-DG-C09` | two accepted admissions create distinct regions and cannot share operational nodes |
| `WMR-VC-04-DG-C10` | dispatch consumes durable Task Network claims only and verifies the complete canonical lowered node plus exact durable admission attribution, authority, and activation generation before claim and invocation, including resumed invocation |
| `WMR-VC-04-DG-C11` | emitted artifacts persist before terminal outcome recording and crash replay cannot duplicate outputs |
| `WMR-VC-04-DG-C12` | the region terminal query waits for every independent branch and returns one exact sink or deterministic failure outcome |
| `WMR-VC-04-DG-C13` | neutral Execution publication is durable and remains distinct from operational completion |
| `WMR-VC-04-DG-C14` | Graph progress is asserted only after exact Event append and remains distinct from publication |
| `WMR-VC-04-DG-C15` | Agent accepts only the Plan-declared terminal milestone and cites the exact operational outcome identifier |
| `WMR-VC-04-DG-C16` | no proof infers Docs correctness, Belief settlement, Goal satisfaction, or owner truth from operational success |
| `WMR-VC-04-DG-C17` | no WMR workspace owner-return source, candidate publication, owner Event, owner Graph claim, or new workspace registration remains |
| `WMR-VC-04-DG-C18` | no public Execution planner, Method library, action realization, world projection, planner projection port or resource adapter, composition lowerer, or Execution Goal writer remains |
| `WMR-VC-04-DG-C19` | root registers `execution.task_admission` once and registers neither retired runtime identity |
| `WMR-VC-04-DG-C20` | no package-plan input, handoff registry, package dispatch branch, or sole-purpose adapter remains |
| `WMR-VC-04-DG-C21` | canonical startup does not open or create the legacy Goal database |
| `WMR-VC-04-DG-C22` | historical planning records replay through a named read-only decoder and current writes cannot emit that format |
| `WMR-VC-04-DG-C23` | generic Task and explicit Workflow persistence remain green, while Workflow cannot claim Agent authority without admission |
| `WMR-VC-04-DG-C24` | all source, tests, fuzz targets, exports, docs, and runtime inventory agree with the responsibility disposition |
| `WMR-VC-04-DG-C25` | one exact candidate passes logical review, then Style Assurance, then fresh Gate Acceptance in that order |

## Required Negative Evidence

- retired source files are deleted
- retired exports and exact runtime identifiers have no current registration or alias
- package-plan dispatch symbols have no source occurrence outside historical design evidence
- the root product catalog proves `workspace_scan` absent from this Task route
- current Task lineage constructors leave retired fields inaccessible
- the public Task Network command contract rejects a serialized forged admission decision
- public graph mutation rejects every attributed insertion
- canonical graph insertion rejects duplicate admitted steps and records the exact full lowered node hash for each step
- graph mutation and dispatch reject attribution that differs from the exact durable admission record
- fresh and resumed dispatch reject executable node content that differs from canonical lowering
- a historical journal fixture replays and rejects a later write
- startup storage tests prove no Goal database path or handle
- explicit Workflow tests pass

## Maturity Rule

Operational terminality means only that Execution recorded a final region outcome. Event publication means only that the neutral outcome envelope was durably appended. Graph progress means only that the appended Event was projected. Agent milestone acceptance means only that the Plan-declared operational dependency was met.

None of these positions establishes semantic owner truth.

## Acceptance Sequence

The logical implementation review must judge one exact manifest and pass before Style Assurance begins. Style Assurance must be satisfied before Gate Acceptance begins. Any material source change after logical review creates a new candidate and requires a fresh logical review.

## Authority Boundary

Gate Acceptance and explicit program-owner close authority close `WMR-VC-04` at its exact uncommitted candidate. They do not authorize commit, push, deployment, `WMR-VC-05`, or later work.
