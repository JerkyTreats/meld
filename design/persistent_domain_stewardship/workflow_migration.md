# Workflow Migration

Date: 2026-07-14  
Status: proposed  
Scope: migration from workflow-authored domain behavior to stewardship packages executed through existing and future Meld runtime mechanisms

## Thesis

Persistent Domain Stewardship should replace workflow profiles as the source of domain behavior.

It should not replace durable execution mechanics with configuration.

```text
before:
    workflow profile is the application and the plan

after:
    stewardship package is the application
    known methods provide reusable decomposition
    cognitive runtime chooses and repairs plans
    execution runtime persists and dispatches work
```

The current workflow subsystem is working infrastructure. It includes retries, gates, persistence, prompt resolution, generation orchestration, lifecycle state, event publication, normalization, and a compatibility path around the task engine.

Those mechanics must be classified and migrated deliberately. A declarative stewardship format that reimplements them as workflow syntax would not simplify the architecture.

## Migration Objective

The target is not a specific line-count deletion.

The target is:

```text
zero domain-specific orchestration branches in generic runtime code
```

A domain should be introduced by loading a compiled stewardship package and executable adapters, not by adding another workflow-specific executor path.

At the same time, generic runtime behavior remains code:

- durable continuation
- idempotency
- task readiness
- dispatch
- retry scheduling
- artifact persistence
- event emission
- cancellation
- repair application
- approval waiting

## Existing Workflow Responsibilities

The current workflow surface combines several concerns that belong to different target authorities.

Typical workflow package content includes:

- accepted target types
- required runtime fields
- seed artifacts
- traversal expansion
- repeated regions
- stage chains
- turn ordering
- prompts
- output types
- gates
- retries
- artifact persistence policy
- prerequisite propagation
- failure behavior

These are not one future abstraction.

## Classification Rule

Every workflow responsibility should move to exactly one of five destinations.

| Destination | Responsibility |
|---|---|
| Stewardship package | domain vocabulary, mandate, objective, known method, authority, and outcome semantics |
| `meld-lang` value | proposition, goal, operator, effect, composition, and method structure |
| Capability | executable preparation, provider call, normalization, and domain transform |
| Execution runtime | generic scheduling, continuation, retry, artifact, dispatch, and lifecycle mechanics |
| Compatibility layer | temporary support for behavior not yet lowered into cognitive-runtime structures |

A concern must not remain simultaneously authoritative in both workflow and cognitive-runtime representations.

## Semantic Mapping

| Existing workflow concept | Stewardship target | Notes |
|---|---|---|
| package id | stewardship package or action-module id | Package identity becomes versioned and content-addressed |
| workflow id | method id or compatibility-method id | A workflow is one known decomposition, not the standing application |
| accepted targets | charter scope template | Valid assignment binding |
| runtime fields | assignment parameters, authority grant, provider/capability binding | Domain and execution concerns split |
| trigger | objective breach, method precondition, or explicit invocation | Depends on semantic meaning |
| seed artifact | typed method input or assignment binding | Must use declared artifact type |
| expansion template | parameterized composition or planner-generated task-network mutation | No longer a top-level application construct |
| traversal strategy | observation capability, domain query, or method | Depends on whether traversal gathers evidence or structures work |
| repeated region | parameterized composition over selected subjects | Planner or method library owns expansion |
| stage chain | task capability graph | Existing task compiler can continue to lower this |
| turn | method step or task | A prompt turn is not necessarily an architectural unit |
| prompt reference | capability asset | Prompt is executable adapter data |
| output type | artifact type | Declared in domain or action module |
| gate | capability validator, method guard, belief evidence, or outcome proposition | Must be classified by meaning |
| retry limit | runtime continuation or repair policy | Not domain control flow by default |
| output persistence | artifact contract | Capability and task runtime own persistence mechanics |
| prerequisite | composition edge or task-network dependency | Typed dependency rather than workflow-specific link |
| final output | expected effect plus outcome contract | Task success does not prove objective restoration |
| target agent | stewardship assignment and Agent perspective | Bound through charter and assignment |

## Gate Classification

Workflow gates are especially likely to conflate layers.

Every gate should be classified as one of:

### Syntactic validator

Examples:

- valid JSON
- required fields present
- schema conformance

Owner: capability or artifact validation.

### Task completion criterion

Examples:

- provider returned an artifact
- file write succeeded

Owner: task and execution runtime.

### Method guard

Examples:

- continue only when evidence artifact exists
- choose branch when an observation has a particular value

Owner: `meld-lang` conditions and composition edges.

### Belief evidence

Examples:

- verification report supports documentation freshness
- benchmark establishes regression

Owner: observation mapping and belief family.

### Objective or outcome criterion

Examples:

- generated documentation is current and accepted
- service health has entered the restore region

Owner: stewardship objective or outcome contract.

A gate should not remain a generic boolean when its semantic category is known.

## Retry And Repair Classification

Retry is not one mechanism.

### Mechanical retry

Used for transient transport or provider failure.

Owner: execution runtime.

### Capability repair

Used when output fails schema or normalization.

Owner: capability and task repair policy.

