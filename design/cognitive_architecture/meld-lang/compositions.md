# Compositions

Date: 2026-05-18
Status: active
Scope: Composition graph, Step, Edge, structural validation

## Thesis

A Composition is a directed graph of steps with typed edges. It is the language representation of a semantic theory of action. Strategy constructs episode candidates and Execution compiles an Agent-authorized candidate into a task-network subgraph.

Compositions are data. The current Strategy slice constructs one by instantiating a configured Method template. Execution preserves authorization lineage but does not reinterpret semantic meaning.

## Composition

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Composition {
    pub steps: Vec<Step>,
    pub edges: Vec<Edge>,
}
```

### Step

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Step {
    /// Identity within this composition. Referenced by edges.
    pub step_id: String,

    /// What this step does.
    pub kind: StepKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StepKind {
    /// An operator to resolve and dispatch.
    Op(Operator),

    /// A sub-goal represented in the shared language.
    Goal(Proposition),
}
```

### Edge

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Edge {
    /// Source step.
    pub from: String,

    /// Target step.
    pub to: String,

    /// What kind of dependency this edge represents.
    pub kind: EdgeKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EdgeKind {
    /// The source step must complete before the target step can begin.
    /// Pure ordering. No data transfer.
    Ordering,

    /// The source step produces an artifact of this type that the
    /// target step consumes. Satisfies the target operator's
    /// Resolution.requires_inputs for this artifact type.
    DataFlow {
        artifact_type: String,
    },

    /// The target step is activated only if the condition holds
    /// against a field of the source step's output.
    /// If the condition does not hold, the target step and its
    /// downstream subgraph are pruned.
    Conditional {
        field_path: String,
        guard: Condition,
    },

}
```

## Composition Design Rules

- A composition is a DAG. Cycles are structural errors caught by validation.
- Steps with no incoming edges are roots. A composition may have multiple roots (parallel starting points).
- Steps with no outgoing edges are leaves. Their effects are the composition's net contribution to world state.
- `StepKind::Goal` preserves recursive language structure. A Strategy proposal resolves each subgoal to an exact child Composition or an exact Method revision, bindings, and expanded hash. Agent judgment authorizes the complete candidate. Execution may inline or nest that path but must not search for novel meaning.
- `EdgeKind::DataFlow` carries artifact type identity, not artifact content. The actual artifact flows through the task network's artifact repository. The edge declares the dependency.
- `EdgeKind::Conditional` uses the same `Condition` type as propositions. The guard is evaluated against a field of the source step's output artifact, not against world state. This is how observation-then-branch works: an evaluate step produces a decision artifact, a conditional edge checks a field of that artifact, and downstream steps activate or prune based on the result.
- `EdgeKind::EvidenceAdmission` waits for an authoritative evidence-admission verdict that binds the prospective contract and exact artifact content. Artifact existence alone never satisfies it.

## Structural Validation

The language crate provides `validate()` for structural soundness of a composition. This is a pure function with no external dependencies.

```rust
pub fn validate(composition: &Composition) -> ValidationResult;

pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
}
```

### Validation Errors

```rust
pub enum ValidationError {
    /// An edge references a step_id not present in the composition.
    DanglingEdge {
        from: String,
        to: String,
        missing: String,
    },

    /// A step_id appears more than once.
    DuplicateStepId(String),

    /// The composition contains a cycle.
    CycleDetected {
        involved_steps: Vec<String>,
    },

    /// A DataFlow edge's artifact type does not appear in the source
    /// operator's Resolution.requires_outputs.
    ArtifactSourceMismatch {
        edge_from: String,
        edge_to: String,
        artifact_type: String,
    },

    /// A Conditional edge references a source step that is a Goal
    /// (goals do not produce artifacts to guard on).
    InvalidGuardOnGoalStep {
        edge_from: String,
    },

    /// A Variable term appears in a position that must be ground
    /// (e.g., after substitution was expected to have resolved it).
    UnboundVariable {
        step_id: String,
        variable: String,
    },
}
```

### Validation Warnings

```rust
pub enum ValidationWarning {
    /// A non-root step has no incoming edges.
    DisconnectedStep(String),

    /// An operator's effects do not contribute to any downstream
    /// precondition or to any Goal step in the composition.
    UnusedEffects {
        step_id: String,
    },

