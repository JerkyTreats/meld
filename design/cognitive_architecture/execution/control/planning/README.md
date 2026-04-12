# Execution Planning

Date: 2026-04-12
Status: active
Scope: planning structure inside execution control

## Thesis

Planning lives inside `execution`, under `control`.
It turns goals and current belief into coordinated deliberate work.

The strongest current planning structures in this repo are:

- HTN lineage
- task-network coordination
- control-program transfer
- repair and reselection
- synthesis escalation when the current action catalog is insufficient

## Now

- HTN lineage is the clearest explicit planning structure
- control programs define dispatch, branch, and wait semantics above tasks
- repair provides re-entry and method-change mechanics
- synthesis is already framed as planning-time capability growth

## Next

- make planning explicitly world-state-aware
- express goals as desired world-state change
- let planning choose sensing when belief is missing or stale
- preserve HTN where it is strong without forcing all planning into one formalism

## Read With

- [Execution Control](../README.md)
- [HTN Model](htn/README.md)
- [HTN Lineage Model](htn/lineage_model.md)
- [Task Network](../task_network.md)
- [Control Graph Model](../program/control_graph.md)
