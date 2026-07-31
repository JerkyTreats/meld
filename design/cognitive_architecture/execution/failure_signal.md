# Failure Signal

Date: 2026-07-31
Status: active
Scope: canonical semantics for failure inside execution — what failure means, who consumes it, and the boundary between retry and belief

## Thesis

Failure is a signal, not only a control-flow event. Static workflows treat a failed check as a reason to retry, because model output is nondeterministic and weaker models often return invalid response types or artifacts. The flywheel keeps that retry discipline and adds a second consumer: belief. A failure that survives retry is evidence about the world — specifically, evidence that the currently selected strategy is off track for its goal.

## The Two-Tier Discipline

- Transient failure belongs to execution alone. A check failure, an invalid artifact, or a retryable provider error resolved within the bounded retry budget must not reach belief. Retries are execution mechanics, not observations about the world.
- Terminal failure belongs to belief. When the retry budget is exhausted and the task fails, that terminal outcome must flow through the ordinary outcome-to-evidence channel so the world model can learn the attempt did not achieve its effect.

The boundary between the two tiers is terminality on the dispatch routes. Exactly one belief-visible outcome is emitted per unit of work, at terminality, regardless of how many attempts execution consumed.

## Gate Outcomes

A gate is an integrity check owned by the workflow layer. Gate verdicts are execution facts and must be durably recorded whichever route evaluates them: a verdict that is computed and discarded is a hidden judgment, and hidden judgments are how empty outputs impersonate finished work. A recorded gate failure feeds the retry tier; it reaches the belief tier only by causing terminal task failure.

Gates check integrity, not quality. Structural requirements such as required fields, non-empty claim sets, and non-empty output are gate territory. Judgments about whether produced content is good ride the epistemic loop, where being wrong is recoverable.

## Invariants

- No belief-visible event is emitted from a non-terminal attempt.
- Every evaluated gate verdict is recorded; none are discarded.
- Terminal failure is mapped into evidence by declared outcome interpretation, never by hardcoded values in execution code.
- Retry budgets are bounded and declared, not implicit.

## Read With

- [Execution Domain](README.md)
- [Task Network](task_network.md)
