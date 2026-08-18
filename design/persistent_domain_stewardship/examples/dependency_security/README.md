# Dependency Security Stewardship

Date: 2026-08-18
Status: discovery example aligned to canonical PDS boundary
Scope: persistent evidence-grounded stewardship of dependency vulnerability exposure

## Working Model

A dependency security steward maintains a bounded view of vulnerability exposure for an assigned software or infrastructure scope. It combines exact dependency inventory, advisory knowledge, applicability rules, runtime evidence, and explicit risk policy. When the maintained condition is breached, it evaluates bounded remediation or escalates under independent authority.

The steward does not equate a scanner match with exploitable exposure. It also does not equate no reported finding with safety. Both positive and negative assessments are bounded by exact scope, inventory, source coverage, source revision, policy, and completeness.

This example presses long-lived external runtime behavior, passive advisory advance, multi-source evidence, assignment-scoped cursors, and post-remediation verification.

## Domain Boundary

The dependency-security domain owns:

- dependency and component identity within declared ecosystems
- advisory normalization with preserved source provenance
- applicability, exposure, and coverage policy
- bounded security assessment products
- actionable violation and remediation-feasibility products
- result validation and admission from scanners or external monitors
- the meaning of clean within coverage, violated, stale, unknown, and conflicted

It does not own generic repository mutation, pull requests, deployment, exception approval, world-model beliefs, planning, execution claims, or package routing.

Code changes remain developer or code-change domain effects. A candidate version change is a dependency-security fact, not a command to another steward.

## Standing Condition

For every assigned subject:

```text
dependency inventory is exact and current
and required advisory coverage is exact and current
and assessment posture is clean within declared coverage
or an explicitly authorized risk posture satisfies installed policy
```

Unknown, stale, conflicted, or coverage-insufficient posture cannot satisfy the condition. An exception must retain principal authority, rationale, exact affected scope, and expiry.

## Subjects And Scope

Primary subjects include:

```text
workspace or repository
service or image
dependency manifest and lockfile
resolved component and dependency path
package version
runtime exposure
advisory record
mitigation
risk exception
remediation scope
```

An assignment begins from one bounded subject. Derived component subjects are deterministic from admitted inventory, remain within scope, and retract idempotently when later inventory supersedes them.

## Observations And Evidence

Useful evidence classes include:

- exact dependency inventory and resolution paths
- advisory source snapshots and revisions
- ecosystem coverage and completeness
- runtime reachability or affected configuration
- source conflicts and revised advisories
- mitigation and resolver feasibility
- accepted exception and expiry
- post-change inventory and advisory reassessment
- build, test, and runtime verification

These classes do not substitute for one another. A commit does not prove resolution. A successful build does not prove advisory absence. A scanner exit status does not prove a security posture.

## Bounded Negative Evidence

`CleanWithinCoverage` requires positive evidence for all of the following:

```text
exact assigned subject
exact dependency inventory
exact required advisory source set
exact source revisions
declared ecosystem coverage
source currency satisfied
adapter completeness satisfied
no admitted applicable finding above the policy threshold
```

Unsupported ecosystems, truncated scans, unavailable sources, stale revisions, or incomplete transitive graphs yield unknown, stale, or coverage-insufficient posture.

## Beliefs

At minimum, coverage and security posture remain distinct concerns.

Coverage asks whether the steward has adequate current dependency and advisory knowledge for the assigned scope. Security posture asks what bounded risk state that exact evidence supports under the installed policy.

Applicability, exploitability, remediation confidence, and evidence freshness may remain separate families when they support different decisions or revision schedules.

## Observation Choices

The steward may:

- inspect manifests, lockfiles, images, and resolved graphs
- acquire exact advisory source revisions
- evaluate affected version and configuration rules
- acquire reachability or runtime exposure evidence
- compare conflicting advisory sources
- evaluate candidate remediation feasibility
- reobserve after source advance or workspace change

Elapsed time alone cannot silently mutate a belief. Currency needs an explicit semantic time signal or source-advance fact under installed policy.

## Actions

Candidate interventions include:

- tolerate under an explicit bounded exception
- propose dependency or base-image changes
- propose a feature or configuration mitigation
- request service isolation or operational mitigation
- acquire additional applicability evidence
- escalate when risk exceeds available authority
- verify exposure after an independently authorized change

Repository edits, deployment, service isolation, and exception approval require separate grants and may belong to other domain stewards.

## Outcomes

A resolution assessment cites the prior violation, new dependency inventory, current advisory source revisions, exact policy, change evidence, and required verification.

Successful command execution does not close an episode. Restoration requires a new admitted security assessment that establishes clean within coverage or an explicitly authorized risk posture. Later advisory revision may reopen the maintained condition without a workspace change.

## Authority

Observation may be broad, but external acquisition, repository read access, resolver execution, code mutation, deployment, exception approval, and emergency mitigation are separate grants.

One physical security product may offer all those behaviors. The activation-local capability catalog exposes only exact activation-selected contracts. Agent judgment and execution independently prevent use outside effective authority. Unselected mutation behavior remains unreachable.

## Why PDS

The simpler baseline is a dependency scanner, vulnerability-management platform, and automated update bot. That remains preferable when every finding has deterministic applicability and one mandatory update action.

PDS earns its cost when the domain needs persistent applicability reasoning, explicit evidence coverage, revised advisory handling, selective reobservation, remediation alternatives, exception continuity, cross-objective tradeoffs, and verified outcome history.

## Physical Runtime Variants

| Placement | Useful behavior | Required proof |
| --- | --- | --- |
| linked local adapter | trusted inventory parser or deterministic matcher | workspace scope, resource bound, result admission |
| owned subprocess | local scanner or resolver | executable identity, environment allowlist, filesystem and network grants |
| shared sidecar | reusable advisory index or scanner daemon | tenant namespace, queue fairness, cursor isolation, crash sharing |
| remote request service | hosted scanning or reachability analysis | endpoint and source identity, credential isolation, response correlation |
| persistent external monitor | advisory and portfolio advance while Meld is idle | authenticated callback, assignment subscription, cursor and source revision |

Equivalent placements preserve domain product meaning and lineage obligations. They need not return byte-identical observations from changing external sources.

## Interaction With Other Stewards

An actionable violation is a canonical fact. A developer assignment may independently consume it and decide whether its own maintained condition and authority justify code work. A reliability assignment may independently evaluate operational mitigation. Docs freshness may wake after an admitted dependency change.

No producer grants consumer authority or names another steward as a required next step.

## Explicit Non Commitments

This example does not approve:

- one scanner or advisory provider
- universal ecosystem identity
- mandatory remote isolation
- automatic code mutation or deployment
- a risk acceptance workflow
- scanner success as domain truth
- missing findings as proof of safety
- external hidden planner state as Meld cognition
- a final package or lifecycle schema

## Router Exercise

The package fanout and theory pressure are developed in [Router Specification](router_spec.md).

## Source Grounding

- [CVE Exposure Catalog Entry](../pds_expression_catalog.md)
- [Compilation Layer Span](../compilation_layer_span.md)
- [CVE Freshness Use Case](../../../use_cases/cve_freshness.md)
- [Dependency Security Consumer Design](../../../plan/integration/dependency_security_pds_consumer_design_spec.md)
- [PDS Router Design](../../../plan/integration/pds_router_design_spec.md)
- [Isolation And Runtime Portability](../../isolation_and_runtime_portability.md)
- [Activation Lifecycle Fanout](../../../plan/integration/pds_activation_lifecycle_fanout_exercise.md)
