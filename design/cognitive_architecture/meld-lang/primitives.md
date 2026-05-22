# Lang Primitives

Date: 2026-05-18
Status: active
Scope: Term, Proposition, Condition, Effect — the three primitives and their grammar

## Thesis

The language has three primitive types. Everything else is structure over them. The primitives define a small, stable grammar. The vocabulary — which objects, dimensions, values, and artifact types fill the grammar — is open-ended and runtime-composed.

## Term

A term is the atomic unit of reference. Every slot in the language that points at something in the world is a `Term`.

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Term {
    /// A reference to a domain object.
    /// Identity shared with meld-events.
    Object(DomainObjectRef),

    /// A reference to a belief dimension.
    /// String-identified. Semantics owned by the world model, not the language.
    Dimension(String),

    /// A reference to an artifact type.
    /// String-identified. Semantics owned by the capability catalog, not the language.
    ArtifactType(String),

    /// A concrete value.
    Literal(Literal),

    /// An unbound variable for pattern matching and composition templates.
    /// Binds during unification. Substituted before execution.
    Variable(String),

    /// A value derived from a step's output at runtime.
    /// Resolved during composition execution, not during planning.
    Derived {
        source_step: String,
        field_path: String,
    },
}
```

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Literal {
    Bool(bool),
    Text(String),
    Number(f64),
    Duration(Duration),
}
```

### Term Design Rules

- `Dimension` and `ArtifactType` are string-identified, not enum variants. New dimensions and artifact types are vocabulary, not grammar. They require no language changes.
- `Variable` terms appear in patterns (method triggers, composition templates). They are replaced by concrete terms via `substitute()` before a composition enters execution.
- `Derived` terms are resolved at dispatch time, not at planning time. They enable data flow within a composition: "use the value of field X from the output of step Y."
- `Object` wraps `DomainObjectRef` directly. No additional indirection.
- `Literal::Number` uses `f64`. Confidence values, thresholds, and numeric belief dimensions are all `f64`. The language does not distinguish between them — that distinction is the world model's concern.
- `Literal::Duration` uses `std::time::Duration`. Freshness thresholds and temporal conditions use this.

## Proposition

A proposition is a typed statement about the world. The enum variants are the grammar — the finite set of statement shapes the evaluator understands.

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Proposition {
    /// A belief dimension of a subject satisfies a condition.
    /// The primary proposition type.
    Holds {
        subject: Term,
        dimension: Term,
        condition: Condition,
    },

    /// An artifact of a given type exists for a given scope.
    Exists {
        scope: Term,
        artifact_type: Term,
    },

    /// A scope is accessible for observation or action.
    Accessible {
        scope: Term,
    },

    /// A relationship holds between two domain objects.
    Related {
        src: Term,
        relation: Term,
        dst: Term,
    },

    /// All sub-propositions hold.
    All(Vec<Proposition>),

    /// At least one sub-proposition holds.
    Any(Vec<Proposition>),

    /// The sub-proposition does not hold.
    Not(Box<Proposition>),
}
```

### Proposition Design Rules

- `Holds` is the most common proposition. It covers confidence ("confidence in docs_freshness for node X is above 0.7"), freshness ("evidence for node X is within 7 days"), categorical state ("test_status for node X equals passing"), and observation status ("any belief exists for docs_freshness of node X").
- `Exists` is the simplest proposition — does an artifact of this type exist for this scope? No belief dimension, no condition. Used for "evidence has been gathered" or "output has been produced."
- `Accessible` is a precondition check — can the system reach this scope? Used by operators that need filesystem access, network access, or repository access.
- `Related` expresses structural relationships. Used for "node X is a child of node Y" or "artifact A was produced by task T."
- `All`/`Any`/`Not` provide standard boolean composition. Goals that require multiple conditions use `All`. Goals with alternative satisfaction paths use `Any`.
- The same `Proposition` type is used everywhere: world state assertions, goal targets, method triggers, operator preconditions, and effect arguments. There is no separate type for patterns. A proposition with `Term::Variable` in some positions is a pattern; a proposition with all concrete terms is ground.

## Condition

A condition is a comparison over a belief dimension's value. Conditions appear inside `Holds` propositions.

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Condition {
    /// subject.dimension > term
    Above(Term),

    /// subject.dimension < term
    Below(Term),

    /// subject.dimension == term
    Equals(Term),

    /// subject.dimension is one of the given values
    In(Vec<Term>),

    /// subject.dimension is within a range (typically temporal: freshness within 7 days)
    Within(Term),

    /// subject.dimension exceeds a range (typically temporal: staleness exceeds threshold)
    Exceeds(Term),

    /// Any value has been observed for this dimension. The dimension is not under-observed.
    Present,

    /// No value has been observed for this dimension. The dimension is under-observed.
    Absent,
}
```

### Condition Design Rules

- `Above` and `Below` are threshold comparisons. The `Term` is the threshold value, typically `Literal::Number`.
- `Equals` is exact match. The `Term` is the expected value. Supports `Literal::Text` for categorical dimensions, `Literal::Bool` for boolean dimensions, `Literal::Number` for numeric dimensions.
- `In` is set membership. Used for "test_status is one of [passing, flaky]."
- `Within` and `Exceeds` are range comparisons, typically temporal. "Freshness within 7 days" means the evidence age is less than 7 days. "Staleness exceeds 14 days" means the evidence age is greater than 14 days. The `Term` is the range bound, typically `Literal::Duration`.
- `Present` and `Absent` test observation status, not value. "Has the world model observed anything about this dimension?" This is the three-valued distinction: `Absent` means indeterminate, not false.
- New `Condition` variants are grammar changes. They should be added deliberately when the evaluator needs a new kind of comparison. Existing conditions cover threshold, equality, set, range, and observation status — sufficient for the first slice.

## Effect

An effect is a state change to the proposition space. Effects are what operators do to the world.

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Effect {
    /// Make a proposition true in the world state.
    Assert(Proposition),

    /// Remove a proposition from the world state.
    Retract(Proposition),

    /// Set a belief dimension to a specific value for a subject.
    Update {
        subject: Term,
        dimension: Term,
        value: Term,
    },
}
```

### Effect Design Rules

- `Assert` adds a ground proposition to the world state. If an equivalent proposition already exists, the assert is idempotent.
- `Retract` removes a matching proposition from the world state. If no matching proposition exists, the retract is a no-op.
- `Update` is a convenience for the common pattern of retracting an old `Holds` proposition and asserting a new one with a different value. The evaluator treats it as: retract any existing `Holds` for this subject and dimension, then assert a new `Holds` with the given value.
- Effects compose as a list. Applied in order, they produce a deterministic new world state. The order matters when `Retract` and `Assert` interact for the same proposition.
- Effect terms must be ground (no variables) when applied. The `apply_effects()` function rejects effects with unbound variables.

## Read With

- [Lang Overview](README.md)
- [Lang Requirements](requirements.md)
- [Operators and Resolution](operators.md)
- [Compositions](compositions.md)
- [Goals and Methods](goals_and_methods.md)
- [World State and Evaluation](world_state.md)
