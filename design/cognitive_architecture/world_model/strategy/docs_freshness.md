# Docs Freshness Strategy

## Purpose

Docs freshness illustrates how Strategy connects an observational maintenance Goal to meaningful action without turning domain theory into a fixed workflow.

The example is explanatory. It does not prescribe one required procedure.

## Maintained condition

An Agent wants documentation to remain an adequate account of the code and design it describes.

The Goal concerns observed correspondence between documentation and its subject. It is not merely a request to run a writer or create a file.

This distinction matters because a completed writing task does not prove that documentation is accurate, complete, or fresh.

## Domain meaning

The docs domain can explain what freshness means through concepts such as:

- the subject covered by a document
- observations of relevant source and design changes
- claims made by existing documentation
- evidence that a document accounts for its subject
- omissions, contradictions, and stale claims
- authority over publication targets
- meaningful relationships between parent and child documentation

This meaning enables Strategy to reason about the Goal. It need not dictate a traversal algorithm, package topology, provider, prompt, or publication procedure.

## Strategy question

Given an observed freshness shortfall, Strategy asks which course of action could produce evidence that the documentation again accounts for its subject.

A candidate might involve reviewing an affected scope, revising selected documents, checking relationships between summaries and detailed material, or deliberately taking no action when the observation is too weak.

The candidate explains why each action belongs and which observable result would support the maintained condition.

## Candidate meaning

One candidate could mean:

```text
Inspect the changed subject and its current documentation.
Revise the documents whose claims no longer match that subject.
Produce an observable publication outcome.
Let the docs evidence domain assess the new correspondence.
```

## Authority boundary

Strategy may propose which documentation meaning should change and why. The Agent decides whether to authorize that proposal.

Execution determines how authorized work is realized against the available workspace, capabilities, and publication mechanics. It may reject work that cannot be performed without changing the authorized meaning.

The docs evidence domain decides whether publication outcomes constitute admissible evidence and whether the documentation now accounts for its subject.

The Agent decides whether reconciled evidence satisfies the Goal.

## Observation boundary

The Strategy may expect that revising and publishing documentation will improve freshness. That expectation justifies action. It does not assert the resulting freshness value.

The flow remains:

```text
observed shortfall
→ candidate theory of action
→ Agent authorization
→ operational realization
→ publication outcome
→ evidence admission
→ reconciled docs state
→ Agent satisfaction judgment
```

Skipping the evidence and reconciliation steps would collapse intended effect into observed truth.

## Why this example matters

Docs freshness demonstrates four enduring Strategy properties:

- the Goal is observational rather than task-shaped
- domain theory supplies meaning without prescribing procedure
- Strategy explains the route from action to prospective evidence
- successful execution remains separate from evidence and satisfaction

## Read with

- [World Model Strategy](README.md)
- [Strategy Boundary Contracts](contracts.md)
- [CVE Freshness Strategy](cve_freshness.md)
