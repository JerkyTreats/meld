# Current Architecture Overhauls

Date: 2026-08-19
Status: architectural discovery only
Scope: current architectural fractures that must be understood before implementation planning resumes

## Purpose

This directory contains only current work. Clearly completed records belong under `design/completed`. Denied and superseded records are removed from the active design corpus and remain recoverable through Git history.

The PDS Boundary program is denied. It has no implementation authority and no successor delivery program is implied here.

PDS remains a real and useful concept. Its [canonical boundary](../cognitive_architecture/persistent_domain_stewardship.md) and [proposal corpus](../persistent_domain_stewardship/README.md) remain available while the concept is reconciled against the three native Meld overhauls below.

## Current Overhauls

Three coupled architectural issues now organize the work:

1. [Native Capability And Task Substrate](architecture_overhauls/native_capability_task_substrate.md)
   restore one Capability, Composition, Task, and Task Network language across Strategy and Execution
2. [Graph-Addressed Epistemic Substrate](architecture_overhauls/graph_addressed_epistemic_substrate.md)
   make continuous epistemic lowering and relation-rich planning coherent without replacing domain-owned records and stores
3. [Agent, Strategy, And Merge Control Loop](architecture_overhauls/agent_strategy_merge_control_loop.md)
   clarify deliberation, repair planning, evidence acquisition, and epistemic curation authority

These are discovery workstreams. They contain no phases, implementation slices, acceptance gates, or authorization.

## Retained Orthogonal Work

- [Spine Compaction Design](events/spine_compaction_design.md)
- [Agent-Native Debugger Requirements](integration/agent_native_debugger_requirements.md)
- [Multiplier Harness Program](integration/multiplier_harness_program.md)
- [Runtime Harness Plan](integration/runtime_harness_plan.md)
- [Runtime Supervisor Invariant Assessment](integration/runtime_supervisor_invariant_domain_assessment.md)
- [Storage Substrate Decision Record](integration/storage_substrate_decision_record.md)
- [Sensory Assessment](sensory/assessment.md)
- [Causation Assessment](world_model/causation/assessment.md)
- [Regime Assessment](world_model/regime/assessment.md)

## Documentation Rule

Evergreen ownership, boundaries, contracts, and invariants live under `design/cognitive_architecture`.

Current gaps and unresolved architectural questions live here. When a question becomes settled architecture, its durable answer moves into cognitive architecture and the discovery record is reduced or removed. Clearly completed implementation records move to `design/completed` for historical reference.
