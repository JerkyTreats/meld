# Software Quality Stewardship Example

Date: 2026-07-14  
Status: illustrative  
Scope: worked decomposition of a multi-perspective codebase stewardship package

> This long-form source case predates the canonical cognition split. Use the normalized [use case](codebase_quality/README.md), [router specification](codebase_quality/router_spec.md), and [Canonical Persistent Domain Stewardship](../../cognitive_architecture/persistent_domain_stewardship.md) when ownership language conflicts.

## Purpose

This example tests whether Persistent Domain Stewardship can express a codebase curator without collapsing into one large workflow.

The package maintains several quality concerns over an indefinitely changing repository:

- persistence safety
- performance
- reliability
- usability
- maintainability
- documentation freshness

The example is intentionally multi-perspective. The same code change can improve one concern while degrading another.

The package must therefore express:

- shared observations and beliefs
- perspective-specific objectives
- selective evidence acquisition
- conflicting proposed interventions
- bounded authority
- verification after action
- repeated stewardship episodes over the same subjects

This is a design example, not a finalized package schema.

## Domain Boundary

### In scope

- one Git repository or workspace
- files, modules, packages, APIs, tests, benchmarks, schemas, migrations, documents, and user-facing surfaces
- source and generated artifacts
- local and CI execution evidence
- repository change history
- issue and incident evidence when explicitly connected
- draft branches and pull requests produced by the steward

### Out of scope

- production deployment control
- organization-wide project prioritization
- employee performance assessment
- unrestricted modification of external repositories
- merging changes without an explicit grant
- claims that cannot be grounded in declared evidence channels

### Principal

The package is assigned by a repository owner or delegated maintainer.

The principal grants:

- subject scope
- branch scope
- permitted evidence sources
- compute and model budgets
- action authority
- approval requirements

## Object Model

Principal object types:

```text
software.repository
software.workspace_node
software.file
software.module
software.package
software.symbol
software.api_surface
software.persistence_boundary
software.schema
software.migration
software.test
software.test_suite
software.benchmark
software.user_flow
software.document
software.change
software.branch
software.pull_request
software.incident
```

Representative relations:

```text
repository contains workspace_node
workspace_node parent_of workspace_node
file defines symbol
module contains file
module depends_on module
api_surface implemented_by symbol
api_surface consumed_by module
persistence_boundary uses schema
migration transforms schema
unit_test covers symbol
integration_test covers user_flow
benchmark evaluates symbol
benchmark evaluates user_flow
document describes module
document describes api_surface
change modifies file
pull_request contains change
incident implicates module
```

The graph represents current anchors and historical lineage. It does not itself determine whether a module is safe, performant, usable, or documented.

## Observation Modules

## Workspace observations

Promoted events:

```text
workspace.node_added
workspace.node_changed
workspace.node_removed
workspace.relationship_changed
```

These events update current content anchors and provide source-churn evidence.

## Git observations

Promoted events:

```text
git.commit_observed
git.branch_head_changed
git.diff_materialized
git.pull_request_opened
git.pull_request_reviewed
git.pull_request_merged
git.pull_request_closed
```

Git events provide provenance and lifecycle evidence. A merged pull request is not proof that its intended quality outcome occurred.

## Test observations

Promoted events:

```text
test.run_completed
test.case_failed
test.case_flaked
test.coverage_measured
test.duration_measured
```

Test evidence includes:

- exact revision
- environment
- test selector
- result schema
- duration
- retries
- artifact references

## Performance observations

Promoted events:

```text
benchmark.run_completed
profile.capture_completed
resource.measurement_completed
```

Benchmark evidence includes baseline identity and comparability metadata. Measurements from incompatible environments do not silently enter the same comparator.

## Persistence observations

Promoted events:

```text
schema.changed
migration.validated
restore_test.completed
compatibility_check.completed
```

## Usability observations

Possible promoted events:

```text
accessibility.scan_completed
user_flow.test_completed
support.issue_classified
usage.friction_signal_observed
```

Usability evidence is weaker and more heterogeneous than compiler, test, or benchmark evidence. The package records that difference through source trust and uncertainty.

## Documentation observations

Promoted events:

```text
document.generated
document.verified
document.accepted
source_document_relationship_changed
```

A generated artifact is evidence of availability. Verification is separate evidence of correctness or freshness.

## Belief Families

The package declares shared epistemic families. Charters bind those families to different objectives.

## `reliability.test_health`

Question:

```text
How likely is the selected subject to satisfy its expected behavior under the tested conditions?
```

Evidence:

- test results
- flake history
- coverage relevance
- incident escapes
- change-to-test dependency

Planner projection:

