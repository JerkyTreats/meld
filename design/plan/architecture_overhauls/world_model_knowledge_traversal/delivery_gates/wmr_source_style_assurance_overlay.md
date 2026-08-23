# World Model Reconciliation Source Style Assurance Overlay

Date: 2026-08-23

Overlay identifier: `WMR-SAO-01`

Revision: 1

Status: active policy overlay for authorized source delivery

Authority: user-approved delivery-program policy

Source implementation authority: `WMR-SI-01` only

## Purpose

Every source-changing World Model Reconciliation slice must complete one dedicated Style Assurance pass after logical implementation review succeeds and before Delivery Gate Acceptance begins.

The overlay separates two different judgments. Implementation review establishes that the source behaves correctly inside the authorized slice. Style Assurance establishes that the exact reviewed candidate meets Meld commenting, structural, formatting, lint, and risk-proportionate testing expectations. Delivery Gate Acceptance then judges cross-deliverable coherence.

This ordering lets logical implementation stabilize before maintainability and test-quality enforcement, while preventing a style pass from becoming a second architecture review.

## Program Insertion Boundary

This is an append-only overlay over source implementation gates. It does not revise the accepted `WMR-DG-01` through `WMR-DG-07` design receipts, the accepted `WMR-DG-04` revision 2 correction, or the accepted `WMR-DG-07` revision 3 Startup amendment.

The exact accepted design manifests remain frozen. This overlay is outside those manifests because it governs later source delivery authority and evidence rather than the accepted design candidate.

The overlay applies to `WMR-SI-01` and every later source-changing World Model Reconciliation slice. A documentation-only correction may record the overlay as not applicable when it changes no source, executable test, build script, schema generator, or source-facing generated artifact.

## Required Sequence

```text
authorized source implementation
-> direct product proof
-> logical implementation review
-> bounded logical corrections and verification
-> exact post-review candidate
-> Style Assurance initial pass
-> bounded style corrections
-> targeted logical review if behavior changed
-> Style Assurance verification
-> satisfied Style Assurance Receipt
-> Delivery Gate Acceptance
```

Gate Acceptance is `not eligible` without a satisfied Style Assurance Receipt or an exact documentation-only not-applicable disposition.

## Assurance Ownership And Budget

The Style Assurance owner is dedicated and read-only during assessment. The owner may not edit the candidate, waive policy, amend architecture, expand the active slice, dispose findings, issue Gate Acceptance, or authorize later work.

The default budget is one initial pass, one frozen finding set, one program-owner disposition, one bounded correction cycle, and one verification pass limited to disposed findings plus correction-caused regressions.

The changed source surface and the smallest surrounding ownership seam form the review horizon. Unchanged repository style is outside scope unless the active change makes it stale, misleading, or unsafe.

## Commenting Assurance

The pass applies the [Commenting Policy](../../../../../governance/commenting_policy.md) without comment quotas.

Public contracts require concise Rustdoc when ownership, semantic meaning, authority, durability, or invariants are not obvious from the type alone. Domain entry modules require enough Rustdoc to explain the boundary and what the domain does not own when confusion is likely.

Non-obvious orchestration requires comments where ordering, commit-before-publish, commit-before-cursor, replay, idempotency, fencing, uncertain effects, compatibility, or recovery would otherwise be easy to violate.

Compatibility seams require the repository removal note and characterization evidence. New code must target the canonical path rather than the compatibility seam.

The pass rejects comments that narrate syntax, restate field names, preserve superseded architecture, imply stronger proof than runtime evidence supplies, or duplicate design prose without helping a source reader preserve an invariant.

For World Model Reconciliation, the highest-value comment surfaces are Curation acceptance and terminality, immutable Plan identity, Agent progression, `TraversalCut` completeness, `PlannerCut` consistency, Task admission, nonce identity, publication ordering, epoch fences, and semantic-owner return.

## Structural Style Assurance

