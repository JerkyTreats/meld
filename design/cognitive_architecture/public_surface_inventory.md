# Public Surface Inventory

Date: 2026-04-25
Status: active
Scope: compatibility surfaces that must not become long term crate APIs

## Purpose

Phase 0 freezes the concrete runtime and store shaped APIs that later phases must reduce.
These surfaces are allowed only as temporary compatibility during migration.

## Inventory

| Surface | Current location | Current role | Long term direction |
| --- | --- | --- | --- |
| `ContextApi` | [src/api.rs](../../src/api.rs) | root super facade for context, provider, prompt artifacts, progress state, and graph access | shrink to compatibility facade while execution moves to narrow ports |
| `EventStore` | [src/events/store.rs](../../src/events/store.rs) | canonical event ledger plus session lifecycle compatibility | keep as event crate internal storage and query primitive without session authority |
| `GraphRuntime` | [src/world_state/graph/runtime.rs](../../src/world_state/graph/runtime.rs) | graph catch up plus derived event append | hide behind world model assembly and public query services |
| `TraversalStore` | [src/world_state/graph/store.rs](../../src/world_state/graph/store.rs) | graph materialization storage | keep behind world model query APIs |
| `WorldStateStore` | [src/world_state/store.rs](../../src/world_state/store.rs) | legacy claim projection storage | keep behind legacy compatibility query services |

## Migration Notes

- no new public API should expose these types as the preferred entrypoint
- tests may keep using these types as characterization seams until parity is proven
- root compatibility wrappers must shrink phase by phase rather than becoming permanent surface

## Exit Signal

This inventory is retired when Phase 5 closes and the listed surfaces are compatibility only or crate private.

## Read With

- [PLAN](PLAN.md)
- [Core Migration](core/MIGRATION.md)
- [Events Migration](events/MIGRATION.md)
- [World Model Migration](world_state/MIGRATION.md)