```text
Holds(subject, reliability.test_health, probability)
Holds(subject, reliability.evidence_freshness, duration)
```

## `persistence.migration_safety`

Question:

```text
How strongly does current evidence support that persisted state can be upgraded, rolled back, restored, and interpreted correctly?
```

Evidence:

- schema diff
- migration tests
- restore tests
- backward-compatibility checks
- production-like fixture coverage
- prior migration incidents

## `performance.health`

Question:

```text
How likely is the subject to meet its declared performance envelope under comparable workload and environment?
```

Evidence:

- benchmark results
- profiles
- resource measurements
- workload comparability
- regression history

## `usability.health`

Question:

```text
How strongly does available evidence support that the selected user flow remains understandable, accessible, and efficient for its intended audience?
```

Evidence:

- accessibility scans
- structured user-flow tests
- support issue clusters
- human evaluation
- observed interaction friction

The posterior is explicitly less mechanically grounded than test health.

## `maintainability.health`

Question:

```text
How strongly does current structural evidence support that the subject can be changed safely and understood at acceptable cost?
```

Evidence:

- dependency fan-in and fan-out
- complexity
- ownership
- duplication
- churn
- review effort
- defect history

## `content.freshness`

Question:

```text
How strongly does current evidence support that documentation still describes the selected subject accurately for its declared audience?
```

Evidence:

- source changes
- document lineage
- generated evidence maps
- verification reports
- human acceptance

## `action.cost`

Question:

```text
What is the expected cost distribution for an action class on the selected subject?
```

Evidence:

- prior execution duration
- model and compute use
- review effort
- failure and retry history
- cleanup cost

## `action.value`

Question:

```text
What downstream value has historically followed this action class for comparable subjects and regimes?
```

Evidence:

- accepted changes
- restored quality beliefs
- reduced recurrence
- user outcomes
- later reverts

This belief should remain conservative until sufficient prospective outcome history exists.

## Steward Charters

The example declares separate charters rather than one universal software-quality Agent.

## Reliability steward

Perspective:

- trusts deterministic test and incident evidence strongly
- tolerates temporary missing evidence less after high-risk changes
- values failure prevention and diagnosability

Standing objectives:

```text
maintain test_health above 0.95
maintain evidence freshness within risk-dependent window
maintain recovery and failure-path coverage for critical modules
```

Candidate actions:

- select and run tests
- generate targeted tests
- investigate flakes
- propose error-handling changes
- request human review

## Persistence steward

Perspective:

- gives high weight to restore and compatibility evidence
- treats destructive migrations as high-cost actions
- requires independent verification for irreversible changes

Standing objectives:

```text
maintain migration_safety above 0.98 for persisted-state changes
maintain tested rollback or compensation path
maintain current recovery evidence
```

Candidate actions:

- inspect schema impact
- generate migration fixtures
- run upgrade and rollback tests
- propose migration changes
- block or escalate unsafe changes

## Performance steward

Perspective:

- trusts comparable measured benchmarks
- discounts unversioned anecdotal performance reports
- accepts some regression when explicitly traded for another declared objective

Standing objectives:

```text
maintain declared latency and resource envelopes
maintain recent comparable benchmark evidence
avoid optimization that breaches reliability or maintainability floors
```

Candidate actions:

- run targeted benchmark
- collect profile
- bisect regression
- propose optimization branch
- recommend tolerance when action cost exceeds expected value

## Usability steward

Perspective:

- admits human and behavioral evidence
- represents higher uncertainty explicitly
- requires approval for broad user-facing changes

Standing objectives:

```text
maintain accessibility requirements
maintain critical user-flow completion conditions
investigate recurring friction evidence
```

Candidate actions:

- run accessibility scan
- generate or update user-flow test
- propose interface revision
- request human evaluation

## Documentation steward

Perspective:

- trusts source lineage and verification reports
- treats generation as provisional until verified

Standing objective:

```text
maintain content_freshness above restore threshold
with evidence no older than configured maximum age
```

Candidate actions:

- gather evidence
- verify existing documentation
- generate content
- open draft change

## Assignment Shape

A repository can instantiate several assignments:

```text
reliability steward
    scope: repository or selected critical modules

persistence steward
    scope: persistence boundaries and migrations

performance steward
    scope: benchmarked modules and user flows

usability steward
    scope: declared user-facing surfaces

documentation steward
    scope: workspace subtree
```

Assignments may share observations and belief revisions while maintaining different objectives and authority.

## Standing Objectives And Episodes

Example performance objective:

