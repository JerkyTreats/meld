# Goals and Methods

Date: 2026-05-18
Status: active
Scope: Goal specification, Method caching, pattern unification

## Thesis

A goal is a proposition the world model agent wants to become true. A method is a cached composition that the planning loop can reuse when a matching goal appears. Goals are the input to planning. Methods are an optimization within planning. Both are expressed entirely in the shared language.

Goals must be constructable at runtime by the world model agent. The agent observes belief divergence, interprets whether it matters (normative judgment), and formalizes the desired state as a proposition. Execution receives the goal and plans mechanically. The entire burden of meaning and intention lives in the world model. Execution never interprets why a goal exists.

Methods must be loadable at runtime from serialized files. New methods do not require recompilation. The planning loop must also function without methods — the world model agent or planning loop can construct compositions directly for novel situations.

## Goal

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Goal {
    /// Stable identity for this goal instance.
    pub goal_id: String,

    /// The agent that curated this goal.
    pub agent_id: String,

    /// What should hold in the world state.
    /// Expressed as a Proposition in the shared language.
    /// Execution evaluates this mechanically — no interpretation.
    pub target: Proposition,

    /// Operational priority.
    pub priority: GoalPriority,

    /// Why this goal exists. Carried for provenance and audit.
    /// Execution does not interpret the source — it is the world model's
    /// epistemic justification, legible to the agent, opaque to execution.
    pub source: GoalSource,

    /// Current lifecycle state.
    pub lifecycle: GoalLifecycle,
}
```

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GoalPriority {
    /// Lower number = higher urgency. 0 is most urgent.
    pub urgency: u32,

    /// Maximum acceptable cost. None means no ceiling.
    /// The planning loop rejects compositions whose aggregated
    /// cost exceeds any dimension of the ceiling.
    pub cost_ceiling: Option<CostEstimate>,
}
```

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GoalSource {
    /// Belief diverged from desired state. The world model agent
    /// detected that a belief dimension crossed a threshold.
    BeliefDivergence {
        dimension: String,
        observed: String,
        desired: String,
    },

    /// User directly requested this goal.
    /// The directive that seeded the requesting agent is durable as an
    /// independent record in the agent meta-layer. This field carries the
    /// directive text as lineage; a future refinement carries the directive
    /// id for a stable reference.
    UserDirected {
        directive: String,
    },

    /// Maintenance invariant. The goal should be continuously
    /// re-evaluated and re-established if it lapses.
    Maintenance {
        invariant_description: String,
    },

    /// Decomposed from a parent goal. The planning loop or a
    /// decompose operator produced this sub-goal.
    Decomposed {
        parent_goal_id: String,
    },
}
```

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GoalLifecycle {
    /// Proposed by the world model agent but not yet active.
    Proposed,

    /// Active. The planning loop should find or construct a composition.
    Active,

    /// Suspended. The planning loop ignores this goal until resumed.
    Suspended { reason: String },

    /// Satisfied. The goal's target proposition holds in the world state.
    Satisfied { at_seq: u64 },

    /// Abandoned. The world model agent determined this goal is no longer worth pursuing.
    Abandoned { reason: String },
}
```

### Goal Design Rules

- `Goal.target` is a `Proposition`. It uses the same type as world state assertions, operator preconditions, and method triggers. This is the single-type property that makes the language a shared substrate.
- Goals may contain `Term::Variable` only when used as method trigger patterns. A goal submitted to the planning loop for execution must have a ground target (all terms concrete). The planning loop rejects goals with unbound variables.
- `GoalSource` is carried for provenance and audit. Execution reads the `source` only for lineage tracking and explanation. It does not interpret the source to decide how to plan.
- `GoalLifecycle` transitions are persisted by execution through public goal APIs. The world model agent owns satisfaction curation for `Active` to `Satisfied` by evaluating projected world state and emitting a satisfy mutation only after `meld_lang::evaluate` returns `EvalResult::Satisfied`. Planning may mechanically observe `EvalResult::Satisfied`, but it does not own lifecycle mutation. `meld_lang::evaluate` remains pure.
- `GoalPriority.cost_ceiling` is optional. When present, the planning loop rejects any composition whose aggregated cost exceeds the ceiling on any dimension. When absent, the planning loop uses cost for method preference ordering but does not enforce a ceiling.