    /// Aggregated cost exceeds a soft threshold (informational).
    HighCost {
        estimated: CostEstimate,
    },
}
```

### What Validation Does NOT Check

- Operator resolution to capabilities. The language crate has no access to the capability catalog. Execution checks this during runtime compile.
- Whether the Composition has semantic support to achieve a Goal. Strategy proves this through authoritative projections and obligation discharge paths.
- Whether Operator preconditions have authoritative semantic support. Strategy validates this before proposal. Execution rechecks current applicability mechanically.
- Artifact schema version compatibility between producers and consumers. Execution's task compiler checks this.

## Composition Examples

### Single observation (simplest possible)

```rust
Composition {
    steps: vec![
        Step {
            step_id: "observe".into(),
            kind: StepKind::Op(Operator {
                operator_id: "observe".into(),
                preconditions: vec![
                    Proposition::Accessible { scope: Term::Variable("?node".into()) },
                ],
                effects: vec![
                    Effect::Assert(Proposition::Exists {
                        scope: Term::Variable("?node".into()),
                        artifact_type: Term::ArtifactType("change_summary".into()),
                    }),
                ],
                cost: CostEstimate { time_ms: 5_000, money_microdollars: 0, provider_calls: 0 },
                resolution: Resolution {
                    requires_inputs: vec![],
                    requires_outputs: vec![
                        SlotConstraint { artifact_type_id: "change_summary".into(), required: true },
                    ],
                    scope_kind: Some("filesystem".into()),
                    tags: vec!["observe".into(), "git".into()],
                    specific: None,
                },
            }),
        },
    ],
    edges: vec![],
}
```

### Observation with conditional action

```rust
Composition {
    steps: vec![
        Step { step_id: "gather".into(), kind: StepKind::Op(/* observe operator */) },
        Step { step_id: "evaluate".into(), kind: StepKind::Op(/* evaluate operator */) },
        Step { step_id: "write".into(), kind: StepKind::Op(/* transform operator */) },
    ],
    edges: vec![
        Edge {
            from: "gather".into(),
            to: "evaluate".into(),
            kind: EdgeKind::DataFlow { artifact_type: "change_summary".into() },
        },
        Edge {
            from: "evaluate".into(),
            to: "write".into(),
            kind: EdgeKind::Conditional {
                field_path: "should_execute".into(),
                guard: Condition::Equals(Term::Literal(Literal::Bool(true))),
            },
        },
    ],
}
```

### Ground recursive Composition after observation

Strategy must first observe and ground the finite child set. The resulting proposal contains concrete child subjects and exact subgoal realizations.

```rust
Composition {
    steps: vec![
        Step {
            step_id: "class_1".into(),
            kind: StepKind::Goal(Proposition::Exists {
                scope: Term::Object(class_1_ref),
                artifact_type: Term::ArtifactType("podcast_script_final".into()),
            }),
        },
        // Every observed class is represented by another concrete Goal step.
    ],
    edges: vec![],
}
```

The `Goal` steps are recursive. Strategy binds each to an exact candidate realization before Agent authorization. A future runtime-produced child set requires another bounded Strategy turn after observation or a finite pre-authorized branch inventory.

## Relationship to Task Network

A validated, resolved composition lowers to the task network through execution's runtime compile step:

| Composition concept | Task network equivalent |
| --- | --- |
| `Step(Op(operator))` | `TaskEntry` in `TaskNetworkGraph` |
| `Step(Goal(proposition))` | exact authorized sub-plan expansion and injection |
| `Edge(Ordering)` | `TaskDependencyEdge { kind: Ordering }` |
| `Edge(DataFlow)` | `TaskDependencyEdge { kind: DataFlow { artifact_type } }` |
| `Edge(Conditional)` | `TaskDependencyEdge { kind: Conditional { guard } }` |
| `Edge(EvidenceAdmission)` | authoritative admission dependency with prospective contract and content binding |
| `Operator.resolution` resolved | `CompiledTaskRecord` via task compiler |

The composition is the plan in the language. The task network is the plan in execution. The runtime compile step bridges them.

## Read With

- [Lang Primitives](primitives.md)
- [Operators and Resolution](operators.md)
- [Goals and Methods](goals_and_methods.md)
- [Task Network](../execution/task_network.md)
- [Planning Pipeline](../execution/planning/planning_pipeline.md)
