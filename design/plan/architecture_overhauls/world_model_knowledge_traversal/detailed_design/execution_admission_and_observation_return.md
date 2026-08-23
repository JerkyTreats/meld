# Execution Admission And Observation Return Detailed Design

Date: 2026-08-22

Slice: `WMR-DD-04`

Status: corrected design candidate for `WMR-DG-04` revision 2

Implementation authorization: none

## Decision

Execution consumes one complete Agent-authorized Task through its Goal Set. It validates the admission without becoming Strategy, considers all admitted executable obligations together, and lowers compatible work into one unified Task Network. After realization, Execution publishes its own outcome. The semantic owner then observes the changed or unchanged world and publishes an owner-qualified product under the accepted observation contract.

The return loop closes only when the exact Plan dependency milestone is durably absorbed by Agent. Task completion, Event append, Graph visibility, Belief revision, Agent absorption, and Goal satisfaction remain independent.

## Active Product Path

```text
Agent-authorized Task
-> Goal-attributed Execution admission receipt
-> cross-admission coherence decision
-> deterministic lowering and unified Task Network mutation
-> ready Task claim
-> fenced attempt and external operation
-> durable Execution outcome
-> neutral Execution Event
-> semantic-owner re-observation
-> owner completeness receipt and publication
-> neutral returned Event
-> independent Graph and configured Belief visibility
-> Agent milestone absorption
```

## Execution Admission Boundary

The offered Task carries its complete semantic body and exact lineage from `WMR-DD-03`: Agent, Goal, Plan family and revision, selection reference, Task identity, frozen context, required Capability contracts, inputs, dependencies, expected outcome, authority, activation generation, and idempotency key. Goal Set admission preserves the Goal attribution without making the Goal an executable aggregate or a Strategy artifact.

Execution checks only consumer-owned obligations:

- required fields and Task internal closure are present
- authorization binds the exact Task and Plan revision
- current installed Capability contracts match the authorized identities
- inputs are structurally admissible to the selected realization seam
- authority and generation fences permit intake
- the idempotency identity has one prior or new consumer decision

Execution does not search for a Method, revise the Plan, repair an incomplete Task, evaluate Goal desirability, reinterpret Epistemic Operations, or compare a fresh world model to Strategy preconditions. A mismatch becomes rejection or conflict evidence returned to Agent.

## Lowering And Task Network Closure

One admitted Task produces one deterministic lowering identity and one idempotent mutation proposal against Execution's single durable Task Network. Several Tasks in one Plan or across several Agents remain independent Goal-attributed admissions. They never become one aggregate authorization, but they always enter the same Execution coherence domain.

Execution Planning compares admitted executable regions before committing network mutation. It may share an operational node only when Capability contract, bound inputs, source revision, observable effects, authority, validity window, result schema, and activation fence are compatible. Otherwise it preserves distinct nodes or regions. A shared node retains every contributing admission, Task, Goal, Agent, and Plan attribution so one operational result can discharge several compatible obligations without collapsing their independent semantic progression.

Priority, resource reservation, executable ordering, parallelism, and compatible-work reuse are Execution-owned decisions over admitted Tasks. They do not alter Task meaning or add a missing semantic step.

The lowering preserves semantic Task identity, Plan lineage, Capability contracts, initialization inputs, internal dependencies, expected outcome, authority, and generation into each compiled node. Task Network identity, node identity, claim identity, and attempt identity remain Execution-owned realization products.

The current process-memory package route is not a valid handoff position. A route used by dispatch or aggregate publication must either be durably recorded through an existing Execution-owned progress or journal seam, or be mandatorily reconstructed from the admitted Task, exact installed binding revision, and exact versioned routing-rule identity after restart. Reconstruction is fenced to those same identities. A binding or routing-rule change produces a successor route rather than silently changing the prior route. The detailed design does not choose storage or add a new store.

## Dispatch And Uncertain Effects

Ready-set selection is always against one durable Task Network revision. A claim records worker, lifecycle epoch, activation generation, exact operational node, and every admission attributed to that node. An attempt records the claim and external operation identity before relying on a provider effect. That operation identity binds the exact installed Capability binding revision, resolved Capability instance, effect target, compatible admission set, contract, input digest, authority, and generation.

Provider retry uses the same external operation identity only against the same resolved Capability instance and effect target when the provider contract supports idempotency. A target or binding change requires a distinct operation identity or an explicit Execution-owned reconciliation that preserves both positions. When effect completion is uncertain, Execution records `uncertain` rather than replaying as fresh work. A successor attempt requires an Execution-owned reconciliation decision under the original authorization fence.

