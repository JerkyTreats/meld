# Failure Signals

Execution distinguishes retryable operational failure from terminal outcome.

A retryable failure remains inside Execution while a bounded retry or recovery policy is active. Attempts and gate verdicts are durably recorded, but no attempt is presented as the final outcome of the Task.

A terminal failure is appended through Events exactly once for the affected execution epoch. The payload preserves Task, Goal attribution, Capability, attempt history, artifacts, observations, and failure classification.

Terminal failure is eligible evidence for the world model. Execution does not interpret that evidence, revise the Strategy Plan, or declare the Goal failed. Agent reconciliation decides the semantic response.

Integrity gates protect executable shape and declared result contracts. Product quality judgments remain with semantic owners and belief admission.
