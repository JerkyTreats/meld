# Belief Families

Date: 2026-05-17
Status: active
Scope: belief family concept, grounding contract, and worked examples using the docs writer concern class

## Thesis

A belief family is the grounding contract for one concern class.

The belief framework defines containers: `BeliefKey`, `EvidenceItem`, `BeliefRevision`, `BeliefView`. Those containers carry typed slots — `BeliefDimension`, `BeliefPredicate`, `EvidenceValue`, `PosteriorSummary` — that the framework deliberately leaves opaque. Without grounding, the framework tracks claims about nothing.

A belief family fills those slots for one kind of question. It defines what the belief is about, what evidence looks like, what the posterior means, which comparator runs assessment, and how cost and value are measured for goal curation. The family is the unit that makes a belief inspectable, calibratable, and semantically meaningful.

Families are not framework internals. They are domain concerns expressed in framework vocabulary. Each family should be readable by someone who understands the concern but has not read the ECS specifications.

This document defines the concept of a belief family, the shared grounding types, and the registration contract that every family must satisfy. The concrete families defined here are worked examples rooted in the docs writer concern class. They demonstrate the grounding pattern, not an exhaustive family registry. Production families will be defined alongside the domains and capabilities that produce their evidence.

## Shared Grounding Types

These types appear across all families. They are the concrete shapes that fill the opaque framework slots. The enum variants shown here are drawn from the worked examples that follow. Production variants will grow as new families are registered.

### `BeliefDimension`

The axis of assessment. One subject may have beliefs along many dimensions.

```rust
enum BeliefDimension {
    // Examples from the docs writer concern class
    ContentFreshness,
    TestHealth,
    ApiStability,
    BuildValidity,
    ExecutionCost { action_class: ActionClass },
    ActionValue { concern_class: ConcernClass },
    // ... additional variants registered by other families
}
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

The normalized claim content carried by an `EvidenceItem`. Tagged by family so comparators can extract typed factors.

```rust
enum EvidenceValue {
    // Content freshness
    FileChanged { lines_added: u32, lines_removed: u32 },
    ApiSurfaceChanged {
        public_api_changed: bool,
        new_exports: u32,
        removed_exports: u32,
        signature_changes: u32,
    },
    ContentAge { days_since_update: u32, no_prior_record: bool },
    CommitActivity { commit_count: u32, since_reference: DomainObjectRef },
    ContentWritten { frame_ref: DomainObjectRef, frame_type: FrameType },

    // Test health
    TestResult { passed: u32, failed: u32, skipped: u32, total: u32 },
    TestFlakiness { flaky_count: u32, window_runs: u32 },

    // API stability
    ExportDelta { added: Vec<String>, removed: Vec<String>, changed: Vec<String> },

    // Build validity
    BuildOutcome { success: bool, error_count: u32, warning_count: u32 },
    ArtifactState { artifact_ref: DomainObjectRef, valid: bool },

    // Execution cost (meta-belief)
    TaskOutcome {
        elapsed_ms: u64,
        token_count: u64,
        success: bool,
        retry_count: u32,
    },

    // Action value (meta-belief)
    DownstreamBeliefShift {
        target_belief_key: BeliefKey,
        prior_posterior: f64,
        new_posterior: f64,
        lag_ms: u64,
    },
}
```

Each variant carries the minimum typed fields that a comparator needs to extract factors. Richer source data remains in the spine fact — the evidence value is the belief-facing projection. These variants are drawn from the worked examples below. New families register new variants.

### `PosteriorSummary`

The assessed answer to the belief question. Shape depends on what the family is asking.

```rust
enum PosteriorSummary {
    /// Scalar probability: "how likely is X?"
    /// Used by: ContentFreshness, TestHealth, ApiStability, BuildValidity
    Probability { value: f64 },