Changed code must preserve domain-first organization, explicit public contracts, thin root adapters, and the repository ban on `mod.rs`.

World-model semantics remain in `meld-world-model`. Root composition may route and bind products without interpreting Curation, Strategy, Agent, Belief, or Execution meaning. Execution remains limited to consumer-shape validation and executable realization. Events and `meld-lang` remain unchanged unless separately authorized after a new architecture finding.

A proposed style correction may rename, document, format, or reorganize code only when ownership and behavior remain unchanged. Moving a contract between domains, changing public semantics, adding a new abstraction, or altering persistence is logical implementation work and returns to implementation review.

## Test Quality Assurance

The pass judges tests by changed-surface risk rather than test count.

Every slice requires direct tests for its observable product increment and focused negative tests for the rejected, stale, conflicted, incomplete, unauthorized, or uncertain states introduced by its contracts.

Durable record, cursor, outbox, journal, projection, and handoff changes require restart, replay, idempotency, crash-window, and compatibility evidence proportionate to the changed contract.

Deterministic identity, ordering, boundedness, normalization, graph, and state-transition contracts require property tests when examples cannot cover their meaningful input space.

Fuzz applicability must be decided for every changed crate. A changed parser, durable deserializer, replay surface, graph walk, planning contract, Task Network contract, or state machine must extend an applicable existing fuzz target or add a focused target within the crate's existing fuzz package. New fuzz crates are not authorized by this overlay.

Straightforward field mapping, private closed-input helpers, and contracts already exercised by a directly applicable target do not require a new fuzz target. The receipt must still record why fuzzing was added, already covered, or not warranted.

New or changed fuzz targets require a successful build and bounded smoke run. Long fuzz campaigns are optional operational evidence unless a source slice explicitly budgets them.

## Crate Precedent For The First Source Slice

| Crate | Current assurance precedent | Expected `WMR-SI-01` use |
| --- | --- | --- |
| `meld-world-model` | `proptest` plus fuzz targets for graph walk and world-model contracts | extend graph-walk and contract coverage for occurrence identity, `TraversalCut`, boundedness, completeness, replay, and restart according to the final changed surface |
| `meld-execution` | `proptest` plus fuzz targets for planning, runtime, Task Network contracts, commands, replay, and readiness | no `WMR-SI-01` work because executable intake begins in `WMR-SI-04` |
| root `meld` | property tests and extensive runtime, initialization, harness, and integration suites | prove existing owner publication through Event, Graph, and Traversal only when thin root wiring or direct integration coverage changes |
| `meld-events` | fuzz targets for Event contracts, replay, and writer behavior | no new work when the accepted neutral Event contracts remain unchanged |
| `meld-lang` | `proptest` over pure language behavior | no new work when Plan grammar remains owned by `meld-world-model` |

## Required Commands For Source Candidates

The exact command set follows the actual changed crates. The minimum candidate includes formatter verification, warning-free lint for every changed crate and the root composition when touched, focused crate tests, direct integration proof, and the required workspace suite.

```text
cargo fmt --all -- --check
cargo clippy -p meld-world-model --all-targets -- -D warnings
cargo clippy -p meld --all-targets -- -D warnings
cargo test -p meld-world-model
cargo test -p meld --lib
cargo test --workspace -- --test-threads=1
```

Commands for unchanged crates may be omitted from the focused set, but public API or workspace-wide changes still require the workspace gates. The receipt records every omission and its changed-surface rationale.

Applicable fuzz targets use the crate-local fuzz package and a bounded smoke run such as the existing `-runs=1` convention. Target selection is evidence-driven and must be listed in the receipt.

Runtime corrections also require the live harness evidence mandated by the [Harness Development Policy](../../../../../governance/harness_development_policy.md). When `WMR-SI-01` changes root runtime composition, unit tests cannot substitute for owner publication visibly clearing through durable Event, Graph, and Traversal positions.

## Criteria