### Goal Construction by the World Model

The world model agent constructs goals at runtime:

```rust
// The world model agent detects stale documentation belief.
// It constructs a goal in the shared language.
let goal = Goal {
    goal_id: generate_id(),
    agent_id: "docs_agent".into(),
    target: Proposition::Holds {
        subject: Term::Object(node_ref.clone()),
        dimension: Term::Dimension("docs_freshness".into()),
        condition: Condition::Above(Term::Literal(Literal::Number(0.7))),
    },
    source: GoalSource::BeliefDivergence {
        dimension: "docs_freshness".into(),
        observed: "confidence 0.3, evidence 14 days stale".into(),
        desired: "confidence above 0.7".into(),
    },
    priority: GoalPriority {
        urgency: 5,
        cost_ceiling: Some(CostEstimate {
            time_ms: 120_000,
            money_microdollars: 100_000,
            provider_calls: 10,
        }),
    },
    lifecycle: GoalLifecycle::Active,
};
```

No predefined goal type. No enum variant for "docs freshness goal." The agent composed it from `Term`s and `Proposition`s. A goal for a completely novel concern — one that didn't exist when the system was compiled — would look identical in shape.

## Method

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Method {
    /// Stable identity for this method.
    pub method_id: String,

    /// When to consider this method.
    /// A Proposition with Term::Variable in bindable positions.
    /// If unify(trigger, goal.target) succeeds, the method is a candidate.
    pub trigger: Proposition,

    /// Additional conditions that must hold in the world state.
    /// Checked after trigger unification, with bindings substituted.
    pub preconditions: Vec<Proposition>,

    /// The pre-built composition template.
    /// Contains Term::Variable references that get substituted
    /// with bindings from trigger unification.
    pub composition: Composition,

    /// Net effects of the full composition.
    /// Used by the planning loop to verify that the method achieves
    /// the goal without expanding the full composition.
    pub net_effects: Vec<Effect>,

    /// Estimated cost of the full composition.
    pub cost: CostEstimate,

    /// Preference ordering. Lower = preferred when multiple methods match.
    pub preference: u32,
}
```

### Method Design Rules

- Methods are optional. The system must function without any methods. Methods are a performance optimization: known decompositions should not be re-derived every planning cycle.
- Methods are serializable. A method library is a directory of serialized method files loaded at runtime. New methods do not require recompilation.
- `Method.trigger` uses `Term::Variable` in positions that should bind against the goal. When `unify(method.trigger, goal.target)` succeeds, it produces `Bindings` that map variable names to concrete terms from the goal.
- `Method.preconditions` are checked after trigger unification. Bindings from the trigger are substituted into preconditions before evaluation against world state. This enables preconditions like "scope ?node must be accessible" where `?node` was bound from the trigger.
- `Method.composition` is a template. It contains `Term::Variable` references matching the trigger's variables. `substitute(composition, bindings)` produces a concrete composition ready for validation and runtime compilation.
- `Method.net_effects` allow the planning loop to check goal achievement at the method level without expanding the full composition. If the net effects, applied to the current world state, produce a state where the goal holds, the method achieves the goal.
- `Method.preference` is used when multiple methods match the same goal. The planning loop considers lower-preference methods first. Cost may further narrow the selection if the goal has a cost ceiling.

### Method Matching Flow

```
Goal arrives at planning loop
    |
    | For each method in library (ordered by preference):
    |     1. unify(method.trigger, goal.target) → Option<Bindings>
    |        MISS → skip
    |        HIT  → bindings
    |
    |     2. substitute(method.preconditions, bindings) → Vec<Proposition>
    |        evaluate each against world_state
    |        ANY UNSATISFIED → skip (or plan to satisfy preconditions)
    |
    |     3. Verify net_effects achieve goal:
    |        world_state.apply(net_effects) → projected_state
    |        evaluate(projected_state, goal.target) → Satisfied?
    |        NOT SATISFIED → skip
    |
    |     4. Check cost:
    |        method.cost.exceeds(goal.priority.cost_ceiling)?
    |        EXCEEDS → skip
    |
    |     5. substitute(method.composition, bindings) → Composition
    |        validate(composition) → must be valid
    |
    |     6. Return composition for runtime compilation
    |
    | No method matched:
    |     Agent constructs composition directly
    |     OR planning loop requests LLM-assisted decomposition
    |     OR goal is reported as unachievable
