# World State and Evaluation

Date: 2026-05-18
Status: active
Scope: WorldState, evaluate(), gap detection, effect application

## Thesis

`WorldState` is the shared representation of what the world model currently believes, expressed as a set of ground propositions in the shared language. It is published by the world model's planner-facing projection and consumed by the planning loop for goal evaluation, precondition checking, and gap detection.

The language crate owns the `WorldState` type and all pure evaluation operations over it. The world model owns the construction of `WorldState` from its internal belief, graph, causation, and regime state. Execution owns the consumption of `WorldState` for planning decisions. The language crate is the bridge.

## WorldState

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorldState {
    propositions: Vec<Proposition>,
}
```

### Construction

```rust
impl WorldState {
    /// Create from a set of ground propositions.
    /// Returns Err if any proposition contains a Term::Variable.
    pub fn new(propositions: Vec<Proposition>) -> Result<Self, GroundingError>;

    /// Create an empty world state.
    pub fn empty() -> Self;
}

pub struct GroundingError {
    pub index: usize,
    pub variable: String,
}
```

A world state contains only ground propositions. The grounding check ensures that every `Term` in every proposition is concrete — no `Term::Variable` allowed. `Term::Derived` is also rejected in world state construction, as derived terms are only meaningful during composition execution.

### Query

```rust
impl WorldState {
    /// Does this world state satisfy the given proposition?
    pub fn satisfies(&self, query: &Proposition) -> bool;

    /// All current ground propositions.
    pub fn propositions(&self) -> &[Proposition];

    /// Find all propositions matching a pattern (proposition with variables).
    /// Returns each match as a Bindings.
    pub fn query(&self, pattern: &Proposition) -> Vec<Bindings>;
}
```

`satisfies()` handles compound propositions:
- `All(children)`: true if all children are satisfied.
- `Any(children)`: true if at least one child is satisfied.
- `Not(child)`: true if child is not satisfied. `Indeterminate` evaluates to `false` under `Not` (conservative: if we don't know, we don't negate).

`query()` enables the planning loop to search world state. "Find all nodes where docs_freshness confidence is below 0.5" is a pattern query that returns bindings for each matching object.

### Effect Application

```rust
impl WorldState {
    /// Apply effects to produce a new world state.
    /// Effects are applied in order. The result is deterministic.
    /// Returns Err if any effect contains a Term::Variable.
    pub fn apply(&self, effects: &[Effect]) -> Result<WorldState, GroundingError>;
}
```

Effect application rules:

- `Effect::Assert(proposition)`: If no equivalent proposition exists, add it. If an equivalent already exists, no-op (idempotent).
- `Effect::Retract(proposition)`: Remove the first proposition that matches. If none matches, no-op.
- `Effect::Update { subject, dimension, value }`: Remove any existing `Proposition::Holds` for this subject and dimension, then add a new `Proposition::Holds` with the given value wrapped as `Condition::Equals(value)`.
- Effects are applied sequentially. The order matters when `Retract` and `Assert` target the same proposition.
- The input `WorldState` is not mutated. A new `WorldState` is returned.

### Proposition Matching

Two ground propositions match if:

- They have the same variant (`Holds`/`Exists`/`Accessible`/`Related`).
- Their terms are equal (`Term::Object(a) == Term::Object(b)` iff `a == b`, etc.).
- For `Holds`: subject and dimension match. Condition matching depends on context:
  - For `satisfies()`: the world state proposition's condition is evaluated against the query condition (see Evaluation below).
  - For `Retract`: the world state proposition's subject and dimension match. Condition is not compared — retract removes the proposition regardless of its condition value.
  - For `Assert` idempotency: full structural equality.

## Evaluation

```rust
/// Evaluate a proposition against a world state.
/// Returns a three-valued result.
pub fn evaluate(state: &WorldState, proposition: &Proposition) -> EvalResult;

#[derive(Debug, Clone, PartialEq)]
pub enum EvalResult {
    /// The proposition holds in the world state.
    Satisfied,

    /// The proposition does not hold. The gap contains the
    /// unsatisfied sub-propositions.
    Unsatisfied {
        gap: Vec<Proposition>,
    },

