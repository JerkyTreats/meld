# WMR-VC-06 Activation Generation And Lifecycle Gate

Date: 2026-09-05

Gate identifier: `WMR-VC-06-DG`

Status: frozen; third corrected successor awaits fresh Logical Review

Source baseline: `85f4b85d`

Initial rejected candidate: `WMR-VC-06::ad41bce802f5b99226d25de41eb9da4d39b005a34f7c9bca599abbf666470f38`

Rejected native-owner successor: `WMR-VC-06::0dc00fd22ddf54951bd18fa016a58cac1cd2eab8c1b0966daef73b023dbab241`

Rejected transition-liveness successor: `WMR-VC-06::871f0be8b14d49adc808250b7c6f28f9849eb408e8d7e5762bcd301cd48e7033`

Current successor candidate: `WMR-VC-06::e4bf8a9578aa44594411607bd1c5a117aabb9dd37beca322c1f1346d6ddff77f`

Initial Logical Review: [invalidated](../reviews/wmr_vc_06_implementation_review_receipt.md)

Initial Style Assurance: [invalidated](../reviews/wmr_vc_06_style_assurance_receipt.md)

Initial Gate candidate: [not eligible](../reviews/wmr_vc_06_gate_acceptance_candidate.md)

First correction record: [rejected](wmr_vc_06_native_owner_correction_record.md)

Second correction record: [rejected](wmr_vc_06_native_transition_liveness_correction_record.md)

Current correction record: [implemented](wmr_vc_06_native_evidence_lineage_correction_record.md)

Successor Logical Review: required

Gate Acceptance: pending

## Acceptance Boundary

Accept only a candidate where one inert prepared closure becomes one exact activation generation whose plan, registration, realization, readiness, current head, admission, recovery, liveness, fencing, and retirement remain coherent through the production supervisor.

## Criteria

| Criterion | Required result |
| --- | --- |
| `WMR-VC-06-DG-C01` | one durable assignment aggregate owns generation head and admission coherence |
| `WMR-VC-06-DG-C02` | lifecycle positions and all structural identities remain distinct |
| `WMR-VC-06-DG-C03` | registration is an exact projection of the accepted participant plan |
| `WMR-VC-06-DG-C04` | required native bodies fail closed and every realized participant supplies exact readiness |
| `WMR-VC-06-DG-C05` | current publication and admission opening are one optimistic transition |
| `WMR-VC-06-DG-C06` | correctness depends on durable owner positions rather than tick order |
| `WMR-VC-06-DG-C07` | waits bind exact checkpoints and complete structural wake references |
| `WMR-VC-06-DG-C08` | health, active work, active idle, quiescent, stalled, interrupted, draining, and retired stay distinct |
| `WMR-VC-06-DG-C09` | recovery closes admission and requires a successor incarnation readiness barrier |
| `WMR-VC-06-DG-C10` | replacement atomically installs one successor and fences one predecessor |
| `WMR-VC-06-DG-C11` | late work retains original lineage and semantic classification stays with destination owners |
| `WMR-VC-06-DG-C12` | fenced quiescence requires closed admission, owner safe points, passive fences, and unresolved-effect summaries |
| `WMR-VC-06-DG-C13` | retirement follows reverse structural order, releases every incarnation, and publishes immutable evidence |
| `WMR-VC-06-DG-C14` | root remains structurally authoritative and semantically blind |
| `WMR-VC-06-DG-C15` | duplicate lifecycle stores, inferred registrations, fixed lifecycle actors, and root operation semantics are retired |
| `WMR-VC-06-DG-C16` | the exact candidate passes logical review, Style Assurance, full tests, and manifest verification |

Every criterion is blocking. The gate has no waiver authority.

## Exclusions

This gate does not accept Startup nonce proof, final native inspection, product migration, commit, push, or deployment. It does not activate `WMR-VC-07`.

## Evidence Order

Logical implementation review precedes Style Assurance. Gate Acceptance follows both over the same exact manifest. Material source, executable-test, manifest, or corrective evidence changes invalidate downstream assurance.
