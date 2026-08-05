# CVE Freshness Strategy

## Purpose

CVE freshness demonstrates that Strategy is not a docs-specific workflow abstraction. The same authority and evidence boundaries apply while the domain meaning and candidate structure differ materially.

The example is explanatory. It does not prescribe one required procedure.

## Maintained condition

An Agent wants a software subject to remain adequately assessed against relevant vulnerability knowledge.

The Goal concerns the relationship between a subject, its dependency and deployment facts, current vulnerability knowledge, and the evidence supporting the assessment. It is not merely a request to run a scanner.

A successful scan can still be incomplete, stale, mis-scoped, or unable to determine whether a reported vulnerability is reachable or applicable.

## Domain meaning

The vulnerability domain can explain concepts such as:

- subject identity and deployed scope
- component and version observations
- vulnerability advisories and their provenance
- applicability, reachability, severity, and remediation meaning
- freshness of vulnerability knowledge
- conflicts between evidence sources
- authority to accept risk or change the subject
- evidence needed to support an assessment

This meaning lets Strategy reason about possible action. It does not require the domain to fix one scanner, provider, batching rule, remediation workflow, or release procedure.

## Strategy question

Given an assessment shortfall, Strategy asks which course of action could produce the evidence or change needed to restore confidence in the maintained condition.

The answer depends on the shortfall. Missing component facts may call for inventory observation. Stale advisory knowledge may call for refreshed sources. Conflicting applicability judgments may call for authoritative resolution. A confirmed applicable vulnerability may call for mitigation or an explicit risk decision.

These are different theories of action, not interchangeable task variants.

## Candidate meaning

One candidate could mean:

```text
Observe the components present in the relevant subject.
Compare them with current authoritative vulnerability knowledge.
Resolve material conflicts or unknown applicability.
Produce evidence that supports an updated assessment.
```

The candidate must preserve the intended response rather than merely authorize any available security tool.

## Authority boundary

Strategy proposes the semantic response and explains its relation to the Goal. The Agent authorizes a candidate and remains responsible for risk-bearing judgment.

Execution realizes authorized assessment or mitigation work within current operational constraints. It does not decide that a different security response is close enough.

Owning evidence domains decide which observations and advisories are admissible, how conflicts are represented, and what the reconciled assessment says.

The Agent decides whether that assessment satisfies the Goal or requires further action.

## Observation boundary

An advisory match is not automatically proof of applicability. A mitigation attempt is not proof that exposure ended. A tool exit status is not an assessment verdict.

Strategy must therefore preserve the route from intended action to authoritative observation:

```text
assessment shortfall
→ candidate theory of action
→ Agent authorization
→ operational realization
→ observations and outcomes
→ evidence admission
→ reconciled vulnerability state
→ Agent satisfaction judgment
```

## Why this example matters

CVE freshness demonstrates that Strategy can support:

- multiple kinds of uncertainty and evidence
- a domain-specific response expressed through generic Strategy boundaries
- nondeterministic consideration with settled replay

Together with docs freshness, it shows that Strategy is organized around domain meaning and authority rather than a single maintenance workflow.

## Read with

- [World Model Strategy](README.md)
- [Strategy Boundary Contracts](contracts.md)
- [Docs Freshness Strategy](docs_freshness.md)
