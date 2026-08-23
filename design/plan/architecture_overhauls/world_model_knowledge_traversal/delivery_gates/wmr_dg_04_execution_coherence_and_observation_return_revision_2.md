# WMR-DG-04 Execution Coherence And Observation Return

Date: 2026-08-22

Gate identifier: `WMR-DG-04`

Revision: 2 proposed

Status: approval candidate

Historical basis: [accepted revision 1](wmr_dg_04_execution_admission_and_observation_return.md)

Gate owner: independent corrective assurance reviewer

Exception authority: user

## Corrective Boundary

This revision preserves the accepted Agent-to-Execution and returned-owner boundary while correcting one gate-definition defect. Execution must consider all admitted executable obligations together and lower them into one unified Task Network. Compatible admissions may share one operational action while retaining complete attribution.

The gate begins with complete Agent-authorized Goal-attributed Tasks and ends with exact Agent absorption of declared returned owner milestones. It includes `WMR-H13` through `WMR-H17`, `WMR-H32` through `WMR-H36`, and expected outcome `WMR-O27`.

Strategy reconstruction, heterogeneous Plan progression, PDS, activation-wide lifecycle, runtime schemas, and source implementation remain outside this gate except as named upstream evidence.

## Deliverables

- [Execution transition ledger](../detailed_design/execution_admission_and_observation_transition_ledger.md)
- [Execution coherence and observation return design](../detailed_design/execution_admission_and_observation_return.md)
- [outcome evidence matrix](../detailed_design/integrated_product_outcome_evidence_matrix.md)
- [handoff and lifecycle ledger](../world_model_reconciliation_handoff_ledger.md)
- [canonical Execution architecture](../../../../cognitive_architecture/execution/README.md)
- [canonical Task Network architecture](../../../../cognitive_architecture/execution/task_network.md)
- exact approval candidate manifest
- independent corrective review recommendation

## Criteria

| Criterion | Cross-deliverable claim | Acceptable evidence | Forbidden substitution | Blocking standard |
| --- | --- | --- | --- | --- |
| `WMR-DG-04-R2-C01` | every complete Task enters Execution through one independent Goal-attributed admission | admission identity and transition account | Plan-wide authorization or Goal as executable body | admission identity collapse blocks |
| `WMR-DG-04-R2-C02` | all admitted executable obligations enter one Execution coherence domain and one durable Task Network | lowering design, handoff `WMR-H14`, and canonical alignment | separate networks per Agent, Goal, Plan, or Task | more than one canonical operational network blocks |
| `WMR-DG-04-R2-C03` | compatible admissions may share one action node only through exact compatibility proof | coherence outcome table and `WMR-O27` | similar names, desired outcome, or Capability label | unproved action reuse blocks |
| `WMR-DG-04-R2-C04` | shared work preserves every admission, Agent, Goal, Plan, Task, authority, generation, and result attribution | identity ledger and shared Merkle trace | merged authorization or one generic completion | lost attribution blocks |
| `WMR-DG-04-R2-C05` | incompatible admissions remain distinct operational nodes inside the same network | negative coherence outcome | forced sharing or separate Task Networks | unsafe reuse or coherence escape blocks |
| `WMR-DG-04-R2-C06` | Execution owns priority, executable ordering, parallelism, resource reservation, and compatible-work reuse without changing Task meaning | Execution boundary account | Strategy scheduling or semantic replanning | authority leakage blocks |
| `WMR-DG-04-R2-C07` | one shared operational outcome produces independently addressable discharge evidence for every admission | outcome and attribution account | shared outcome as merged Goal satisfaction | missing discharge lineage blocks |
| `WMR-DG-04-R2-C08` | returned owner observation and each Agent milestone remain independent after shared execution | return milestone table and `WMR-H32` through `WMR-H36` | one Agent acceptance as another Agent acceptance | cross-Agent semantic collapse blocks |
| `WMR-DG-04-R2-C09` | original revision 1 admission, uncertain-effect, publication, and semantic-return guarantees remain intact | corrected candidate diff and historical receipt | corrective scope as permission to weaken prior criteria | regression blocks |
| `WMR-DG-04-R2-C10` | candidate remains documentation-only and introduces no new store, protocol, service, crate, or runtime owner | manifest and diff | implementation convenience | scope expansion blocks |

Every criterion is blocking. Approval requires an independent review recommendation over the exact manifest. Gate Acceptance remains a separate user-authorized action.

## Acceptance Budget

- one independent corrective review
- one frozen finding set
- one bounded correction cycle if needed
- one verification limited to frozen findings and correction-caused regressions
- separate Gate Acceptance only after review passes

The reviewer may recommend approval or changes. The reviewer may not alter this gate, edit the candidate, waive a criterion, issue Gate Acceptance, or authorize implementation.
