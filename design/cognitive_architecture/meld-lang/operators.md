# Operators and Resolution

Date: 2026-05-18
Status: active
Scope: Operator contract, Resolution query, and the bridge to the capability catalog

## Thesis

An operator is a runtime-constructed contract defined by its typed boundary — preconditions, effects, cost, and a resolution hint — not by its identity or by a compile-time enum variant. The language does not know what capabilities exist. It describes what an operator needs and produces. The capability catalog, owned by execution, resolves operators to registered capabilities at dispatch time.

This separation is critical for runtime composition. Strategy can define Operators by semantic contracts without knowing which capabilities implement them. If a matching capability exists, the Operator resolves. Otherwise Execution returns an explicit unresolved result. Synthesis occurs only through a separately authorized Strategy candidate.

## Operator

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Operator {
    /// Instance identity within a composition. Runtime-assigned.
    /// Unique within the composition, not globally.
    pub operator_id: String,

    /// What must hold in the world state before this operator runs.
    pub preconditions: Vec<Proposition>,

    /// What changes in the world state after this operator runs.
    pub effects: Vec<Effect>,

    /// Expected resource consumption.
    pub cost: CostEstimate,

    /// How to find a capability that implements this operator.
    pub resolution: Resolution,
}
```

### Operator Design Rules

- `operator_id` is local to the composition. Edges reference it. It has no meaning outside the composition.
- `preconditions` are checked against world state during planning. An Operator whose preconditions are not met is not dispatchable. Strategy may construct an upstream semantic action when authoritative projections support that causal role.
- `effects` are the typed contract of predicted change. Strategy uses effect chaining when constructing a Composition. Execution mechanically validates the authorized chain against current state and capability contracts.
- `cost` is an estimate. Actual cost may differ. Strategy uses projected cost for candidate comparison. Execution rechecks operational cost before commitment.
- An operator does not name a specific action family, action type, or method. It describes what it needs. Resolution finds the implementation.

## Resolution

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Resolution {
    /// Artifact types this operator must consume as input.
    pub requires_inputs: Vec<SlotConstraint>,

    /// Artifact types this operator must produce as output.
    pub requires_outputs: Vec<SlotConstraint>,

    /// The kind of scope this operator requires.
    /// e.g., "filesystem", "repository", "network"
    pub scope_kind: Option<String>,

    /// Freeform tags for narrowing the catalog search.
    /// e.g., ["observe", "git"], ["transform", "generation"], ["evaluate", "deterministic"]
    pub tags: Vec<String>,

    /// When the exact capability is known, bypass catalog search.
    /// This is an escape hatch, not the primary path.
    pub specific: Option<CapabilityRef>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SlotConstraint {
    pub artifact_type_id: String,
    pub required: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CapabilityRef {
    pub capability_type_id: String,
    pub capability_version: u32,
}
```

### Resolution Design Rules

- `requires_inputs` and `requires_outputs` describe the artifact type contract. The catalog matches these against `InputSlotSpec` and `OutputSlotSpec` in registered `CapabilityTypeContract` values.
- `scope_kind` narrows to capabilities that support the required scope type. A "filesystem" scope kind matches capabilities with `ScopeContract.scope_kind == "filesystem"`.
- `tags` are freeform. They enable soft matching: "find a capability tagged 'observe' and 'git'" narrows the search without requiring exact identity. Tags are not authoritative — they are hints.
- `specific` is the escape hatch. When the agent knows exactly which capability it wants, it provides a `CapabilityRef`. The catalog validates that the referenced capability exists and that its contract satisfies the operator's preconditions and effects. If validation fails, the specific reference is rejected and the catalog falls back to search.
- Resolution is the language crate's type. Resolution logic (actually querying the catalog) is execution's concern. The language crate defines the query shape. Execution implements the query.

## Resolution Flow

The language crate does NOT resolve operators. It defines the types. The flow across crate boundaries:

```
meld-lang: defines Operator, Resolution, SlotConstraint, CapabilityRef
    |
    | Composition containing Operators with Resolution hints
    |
    v
meld-execution: runtime compile step
    |
    | For each Operator in Composition:
    |   catalog.resolve(operator.resolution) -> Result<CapabilityTypeContract, UnresolvedReason>
    |
    | Validates input/output wiring between resolved capabilities
    | Validates scope compatibility
    | Validates current state and capability contracts against
    | the Strategy-authorized effect chain
    |
    | Produces: CompiledTaskRecord (existing task compiler output)
    |
    v
Task Network: inject mutation
```

The language crate provides everything up to and including the `Composition` with `Operator`s. Execution takes it from there.

## Cost Estimate

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CostEstimate {
    /// Estimated wall-clock time in milliseconds.
    pub time_ms: u64,

    /// Estimated monetary cost in microdollars (1 dollar = 1_000_000).
    pub money_microdollars: u64,

    /// Estimated number of provider calls (LLM invocations, API calls).
    pub provider_calls: u32,
}
```

### Cost Algebra

```rust
impl CostEstimate {
    /// Sum two estimates dimension-wise.
    pub fn add(&self, other: &CostEstimate) -> CostEstimate;

    /// Does this estimate exceed any dimension of the ceiling?
    pub fn exceeds(&self, ceiling: &CostEstimate) -> bool;

    /// Sum costs across all operators in a composition.
    pub fn aggregate(composition: &Composition) -> CostEstimate;

    /// Zero cost.
    pub fn zero() -> CostEstimate;
}
```

### Cost Design Rules

- Cost is multi-dimensional. A cheap-in-time but expensive-in-money composition is not the same as a cheap-in-money but slow composition. The planning loop compares dimensions individually.
- `money_microdollars` avoids floating point for monetary values. $0.08 is 80_000 microdollars.
- `provider_calls` tracks LLM invocations separately because they have latency, rate limit, and quality implications beyond their monetary cost.
- Cost estimates on operators are predictions, not guarantees. Actual cost is recorded by execution and may feed back into cost estimation calibration (deferred requirement).

## Relationship to Existing Capability Contracts

The existing `CapabilityTypeContract` in `meld-execution` has:

- `input_contract: Vec<InputSlotSpec>` — maps to `Resolution.requires_inputs`
- `output_contract: Vec<OutputSlotSpec>` — maps to `Resolution.requires_outputs`
- `scope_contract: ScopeContract` — maps to `Resolution.scope_kind`
- `effect_contract: Vec<EffectSpec>` — related but different: `EffectSpec` describes ordering effects (Read/Write/Append/Emit/Acquire), while `Operator.effects` describes world-state effects (Assert/Retract/Update propositions)

The two effect models are complementary:

- `EffectSpec` (execution's concern): "this capability writes to file X exclusively" — used for ordering within a task
- `Effect` (language's concern): "this operator makes proposition P true" — used for planning across the composition

Execution resolves the language-level operator to a capability, then uses the capability's `EffectSpec` for internal task ordering. The two effect levels do not conflict.

## Read With

- [Lang Primitives](primitives.md)
- [Compositions](compositions.md)
- [Lang Requirements](requirements.md)
