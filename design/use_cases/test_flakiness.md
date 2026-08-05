# Use Case: Test Flakiness

Date: 2026-07-24
Status: proposed
Scope: keep per-test reliability above threshold across a continuously changing suite

## Use case

A test suite runs continuously against a changing codebase. Some tests fail intermittently: the same code passes one run and fails the next. Flaky tests poison the signal of the whole suite — real regressions hide behind expected noise, and engineers learn to rerun rather than investigate. A team wants every test's reliability held above a threshold, wants flakiness detected from run history rather than anecdote, and wants principled responses: quarantine the test, diagnose and fix the root cause, delete it, or explicitly tolerate it — each a different trade of cost against signal.

Two properties shape the work. First, flakiness is statistical: no single run proves anything, and confidence in a flakiness estimate grows only with samples, which cost CI time and money. Second, the response is genuinely branched: whether to quarantine or diagnose depends on what diagnosis reveals, which is not knowable when work begins.

The deterministic baseline is a flaky-test detector that auto-quarantines on a failure-rate trigger. It does not diagnose, does not weigh a test's evidence value against its noise cost, and does not reopen the question when the code under test changes.

## Meld semantics conversion

This conversion is fluid and has not been worked at canonical depth.

- Subjects are tests, test runs, suites, and commits; relations are contains, executes, and affects.
- The maintained condition holds each test's flakiness posterior below threshold at a declared confidence.
- Flakiness is an observational dimension whose settlement is statistical: the belief family declares the settlement criterion — minimum samples or credible-interval width — and settled means the comparator reports confidence above that criterion, mapping to the existing settled belief status.
- The settling action is run test N times. Choosing N remains a domain-policy question outside the minimal Strategy slice.
- Alternatives diverge in value posture, not just path: quarantine settles suite reliability cheaply but abandons the test's evidence value; diagnose-and-fix is expensive and preserves it; retry-wrapping masks the question and a sound value posture rejects it.
- Diagnosis is not encoded as in-plan branching. A diagnosis candidate is a bounded observation plan; its outcome reconciles; the next bounded attempt constructs the fix from revised beliefs. The convergence loop absorbs what a workflow would express as conditionals.

## Required semantics

For this use case to be usable on paper, the following must hold. Standing is given per entry.

1. **Family-declared settlement criteria.** Settlement of a statistical dimension must be definable by the owning family as a confidence condition over accumulated samples, not as a single evaluation event. How Strategy consumes that meaning remains open.
2. **Repeated-observation affordances with sample budgets.** An affordance must express acquire N more samples with cost proportional to N, and construction must consume the budget as a typed capacity fact. New affordance shape; fits the fit-verdict pattern.
3. **Convergence in place of conditionals.** Successive bounded attempts over reconciled outcomes are a design hypothesis, not current Strategy behavior.
4. **Value-posture-sensitive ranking.** Alternatives with equal feasibility but opposed value consequences may require Agent posture to distinguish them. This is a design hypothesis outside the minimal Strategy slice.
5. **Invalidation on code change.** A commit touching the code under test must invalidate settled flakiness beliefs for affected tests. Requires the affects relation projected into frames; same enrichment delta as other domains.

## Read with

- [Use Case Catalog](README.md)
- [World Model Strategy](../cognitive_architecture/world_model/strategy/README.md)
- [Goal Curation](../cognitive_architecture/world_model/agent/goal_curation.md)
