# Agent Runtime Surface

Agent is a durable participant in one activation generation.

Its inputs are Event cursors for relevant belief revisions, Goal lifecycle facts, Curation results, execution outcomes, and activation control. Its outputs are durable Agent decisions, Strategy construction requests, Curation authorizations, Task admissions, and Goal lifecycle mutations.

The participant reports waits against exact durable conditions. A wait names the producer, product identity, cursor or query condition, activation generation, and wake policy. Process polling is not the liveness contract.

Restart resumes from durable Agent decisions and input cursors. It cannot duplicate product authorization or lose the distinction between publication and completion.
