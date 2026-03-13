# Migration Plan

Date: 2026-03-11
Status: active

## Parent Roadmap

- [Workflow Orchestrator Roadmap](../README.md)

## Intent

Define a compatibility path from the current turn workflow runtime to an HTN ready workflow foundation.

## HTN Position

- the current turn runtime should be treated as a compatibility workflow shape rather than the final workflow model
- migration should stabilize atomic task contracts and durable workflow records before introducing broader decomposition features
- existing commands should compile into the new workflow substrate before their public surfaces change
- first migration steps should improve identifiers, slots, checkpoints, and records without forcing immediate authoring changes

## Provisional Answers

### Current Turn Runtime Fit

- the current turn runtime fits as a compatibility shape that can later map into one compiled task network
- its turns can be treated as early workflow stage records while the new substrate introduces stable task ids, plan ids, and artifact handoff slots
- the current docs writer flow should become the first canonical compatibility mapping into the HTN ready workflow substrate

### Command Compatibility Layer

- `context generate` should remain stable while its internal execution compiles into ordering plus context generate tasks
- `workflow execute` should remain stable while current turn profiles execute through compatibility compilation paths
- compatibility wrappers should live at the workflow boundary rather than leaking orchestration logic back into domain modules

### Safe First Config Changes

- add stable `plan_id` support
- add stable `task_instance_id` support
- add explicit input and output slot declarations
- add task type ids and schema versions where they are still implicit
- add checkpoint and repair record structures before broader decomposition features

### Stored State Migration

- existing docs writer profiles and bindings should migrate through compatibility records rather than destructive schema replacement
- old state should remain readable until parity and resume behavior are verified on the new records
- cutover should prefer additive records first, then wrapper translation, then old path removal

## Initial Requirements

- keep current command behavior stable during the transition period
- treat the current turn runtime as a compatibility workflow shape behind the new workflow contract
- allow new task based workflows to coexist with current turn profiles during migration
- define cutover rules for profile ids, frame types, plan ids, task ids, and resume state
- require characterization tests before removing old execution paths

## Migration Stages

### Stage 1

- stabilize atomic task contracts
- stabilize explicit artifact slots and task type ids
- preserve command behavior

### Stage 2

- enrich workflow durable state with plan ids, task instance ids, checkpoints, and repair records
- emit telemetry aligned to those new workflow records

### Stage 3

- compile current turn workflows through compatibility translation into the new workflow substrate
- compile `context generate` through ordering plus context generate tasks

### Stage 4

- re express the docs writer path as the first canonical hierarchical example on the stabilized substrate
- remove old execution ownership only after characterization and parity gates pass

## Residual Questions

- when should the first true method library record appear in config rather than only in compatibility translation
- how much of the current turn runtime should remain visible in operator tooling during the compatibility window
- whether decomposition records should reuse current workflow state storage or land in a new adjacent record family

## Related Areas

- [HTN Glossary](../htn_glossary.md)
- [Task Model](../task_model/README.md)
- [Workflow Definition](../workflow_definition/README.md)
- [Context Generate Task](../context_generate_task/README.md)
- [Completed Workflow Bootstrap](../../completed/workflow_bootstrap/README.md)