### Method repair

Used when a task cannot complete but an alternate decomposition may achieve the same effect.

Owner: planning loop and method library.

### Epistemic revision

Used when an action succeeds mechanically but outcome evidence does not restore the objective.

Owner: world model, steward charter, and a new episode or goal mutation.

Workflow profiles currently encode several of these through one retry limit. Migration should separate them.

## Migration Strategy

The migration proceeds by semantic elevation rather than replacement.

```text
working workflow
    ↓ classify responsibilities
compatibility method
    ↓ add standing objective and assignment
stewardship package using workflow-backed method
    ↓ lower workflow stages into ordinary task composition
native method and task-network execution
    ↓ remove obsolete workflow-only facade
```

At each stage, one representation is authoritative for each concern.

## Phase 0: Freeze And Characterize

Before changing execution:

- record current workflow behavior with integration tests
- capture event sequences and persisted state
- identify all runtime inputs and produced artifacts
- identify gate and retry behavior
- identify direct-executor paths that bypass task compilation
- establish latency, cost, and failure baselines

The purpose is behavioral parity, not preserving every internal API.

## Phase 1: Wrap A Workflow As A Known Method

A current workflow can be imported as an opaque compatibility method.

```text
stewardship objective
    ↓
method selection
    ↓
compatibility method: execute workflow profile
    ↓
workflow runtime
    ↓
task and artifact events
    ↓
outcome verification
```

The stewardship package owns:

- standing responsibility
- subject scope
- belief and breach semantics
- action authorization
- outcome verification

The workflow remains authoritative only for its internal sequence and continuation.

This phase proves that workflow execution can operate below the stewardship layer without being the application model.

## Phase 2: Expose Typed Inputs And Outputs

Replace implicit runtime fields and untyped profile values with:

- assignment bindings
- declared artifact types
- capability requirements
- method parameters
- explicit authority class

The compatibility method remains, but its boundary becomes identical to other `meld-lang` methods.

## Phase 3: Lower Stage Chains Into Task Compositions

Existing stage chains such as:

```text
prepare
→ provider execution
→ finalize
```

become ordinary task capability graphs.

Prompt resolution, provider invocation, and normalization remain capability concerns.

The workflow runtime stops owning stage-level semantics after parity is proven.

## Phase 4: Lower Turns Into Methods And Compositions

A turn sequence becomes one or more known methods.

```text
workflow turns
    ↓
method composition steps
    ↓
task-network nodes and dependencies
```

Fixed structure may remain fixed. The migration does not require the planner to rediscover every proven sequence.

The gain is that the same method can be:

- selected by a standing objective
- composed with other methods
- repaired or replaced
- shared across stewardship packages
- inspected as typed planning data

## Phase 5: Move Gates To Their Semantic Owners

Syntactic gates move to capabilities.

Method guards move to `meld-lang` propositions and graph edges.

Evidence gates become evidence mappings.

Outcome gates become outcome contracts.

Workflow-generic gate evaluation should shrink as classifications become explicit.

## Phase 6: Replace Workflow Lifecycle With Task-Network Continuation

Workflow thread state maps into generic continuation concepts:

| Workflow state | Target runtime state |
|---|---|
| pending | task or composition not ready |
| running | task-network node active |
| waiting | dependency, observation, approval, or resource wait |
| retrying | continuation or repair action scheduled |
| completed | method/task completed mechanically |
| failed | task or method failure fact |
| cancelled | task-network cancellation and compensation state |

Stewardship episode state remains separate from task-network lifecycle.

A completed workflow-backed method may leave the episode in `Verifying` or `Open` if outcome evidence does not restore the objective.

## Phase 7: Remove Workflow-Only Facades

A workflow path can be removed only when:

- all callers enter through stewardship assignment, goal, method, or task APIs
- task-network continuation provides equivalent durability
- artifacts and events preserve replay
- repair and retry semantics are owned elsewhere
- no package depends on workflow-only fields

The remaining code may retain historical origin but should be named by its generic runtime responsibility.

## Docs Writer Migration

The docs writer is the first proposed migration target because it has:

- a current workflow package
- a known multi-turn method
- task/capability integration
- a first belief family for content freshness
- reversible generated artifacts
- an observable verification path

### Current behavior

Conceptually:

```text
select target
→ expand directories bottom-up
→ for each node:
    prepare
    → evidence gather
    → verification
    → structure
    → style refinement
    → finalize and persist
→ propagate child output to parent context
```

### Stewardship decomposition

#### Domain

```text
workspace node
source content
context frame
documentation artifact
parent-child relationship
```

#### Observations

```text
workspace content changed
node added or removed
context frame written
verification report emitted
```

#### Belief

```text
content_freshness(subject)
```

The belief family consumes source churn, existing artifact state, and verification evidence.

#### Charter

```text
maintain current documentation for selected workspace subtree
```

The charter remains active after one generation run.

#### Objective

Illustrative policy:

```text
breach:
    content_freshness below 0.75
    or freshness evidence indeterminate beyond maximum age

restore:
    content_freshness above 0.95
    and verification evidence current
    and generated artifact available
```

