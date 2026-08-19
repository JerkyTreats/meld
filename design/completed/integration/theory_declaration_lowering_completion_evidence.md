# Theory Declaration Lowering Completion Evidence

Date: 2026-08-13
Status: complete
Parent program: [Theory Elevation Program](theory_elevation_program.md)
Implementation design: [Theory Declaration Lowering Implementation Design](theory_declaration_lowering_implementation_design.md)
Domain assessment: [Theory Declaration Lowering Assessment By Domain](theory_declaration_lowering_domain_assessment.md)

## Outcome

Theory Elevation Step 2 is implemented. Named stewardship declarations now lower through exact installed receipts without selecting semantics by expression name. The shipped docs package runs with the canonical expression identity `documentation_maintenance`, proving that the package is not activated by a `docs_freshness` expression branch.

This evidence closes Step 2 only. Standing maintained conditions, effective authority, a second dissimilar expression, and settled replay remain in steps 3 through 6.

## Declaration Lowering

- `StewardshipConfig` accepts deterministic named declarations under `stewardship.declarations`.
- The legacy `stewardship.docs_freshness` table lowers into the same canonical declaration and remains characterized as a compatibility reader.
- Physical bindings resolve by canonical target. Duplicate declarations for one target fail as ambiguous, while unavailable declarations for other targets do not block the selected target.
- Source-aware validation retains full declaration paths and source origins.

## Owner Activation

- Meld world model activates an installed Strategy package from owner data plus concrete subject and Agent grounding.
- Docs publishes exact invoker implementations by capability type, version, and content identity.
- Runtime assembly rejects any installed executable contract that lacks an exact process implementation.
- Planning and execution consume the resulting catalog and registry without application vocabulary.

## Root Dispatch Removal

- Production CLI, initialization, and runtime assembly do not compare declaration expression identity to an application name.
- Production runtime assembly no longer calls the docs PDS composer.
- `src/docs/pds.rs` is test-only and exercises the same owner activation contracts as production.
- Belief-context hydration receives its family identity from the selected physical binding through the existing package trigger. Context has no compiled application-family fallback.

## Compatibility Retirement

The expression-keyed planning loaders are absent from root initialization. The following stale Method-era artifacts were deleted:

- `theory/docs_freshness/available_actions.docs_freshness.json`
- `theory/docs_freshness/method_realizations.docs_freshness.json`
- `theory/docs_freshness/methods/refresh_docs_v1.json`

## Runtime Proof

The integration test `stewardship_receipts_activate_routes_and_preserve_a_b_lineage` uses a canonical declaration with expression `documentation_maintenance` and the shipped docs theory identities. It proves exact receipt activation, five-capability Strategy closure without Method or workflow routing, revision A and revision B lineage, and quiet convergence.

Focused contract coverage also proves canonical and legacy lowering, target isolation, duplicate-target rejection, owner Strategy activation, exact docs implementation activation, implementation drift rejection, and declaration-selected belief-context routing.

## Quality Gates

The following gates completed successfully on 2026-08-13:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
git diff --check
```
