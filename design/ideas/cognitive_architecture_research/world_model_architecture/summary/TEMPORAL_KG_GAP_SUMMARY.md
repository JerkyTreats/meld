# Temporal KG Gap Summary

Source report: [Temporal Knowledge Graph Design Lessons for a Belief-Enhanced Event-Sourced Fact Graph](../pdf/Temporal%20Knowledge%20Graph%20Design%20Lessons%20for%20a%20Belief-Enhanced%20Event-Sourced%20Fact%20Graph.pdf)

Prompt source: [Temporal Knowledge Graphs and Temporal Reasoning](../../research_prompts.md#4-temporal-knowledge-graphs-and-temporal-reasoning)

## Definitions

- **Temporal knowledge graph (TKG):** A knowledge graph whose facts or events are indexed by time. In the surveyed literature, this often means scoring temporal quadruples such as `(subject, relation, object, time)`.
- **Event spine:** Meld's append-only record of semantic events. It preserves what happened, in what order, and with what provenance.
- **Anchor:** A materialized graph pointer from a subject to a target under a perspective. Anchors are the world model's current or interval-valid state derived from events.
- **Current anchor:** The anchor state considered current for a chosen reference time and transaction time. It should not mean only "latest sequence number."
- **Valid time:** The time when a fact is true in the modeled world.
- **Transaction time:** The time when the system learned, stored, corrected, or superseded a fact.
- **Reference time:** The time from which a query asks "what is current" or "what was true then."
- **Bitemporal query:** A query that uses both valid time and transaction time, such as "what did we believe was true on Tuesday, given what we had ingested by Wednesday?"
- **Validity interval:** The time span over which a fact or anchor is treated as true. Open-ended intervals represent facts that remain valid until an explicit ending, supersession, or contradiction arrives.
- **Coalescing:** Merging adjacent intervals that represent the same state so the graph has one maximal interval instead of many redundant slices.
- **Supersession:** An explicit replacement relationship where a later event or anchor revision takes over from an earlier state while preserving lineage.
- **Invalidation:** An explicit event or rule that closes a fact's validity, marks it as no longer holding, or proves it cannot coexist with another fact.
- **Temporal decay:** A belief-layer function that lowers freshness, confidence, or relevance as time passes. Decay is not the same as invalidation.
- **Stale belief:** A belief whose supporting evidence is aging or overdue for refresh, but has not been contradicted.
- **Contradicted belief:** A belief that conflicts with explicit counterevidence, a superseding event, an invalidation event, or a local completeness rule.
- **Temporal embedding model:** A learned model, such as TComplEx, TimePlex, TeRo, or Temporal TransE, that scores or predicts time-indexed graph facts. In this architecture, these models are optional scoring tools, not truth storage.

## Prompt Gap

The prompt targeted this gap:

> The current system has a solid temporal fact graph implementation but has not drawn from the temporal knowledge graph research community, which has developed sophisticated representations for time-varying knowledge and temporal reasoning.

The research question was not whether the current graph should be replaced by a temporal KG embedding model. It was whether the temporal KG literature changes the semantics of the existing event spine, current-anchor materialization, anchor lineage, and future belief layer.

## Answer To The Gap

The report answers the gap by validating the current layered architecture, while sharpening the semantics each layer should own.

The strongest conclusion is that temporal KG embedding models are useful references for scoring, interval inference, forecasting, and temporal query design, but they are not a replacement for event-sourced truth maintenance. Most temporal KG models score `(subject, relation, object, time)` plausibility. They do not natively preserve append-only revision history, explicit invalidation events, provenance lineage, or a clean distinction between stale and contradicted beliefs.

For this architecture, the durable answer is:

- Keep the event spine as the immutable record of what happened.
- Keep the graph or anchor layer as the materialized state of what currently, historically, or as-of-a-time holds.
- Keep belief as a separate layer for confidence, freshness, calibration, contradiction, and revision.

That separation is the main bridge from the temporal KG literature back into the Meld world model.

## Prompt Questions Mapped To Report Answers

### Temporal Validity In Embedding Models

The report finds that TComplEx, TNTComplEx, Temporal TransE, TimePlex, and TeRo mostly represent temporal validity through timestamp modulation, interval scoring, or boundary-aware embeddings. TeRo is the closest match to explicit state boundaries because it handles relation starts and ends separately. TimePlex is useful for interval prediction. TComplEx and TNTComplEx fit snapshot scoring. Temporal TransE fits only after flattening events or intervals into timestamped facts.

Answer: these models can inform ranking and interval inference, but none gives the system native event-based invalidation or auditable truth revision. The append-only spine remains the right foundation for truth changes.

### Semantics Of Current

The report clarifies that "current" is underspecified unless the time axis is named. The system needs at least valid time, transaction time, and reference time.

Answer: a current anchor should mean "valid at reference time, given all events ingested through transaction time," not simply "latest sequence number." The report's compact semantics are:

```text
current(anchor, reference_time, transaction_time) =
  the maximal coalesced anchor state whose valid interval contains reference_time,
  computed from all events with transaction time <= transaction_time
```

This lets the graph distinguish current-as-known-now, current-as-believed-then, late corrections, historical replay, and mixed granularity.

### Event Facts Versus Persistent Facts

The report maps ICEWS and GDELT to the event spine because they model timestamped episodic events. It maps YAGO-style temporal facts to the anchor layer because they represent interval-scoped persistent relations.

Answer: the existing spine-versus-anchor split matches the literature. Spine facts preserve episodic history. Anchor facts are projected, current-valid, or interval-valid state.

### Missing Query Patterns

The report identifies several temporal query capabilities that are not yet explicit in the current design:

- time-slice or snapshot queries: what was true at time `T`
- bitemporal queries: what was believed then versus what is known now
- interval prediction: when a fact likely held or will hold
- multi-hop temporal logic: before, after, between, first, last, and time joins
- durable temporal joins: which facts overlapped for a meaningful interval
- forecasting: expected future state changes or next events

Answer: lineage traversal is necessary but not sufficient. The world model query surface should eventually expose as-of materialization and interval-aware joins, not only current-anchor lookup plus backward lineage.

### Staleness Versus Contradiction

The report's clearest design answer is that validity intervals, event-based invalidation, and temporal decay must remain separate.

Answer: validity intervals say when a fact holds; invalidation or supersession events say when and why it stopped holding; decay says how confidence or freshness changes over time. If decay is used as truth semantics, stale and contradicted collapse into the same signal. If decay stays in belief and invalidation stays in event/anchor truth maintenance, the distinction remains queryable.

## Design Consequences

This report supports the current world-model direction but turns several implicit choices into explicit requirements:

- The graph layer should support open-ended valid-time intervals, interval closure, supersession, and coalescing.
- "Current" should become a bitemporal query over reference time and transaction time.
- Anchor lineage should remain provenance, not the entire temporal query model.
- Belief freshness should not delete or terminate facts.
- Contradiction should require an invalidating event, supersession event, conflict rule, or locally complete negative evidence.
- Temporal KG embeddings should be treated as optional scoring or forecasting tools above the spine, not as the source of truth.

## What Remains Open

The report does not claim the temporal KG literature fully solves the architecture. It leaves three important gaps:

- Most TKG benchmarks optimize link completion or forecasting rather than auditable belief revision.
- Common benchmarks split between event streams and interval facts, while Meld uses a monotonic event spine with anchor lineage and a future calibrated belief layer.
- Mixed granularity and relation-specific volatility are recognized in the literature, but not yet mature enough to prescribe a complete implementation.

The practical conclusion is architectural: use temporal KG research to enrich query patterns and scoring models, but keep event capture, state materialization, and belief maintenance as separate responsibilities.
