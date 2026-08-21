# Belief Families

Scope: belief family concept, grounding contract, and worked examples using the docs writer concern class

## Thesis

A belief family is the grounding contract for one concern class.

The belief framework defines containers: `BeliefKey`, `EvidenceItem`, `BeliefRevision`, `BeliefView`. Those containers carry runtime content slots — dimension id, predicate id, evidence schema id, evidence payload, and posterior summary — that the framework deliberately leaves opaque. Without grounding, the framework tracks claims about nothing.

A belief family fills those slots for one kind of question. It defines what the belief is about, what evidence looks like, what the posterior means, which comparator engine runs assessment, and how cost and value are measured for goal curation. The family is the unit that makes a belief inspectable, calibratable, and semantically meaningful.

Families are not framework internals. They are domain concerns expressed in framework vocabulary. Each family should be readable by someone who understands the concern but has not read the consolidated domain specs.

This document defines the concept of a belief family, the shared grounding types, and the runtime configuration contract that every family must satisfy. The concrete families defined here are worked examples rooted in the docs writer concern class. They demonstrate the grounding pattern, not an exhaustive family registry. Production families will be defined as external configuration alongside the domains and capabilities that produce their evidence.

## Runtime Configuration Boundary

Belief family content is runtime configuration, not framework code.

The belief framework may define generic containers, schema validators, comparator engines, stores, leases, revision records, and planner-facing views. It must not define a Rust module, enum variant, comparator type, or source mapping branch for a specific belief family.

`docs_freshness` is the first loaded family configuration. It is not a Rust subsystem.

Family configuration supplies:

- family id
- dimension id
- predicate ids
- evidence schema ids
- source mappings
- comparator engine id
- comparator parameters
- default prior
- freshness policy
- planner projection fields
- observationality declaration per dimension
- config version

The observationality declaration states whether a dimension's value is established only by admitted evidence that arrives after action. Strategy consumes that meaning when identifying a prospective evidence route, as required by the [Strategy Boundary Contracts](../strategy/contracts.md).

A dimension without an observationality declaration is observational. The permissive derived form must be declared explicitly, never assumed from absence.

Revisions must record the config id or config snapshot hash used for assessment so replay is stable when runtime configuration changes.

## Shared Grounding Types

These types appear across all families. They are the concrete shapes that fill the opaque framework slots. The examples shown here are runtime ids and schema names drawn from the worked examples that follow. Production values come from loaded family configuration.

### `BeliefDimensionId`

The runtime id for the axis of assessment. One subject may have beliefs along many dimensions.

```rust
struct BeliefDimensionId(String);

// Example runtime values:
// content_freshness
// test_health
// api_stability
// build_validity
// execution_cost
// action_value
```

A dimension is not a question. It is the kind of question. The question becomes specific when scoped by subject, perspective, and branch.

### `BeliefPredicate`

A condition over `BeliefView` fields that can evaluate to true or false. Used in goal desired state and satisfaction criteria.

```rust
enum BeliefPredicate {
    /// Posterior crosses a threshold
    PosteriorAbove { threshold: f64 },
    PosteriorBelow { threshold: f64 },

    /// Status matches
    StatusIs { status: BeliefStatus },

    /// Freshness constraint
    FresherThan { max_age: Duration },

    /// Compound
    All { predicates: Vec<BeliefPredicate> },
    Any { predicates: Vec<BeliefPredicate> },
}
```

Predicates are evaluated by the world model agent during goal curation and satisfaction checking. They do not carry domain semantics — they are conditions over the shaped view. Domain meaning lives in the dimension and the posterior.

### `EvidenceValue`

The normalized claim content carried by an `EvidenceItem`. Tagged by runtime evidence schema id so comparator engines can extract typed factors.

```rust
struct EvidenceValue {
    schema_id: EvidenceSchemaId,
    payload: serde_json::Value,
}
```

Each runtime schema carries the minimum typed fields that a comparator engine needs to extract factors. Richer source data remains in the spine fact — the evidence value is the belief-facing projection. The schema names in the worked examples below are loaded configuration values. New families add new runtime schemas, not Rust enum variants.

### `PosteriorSummary`

The assessed answer to the belief question. Shape depends on what the family is asking.