```yaml
objective:
  id: maintain-latency-envelope

  breach:
    any:
      - holds:
          subject: $subject
          dimension: performance.health
          condition:
            below: 0.75
      - holds:
          subject: $subject
          dimension: performance.evidence_freshness
          condition:
            above_duration: 7d

  restore:
    all:
      - holds:
          subject: $subject
          dimension: performance.health
          condition:
            above: 0.90
      - holds:
          subject: $subject
          dimension: reliability.test_health
          condition:
            above: 0.95

  stability_window:
    revisions: 3
```

Possible episode:

```text
benchmark regression observed
    ↓
performance belief enters breach region
    ↓
episode opened
    ↓
planner sees insufficient causal localization
    ↓
observation goal: profile changed paths
    ↓
profile evidence identifies allocator hotspot
    ↓
intervention goal: reduce allocation
    ↓
optimization method creates branch and patch
    ↓
benchmark improves, but maintainability belief drops
    ↓
performance steward proposes acceptance
maintainability steward proposes modification
    ↓
conflict requires additional method or approval
    ↓
revised patch restores both required floors
    ↓
verification window completes
    ↓
episode restored
```

The standing performance objective remains active after closure.

## Known Methods

Strategy may admit known Methods separately rather than require unrestricted search.

## `software.run_targeted_validation`

Trigger:

```text
missing or stale evidence for selected quality dimensions
```

Composition:

```text
select relevant tests and benchmarks
→ execute independent validation in parallel
→ normalize reports
→ publish promoted observations
```

## `software.investigate_regression`

Trigger:

```text
quality belief breached and cause unresolved
```

Possible composition:

```text
materialize candidate change range
→ bisect or compare revisions
→ collect focused profile or failure trace
→ publish causal evidence candidate
```

## `software.propose_bounded_patch`

Trigger:

```text
actionable hypothesis exists
and branch-write authority is present
```

Composition:

```text
create isolated branch
→ modify bounded subject set
→ run required validation
→ open draft pull request
```

Expected effects are planning predictions only.

## `documentation.bottom_up_generate`

This method can initially wrap the existing docs-writer workflow.

Composition:

```text
traverse selected subtree bottom-up
→ gather evidence
→ verify current content
→ generate structured document
→ refine style
→ persist generated artifact
```

The outcome contract determines whether freshness is restored.

## Authority Model

Illustrative grants:

| Action | Authority |
|---|---|
| inspect repository and history | Observe |
| run bounded local tests | ExecuteReversible |
| run budgeted benchmarks | ExecuteBounded |
| create isolated branch | ExecuteReversible |
| modify files in isolated branch | Draft |
| open draft pull request | Draft |
| request reviewer | Recommend |
| merge pull request | approval required |
| deploy to production | outside package scope |
| delete persisted data | Prohibited |

Effective authority remains the intersection of assignment request, principal grant, runtime policy, and current restrictions.

## Outcome Contracts

## Documentation restoration

Mechanical completion:

```text
context frame or README artifact written
```

Verification observations:

- artifact schema valid
- evidence map covers declared subject scope
- verification report passes
- subsequent `content.freshness` belief enters restore region

Failure:

- artifact exists but contradicts source evidence
- verification evidence is stale
- required subject coverage is missing

## Performance optimization

Mechanical completion:

```text
patch and benchmark report produced
```

Verification observations:

- benchmark comparable to baseline
- required tests pass
- resource regression absent
- maintainability and reliability floors remain satisfied

Success:

```text
performance health restored
and protected concern floors preserved
```

Harmful outcome:

```text
performance improved
but reliability or persistence objective breached
```

## Reliability remediation

Verification observations:

- targeted failure reproduced before change
- failure absent after change
- relevant regression tests pass
- later CI or incident window contains no contradictory evidence

Task success alone does not close the episode.

## Conflict Model

Stewards may propose incompatible changes.

Representative conflicts:

```text
performance optimization
    vs maintainability complexity

persistence safety
    vs delivery speed

usability simplification
    vs API compatibility

documentation regeneration
    vs source instability during active refactor
```

The first slice should not attempt a complete multi-Agent social-choice system.

Minimum conflict handling:

- detect overlapping subject and effect claims
- expose which objectives improve or degrade
- preserve independent evidence and rationale
- avoid simultaneous incompatible dispatch
- request a combined method or human decision

A shared task network may deduplicate common observation work without deciding normative priority.

## Endless-Horizon Context

The steward does not retain one growing prompt.

It maintains:

```text
unbounded event and decision history
+ current graph and belief projections
+ bounded per-decision context hydration
```

For a new episode, context selection may include:

- current subject anchors
- relevant dependency neighborhood
- recent belief revisions
- prior similar episodes
- accepted and reverted interventions
- active goals and conflicting assignments
- exact package and method versions

Historical material remains addressable without occupying every active model context.

## Self-Improvement Boundary

