# Task Expansion Plan

Date: 2026-04-05
Status: active
Scope: coherent plan tying compile-shape integration tests, dynamic task expansion, and docs-writer template extraction into one implementation path

## Intent

Define the next implementation path for dynamic task growth without building the full task-network first.

The core position is:

- traversal remains a capability
- prerequisite injection is a compiler concern
- task growth can happen after capability success through a structured expansion artifact
- the integration tests define the required graph shape before generic authoring lands

## Why This Plan Exists

The current docs-writer template still carries logic that should not remain package-specific forever.

Three concerns are currently mixed:

- traversal discovery
- prerequisite dependency injection
- package-specific graph expansion

At the same time, the existing compile-shape integration tests already give us a strong specification target for bottom-up prerequisite chaining.

This plan uses those tests as the primary contract while introducing a generic `TaskExpansion` path.

## Core Model

### Structural rule

For a package such as docs-writer, the package does not directly say "run this parent after this child by code path".

It says:

- traversal strategy provides a relation set
- prerequisite rule selects producer regions and consumer regions over that relation set
- compiler injects concrete dependency edges

### Runtime rule

A task may start from a smaller graph, then expand after discovery work completes.

That means:

- a capability emits a structured task expansion artifact
- executor detects that artifact
- compiler or expansion applier derives the task delta
- task graph is appended with new capability instances and dependency edges
- readiness is reevaluated normally

This keeps control out of package-local graph growth.

## Integration Test Role

The integration tests in [task_bottom_up_compile_shape.rs](/home/jerkytreats/meld/tests/integration/task_bottom_up_compile_shape.rs) are now the primary shape contract.

They already express:

- filesystem tree input
- target scope input
- expected active node set
- expected cross-node prerequisite edges

These tests should drive all compiler work in this area.

### Rule

Before adding new compiler expansion behavior:

1. add or update an integration compile-shape case
2. make the expected dependency shape explicit in test data
3. implement only enough compiler or expansion logic to satisfy that case

This keeps the work generic and prevents docs-writer code from becoming the hidden specification.

## TaskExpansion

### Purpose

`TaskExpansion` is the structured mechanism by which capability output can request more graph structure.

It should not be an implicit boolean such as `compile_on_success`.

It should be an explicit artifact family and executor rule.

### Recommended first slice

The first slice should support:

- append-only new capability instances
- append-only new dependency edges
- deterministic instance id generation
- duplicate expansion rejection or idempotent replay

The first slice should not support:

- arbitrary graph deletion
- silent rewrite of completed capability instances
- hidden mutation of prior dependency edges

### Recommended shape

Conceptual request artifact:

```rust
struct TaskExpansionRequest {
    expansion_id: String,
    expansion_kind: String,
    content: serde_json::Value,
}
```

For the docs-writer path, the first useful expansion kind is:

- `traversal_prerequisite_expansion`

Its content should describe:

- traversed node set or batches
- repeated region template to instantiate
- prerequisite injection rules such as direct child final output into parent first stage

## Compiler Responsibilities

The compiler needs to grow in one specific direction:

- accept structural relation input from traversal
- accept prerequisite templates from expansion content
- derive concrete dependency edges over instantiated regions

This is not docs-writer specific.
It is generic relation-based dependency injection.

### Required generic concepts

- traversal relation set
- repeated region template
- producer selector
- consumer selector
- prerequisite relation selector
- deterministic capability instance id generation

### Naming posture

Do not name this feature around filesystem parent and child semantics.

Use relation-neutral language such as:

- prerequisite node
- dependent node
- upstream relation
- downstream relation

Traversal strategy may be `bottom_up`, `top_down`, or something else later.
Prerequisite semantics are separate from traversal naming.

## Docs-writer Template Extraction Map

Current file: [docs_writer.rs](/home/jerkytreats/meld/src/task/templates/docs_writer.rs)

### Move out into capability or compiler tool belt

- traversal discovery
  already belongs to `merkle_traversal`
- cross-node prerequisite injection
  should move into compiler expansion logic
- active graph growth after traversal output
  should move into `TaskExpansion`

### Move out into task runtime support

- expansion artifact detection
- expansion application
- append-only compiled task delta merge

### Keep temporarily in package lowering

- docs-writer stage list
- prompt asset binding
- gate binding
- output-type binding
- region template shape

That package structure is still authored behavior, even if it is currently written in Rust.

## Ordered Implementation Steps

### Step 1

Keep growing compile-shape integration data.

Add more cases for:

- seeded existing outputs
- multiple branch trees
- traversal strategy variants once available
- exclusion or filtered traversal once supported

### Step 2

Define the `TaskExpansionRequest` artifact contract.

Add:

- artifact type id
- schema version
- executor detection rule
- duplicate protection rule

### Step 3

Teach executor to apply append-only expansion.

Add:

- expansion record persistence
- compiled task delta merge
- idempotent expansion guard
- readiness recomputation after expansion

### Step 4

Move cross-node prerequisite injection out of the docs-writer template and into compiler expansion logic.

Goal:

- template supplies region template and prerequisite template
- compiler creates the concrete edges

### Step 5

Reduce docs-writer template to package structure only.

Goal:

- no direct relation walking for dependency injection
- no package-local graph mutation logic
- no hidden traversal-to-edge code in the template

## Test Gates

The minimum gates for each step should be:

- `cargo fmt -- --check`
- `cargo check`
- `cargo test --test integration_tests task_bottom_up_compile_shape -- --nocapture`
- `cargo test --test integration_tests docs_writer_task -- --nocapture`
- `cargo test --test integration_tests task_compiler -- --nocapture`
- `cargo test --test integration_tests workflow_task_compatibility -- --nocapture`

When executor expansion lands, add:

- new integration tests for expansion idempotence
- new integration tests for append-only task delta merge
- new integration tests for failure of expansion capability and retained task consistency

## Exit Criteria

This plan is complete when:

- compile-shape integration data is the primary specification for prerequisite injection
- `TaskExpansion` exists as an explicit artifact-driven path
- executor can append graph structure safely after capability success
- docs-writer template no longer performs package-local prerequisite edge construction

## Bottom Line

The next coherent path is not "add more docs-writer logic".

It is:

- tests define the graph shape
- expansion artifacts define graph growth
- compiler injects relation-based prerequisite edges
- docs-writer template shrinks toward pure package structure