```rust
enum PosteriorSummary {
    /// Scalar probability: "how likely is X?"
    /// Used by scalar probability families
    Probability { value: f64 },

    /// Cost or value distribution: "what does X cost / produce?"
    /// Used by cost and value families
    Distribution { mean: f64, variance: f64, sample_count: u32 },
}
```

The `Probability` variant is the common case. A belief about "are docs stale" resolves to a number between 0 and 1. The `Distribution` variant serves meta-beliefs where the agent needs not just the point estimate but the spread — cost estimates with wide uncertainty should be treated differently from narrow ones.

---

## Worked Examples

The families below are concrete runtime configuration examples grounded in the docs writer concern class and its adjacent concerns. Content freshness is the primary example — it is traced end-to-end from spine fact through goal satisfaction. The remaining families demonstrate how the same pattern applies to related concerns, how cross-family evidence flows, and how meta-beliefs feed goal curation.

These are not the only belief families the system will need. They are the first families that exercise the full flywheel and establish the grounding pattern for families defined later.

### Family: Content Freshness

The docs writer's belief. The first family that exercises the full flywheel.

### Identity

| Field | Value |
|---|---|
| Dimension id | `content_freshness` |
| Subject | `DomainObjectRef` for any documented workspace node |
| Question | "Is this node's documentation stale relative to source changes?" |
| Posterior shape | `Probability` — probability that docs are stale, range [0.0, 1.0] |

### Evidence Sources

| Source | Spine domain | Evidence value | Polarity |
|---|---|---|---|
| File changed in node scope | `sensory.workspace` | `file_changed` schema with `lines_added`, `lines_removed` | Supporting |
| Public API changed | `execution` | `api_surface_changed` schema with `public_api_changed` | Supporting |
| Commits accumulated | `sensory.git` | `commit_activity` schema with `commit_count`, `since_reference` | Supporting |
| Time since last doc update | `context` | `content_age` schema with `days_since_update`, `no_prior_record` | Supporting |
| Documentation written | `execution` | `content_written` schema with `frame_ref`, `frame_type` | Contradicting |

### Comparator

`BayesianComparator` — the weighted logistic model from the bayesian evaluation capability.

| Factor | Source evidence | Weight | Normalization |
|---|---|---|---|
| `age` | `content_age.days_since_update` | 0.35 | `min(days / 30.0, 1.0)` |
| `churn` | `file_changed.lines_added + lines_removed` | 0.30 | `min(lines / 150.0, 1.0)` |
| `api_change` | `api_surface_changed.public_api_changed` | 0.25 | `1.0 if true, 0.0 if false` |
| `commit_rate` | `commit_activity.commit_count` | 0.10 | `min(commits / 10.0, 1.0)` |

Prior: `0.3` as a configured default, refined by prior store keyed on subject and runtime dimension id.

Posterior: `sigmoid(logit_prior + evidence_score * 3.0)`

Decision threshold for goal curation: `0.6` — posterior above this means the agent's cost-benefit comparator receives a "probably stale" input.

### Belief View For Consumers

```
BeliefViewSummary {
    belief_key: ("src/task/executor.rs", "content_freshness", default, main),
    status: Active,
    posterior_summary: Probability { value: 0.74 },
    confidence: 0.74,
    uncertainty: Low,
    precision: Adequate,
}

BeliefViewFreshness {
    freshness_status: Fresh,       // the belief itself is fresh (recently assessed)
    last_evidence_time: 2026-05-17T09:14:00Z,
    stale_reason: None,
}

BeliefViewConflict {
    contradiction_status: None,
    unresolved: false,
}
```

The posterior says that docs are probably stale. Freshness says that assessment was recent. Those are different statements. A belief can be fresh while its posterior says the subject is stale. The framework distinguishes these meanings and the family makes them concrete.

### Goal Curation Binding

| Concern class | `content_freshness` |
|---|---|
| Subscription filter | all belief keys where dimension id = `content_freshness` |
| Desired state | `PosteriorBelow { threshold: 0.3 }` — docs are probably not stale |
| Satisfaction | `All { PosteriorBelow(0.3), FresherThan(1 hour) }` |
| Action class | `docs_writer_update` |
| Cost measurement | elapsed_ms + token_count from `task_outcome` evidence |
| Value measurement | downstream `content_freshness` posterior drop correlated with execution |

### End-to-End Trace