This package supports continuous self-curation, not unrestricted recursive self-modification.

The codebase may improve through repeated bounded episodes:

```text
observe
→ identify justified improvement
→ create isolated change
→ verify independently
→ obtain required approval
→ observe downstream outcomes
→ update future action beliefs
```

The package, compiler, governance policy, protected invariants, and effective authority are outside ordinary code-improvement authority unless separately granted.

The steward must not silently modify:

- its own authority policy
- package validation rules
- protected evaluation fixtures
- outcome definitions used to judge its change
- trusted sensor or comparator implementations

Those changes require separate stewardship scope and approval.

## Scenarios

## Scenario 1: Tolerated performance movement

Given:

- latency worsens by 1%
- benchmark variance includes the change
- action cost is high
- no protected objective is breached

Expect:

- belief revision recorded
- no episode or a tolerated episode
- no patch generated
- optional future remeasurement scheduled

## Scenario 2: Confirmed regression

Given:

- comparable benchmark regression of 12%
- evidence freshness current
- recent change range known

Expect:

- performance episode opens
- investigation method selected
- bounded profile or bisect work dispatched
- intervention deferred until actionable evidence exists

## Scenario 3: Optimization harms reliability

Given:

- generated patch restores performance
- a concurrency test fails

Expect:

- performance expected effect recorded
- reliability belief enters breach region
- performance episode does not close
- merge action prohibited or approval denied
- repair or alternate method considered

## Scenario 4: Documentation generated but unverified

Given:

- docs-writer method completes
- artifact is persisted
- verification report is missing

Expect:

- task succeeds mechanically
- documentation episode enters `Verifying`
- `content.freshness` remains indeterminate or breached
- no restoration event emitted

## Scenario 5: Persistence action denied

Given:

- migration safety breached
- proposed method requires destructive fixture reset
- assignment lacks authority

Expect:

- no destructive task dispatched
- escalation or approval request emitted
- episode remains open

## Scenario 6: External restoration

Given:

- steward has an active documentation goal
- a human merges a correct documentation update
- new evidence restores freshness

Expect:

- active redundant work cancelled or pruned where safe
- external action receives provenance
- episode closes without requiring steward-authored change

## Scenario 7: Concurrent repository change

Given:

- optimization branch is active
- main branch changes overlapping code

Expect:

- affected preconditions become indeterminate or unsatisfied
- completed unaffected validation is preserved
- stale patch work is suspended or repaired
- switching cost informs replan rather than unconditional restart

## Illustrative Package Fragment

```yaml
package:
  id: software.quality-stewards
  version: 0.1.0

imports:
  - package: software.core
    version: "^1"
  - package: software.git-observations
    version: "^1"
  - package: software.validation-actions
    version: "^1"

charters:
  - id: performance-curator
    scope:
      root_type: software.module
      parameters:
        - module_ref

    perspective:
      trust_profile: measured-performance

    concerns:
      - id: maintain-performance
        belief_family: performance.health
        objective:
          breach:
            holds:
              subject: $module_ref
              dimension: performance.health
              condition:
                below: 0.75
          restore:
            all:
              - holds:
                  subject: $module_ref
                  dimension: performance.health
                  condition:
                    above: 0.90
              - holds:
                  subject: $module_ref
                  dimension: reliability.test_health
                  condition:
                    above: 0.95

        methods:
          - software.run_targeted_validation
          - software.investigate_regression
          - software.propose_bounded_patch

    authority:
      autonomous:
        - benchmark.run
        - profiler.run
        - git.create_branch
        - pull_request.open_draft
      approval_required:
        - pull_request.merge
```

## What This Example Tests

The example supports the stewardship thesis only if it can be represented without adding software-specific behavior to the runtime.

The package should provide:

- software vocabulary
- observation routes
- belief families
- charters
- methods
- outcome contracts
- governance

The runtime should remain responsible for:

- event ordering and replay
- graph and belief maintenance
- Agent perspective
- objective evaluation and goal mutation
- planning and task-network execution
- capability dispatch
- authority enforcement
- outcome-event routing

## Simpler Baseline And Acceptance Test

The baseline is a combination of CI, linters, benchmark alerts, documentation generation, and scheduled coding agents.

Persistent stewardship earns its complexity only if it improves measurable outcomes such as:

- fewer unnecessary remediation changes
- fewer stale or contradictory patches
- lower validation cost at equal defect detection
- higher accepted-change rate
- better preservation of work during concurrent changes
- more accurate abstention under weak evidence
- fewer cross-quality regressions
- reproducible explanations for actions
- improved restoration time for persistent objectives

If those gains do not appear, the package should collapse toward simpler workflows and controllers.
