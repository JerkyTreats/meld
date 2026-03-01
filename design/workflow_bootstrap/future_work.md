# Workflow Bootstrap Future Work Backlog

Date: 2026-03-01
Status: active backlog

## Intent

Capture exploration and hardening work that should start after bootstrap feature completion.
This backlog is intentionally outside current milestone scope.

## Entry Criteria

Start this backlog after all bootstrap completion criteria are met in [Workflow Bootstrap Roadmap](README.md).

## Metadata And Prompt Context

- artifact encryption policy with key lifecycle and rotation model
- policy driven retention windows for prompt and context artifacts
- pluggable CAS storage backends beyond local filesystem
- metadata growth telemetry for per key and per frame budgets
- compaction jobs for metadata heavy records and expired artifacts
- cross domain metadata governance for tree store context and agent metadata domains

## Workflow Runtime

- policy profiles for workflow families
- cross workflow activation graph for chained workflow execution
- richer retry and branching controls with deterministic replay guarantees
- stronger gate diagnostics model for fast failure triage
- workflow migration strategy for profile version upgrades
- many workflows per agent with explicit selection policy
- conditional branching turns for profile defined decision points
- parallel turn segments where turn dependencies allow safe fan out
- profile inheritance and composition for shared workflow primitives
- configurable gate thresholds by profile
- optional branch path for low confidence verification outcomes
- expanded final output targets beyond `README.md`

## Security And Exposure

- privileged prompt and context query authorization model
- default redaction profile presets by workflow class
- audit trail model for privileged artifact reads
- log sink contracts that prevent sensitive payload emission

## Validation And Testing

- large scale workload tests for artifact and metadata growth behavior
- long run durability tests for thread and turn resume correctness
- fault injection tests for digest mismatch and budget rejection paths
- conformance suite for workflow profile schema validation

## Governance

- each accepted item must be linked to a concrete spec under the owning workload folder
- each item should define owner exit criteria and verification gates before implementation
- items that expand current milestone scope must remain here until roadmap revision
- retention policy work should define differential windows by workflow profile