```
Spine fact: sensory.workspace.file_change for src/task/executor.rs
  → EvidenceValue { schema_id: "file_changed", payload: { lines_added: 52, lines_removed: 35 } }
  → BeliefKey("src/task/executor.rs", "content_freshness", default, main)
  → BayesianComparator(prior: 0.3, age: 0.47, churn: 0.58, api: 1.0, commits: 0.8)
  → PosteriorSummary::Probability { value: 0.74 }
  → BeliefView { status: Active, posterior: 0.74, confidence: 0.74 }
  → Agent cost-benefit: value(0.74 divergence) > cost(~8min) → act
  → Goal: "update docs for executor.rs" { desired: PosteriorBelow(0.3) }
  → Execution: docs_writer_update task
  → Spine fact: execution.frame_written for src/task/executor.rs
  → EvidenceValue { schema_id: "content_written", payload: { frame_ref: ..., frame_type: readme } }
  → BayesianComparator(prior: 0.74, age: 0.0, churn: 0.0, api: 0.0, commits: 0.0)
  → PosteriorSummary::Probability { value: 0.05 }
  → BeliefView { status: Active, posterior: 0.05 }
  → Agent: satisfaction criteria met → Goal::Satisfied
```

---

### Family: Test Health

### Identity

| Field | Value |
|---|---|
| Dimension id | `test_health` |
| Subject | `DomainObjectRef` for a test suite, module, or workspace |
| Question | "Do tests pass reliably?" |
| Posterior shape | `Probability` — probability that tests are healthy, range [0.0, 1.0] |

Note: polarity is inverted from content freshness. A high posterior here means good health, not staleness. Each family defines its own posterior semantics.

### Evidence Sources

| Source | Spine domain | Evidence value | Polarity |
|---|---|---|---|
| Test run completed | `execution` | `test_result` schema | Direct observation |
| Flakiness detected | `sensory.test` or `execution` | `test_flakiness` schema | Contradicting |
| Source changed in test scope | `sensory.workspace` | `file_changed` schema | Supporting |
| Build failed | `execution` | `build_outcome` schema | Contradicting |

### Comparator

Dual comparator:

**Primary**: `RuleComparator` — test pass/fail is deterministic per run. If the most recent run has `failed > 0`, posterior drops to match failure rate.

**Secondary**: `BayesianComparator` — reliability over time. Factors:

| Factor | Source evidence | Weight | Normalization |
|---|---|---|---|
| `pass_rate` | `test_result.passed / total` | 0.50 | direct ratio |
| `flake_rate` | `test_flakiness.flaky_count / window_runs` | 0.30 | inverse: `1.0 - rate` |
| `recency` | time since last test run | 0.20 | `1.0 - min(hours / 24.0, 1.0)` |

The rule comparator handles a hard signal such as tests failing now. The Bayesian comparator handles a soft signal such as tests being unreliable over time. The revision records which comparator produced the posterior and whether the result is settled or provisional.

### Goal Curation Binding

| Concern class | `test_health` |
|---|---|
| Desired state | `PosteriorAbove { threshold: 0.9 }` — tests reliably pass |
| Satisfaction | `All { PosteriorAbove(0.9), FresherThan(1 hour), stable for 3 revisions }` |
| Action class | `test_fix` |
| Maintenance | standing invariant — goal reactivates on violation |

---

### Family: API Stability

### Identity

| Field | Value |
|---|---|
| Dimension id | `api_stability` |
| Subject | `DomainObjectRef` for a module, crate, or public interface boundary |
| Question | "Has the public API changed in ways that downstream consumers need to know about?" |
| Posterior shape | `Probability` — probability of material API change, range [0.0, 1.0] |

### Evidence Sources

| Source | Spine domain | Evidence value | Polarity |
|---|---|---|---|
| AST impact analysis | `execution` | `api_surface_changed` schema | Supporting |
| Export delta | `sensory.code_analysis` | `export_delta` schema | Supporting |
| Source churn in public modules | `sensory.workspace` | `file_changed` schema | Weak supporting |
| Documentation updated for API | `execution` | `content_written` schema | Contradicting |

### Comparator

`BayesianComparator` — factors:

| Factor | Source evidence | Weight | Normalization |
|---|---|---|---|
| `signature_changes` | `api_surface_changed.signature_changes` | 0.40 | `min(count / 5.0, 1.0)` |
| `export_delta` | `new_exports + removed_exports` | 0.35 | `min(count / 10.0, 1.0)` |
| `source_churn` | `file_changed` in public module paths | 0.15 | `min(lines / 200.0, 1.0)` |
| `doc_coverage` | inverse: has API change been documented? | 0.10 | `0.0 if documented, 1.0 if not` |

### Cross-Family Dependency

API stability feeds content freshness. When `api_stability` posterior crosses threshold, it produces evidence for `content_freshness`. This is the first concrete instance of cross-family evidence flow: one family's revision becomes another family's evidence.

The flow:
```
api_stability revision with posterior 0.8
  → EvidenceValue { schema_id: "api_surface_changed", payload: { public_api_changed: true } }
  → assigned to content_freshness belief key for same subject
  → configured comparator receives api_change = 1.0
```

This is not message passing or hierarchical inference. It is evidence that is relevant to two families simultaneously. The evidence normalizer assigns it to both belief keys. Each family's comparator uses it independently.

---

### Family: Build Validity

### Identity

| Field | Value |
|---|---|
| Dimension id | `build_validity` |
| Subject | `DomainObjectRef` for a build target, crate, or workspace |
| Question | "Does the build succeed and produce valid artifacts?" |
| Posterior shape | `Probability` — probability build is valid, range [0.0, 1.0] |

### Evidence Sources

| Source | Spine domain | Evidence value | Polarity |
|---|---|---|---|
| Build completed | `execution` | `build_outcome` schema | Direct observation |
| Artifact validated | `execution` | `ArtifactState { artifact_ref, valid }` | Direct observation |
| Dependency changed | `sensory.workspace` | `file_changed` schema in dependency files | Supporting |

### Comparator

`RuleComparator` — build success is binary. The most recent build outcome sets the posterior directly:
- `success: true, error_count: 0` produces posterior `0.95`, high but not absolute
- `success: false` → posterior: 0.05

Freshness decay moves the posterior toward its prior as time passes without a build. After the freshness policy window, the belief becomes stale and the Agent may generate an observation Goal.

### Cross-Family Dependency

Build validity is a precondition for test health. The agent's normative framework should suspend test-fix goals when build validity posterior is low — there is no point fixing tests that cannot compile.

---

### Family: Execution Cost Meta-Belief

### Identity

| Field | Value |
|---|---|
| Dimension id | `execution_cost` |
| Subject | `DomainObjectRef` for the action class |
| Question | "What does executing this action class cost?" |
| Posterior shape | `Distribution { mean, variance, sample_count }` — cost distribution |

### Evidence Sources

| Source | Spine domain | Evidence value | Polarity |
|---|---|---|---|
| Task completed | `execution` | `task_outcome` schema | Direct observation |

Every execution outcome for the action class is evidence. No other source feeds cost beliefs.

### Comparator

`BayesianComparator` — exponential moving average with variance tracking.

```
new_mean = alpha * observed_cost + (1 - alpha) * prior_mean
new_variance = alpha * (observed_cost - new_mean)^2 + (1 - alpha) * prior_variance
sample_count += 1
```

Alpha defaults to `0.3` — recent observations have moderate weight. The comparator narrows uncertainty as samples accumulate.

### Cold Start

Before any execution history exists:

1. Capability contracts declare estimated cost envelopes → weak prior
2. Agent initialization can set explicit cost priors → uncalibrated prior
3. Default: wide `Distribution { mean: unknown, variance: high, sample_count: 0 }` → agent acts only on strong divergences where value clearly dominates uncertain cost

### Goal Curation Binding

Cost beliefs are not goal targets. They are inputs to the cost-benefit comparator that decides whether state-belief divergences warrant action. Lower cost makes more goals pass the threshold. Higher cost makes fewer goals pass. The agent does not try to reduce cost — it uses cost estimates to make better act/tolerate decisions.

---

### Family: Action Value Meta-Belief

### Identity

| Field | Value |
|---|---|
| Dimension id | `action_value` |
| Subject | `DomainObjectRef` for the concern class |
| Question | "What downstream value does acting on divergence in this concern class produce?" |
| Posterior shape | `Distribution { mean, variance, sample_count }` — value distribution |

### Evidence Sources

| Source | Spine domain | Evidence value | Polarity |
|---|---|---|---|
| Downstream belief improved after action | `world_model` | `downstream_belief_shift` schema | Supporting |
| Downstream belief unchanged after action | `world_model` | `downstream_belief_shift` schema | Contradicting |

