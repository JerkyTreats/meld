# Semantic Linting

Date: 2026-06-06
Status: thought experiment
Scope: principle-aware architecture assessment as a future Meld capability

## Status Note

This document is exploratory.

The views here are not canonical architecture. They should not be treated as accepted governance, implementation direction, or product requirement.

Further research is required before implementation. The idea also depends on a more mature Meld loop than the current system has available.

## Thought Experiment

Semantic linting is the idea that Meld could assess whether code changes preserve architectural meaning, ownership, and boundary contracts.

It is not syntax linting. It is not formatting. It is not a hardcoded checklist.

It asks whether a change still tells the same truth the architecture says it should tell.

For example, the semantic unit preservation rule says a canonical domain product should travel intact until another domain accepts ownership and converts it into its own model. A mechanical tool can detect copied fields, but the semantic question is larger: whether the copied fields are still inside a transport envelope or have crossed into a receiver owned model.

That question requires domain ownership context.

## Meld Framing

The mature Meld framing would treat semantic linting as observation, not authority.

```text
source change
→ semantic analysis capability
→ typed assessment artifact
→ event fact
→ graph and evidence
→ architecture belief revision
→ world model agent judgment
→ execution goal, waiver, or tolerated exception
```

In this framing, a semantic lint pass is a sense organ. It observes possible architecture drift and emits evidence.

The world model would assess that evidence. The agent would decide whether the issue warrants action. Execution would perform the fix or record the chosen operational response.

## Why This Is Not A Normal Linter

Many architecture principles depend on ownership.

The same code shape can be correct or incorrect depending on which domain owns the model.

- An envelope that copies a product field may be duplicated truth.
- A receiver owned command that names the same field may be legitimate translation.
- A compatibility shim may be allowed during migration if characterization and parity tests exist.
- A storage index may duplicate identity for lookup but should not become a second semantic product.

Static scans can detect evidence. They cannot reliably settle ownership.

## Candidate Evidence

Semantic linting could gather evidence from:

- Rust AST scans
- constructor and conversion patterns
- field names shared across contained products
- serialization and storage record shapes
- tests that assert copied field parity
- governance policies
- cognitive architecture docs
- prior assessment artifacts
- compatibility notes
- explicit waivers

Each finding should include confidence, severity, source evidence, suspected ownership boundary, and a proposed next action.

## Candidate Output Shape

A future semantic lint artifact might include:

- principle id
- assessed source root
- source evidence
- ownership context
- confidence score
- severity
- suspected receiver boundary
- remediation options
- compatibility rationale
- needed tests
- unresolved questions

The artifact should be readable by humans and reducible into world model evidence.

## Maturity Requirements

This should not be implemented as a first-class Meld loop until several foundations are more mature.

Needed maturity includes:

- event facts for assessment artifacts
- stable source identity for code and design artifacts
- world model belief families for architecture conformance
- agent subscriptions over architecture concern classes
- execution goals that can request refactors, tests, research, or waivers
- compatibility policy integration
- durable provenance from source evidence to decision

Before that point, semantic linting can exist as a manual or Codex-assisted assessment workflow.

## Research Questions

- What is the minimal evidence schema for an architecture principle finding.
- How should confidence be calibrated across static evidence, design evidence, and test evidence.
- Which principles are suitable for semantic linting first.
- How should waivers expire or become stale.
- How should compatibility migrations distinguish tolerated drift from architectural debt.
- How should semantic lint findings become goals without creating noisy churn.
- Which findings should block development, and which should become belief evidence only.

## Near-Term Use

The useful near-term version is a workflow, not a product feature.

A Codex skill or local command can:

1. read governance and design context
2. inspect source roots
3. apply one principle as a test
4. produce an assessment artifact
5. summarize violations and uncertainty

This gives the team structured evidence without pretending the system can yet enforce the principle autonomously.

## Non Goals

- Replacing Rust compiler checks
- Replacing normal lint tools
- Making architecture decisions from syntax alone
- Treating every duplicated field name as a violation
- Blocking all development on unresolved architectural uncertainty
- Making the current thought experiment canonical

## Related Current Work

- canonical assessment-by-domain skill
- `design/cognitive_architecture/README.md`
- `design/ideas/observe_merge_push.md`
- `design/cognitive_architecture/events/multi_domain_spine.md`
- `design/cognitive_architecture/world_model/README.md`
- `design/cognitive_architecture/execution/README.md`
- `design/cognitive_architecture/meld-lang/README.md`
- `design/plan/semantic_unit_assessment/report.md`