    /// The proposition references dimensions or objects not present
    /// in the world state. Not false — unknown.
    Indeterminate {
        missing: Vec<Term>,
    },
}
```

### Evaluation Rules

**`Holds { subject, dimension, condition }`**

1. Find all propositions in the world state where subject and dimension match.
2. If none found: return `Indeterminate { missing: [dimension for subject] }`. The world model has not asserted anything about this dimension for this subject.
3. If found: evaluate the condition against the stored value.
   - The world state stores propositions with `Condition::Equals(value)` as the canonical form for known values. The query condition is compared against this stored value.
   - `Condition::Above(threshold)`: satisfied if stored value > threshold.
   - `Condition::Below(threshold)`: satisfied if stored value < threshold.
   - `Condition::Equals(expected)`: satisfied if stored value == expected.
   - `Condition::In(set)`: satisfied if stored value is in the set.
   - `Condition::Within(range)`: satisfied if stored value < range (temporal: age within limit).
   - `Condition::Exceeds(range)`: satisfied if stored value > range (temporal: age exceeds limit).
   - `Condition::Present`: satisfied if any proposition exists for this subject and dimension.
   - `Condition::Absent`: satisfied if NO proposition exists for this subject and dimension. (Opposite of `Present`.)

**`Exists { scope, artifact_type }`**

1. Find any `Exists` proposition in the world state with matching scope and artifact type.
2. If found: `Satisfied`.
3. If not found: `Unsatisfied { gap: [the Exists proposition] }`.
4. `Exists` is never indeterminate — artifact existence is a binary fact.

**`Accessible { scope }`**

1. Find any `Accessible` proposition in the world state with matching scope.
2. If found: `Satisfied`.
3. If not found: `Unsatisfied { gap: [the Accessible proposition] }`.

**`Related { src, relation, dst }`**

1. Find any `Related` proposition in the world state with matching src, relation, and dst.
2. If found: `Satisfied`.
3. If not found: `Unsatisfied { gap: [the Related proposition] }`.

**`All(children)`**

1. Evaluate each child.
2. If all `Satisfied`: `Satisfied`.
3. If any `Indeterminate`: `Indeterminate` (propagate the missing terms).
4. Otherwise: `Unsatisfied { gap: union of all children's gaps }`.

**`Any(children)`**

1. Evaluate each child.
2. If any `Satisfied`: `Satisfied`.
3. If all `Unsatisfied`: `Unsatisfied { gap: [the Any proposition itself] }`.
4. If any `Indeterminate` and none `Satisfied`: `Indeterminate`.

**`Not(child)`**

1. Evaluate child.
2. If child `Satisfied`: `Unsatisfied { gap: [the Not proposition] }`.
3. If child `Unsatisfied`: `Satisfied`.
4. If child `Indeterminate`: `Unsatisfied` (conservative — do not assert negation of unknown).

### Gap Detection

```rust
impl WorldState {
    /// What propositions in the query are not satisfied?
    /// Returns the sub-propositions that are Unsatisfied or Indeterminate.
    pub fn gap(&self, query: &Proposition) -> Vec<Proposition>;
}
```

`gap()` is a convenience over `evaluate()`. It flattens the `Unsatisfied` and `Indeterminate` sub-propositions into a list. Strategy may use the typed result as one input to semantic construction. A gap does not itself select an Operator or action path.

### Indeterminate vs. Unsatisfied

This distinction is critical for world-model and Strategy behavior:

- **Unsatisfied**: the world model has asserted a value for this dimension, but it does not meet the condition.
- **Indeterminate**: the world model has not asserted anything about this dimension for this subject.

Neither result prescribes action. Strategy may propose observation, intervention, reuse, or abstention from the full authoritative context. The Agent decides whether to authorize the proposal.

Example:
- Goal: "confidence in docs_freshness for node X is above 0.7"
- World state contains: `Holds { subject: node_X, dimension: "docs_freshness", condition: Equals(0.3) }`
- Evaluation: `Unsatisfied` — confidence is 0.3, below 0.7.

- Goal: "confidence in docs_freshness for node X is above 0.7"
- World state contains no proposition about docs_freshness for node X.
- Evaluation: `Indeterminate` — no belief exists.

## World State Construction by the World Model

The world model's planner-facing projection constructs `WorldState` from its internal layers:

```rust
// In meld-world-model, constructing WorldState for the planning loop:

fn project_world_state(&self, perspective: &PerspectiveKey) -> WorldState {
    let mut propositions = Vec::new();

    // From belief layer: confidence, freshness, observation status
    for belief_view in self.belief_views(perspective) {
        propositions.push(Proposition::Holds {
            subject: Term::Object(belief_view.subject.clone()),
            dimension: Term::Dimension(belief_view.dimension.clone()),
            condition: Condition::Equals(
                Term::Literal(Literal::Number(belief_view.confidence))
            ),
        });
        // Freshness as a separate dimension
        propositions.push(Proposition::Holds {
            subject: Term::Object(belief_view.subject.clone()),
            dimension: Term::Dimension(format!("{}_freshness_age", belief_view.dimension)),
            condition: Condition::Equals(
                Term::Literal(Literal::Duration(belief_view.evidence_age))
            ),
        });
    }

    // From graph layer: scope accessibility, relationships
    for node in self.accessible_nodes(perspective) {
        propositions.push(Proposition::Accessible {
            scope: Term::Object(node.clone()),
        });
    }

    // From execution outcomes: artifact existence
    for artifact in self.known_artifacts(perspective) {
        propositions.push(Proposition::Exists {
            scope: Term::Object(artifact.scope.clone()),
            artifact_type: Term::ArtifactType(artifact.artifact_type_id.clone()),
        });
    }

    WorldState::new(propositions).expect("world model projects ground propositions only")
}
```

The world model owns the translation from internal belief state to the shared proposition language. The language crate provides the types. The world model provides the semantics.

## World State Consumption by Execution

```rust
// Pure evaluation produces typed facts. It does not construct work.

fn evaluate_goal(goal: &Goal, state: &WorldState) -> GoalEvaluationFact {
    match evaluate(state, &goal.target) {
        EvalResult::Satisfied => GoalEvaluationFact::AppearsSatisfied,

        EvalResult::Indeterminate { missing } =>
            GoalEvaluationFact::Indeterminate { missing },

        EvalResult::Unsatisfied { gap } =>
            GoalEvaluationFact::Unsatisfied { gap },
    }
}
```

Agent satisfaction curation may consume the satisfied result. Strategy may consume indeterminate and unsatisfied results when constructing a proposal. Execution uses evaluation only for no-op detection and current applicability of an already authorized candidate. It does not construct observation or action work from the result.

## Read With

- [Lang Primitives](primitives.md)
- [Goals and Methods](goals_and_methods.md)
- [Lang Requirements](requirements.md)
- [World Model Planner](../world_model/planner/README.md)
- [Execution Domain](../execution/README.md)
