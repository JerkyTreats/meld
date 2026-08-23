# WMR-SI-01 Source Activation Record

Date: 2026-08-23

Slice identifier: `WMR-SI-01`

Status: authorized and ready, paused for design and documentation alignment acceptance

Authorization authority: user

Implementation started: no

Baseline commit: `e0f8738e`

## Authorized Outcome

Implement the non-blocking `meld_startup` PDS as the first World Model Reconciliation runtime product proof. One fresh admission epoch must carry one deterministic nonce from standing epistemic mismatch through Agent Goal inception, Strategy Plan construction, Agent product authorization, Execution, owner-issued Event publication, Graph and Traversal visibility, planned confirmation Curation, configured Belief settlement, Agent milestone acceptance, and Goal satisfaction.

Startup is first in delivery order so later products can use its inspection account while they are implemented. Runtime activation does not block on nonce satisfaction.

## Accepted Design Basis

- [WMR-DG-04 revision 2 receipt](wmr_dg_04_revision_2_acceptance_receipt.md)
- [WMR-DG-07 revision 3 receipt](wmr_dg_07_revision_3_acceptance_receipt.md)
- [exact accepted candidate manifest](../reviews/world_model_reconciliation_startup_integration_candidate.sha256)
- [Startup full design](../startup_pds_design_requirements/startup_pds_design_specification.md)
- [Startup transition ledger](../startup_pds_design_requirements/semantic_transition_ledger.md)
- [current cognitive architecture](../../../../cognitive_architecture/README.md)
- [source Style Assurance overlay](wmr_source_style_assurance_overlay.md)

The accepted manifest remains the immutable design candidate. This activation record is later authority evidence and is intentionally outside that manifest.

## Active Boundary

The slice may implement only behavior required to prove the accepted Startup path through existing domain ownership. Required owner behavior may be added within world-model Curation, Graph and Traversal, Belief, Planner, Strategy, Agent, PDS compilation and activation, root runtime composition, the reusable nonce owner, Execution Goal Set and lowering, native lifecycle, and read-only inspection.

`meld-events` and `meld-lang` remain unchanged unless direct implementation evidence proves the accepted contracts cannot be realized without a specific amendment. Execution remains ignorant of Strategy Plans, epistemic canonicality, and Startup meaning.

The slice may add the exact reusable nonce owner and `nonce.emit.v1` Capability authorized by the revision 3 exception. It may not add arbitrary Event emission, a second Event path, another graph authority, another Task Network, another runtime coordinator, or a Startup-specific Execution rule.

## Explicitly Unauthorized

- Docs Freshness migration
- Dependency Security migration
- broad cross-Agent Merkle-scan product proof beyond behavior strictly required by the Startup slice
- fail-closed dependent-PDS activation
- legacy Workflow integration
- new crate, store, protocol, service, or background runtime not already authorized by accepted design
- commit or push without the applicable repository and user authority

## Delivery Sequence

```text
source implementation
-> direct fresh-epoch product proof
-> logical implementation review
-> bounded logical correction and verification
-> Style Assurance
-> WMR-SI-01 Delivery Gate Acceptance
-> handoff eligible
```

The [source Delivery Gate](wmr_si_01_startup_reconciliation_gate.md) is frozen before source edits. Passing it does not authorize Docs Freshness, Dependency Security, fail-closed Startup policy, commit, or push.

## Stop Conditions

Pause before architectural expansion, movement of semantic ownership, edits to `meld-events` or `meld-lang`, a second persistence authority, any runtime gate on nonce satisfaction, or source scope that cannot be tied to a declared Startup proof position.

Pause when implementation evidence contradicts an accepted identity, authority, lifecycle, or producer-consumer boundary. Record the conflict as a design finding rather than hiding it behind a compatibility guard.

## Current State

The accepted design baseline is committed and clean. Source implementation is authorized but has not started. This activation becomes operational only after the current design and documentation alignment pass is accepted.