#### Known method

The existing four-turn sequence remains one method:

```text
docs_writer.bottom_up_generate
```

The method can initially dispatch the current workflow profile.

#### Capabilities

```text
context_generate_prepare
provider_execute_chat
context_generate_finalize
output schema validation
frame persistence
```

#### Authority

Illustrative first slice:

```text
autonomous:
    inspect workspace
    run generation method
    write generated context frame

approval required:
    modify tracked source documentation
    open external pull request

prohibited:
    merge or publish without grant
```

#### Outcome

Mechanical success:

```text
frame written
```

Stewardship success:

```text
verification evidence accepted
content_freshness belief enters restore region
```

The distinction prevents a generated but inaccurate README from closing the episode.

## Docs Writer Compatibility Package

Illustrative source shape:

```yaml
package:
  id: software.documentation-steward
  version: 0.1.0

charters:
  - id: workspace-documentation
    scope:
      root_type: software.workspace_node
      parameters:
        - root_node

    concerns:
      - id: maintain-content-freshness
        belief_family: content.freshness
        objective:
          breach:
            holds:
              subject: $root_node
              dimension: content_freshness
              condition:
                below: 0.75
          restore:
            holds:
              subject: $root_node
              dimension: content_freshness
              condition:
                above: 0.95

        methods:
          - docs_writer.workflow_compatibility_v1

outcomes:
  - id: documentation.restored
    action_class: documentation.generate
    verification:
      - documentation.verification_completed
      - context.frame_written
```

The example does not freeze the package schema.

## Ownership Mapping

| Current subsystem | Approximate strategic disposition | Target owner |
|---|---|---|
| turn executor | lower into task-network dispatch | execution runtime |
| gate evaluation | split by semantic category | capability, belief, method, or outcome |
| state persistence | retain and generalize | continuation runtime |
| prompt resolution | retain | capability |
| generation orchestration | lower into task graph | task and execution runtime |
| direct executor | compatibility, then remove | task network |
| lifecycle state machine | generalize | execution runtime |
| event emission | retain and enrich | events/execution |
| retry and failure | split mechanical retry from repair | runtime and planning |
| normalization | retain | capability |
| workflow package schema | replace as application truth | stewardship package plus methods |

## Coherence Rule

At every migration stage:

> One execution concern has one active authority.

Examples:

- if workflow owns turn retry, planner repair must not also retry the same unit
- if an outcome contract owns restoration, a workflow success gate must not close the stewardship episode
- if task-network continuation owns persistence, workflow thread persistence becomes read-only migration state
- if a capability owns JSON validation, a generic workflow gate must not independently reinterpret the schema

Hybrid implementation is permitted. Hybrid authority is not.

## Tests

Migration tests should include:

### Behavioral parity

- same accepted targets
- same bottom-up ordering
- same artifact schemas
- same prompt assets
- same generated frame persistence
- same retry outcome under transient provider failure

### Stewardship semantics

- no episode when freshness remains acceptable
- observation goal when evidence is stale or indeterminate
- generation goal when breach is established
- episode remains open after mechanical success without verification
- external restoration cancels or prunes redundant work
- denied authority prevents tracked-file modification

### Replay

Given the same event history, package hash, and external artifacts:

- the same belief revision is produced
- the same objective breach is identified
- the same compatibility method is selected
- the same authority decision is made

### Migration compatibility

- existing workflow state can be read until active runs complete
- new assignments do not create new workflow-only state after the cutover boundary
- old event history remains interpretable

## Deletion Criteria

Workflow-only code is removable when:

- no production package uses workflow profiles as top-level behavior
- all active runs have completed or migrated
- method loading and task-network execution cover required sequencing
- task continuation covers durable state
- capability validation covers output gates
- planner repair covers non-mechanical retry
- outcome contracts cover domain success
- compatibility event readers remain available for historical replay

## Risks

### Recreating workflows in YAML

The largest risk is moving turn control flow into a more elaborate package file. Package review should reject procedural declarations that are not reusable methods.

### Premature planner dependence

The first package should use known methods and existing execution. It should not wait for a complete general planner.

### Semantic drift

Running workflow and stewardship representations in parallel can disagree. Sequence-bound cutovers and one-authority ownership are required.

### Event incompatibility

Historical workflow events may lack package, method, or objective references. Migration projections must preserve provenance without rewriting history.

### False restoration

Mechanical completion can be mistaken for stewardship success. Outcome verification must be introduced before episodes become authoritative.

## Success Criteria

The migration succeeds when:

- the docs writer is instantiated from a stewardship assignment
- its standing objective remains visible independently of active work
- the current workflow can run as a compatibility method
- task and artifact events close the same factual loop
- an outcome contract, not workflow completion, determines restoration
- subsequent implementation can lower the method without changing the package charter
- new stewardship packages do not require new workflow runtime branches

## Non-Goals

This migration does not require:

- deletion of all workflow code in one change
- dynamic planning of every fixed sequence
- replacing prompts with declarative expressions
- encoding retries and persistence in the package language
- changing the behavior of existing workflows before parity tests exist
- renaming every workflow-derived type immediately