    /// Cost or value distribution: "what does X cost / produce?"
    /// Used by: ExecutionCost, ActionValue
    Distribution { mean: f64, variance: f64, sample_count: u32 },
}
```

The `Probability` variant is the common case. A belief about "are docs stale" resolves to a number between 0 and 1. The `Distribution` variant serves meta-beliefs where the agent needs not just the point estimate but the spread — cost estimates with wide uncertainty should be treated differently from narrow ones.

---

## Worked Examples

The families below are concrete examples grounded in the docs writer concern class and its adjacent concerns. Content Freshness is the primary example — it is traced end-to-end from spine fact through goal satisfaction. The remaining families demonstrate how the same pattern applies to related concerns, how cross-family evidence flows, and how meta-beliefs (cost and value) feed goal curation.

These are not the only belief families the system will need. They are the first families that exercise the full flywheel and establish the grounding pattern for families defined later.

### Family: Content Freshness

The docs writer's belief. The first family that exercises the full flywheel.

### Identity

| Field | Value |
|---|---|
| Dimension | `ContentFreshness` |
| Subject | `DomainObjectRef` for any documented workspace node |
| Question | "Is this node's documentation stale relative to source changes?" |
| Posterior shape | `Probability` — probability that docs are stale, range [0.0, 1.0] |

### Evidence Sources

| Source | Spine domain | Evidence value | Polarity |
|---|---|---|---|
| File changed in node scope | `sensory.workspace` | `FileChanged { lines_added, lines_removed }` | Supporting (staleness) |
| Public API changed | `execution` (ast_change_impact artifact) | `ApiSurfaceChanged { public_api_changed, ... }` | Supporting (staleness) |
| Commits accumulated | `sensory.git` | `CommitActivity { commit_count, since_reference }` | Supporting (staleness) |
| Time since last doc update | `context` (frame age computation) | `ContentAge { days_since_update, no_prior_record }` | Supporting (staleness) |
| Documentation written | `execution` (frame_written event) | `ContentWritten { frame_ref, frame_type }` | Contradicting (freshness) |

### Comparator

`BayesianComparator` — the weighted logistic model from the bayesian evaluation capability.

| Factor | Source evidence | Weight | Normalization |
|---|---|---|---|
| `age` | `ContentAge.days_since_update` | 0.35 | `min(days / 30.0, 1.0)` |
| `churn` | `FileChanged.lines_added + lines_removed` | 0.30 | `min(lines / 150.0, 1.0)` |
| `api_change` | `ApiSurfaceChanged.public_api_changed` | 0.25 | `1.0 if true, 0.0 if false` |
| `commit_rate` | `CommitActivity.commit_count` | 0.10 | `min(commits / 10.0, 1.0)` |

Prior: `0.3` (static default), refined by prior store keyed on `(subject, ContentFreshness)`.

Posterior: `sigmoid(logit_prior + evidence_score * 3.0)`

Decision threshold for goal curation: `0.6` — posterior above this means the agent's cost-benefit comparator receives a "probably stale" input.

### Belief View (What the consumer sees)

```
BeliefViewSummary {
    belief_key: ("src/task/executor.rs", ContentFreshness, default, main),
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

The posterior says "docs are probably stale." The freshness says "we assessed this recently." Those are different statements. A belief can be fresh (recently checked) while its posterior says the subject is stale (docs are outdated). The framework distinguishes these; the family gives them concrete meaning.

### Goal Curation Binding

| Concern class | `content_freshness` |
|---|---|
| Subscription filter | all belief keys where dimension = `ContentFreshness` |
| Desired state | `PosteriorBelow { threshold: 0.3 }` — docs are probably not stale |
| Satisfaction | `All { PosteriorBelow(0.3), FresherThan(1 hour) }` |
| Action class | `docs_writer_update` |
| Cost measurement | elapsed_ms + token_count from `TaskOutcome` evidence |
| Value measurement | downstream `ContentFreshness` posterior drop correlated with execution |

### End-to-End Trace

```
Spine fact: sensory.workspace.file_change (src/task/executor.rs, +52 -35)
  → EvidenceValue::FileChanged { lines_added: 52, lines_removed: 35 }
  → BeliefKey("src/task/executor.rs", ContentFreshness, default, main)
  → BayesianComparator(prior: 0.3, age: 0.47, churn: 0.58, api: 1.0, commits: 0.8)
  → PosteriorSummary::Probability { value: 0.74 }
  → BeliefView { status: Active, posterior: 0.74, confidence: 0.74 }
  → Agent cost-benefit: value(0.74 divergence) > cost(~8min) → act
  → Goal: "update docs for executor.rs" { desired: PosteriorBelow(0.3) }
  → Execution: docs_writer_update task
  → Spine fact: execution.frame_written (src/task/executor.rs, readme)
  → EvidenceValue::ContentWritten { frame_ref: ..., frame_type: readme }
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
| Dimension | `TestHealth` |
| Subject | `DomainObjectRef` for a test suite, module, or workspace |
| Question | "Do tests pass reliably?" |
| Posterior shape | `Probability` — probability that tests are healthy, range [0.0, 1.0] |

Note: polarity is inverted from content freshness. A high posterior here means good health, not staleness. Each family defines its own posterior semantics.

### Evidence Sources

| Source | Spine domain | Evidence value | Polarity |
|---|---|---|---|
| Test run completed | `execution` | `TestResult { passed, failed, skipped, total }` | Direct observation |
| Flakiness detected | `sensory.test` or `execution` | `TestFlakiness { flaky_count, window_runs }` | Contradicting (undermines health) |
| Source changed in test scope | `sensory.workspace` | `FileChanged { lines_added, lines_removed }` | Supporting (uncertainty — tests may need re-run) |
| Build failed | `execution` | `BuildOutcome { success: false, ... }` | Contradicting (tests cannot pass if build fails) |

### Comparator

Dual comparator:

**Primary**: `RuleComparator` — test pass/fail is deterministic per run. If the most recent run has `failed > 0`, posterior drops to match failure rate.

**Secondary**: `BayesianComparator` — reliability over time. Factors:

| Factor | Source evidence | Weight | Normalization |
|---|---|---|---|
| `pass_rate` | `TestResult.passed / total` | 0.50 | direct ratio |
| `flake_rate` | `TestFlakiness.flaky_count / window_runs` | 0.30 | inverse: `1.0 - rate` |
| `recency` | time since last test run | 0.20 | `1.0 - min(hours / 24.0, 1.0)` |

The rule comparator handles the hard signal (tests failed right now). The Bayesian comparator handles the soft signal (tests have been unreliable over time). The revision records which comparator produced the posterior and whether the result is settled (rule) or provisional (Bayesian without recent data).

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
| Dimension | `ApiStability` |
| Subject | `DomainObjectRef` for a module, crate, or public interface boundary |
| Question | "Has the public API changed in ways that downstream consumers need to know about?" |
| Posterior shape | `Probability` — probability of material API change, range [0.0, 1.0] |

### Evidence Sources

| Source | Spine domain | Evidence value | Polarity |
|---|---|---|---|
| AST impact analysis | `execution` (ast_change_impact artifact) | `ApiSurfaceChanged { ... }` | Supporting (change detected) |
| Export delta | `sensory.code_analysis` | `ExportDelta { added, removed, changed }` | Supporting (change detected) |
| Source churn in public modules | `sensory.workspace` | `FileChanged { ... }` | Weak supporting (may indicate change) |
| Documentation updated for API | `execution` (frame_written) | `ContentWritten { ... }` | Contradicting (change has been documented) |

### Comparator

`BayesianComparator` — factors:

| Factor | Source evidence | Weight | Normalization |
|---|---|---|---|
| `signature_changes` | `ApiSurfaceChanged.signature_changes` | 0.40 | `min(count / 5.0, 1.0)` |
| `export_delta` | `new_exports + removed_exports` | 0.35 | `min(count / 10.0, 1.0)` |
| `source_churn` | `FileChanged` in public module paths | 0.15 | `min(lines / 200.0, 1.0)` |
| `doc_coverage` | inverse: has API change been documented? | 0.10 | `0.0 if documented, 1.0 if not` |

### Cross-Family Dependency

API stability feeds content freshness. When `ApiStability` posterior crosses threshold, it produces evidence for `ContentFreshness` (the `api_change` factor). This is the first concrete instance of cross-family evidence flow: one family's revision becomes another family's evidence.

The flow:
```
ApiStability revision (api changed, posterior: 0.8)
  → EvidenceValue::ApiSurfaceChanged { public_api_changed: true, ... }
  → assigned to ContentFreshness belief key for same subject
  → ContentFreshness comparator receives api_change = 1.0
```

This is not message passing or hierarchical inference. It is evidence that is relevant to two families simultaneously. The evidence normalizer assigns it to both belief keys. Each family's comparator uses it independently.

---

### Family: Build Validity

### Identity

| Field | Value |
|---|---|
| Dimension | `BuildValidity` |
| Subject | `DomainObjectRef` for a build target, crate, or workspace |
| Question | "Does the build succeed and produce valid artifacts?" |
| Posterior shape | `Probability` — probability build is valid, range [0.0, 1.0] |

### Evidence Sources

| Source | Spine domain | Evidence value | Polarity |
|---|---|---|---|
| Build completed | `execution` | `BuildOutcome { success, error_count, warning_count }` | Direct observation |
| Artifact validated | `execution` | `ArtifactState { artifact_ref, valid }` | Direct observation |
| Dependency changed | `sensory.workspace` | `FileChanged` in dependency files | Supporting (uncertainty — build may break) |

### Comparator

`RuleComparator` — build success is binary. The most recent build outcome sets the posterior directly:
- `success: true, error_count: 0` → posterior: 0.95 (high but not 1.0 — next change may break it)
- `success: false` → posterior: 0.05

Freshness decay: posterior decays toward prior as time passes without a build. After the freshness policy window, the belief becomes stale and the agent may generate an observation goal ("run the build to check").

### Cross-Family Dependency

Build validity is a precondition for test health. The agent's normative framework should suspend test-fix goals when build validity posterior is low — there is no point fixing tests that cannot compile.

---

### Family: Execution Cost (Meta-Belief)

### Identity

| Field | Value |
|---|---|
| Dimension | `ExecutionCost { action_class }` |
| Subject | `DomainObjectRef` for the action class (e.g., `docs_writer_update`, `test_fix`) |
| Question | "What does executing this action class cost?" |
| Posterior shape | `Distribution { mean, variance, sample_count }` — cost distribution |

### Evidence Sources

| Source | Spine domain | Evidence value | Polarity |
|---|---|---|---|
| Task completed | `execution` | `TaskOutcome { elapsed_ms, token_count, success, retry_count }` | Direct observation |

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

### Family: Action Value (Meta-Belief)

### Identity

| Field | Value |
|---|---|
| Dimension | `ActionValue { concern_class }` |
| Subject | `DomainObjectRef` for the concern class |
| Question | "What downstream value does acting on divergence in this concern class produce?" |
| Posterior shape | `Distribution { mean, variance, sample_count }` — value distribution |

### Evidence Sources

| Source | Spine domain | Evidence value | Polarity |
|---|---|---|---|
| Downstream belief improved after action | `world_model` | `DownstreamBeliefShift { target_key, prior_posterior, new_posterior, lag_ms }` | Supporting (action produced value) |
| Downstream belief unchanged after action | `world_model` | `DownstreamBeliefShift { ..., delta ≈ 0 }` | Contradicting (action did not produce value) |

### Comparator

`BayesianComparator` — outcome correlation with wide initial uncertainty.

Value beliefs require longer calibration windows than cost beliefs. The causal chain from goal → execution → outcome → downstream belief change is longer and noisier. Value priors should start wide and narrow slowly.

### Cold Start

Default: act on strong divergences with clear value signals (user-directed, maintenance invariant violation). Defer marginal divergences until outcome data calibrates the value posterior.

---

## Family Registration Contract

When a new belief family is introduced, it must provide:

| Requirement | Purpose |
|---|---|
| `BeliefDimension` variant | identity axis |
| At least one `EvidenceValue` variant | what evidence looks like |
| `PosteriorSummary` shape | what the posterior means |
| Default comparator selection | how assessment works |
| At least one evidence source with spine domain and event type | where evidence comes from |
| Default prior | cold start value |
| Freshness policy | when the belief becomes stale without new evidence |
| Goal curation binding: desired state predicate, satisfaction criteria, action class | how the agent uses this belief |

A family without all of these can exist as an ungrounded framework entity — a `BeliefKey` with `MissingComparator` status. But it cannot participate in the flywheel until grounded.

## Cross-Family Evidence Rules

Evidence that is relevant to multiple families is assigned to each relevant belief key independently by the evidence normalizer. Each family's comparator processes it according to its own factor model.

Cross-family evidence is not message passing. It is shared observation:

```
Spine fact: sensory.workspace.file_change (src/lib.rs, +30 -10)
  → assigned to ContentFreshness("src/lib.rs") as FileChanged evidence
  → assigned to ApiStability("src/lib.rs") as FileChanged evidence (weak)
  → assigned to BuildValidity("workspace") as FileChanged evidence (uncertainty)
  → assigned to TestHealth("src/lib.rs") as FileChanged evidence (uncertainty)
```

The same fact means different things to different families. The evidence normalizer handles this through multi-assignment. The evidence role and polarity may differ per assignment.

## Cross-Family Dependency Patterns

Some families have structural relationships:

```
BuildValidity ──precondition──▶ TestHealth
    "tests cannot pass if build fails"

ApiStability ──evidence──▶ ContentFreshness
    "api change is strong evidence of doc staleness"

ContentFreshness ──evidence──▶ ActionValue(docs_writer)
    "freshness improvement after execution calibrates value"

ExecutionCost(any) ──input──▶ cost-benefit comparator
ActionValue(any) ──input──▶ cost-benefit comparator
    "meta-beliefs feed the goal curation decision, not other families"
```

These dependencies are not hard-wired in the framework. They emerge from evidence assignment rules and the agent's normative framework. The agent learns which dependencies matter through calibration.

## Regime Sensitivity

All families carry regime-scoped priors. The same family operates differently under different regimes:

| Family | Normal development | Incident response |
|---|---|---|
| ContentFreshness | standard thresholds, standard cost tolerance | near-zero value — suspend docs goals |
| TestHealth | maintenance invariant | critical — highest priority |
| BuildValidity | maintenance invariant | critical — highest priority |
| ApiStability | standard monitoring | reduced monitoring — stability matters more than tracking |
| ExecutionCost | calibrated priors | widen uncertainty — incident costs differ |
| ActionValue | calibrated priors | reset — incident value landscape differs |

When a regime shift is detected, the agent scopes all family priors to the new regime. Archived priors from previous instances of the same regime are retrieved from the regime library when available.

## What This Document Does Not Cover

### Domain-specific families beyond the first slice

Families for dependency safety, security posture, performance regression, code complexity, and other concerns are expected but not specified. Each follows the same registration contract.

### Hierarchical belief families

A family where multiple subjects share a parent belief (e.g., "module documentation" as an aggregate over file-level freshness beliefs) requires the inference epoch mechanism. The family registration contract does not yet address aggregation.

### Dynamic family creation

An agent discovering a new concern class and registering a new family at runtime is architecturally supported (the agent bootstrap survey step discovers relevant dimensions). The creation protocol — including comparator selection, prior initialization, and evidence source binding — is not specified beyond the registration contract.

## Read With

- [Belief](README.md)
- [Fact To Belief](fact_to_belief.md)
- [Comparator Model](comparator_model.md)
- [Belief Entities](entities.md)
- [Belief Components](components.md)
- [Goal Curation](../agent/goal_curation.md)
- [Goals](../../execution/goals/README.md)
- [Bayesian Evaluation Example](../../execution/examples/bayesian_evaluation.md)
- [Git Diff Summary Example](../../execution/examples/git_diff_summary.md)
- [AST Change Impact Example](../../execution/examples/ast_change_impact.md)
- [Regime Layer](../regime/README.md)