| Criterion | Required claim | Blocking condition |
| --- | --- | --- |
| `WMR-SAO-C01` | the candidate exactly matches the logically reviewed source candidate | identity drift or missing implementation-review receipt blocks |
| `WMR-SAO-C02` | changed public contracts and domain entries carry concise ownership and invariant Rustdoc where needed | an ambiguous public boundary blocks |
| `WMR-SAO-C03` | non-obvious ordering, persistence, replay, fencing, and compatibility logic explains the reason for its invariant | a correctness-relevant undocumented invariant blocks |
| `WMR-SAO-C04` | comments are current, specific, non-narrative, and no stronger than runtime proof | stale or misleading comments block |
| `WMR-SAO-C05` | changed structure remains domain-first with thin adapters and no `mod.rs` | ownership-obscuring structure blocks |
| `WMR-SAO-C06` | formatter and required lint commands pass on the exact candidate | formatting or warning failure blocks |
| `WMR-SAO-C07` | direct and negative tests cover every new product and terminal distinction in the slice | an unproved changed behavior blocks |
| `WMR-SAO-C08` | durable changes carry restart, replay, idempotency, crash-window, and compatibility evidence where applicable | an uncovered durability risk blocks |
| `WMR-SAO-C09` | property-test applicability is explicit and applicable invariants are exercised | an untested broad deterministic invariant blocks |
| `WMR-SAO-C10` | fuzz applicability is explicit for every changed crate and applicable targets build and smoke successfully | unjustified omission or broken target blocks |
| `WMR-SAO-C11` | focused tests, direct product proof, and required workspace gates pass | a required test or product proof failure blocks |
| `WMR-SAO-C12` | runtime behavior is demonstrated through truthful harness evidence when the slice changes runtime composition | test-only substitution for required live proof blocks |
| `WMR-SAO-C13` | style corrections preserve semantics or return through targeted implementation review | unreviewed behavioral change blocks |
| `WMR-SAO-C14` | the final receipt records findings, dispositions, corrections, verification, commands, applicability decisions, and exceptions | incomplete assurance evidence blocks |

Every applicable criterion is blocking. No criterion authorizes repository-wide cleanup, additional architecture, or a stronger testing surface than the changed risk warrants.

## Style Finding Disposition

The Style Assurance owner classifies findings as `comment quality`, `structural style`, `format or lint`, `test quality`, `compatibility annotation`, or `style-discovered logical defect`.

The program owner assigns correction scope. A style-discovered logical defect cannot be corrected under style-only authority. It returns to the implementation owner, direct product proof, and targeted implementation review. The successor candidate then re-enters Style Assurance.

## Receipt

The Style Assurance record uses `satisfied`, `not satisfied`, or `not eligible`. A satisfied record becomes the Style Assurance Receipt.

The receipt names the active slice, exact candidate, changed surface, applicable policies, crate precedents, criterion verdicts, formatter and lint evidence, focused and workspace tests, property and fuzz applicability, harness evidence, frozen findings, dispositions, correction diff, verification, exceptions, and final verdict.

The receipt establishes style and test-quality readiness only. It does not prove the architecture was implemented correctly, accept a Delivery Gate, authorize a commit, or activate another slice.

## Current Program State

This overlay is active for the authorized `WMR-SI-01` owner-publication-to-`TraversalCut` source slice. Source implementation has not started. `WMR-SI-02` through `WMR-SI-06` remain dependency-ordered backlog. Startup is `WMR-SI-07`, the first integrated PDS runtime proof after those substrate slices. Docs Freshness is `WMR-SI-08` and Dependency Security is `WMR-SI-09`. Every later slice remains unauthorized until separately activated.

The [source activation record](wmr_si_01_source_activation_record.md) preserves the exact authorization boundary and the [source Delivery Gate](wmr_si_01_owner_publication_to_traversal_cut_gate.md) defines the cross-domain acceptance claims that follow logical review and this Style Assurance pass. The [Startup gate](wmr_si_07_startup_reconciliation_gate.md) remains provisional backlog.