### Comparator

`BayesianComparator` — outcome correlation with wide initial uncertainty.

Value beliefs require longer calibration windows than cost beliefs. The causal chain from goal → execution → outcome → downstream belief change is longer and noisier. Value priors should start wide and narrow slowly.

### Cold Start

Default behavior acts on strong divergences with clear value signals such as user direction or a maintenance invariant violation. Marginal divergences wait until outcome data calibrates the value posterior.

---

## Family Runtime Configuration Contract

When a new belief family is introduced, it must provide:

| Requirement | Purpose |
|---|---|
| family id | stable runtime identity |
| dimension id | identity axis |
| At least one evidence schema | what evidence looks like |
| `PosteriorSummary` shape | what the posterior means |
| Default comparator engine selection | how assessment works |
| At least one evidence source with spine domain and event type | where evidence comes from |
| Default prior | cold start value |
| Freshness policy | when the belief becomes stale without new evidence |
| Goal curation binding: desired state predicate, satisfaction criteria, action class | how the agent uses this belief |

A family without all of these can exist as an ungrounded framework entity — a `BeliefKey` with `MissingComparator` status. But it cannot participate in the flywheel until grounded.

The framework must load this contract from runtime configuration. Adding or changing a family must not require editing Rust source that names that family.

## Cross-Family Evidence Rules

Evidence that is relevant to multiple families is assigned to each relevant belief key independently by the evidence normalizer. Each family's comparator processes it according to its own factor model.

Cross-family evidence is not message passing. It is shared observation:

```
Spine fact: sensory.workspace.file_change (src/lib.rs, +30 -10)
  → assigned to content_freshness for src/lib.rs as file_changed evidence
  → assigned to api_stability for src/lib.rs as file_changed evidence
  → assigned to build_validity for workspace as file_changed evidence
  → assigned to test_health for src/lib.rs as file_changed evidence
```

The same fact means different things to different families. The evidence normalizer handles this through multi-assignment. The evidence role and polarity may differ per assignment.

## Cross-Family Dependency Patterns

Some families have structural relationships:

```
build_validity ──precondition──▶ test_health
    "tests cannot pass if build fails"

api_stability ──evidence──▶ content_freshness
    "api change is strong evidence of doc staleness"

content_freshness ──evidence──▶ action_value for docs_writer
    "freshness improvement after execution calibrates value"

execution_cost ──input──▶ cost-benefit comparator
action_value ──input──▶ cost-benefit comparator
    "meta-beliefs feed the goal curation decision, not other families"
```

These dependencies are not hard-wired in the framework. They emerge from evidence assignment rules and the agent's normative framework. The agent learns which dependencies matter through calibration.

## Regime Sensitivity

All families carry regime-scoped priors. The same family operates differently under different regimes:

| Family | Normal development | Incident response |
|---|---|---|
| content_freshness | standard thresholds, standard cost tolerance | near-zero value — suspend docs goals |
| test_health | maintenance invariant | critical — highest priority |
| build_validity | maintenance invariant | critical — highest priority |
| api_stability | standard monitoring | reduced monitoring — stability matters more than tracking |
| execution_cost | calibrated priors | widen uncertainty — incident costs differ |
| action_value | calibrated priors | reset — incident value landscape differs |

When a regime shift is detected, the agent scopes all family priors to the new regime. Archived priors from previous instances of the same regime are retrieved from the regime library when available.

## What This Document Does Not Cover

### Additional Domain-Specific Families

Families for dependency safety, security posture, performance regression, code complexity, and other concerns follow the same runtime configuration contract.

### Hierarchical belief families

A family where multiple subjects share a parent belief uses the inference epoch mechanism and declares its aggregation policy in runtime configuration.

### Dynamic family creation

An Agent discovering a new concern class may load a new family at runtime. The runtime configuration contract declares comparator selection, prior initialization, and evidence source binding.

## Read With

- [Belief](README.md)
- [Fact To Belief](fact_to_belief.md)
- [Comparator Model](comparator_model.md)
- [Belief Spec](spec.md)
- [Goal Curation](../agent/goal_curation.md)
- [Goals](../../execution/goals/README.md)
- [Regime Layer](../regime/README.md)