Late callbacks retain attempt and generation lineage. Execution classifies their operational effect. Lifecycle later decides whether the generation can retire; root never reinterprets provider semantics.

## Outcome And Publication

Execution persists the operational outcome and complete per-admission discharge set before publishing. One deterministic publication identity carries the outcome, discharge set, and declared artifacts through Events. An Event append receipt proves only neutral durability.

When one shared action serves several admissions, Execution records one operational outcome plus a discharge account for every contributing admission. Sharing the effect never merges Goal, Task, Plan, or Agent identities and never makes one Agent's semantic milestone decision authoritative for another.

Graph projection of the Execution Event may support inspection, but the Event is not semantic-owner truth. A docs-writing Task can succeed even if the resulting README is incomplete. A dependency mitigation Task can succeed even if the advisory still applies.

## Semantic-Owner Observation Return

The owner named by the Plan observation dependency re-observes its source under a successor or unchanged owner revision. It publishes the accepted `WMR-DD-01` typed batch and completeness receipt with exact outcome and artifact lineage where relevant.

The owner may report:

- changed observation under a successor revision
- unchanged observation under a completed scope
- incomplete observation with exact frontier, exclusions, and failures
- conflicted observation when the source or fence no longer matches

Execution cannot manufacture these products. The owner observation may be triggered by an outcome, filesystem or source change, or its own durable selection rule, but it must bind the exact resulting source revision rather than infer truth from the Task outcome.

## Independent Return Milestones

The Plan dependency names the exact milestone required for progression:

| Milestone | Owner evidence | Does not prove |
| --- | --- | --- |
| Execution terminal | exact operational outcome, admission discharge, and attempt lineage | Event append or semantic change |
| Execution Event durable | append receipt for outcome publication | returned owner observation |
| owner observation durable | exact owner product and completeness receipt | Graph or Belief visibility |
| returned Event durable | append receipt for owner publication | Graph visibility |
| Graph visible | projection cursor covers the returned Event | configured Belief settlement |
| Belief settled | immutable revision cites the owner evidence and route | Agent absorption |
| Agent accepted | milestone decision cites exact Plan dependency and owner position | Goal satisfaction |

Agent may progress when the declared milestone is satisfied. It may request successor work, hold, reject stale evidence, or separately judge Goal satisfaction.

## Product Proof

### Missing README

1. Agent authorizes one complete README-writing Task under Plan revision P.
2. Execution accepts exactly that Task and lowers it without Method search or world-model replanning.
3. A fenced attempt writes or updates the README and produces a durable outcome with artifact lineage.
4. Docs and workspace owners re-observe the governed scope under successor revisions and publish completeness receipts.
5. Events carries the returned owner publications, Graph materializes them, and configured Belief may produce a revision.
6. Agent absorbs the exact milestone named by P and separately decides whether the desired condition is satisfied.

A succeeded Task without a complete owner observation cannot close the Plan dependency when the dependency requires owner observation or Belief settlement.

### Partial Or Failed README Work

An uncertain or failed attempt remains an Execution product. If the filesystem changed, workspace and docs may still publish partial or successor observations. Agent uses the declared milestone and Plan policy to wait, reconcile, or request a successor. It never treats operational failure as proof that no semantic change occurred.

### Dependency Security

A mitigation Task outcome names the exact operation and artifacts. Dependency security then re-observes manifest and lockfile state, updates inventory under its own revision, reevaluates advisory applicability, and emits owner-owned assessment or verification products. Mitigation success, inventory change, advisory non-applicability, assessment, and verification remain distinct milestones.

### Shared Merkle Scan

1. Agent A and Agent B independently submit complete Goal-attributed Tasks requesting the same Merkle scan over the same workspace revision.
2. Goal Set records two independent admissions and preserves both Agent, Goal, Plan, Task, authority, and generation lineages.
3. Execution Planning proves that the Capability contract, inputs, workspace revision, effects, authority, validity window, and result schema are compatible.
4. One shared action node enters the unified Task Network with attribution to both admissions.
5. One fenced attempt produces one operational result and two admission-discharge records.
6. Each Agent independently absorbs only the milestone declared by its own Plan and separately determines successor work or Goal disposition.

If any compatibility input differs, Execution creates distinct operational nodes. It never forces sharing merely because labels or intended outcomes look similar.

## Deferred Boundaries

- `WMR-DD-05` defines how PDS installs exact Capability and participant inputs before Agent genesis.
- `WMR-DD-06` defines activation generation, participant incarnation, wake closure, safe points, replacement, and retirement.
- `WMR-DD-07` composes the complete product proof without changing owner contracts.

No deferred boundary is borrowed as current runtime proof.
