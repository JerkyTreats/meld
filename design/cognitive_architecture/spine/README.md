# Spine Redirect

Date: 2026-04-22
Status: redirected
Scope: compatibility pointer for the former standalone spine concern

## Position

The standalone spine concern has been collapsed into `events`.

Use [Events Domain](../events/README.md) for active architecture.
Use [Events Crate](../events/CRATE.md) for the code routing point.

The word spine may still appear in older notes as a name for the shared temporal ledger.
New design should route ownership through `events` and the target `meld-events` crate.

## Historical Meaning

The old spine concern meant:

- canonical event envelope
- runtime-wide sequence assignment
- durable append
- replay and subscription
- cross-domain object refs and relation edges

Those responsibilities now belong to `events`.

## Read With

- [Events Domain](../events/README.md)
- [Events Crate](../events/CRATE.md)
- [Event Ledger Requirements](../events/event_manager_requirements.md)
- [Multi-Domain Event Ledger](../events/multi_domain_spine.md)
