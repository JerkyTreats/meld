# WMR-DD-03 Worker Packet

Date: 2026-08-22

Slice: `WMR-DD-03`, PlannerCut Assembly, Heterogeneous Plan Construction, And Agent Progression

Status: active

Implementor: Codex detailed-design implementor

Branch: `design/world-model-reconciliation`

## Product Increment

Define one owner-to-owner vertical from exact world-model source revisions and a ground Goal through immutable `PlannerCut` assembly, pure heterogeneous Plan construction, durable Agent judgment, exact product authorization, admitted milestone progression, and successor Plan lineage.

The direct design proof must settle the authority split. Planner assembles context. Strategy constructs and verifies semantic causal products. Agent judges the Plan and separately authorizes each eligible product. Curation or Execution performs only the product it owns.

## Maturity Envelope

Posture: exploratory

Obligation floor: operational durability for incumbent Event, Graph, Belief, Agent, Goal, and Execution handoff records

Hard limits:

- design artifacts only
- zero source-code changes
- zero new crates, dependencies, durable stores, services, background runtimes, generalized protocols, migrations, or compatibility systems
- no change to Events or `meld-lang` semantic ownership
- Planner does not select products
- Strategy remains pure and has no runtime progression authority
- Agent does not perform Curation or Execution work
- Execution never receives the heterogeneous Plan or an Epistemic Operation
- Execution intake and returned observation remain deferred to `WMR-DD-04`
- one active detailed-design slice

Tripwires:

- any source revision used by Strategy is outside the frozen construction context
- `WorldModelView` becomes a competing cut authority
- one Plan judgment implicitly authorizes every product
- a generic completed state replaces owner milestones
- Plan identity changes without source, policy, semantic body, or predecessor change
- one Goal is constrained to one Task
- a historical authorization can escape its Plan revision or activation fence
- the design must select new persistence placement or runtime topology

## Direct Product Proof

### Already-correct docs

If an admitted cut already establishes the desired condition, Strategy may return a Plan revision with the condition proven satisfied and no discharge product. Agent may record satisfaction only through the Goal's owning admitted semantics. No Task is manufactured merely because a Goal or construction request existed.

### Missing or incorrect docs

The cut preserves exact expected README, observed state, claim, Belief, causal, regime, directive, and Capability inputs. Strategy may construct several products, including a bounded Epistemic Operation, a complete Task, and a later epistemic assessment, with exact dependency milestones. Agent authorizes only currently eligible products and sends only the Task product to the deferred Execution seam.

### Dependency security dissimilarity

Strategy may combine assessment Curation, inventory or advisory refresh, mitigation, later owner observation, and verification without treating scan completion as applicability or mitigation completion as resolution. Product and milestone types remain owner-shaped.

## Exact Write Scope

- this worker packet
- `planner_cut_and_plan_transition_ledger.md`
- `planner_cut_strategy_plan_and_agent_progression.md`
- `../world_model_reconciliation_handoff_ledger.md`
- `../delivery_gates/wmr_dg_03_planner_cut_plan_and_progression.md`
- `../reviews/wmr_dd_03_subagent_review_recommendation.md`
- `../reviews/wmr_dd_03_integrated_design_review_receipt.md`
- `../delivery_gates/wmr_dg_03_subagent_acceptance_recommendation.md`
- `../delivery_gates/wmr_dg_03_acceptance_receipt.md`
- `../world_model_reconciliation_delivery_program_ledger.md`
- `../README.md`

## Existing Seams

- accepted `TraversalCut`, owner publication, and hydration design from `WMR-DD-01`
- accepted Curation operation, terminal result, Event, Graph, and configured Belief positions from `WMR-DD-02`
- canonical Planner source owners and immutable `PlannerCut`
- canonical heterogeneous `StrategyPlan`, complete Task, bounded Epistemic Operation, verification, and consumer boundaries
- canonical Agent Plan judgment, product authorization, and durable progression ownership
- current durable Agent decision-before-sink and Execution Goal acceptance patterns as implementation evidence only

## Required Deliverables

- complete cut source inventory, identity, consistency, refusal, and reconstruction rules
- `PlannerCut` and `WorldModelView` reconciliation
- Plan family, revision, condition, product, dependency, milestone, judgment, authorization, and successor identities
- several-Tasks-per-Plan cardinality and complete Task boundary
- exact Curation and deferred Execution handoff envelopes
- Agent transition, wait, wake, fence, restart, and local quiescence semantics
- direct product traces and downstream preconditions for `WMR-DD-04`
- exact candidate manifests for independent review recommendations

## Forbidden Changes

- runtime types, fields, APIs, schemas, actors, scheduling, or storage placement
- new language-layer Plan grammar
- a universal milestone or cross-domain status protocol
- Strategy listeners, cursors, or durable progression state
- Execution validation, lowering, Task Network, or returned-observation design
- activation-wide readiness, quiescence, or retirement
- reviewer or Gate Acceptance behavior optimized into the worker deliverables

## Review And Acceptance

Integrated review owner: Codex integrated architecture review lane

Review recommender: dedicated read-only subagent

Review budget: one initial recommendation and one verification recommendation only after accepted corrections

Gate: `WMR-DG-03` revision 1 frozen

Gate Acceptance owner: Codex separate cross-deliverable lane

Gate recommender: a distinct read-only subagent

Acceptance budget: one initial recommendation, one frozen violation set, one program-owner disposition, one bounded remediation cycle, and one verification recommendation

Commit expectation: an accepted Gate Receipt closes through a delivery commit before another slice can activate.

## Stop Conditions

Stop and return to the program owner if any tripwire is crossed, if a source owner cannot supply an exact revision, if product authorization cannot remain separate from Plan judgment, if Execution consumer detail is required to settle the producer envelope, or if a reviewer identifies a gate-definition defect.
