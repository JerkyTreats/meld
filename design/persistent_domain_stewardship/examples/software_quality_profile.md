# Software Quality Steward Profile Example

Date: 2026-07-16  
Status: illustrative  
Scope: customer-facing profile over the full software-quality package decomposition

> This long-form profile predates the canonical cognition split. Use the normalized [codebase quality use case](codebase_quality/README.md) and [Canonical Persistent Domain Stewardship](../../cognitive_architecture/persistent_domain_stewardship.md) when ownership language conflicts.

## Purpose

`software_quality.md` demonstrates the expert package and operational-domain-theory view.

This document demonstrates the intended customer abstraction above that package.

It is not a finalized language syntax.

## Customer Intent

A repository owner wants Meld to:

- maintain reliability and persistence safety as primary concerns;
- monitor performance and documentation as secondary concerns;
- run bounded validation automatically;
- create draft branches and pull requests;
- require approval for merge or production action;
- escalate unresolved high-risk breaches;
- remain within a standard compute and model budget.

## Candidate Textual Profile

```text
steward "Meld repository quality"
    using software.quality

scope repository "JerkyTreats/meld"

maintain reliability strict
maintain persistence safety_critical
maintain performance balanced
maintain documentation balanced

prioritize
    reliability
    persistence
    performance
    documentation

autonomy draft
budget standard
verify with pull_request_grade

may
    inspect_repository
    run_targeted_tests
    run_benchmarks
    create_isolated_branch
    open_draft_pull_request

require approval for
    merge_pull_request
    production_deploy

prohibit
    delete_persisted_data
    rewrite_protected_history

escalate
    unresolved_high_risk after 24h
    repeated_failed_intervention after 2 attempts
    authority_required to team "platform-reliability"
```

## Candidate Simplified YAML

```yaml
steward:
  name: Meld repository quality
  type: software.quality

scope:
  repository: JerkyTreats/meld

objectives:
  - reliability:
      sensitivity: strict
      priority: primary
  - persistence:
      sensitivity: safety_critical
      priority: primary
  - performance:
      sensitivity: balanced
      priority: secondary
  - documentation:
      sensitivity: balanced
      priority: secondary

autonomy: draft
budget: standard
verification: pull_request_grade

may:
  - inspect_repository
  - run_targeted_tests
  - run_benchmarks
  - create_isolated_branch
  - open_draft_pull_request

approval_required:
  - merge_pull_request
  - production_deploy

prohibited:
  - delete_persisted_data
  - rewrite_protected_history

escalation:
  unresolved_high_risk: 24h
  repeated_failed_intervention: 2
  authority_required: platform-reliability
```

The two source forms should compile to the same canonical profile.

## Package Expansion

The `software.quality` package may expand these selections into a considerably larger operational representation.

### `reliability strict`

Could resolve to:

- `reliability.test_health` belief family;
- maximum evidence age based on subject criticality;
- breach and restore thresholds;
- three-revision restore stability;
- targeted-test observation method;
- reliability remediation methods;
- task and incident outcome contracts;
- escalation after repeated failed remediation.

### `persistence safety_critical`

Could resolve to:

- migration-safety and recovery-readiness belief families;
- restore and rollback verification requirements;
- low tolerance for missing evidence;
- independent evaluator requirement;
- approval requirements for destructive migration;
- prohibition on unbounded data modification.

### `performance balanced`

Could resolve to:

- benchmark and profile observation actions;
- comparable-environment requirement;
- moderate breach threshold;
- action-cost and value beliefs;
- reliability and maintainability protected floors;
- draft optimization method.

### `documentation balanced`

Could resolve to:

- source-to-document projection;
- `content.freshness` belief;
- existing docs-writer compatibility method;
- verification and artifact outcome mappings.

## Effective Authority

The profile requests `draft` autonomy.

The effective authority remains the intersection of:

```text
package-supported actions
∩ profile request
∩ repository-owner grant
∩ organization policy
∩ current runtime restrictions
```

A semantic preview could show:

```text
Autonomous:
- inspect repository
- run bounded tests
- run budgeted benchmarks
- create isolated branch
- modify files in isolated branch
- open draft pull request

Approval required:
- request broad external changes
- merge pull request

Prohibited:
- production deployment
- protected-history rewrite
- persisted-data deletion
```

## Assignment

One assignment could bind the profile to:

```text
principal:
    repository owner

scope:
    repository JerkyTreats/meld
    default branch master
    protected paths inherited from repository policy

authority grant:
    organization/software-maintainer-draft-v1
```

The same profile could be assigned to another repository without changing the package.

## Activation

A physical activation could bind:

```text
source adapters:
    GitHub installation
    workspace checkout
    CI provider

providers:
    configured model provider

capabilities:
    git branch adapter
    test runner
    benchmark runner
    pull-request adapter

runtime:
    local or hosted Meld supervisor

quotas:
    standard software-quality budget
```

Connector or credential changes should not necessarily change profile intent.

## User-Facing Episode

A performance regression episode may appear as:

```text
Objective:
    Maintain performance within the balanced envelope.

Trigger:
    Comparable benchmark declined by 11% after commit abc123.

Current belief:
    Performance health 0.62, confidence moderate.

Decision:
    Additional profile evidence required before proposing a patch.

Active work:
    Profiling changed allocation paths.

Authority:
    May run profiler and create draft branch.
    May not merge.

Verification:
    Benchmark recovery, tests passing, reliability floor preserved.
```

The display correlates world-model, Agent, execution, and outcome records. It does not replace them.

## Profile Refinement

Example conversational edit:

```text
User:
Performance checks are too expensive. Keep the same safety posture,
but reduce routine benchmark spend by about one third.
```

Candidate semantic mutation:

```text
Budget changes:
- Performance routine benchmark allocation: 90 → 60 minutes/day.

Policy changes:
- Routine benchmark cadence: daily → every 2 days.
- Breach-triggered benchmark budget: unchanged.
- Reliability and persistence budgets: unchanged.

Authority changes:
- None.
```

The edit changes structured profile state only after validation and approval.

## Advanced Override

An expert may need a bounded override:

```text
override performance {
    evidence max_age 48h
    restore after 5 healthy revisions
}
```

The compiler should report the deviation from the selected `balanced` preset.

## Abstraction Test

The profile abstraction succeeds if the repository owner can make normal changes without understanding:

- Bayesian factor weights;
- evidence normalization schemas;
- graph projection routes;
- `meld-lang` composition edges;
- task-network records;
- retry and repair state;
- runtime actor handles.

The package author and runtime remain responsible for those mechanics.
