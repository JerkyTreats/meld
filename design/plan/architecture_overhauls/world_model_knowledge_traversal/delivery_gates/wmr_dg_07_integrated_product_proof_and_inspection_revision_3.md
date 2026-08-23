# WMR-DG-07 Integrated Product Proof And Inspection

Date: 2026-08-23

Gate identifier: `WMR-DG-07`

Revision: 3 proposed

Status: Startup integration candidate

Historical basis: [accepted revision 1](wmr_dg_07_integrated_product_proof_and_inspection.md)

Corrective basis: [revision 2 candidate](wmr_dg_07_integrated_product_proof_and_inspection_revision_2.md)

Gate owner: independent Startup integration reviewer

Exception authority: user

## Amendment Boundary

This revision preserves every accepted revision 1 criterion and every revision 2 corrective criterion. It adds the approved core `meld_startup` product as a bounded integrated proof over the existing owner contracts.

The product establishes one deterministic nonce reconciliation round trip for each open admission epoch. It uses one reusable `nonce.emit.v1` Capability and Event kind `nonce`, whose owner contract contains no Startup, Agent, Goal, product, lifecycle, or runtime-health meaning.

This amendment does not create `WMR-DD-08`, change `WMR-DD-01` through `WMR-DD-06`, or authorize source implementation. It does not select the optional policy that blocks dependent PDS activation on Startup nonce satisfaction.

## Bounded Criterion Supersession

The user-approved Startup design supersedes only the no-new-owner clause in `WMR-DG-07-C17` and `WMR-DG-07-R2-C22` for the exact reusable `nonce` semantic owner and its `nonce.emit.v1` contract. This is the only authorized exception.

Every other clause remains binding. The candidate remains documentation-only and adds no implementation phase, source behavior, store, protocol, service, crate, background runtime, compatibility path, or arbitrary Event authority. The exception does not reopen any earlier detailed design or permit another owner.

## Deliverables

- [Startup PDS design package](../startup_pds_design_requirements/README.md)
- [Startup PDS full design](../startup_pds_design_requirements/startup_pds_design_specification.md)
- [Startup semantic transition ledger](../startup_pds_design_requirements/semantic_transition_ledger.md)
- [program insertion assessment](../startup_pds_design_requirements/program_insertion_assessment.md)
- [outcome evidence matrix](../detailed_design/integrated_product_outcome_evidence_matrix.md)
- [integrated product proof and inspection design](../detailed_design/integrated_product_proof_and_inspection.md)
- [cross-gate expected outcomes report](../reviews/world_model_reconciliation_cross_gate_expected_outcomes_evidence_report.md)
- [handoff and lifecycle ledger](../world_model_reconciliation_handoff_ledger.md)
- [WMR-DG-04 revision 2](wmr_dg_04_execution_coherence_and_observation_return_revision_2.md)
- exact Startup integration candidate manifest
- independent Startup integration review recommendation

## Criteria

The seventeen criteria in [accepted revision 1](wmr_dg_07_integrated_product_proof_and_inspection.md) and the five criteria in [revision 2](wmr_dg_07_integrated_product_proof_and_inspection_revision_2.md) remain binding subject only to the exact bounded supersession above.

| Criterion | Cross-deliverable claim | Acceptable evidence | Forbidden substitution | Blocking standard |
| --- | --- | --- | --- | --- |
| `WMR-DG-07-R3-C23` | one current `meld_startup` Agent derives one deterministic nonce instance per open admission epoch from exact product, assignment, Agent, generation, epoch, and contract lineage | `WMR-O29`, full design, and `SPDS-H01` through `SPDS-H04` | process start, readiness, or historical nonce | missing or ambiguous nonce identity blocks |
| `WMR-DG-07-R3-C24` | complete standing Curation evidence authors the expected Event and bounded non-realization before Agent alone incepts the deterministic Goal | `WMR-O30`, full design, and `SPDS-H05` through `SPDS-H08` | missing storage as absence, or coordinator-created Goal | incomplete negative evidence or wrong authority blocks |
| `WMR-DG-07-R3-C25` | Strategy constructs one immutable mixed Plan whose Task and confirmation Epistemic Operation remain independently complete and separately authorized by Agent | `WMR-O30` through `WMR-O32`, integrated proof, and `SPDS-H09` through `SPDS-H21` | Plan as Task, Strategy authorization, or Execution awareness of epistemic meaning | collapsed product or authority boundary blocks |
| `WMR-DG-07-R3-C26` | global `nonce.emit.v1` publishes only deterministic owner-issued Event kind `nonce` from generic issuer, subject, bounded correlations, fence, and revision inputs | `WMR-O31`, full design, and transition ledger | Startup-specific Capability grammar or arbitrary Event append | non-reusable or overpowered Capability blocks |
| `WMR-DG-07-R3-C27` | Event append, Graph visibility, configured Belief settlement, Agent acceptance, Goal satisfaction, and Execution outcome remain independently observable positions | `WMR-O31`, `WMR-O32`, `WMR-O34`, integrated proof, and transition ledger | one combined success or health status | collapsed evidence positions block |
| `WMR-DG-07-R3-C28` | successor admission epoch derives a successor nonce and refuses prior epoch evidence as current satisfaction | `WMR-O33`, full design, and `SPDS-H03`, `SPDS-H04`, `SPDS-H22` | stable nonce across epochs or process recovery as reconciliation | missing epoch fence blocks |
| `WMR-DG-07-R3-C29` | read-only Startup inspection localizes the first missing critical owner position while preserving parallel unresolved obligations | `WMR-O34`, full design, integrated inspection, and `SPDS-H23` | inspection-created truth, timeout terminality, or nonce satisfaction as global health | false localization or semantic aggregation blocks |
| `WMR-DG-07-R3-C30` | core Startup proof remains non-gating and first in implementation proof order, while dependent PDS activation remains explicitly unselected | program ledger, insertion assessment, full design, manifest, and diff | silent fail-closed activation policy or source authorization | scope expansion blocks |

Every criterion is blocking. Approval requires an independent review recommendation over the exact manifest. Gate Acceptance remains a separate user-authorized action.

## Acceptance Budget

- one independent Startup integration review
- one frozen finding set
- one bounded correction cycle if needed
- one verification limited to frozen findings and correction-caused regressions
- separate Gate Acceptance only after review passes

The reviewer may recommend approval or changes. The reviewer may not alter this gate, edit the candidate, waive a criterion, issue Gate Acceptance, authorize implementation, or select dependent-product activation gating.

## Candidate Lineage

The revision 2 manifest and approval recommendation remain historical evidence for the exact candidate reviewed before this amendment. Revision 3 receives a new manifest and recommendation. It does not rewrite the revision 2 record or imply revision 2 Gate Acceptance.
