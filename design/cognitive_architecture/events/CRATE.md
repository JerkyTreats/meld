# meld-events

Scope: canonical Event envelope, append authority, ordered replay, durable cursors, and observability projection

The crate owns Event identity, ledger position, append authorization, replay ordering, cursor durability, schema routing, and generic observability metadata.

It does not own payload semantics or domain lifecycle. Domain adapters encode and decode their own typed payloads.

The public contracts support authorized append, bounded replay, cursor commit, consumer lag, Event lookup, and causal lineage queries.