```

### Unification

```rust
/// Attempt to unify a pattern proposition against a concrete proposition.
/// Returns bindings mapping variable names to concrete terms.
pub fn unify(pattern: &Proposition, concrete: &Proposition) -> Option<Bindings>;
```

Unification rules:

- Two propositions unify if they have the same shape (same variant, same structure).
- `Term::Variable(name)` in the pattern unifies with any term in the concrete proposition. The variable binds to that term.
- `Term::Object(a)` unifies with `Term::Object(b)` only if `a == b`.
- `Term::Dimension(a)` unifies with `Term::Dimension(b)` only if `a == b`.
- `Term::ArtifactType(a)` unifies with `Term::ArtifactType(b)` only if `a == b`.
- `Term::Literal(a)` unifies with `Term::Literal(b)` only if `a == b`.
- If the same variable appears twice in the pattern, both occurrences must bind to the same term. Otherwise unification fails.
- `All`/`Any`/`Not` unify structurally: same number of children, each child unifies pairwise.
- Unification is not symmetric. The pattern may contain variables; the concrete proposition should not (it is ground from the goal).

### Bindings

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct Bindings {
    entries: HashMap<String, Term>,
}

impl Bindings {
    pub fn empty() -> Self;
    pub fn bind(&self, variable: String, term: Term) -> Option<Bindings>;
    pub fn get(&self, variable: &str) -> Option<&Term>;
    pub fn merge(&self, other: &Bindings) -> Option<Bindings>;
    pub fn iter(&self) -> impl Iterator<Item = (&String, &Term)>;
}
```

- `bind()` returns `None` if the variable is already bound to a different term.
- `merge()` combines two binding sets. Returns `None` if any variable is bound to different terms in the two sets.
- `Bindings` is immutable. `bind()` and `merge()` return new values.

### Substitution

```rust
/// Replace all Variable terms in a composition with their bound values.
/// Returns Err if any Variable in the composition has no binding.
pub fn substitute(
    composition: &Composition,
    bindings: &Bindings,
) -> Result<Composition, SubstitutionError>;

pub struct SubstitutionError {
    pub unbound_variables: Vec<UnboundVariable>,
}

pub struct UnboundVariable {
    pub step_id: String,
    pub variable: String,
}
```

Substitution walks every `Term` in the composition — operator preconditions, effects, resolution hints, sub-goal propositions — and replaces `Term::Variable(name)` with `bindings.get(name)`. After substitution, the composition should be ground (no variables remaining). If any variable is unbound, substitution fails with an explicit error listing the unbound variables.

## Method as Serialized File

A method library is a directory of files:

```
methods/
├── evaluate_and_write_docs_v1.json
├── quick_check_docs_v1.json
├── force_rewrite_docs_v1.json
├── run_tests_v1.json
├── decompose_course_v1.json
└── podcast_transform_v1.json
```

Each file is a serialized `Method`. Loaded at startup or reloaded at runtime. New files extend the method library without recompilation.

The method library is an execution concern (it decides which methods to load and how to index them). The language crate provides the `Method` type and the operations over it (`unify`, `substitute`). It does not own method storage or loading.

## Read With

- [Lang Primitives](primitives.md)
- [Operators and Resolution](operators.md)
- [Compositions](compositions.md)
- [World State and Evaluation](world_state.md)
- [Execution Goals](../execution/goals/README.md)
- [World Model Agent](../world_model/agent/README.md)
