# Multi-Domain Event Spine

One Event spine connects independent domain owners without making them one domain.

## Publication Families

Sensory owners publish observations. Graph owners publish admitted objects and relation occurrences. Belief publishes revisions. Curation publishes epistemic operation results. Agent publishes decisions and authorizations. Execution publishes progress and outcomes. PDS and lifecycle owners publish activation facts.

Each family retains producer authority, schema identity, causal lineage, perspective where relevant, and activation generation.

## Consumption

Each consumer owns a durable cursor or revision query. It commits its state change and cursor consistently, tolerates duplicate delivery, reports exact waits, and rejects records fenced by another activation generation.

## Liveness

Producer publication exposes a visibility milestone. Consumer wait registration names that milestone. Wake routing connects the two durably. Runtime tick order is irrelevant to correctness.
