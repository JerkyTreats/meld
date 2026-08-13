# Standing Maintained Condition Completion Evidence

Date: 2026-08-13
Status: complete
Parent program: [Theory Elevation Program](theory_elevation_program.md)
Implementation design: [Standing Maintained Condition Implementation Design](standing_maintained_condition_implementation_design.md)
Domain assessment: [Standing Maintained Condition Assessment By Domain](standing_maintained_condition_domain_assessment.md)

## Outcome

Theory Elevation Step 3 is implemented as one complete vertical. Standing responsibility is an Agent-owned maintained-condition body with append-only exact revisions. Complete stewardship receipts select that revision, initialization binds it to the Agent, runtime assembly activates the exact body, and curation derives transient Goals from its evaluation.

The docs expression publishes `maintained_condition.docs_freshness.json`. Its condition identity, belief dimension, desired comparison, Goal priority, and desired summary are no longer reconstructed by root runtime code. Older threshold-only records remain executable through an explicit compatibility lowering path.

## Acceptance Evidence

| Criterion | Evidence |
| --- | --- |
| First-class owner contract | `AgentMaintainedCondition` and `AgentMaintainedConditionBinding` live under the world-model Agent domain |
| Durable exact revision | `AgentMaintainedConditionRegistryStore` preserves unchanged installation identity and resolves historical content hashes |
| Complete receipt | `TheoryInstallationReceipt` pins the maintained-condition revision and exact resolution rejects absent or inconsistent owner revisions |
| Owner curation | Agent curation grounds the selected condition against the Agent subject and evaluates it through the planner world state |
| Causal Goal provenance | `GoalSource::MaintainedConditionBreach` and the exact dedupe key name the causing condition |
| Zero or more transient Goals | focused tests prove held, breached, repeated active, restored, and later-drift lifecycle behavior |
| Compatibility parity | threshold-only Agent fixtures remain green through compatibility lowering |
| Runtime parity | runtime CLI and workspace integration tests preserve exact receipt, Strategy authorization, publication, and convergence paths |

## Lifecycle Proof

The exact-condition tests establish the maturity boundary directly:

```text
held condition
  -> zero Goal commands

breached condition without an open Goal
  -> one proposed Goal with maintained-condition provenance

same breach with a matching active Goal
  -> zero duplicate Goal commands

restored condition
  -> existing satisfaction mutation

later breach
  -> existing reopen mutation
```

The maintained condition remains installed across every Goal lifecycle transition. No scheduler, temporal stability policy, multiple-condition model, or authority system was added.

## Cross-Domain Coherence

Meld world model owns the condition body, revision, grounding, evaluation, and curation. Meld lang owns the causal Goal provenance shape. Config selects identity. Docs publishes data. Initialization and runtime install, resolve, and route exact owner revisions. Execution persists the resulting Goal without interpreting maintained-condition semantics.

This preserves the assessment boundary and avoids a second desired-state owner. Exact runtime behavior reads the maintained condition. Curation-rule threshold fields survive only for stored-record and fixture compatibility.

## Verification

The completed gate requires these commands to pass from the repository root:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
git diff --check
```
