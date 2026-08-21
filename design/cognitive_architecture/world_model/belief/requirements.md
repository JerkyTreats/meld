# Belief Requirements

Scope: canonical requirements for `world_model/belief`

## Thesis

Belief proves a replayable path from durable facts to normalized evidence, belief revisions, observation opportunities, calibration records, and shaped views.

Every inference method preserves the full boundary shape.

## Functional Requirements

### Runtime Family Configuration

- Load belief family content from external runtime configuration.
- Treat family id, dimension id, predicate ids, evidence schema ids, source mappings, priors, thresholds, factor weights, freshness policy, and planner projection fields as configuration data.
- Keep framework code generic over loaded family ids and schema ids.
- Do not add Rust modules, enum variants, comparator types, or source mapping match arms for specific belief families.
- Validate loaded family configuration before evidence normalization, assignment, or assessment can run.
- Persist a config id, config version, or config snapshot hash on each revision and lease that depends on that configuration.
- Mark affected belief views stale when runtime family configuration or evidence policy changes.

### Identity

- Define `BeliefKey` as the stable assessed question identity.
- Include subject, runtime dimension id, runtime predicate id, perspective, branch scope, and evidence policy id in key material.
- Define durable object refs for belief, evidence, and revision records.
- Preserve compatibility with graph and spine `DomainObjectRef`.

### Evidence

- Normalize spine facts, graph anchors, execution outcomes, corrections, and derived facts into `EvidenceItem`.
- Preserve source fact ids, graph anchor ids, source cursors, reference time, transaction time, and provenance.
- Support evidence roles for support, contradiction, context, calibration, and supersession.
- Support one fact affecting many beliefs.
- Support one belief revision using many evidence items.

### Belief Assessment

- Select comparator engine by runtime family configuration, belief policy, and comparator availability.
- Support Bayesian comparator, rule comparator, semantic settlement comparator, missing comparator, message passing inference, and predictive residual inference as generic method engines.
- Mark semantic settlement provisional unless policy grants settled status.
- Emit needs-assessment or needs-observation when comparator support is missing.
- Record comparator engine id, version, config ref or snapshot hash, input evidence ids, and contradicted evidence ids.

### Revision

- Append `BeliefRevision` records.
- Preserve prior revision links.
- Move revision head without editing prior revisions.
- Record evidence window and source cursor range.
- Record posterior summary, uncertainty, precision, confidence, freshness, contradiction, status, and provenance.

### Freshness

- Mark stale when new evidence arrives after current revision.
- Mark stale when semantic settlement expires.
- Mark stale when confidence decay crosses threshold.
- Mark stale when graph anchor evidence is superseded.
- Mark stale when evidence policy changes.
- Expose stale reason in `BeliefView`.

### Contradiction

- Preserve supporting and contradicting evidence ids.
- Distinguish counterevidence, supersession, invalidation, weak coverage, and competing hypothesis.
- Keep unresolved conflict explicit.
- Do not collapse contradiction into a generic confidence field.

### Hypotheses

- Represent latent alternatives as `HypothesisSet` when alternatives materially affect posterior, uncertainty, observation need, or regime signals.
- Record candidate weights, evidence links, and inference epoch refs.
- Prune or archive alternatives without losing provenance.

### Observation Opportunities

- Emit `ObservationOpportunity` for missing evidence, ambiguity, unresolved conflict, stale state, or missing comparator state.
- Include target belief key, evidence type, evidence channel, suggested artifact type, expected information gain, cost, delay, expiry, and provenance when available.
- Treat observation opportunities as world-model outputs, not execution commands.

### Belief Views

- Publish `BeliefView` as the public belief read shape.
- Include key, perspective, current revision id, status, posterior summary, confidence, uncertainty, precision, freshness, contradiction, observation state, assessment state, advisory posture, and provenance summary.
- Exclude raw spine payloads, active lease internals, comparator drafts, and unpublished evidence churn.
- Attach hydration handles for task construction and explanation.

### Calibration

- Ingest later outcomes and observations as calibration evidence.
- Compare prior posterior summaries with observed outcomes.
- Adjust future reliability, precision, prior selection, and comparator trust.
- Preserve calibration records without rewriting old revisions.

### Scheduling And Recovery

- Use `BeliefKey` as the scheduling unit.
- Keep one active assessment lease per belief key.
- Coalesce storms behind active leases.
- Persist lease state enough for recovery.
- Recover from expired leases by replaying durable evidence and revisions.
- Rebuild belief views from revisions.

## Public Interface Requirements

- Provide query by belief key.
- Provide query by subject and perspective.
- Provide query for current revision.
- Provide query for evidence and provenance by revision.
- Provide query for observation opportunities.
- Provide query for freshness by subject and perspective.
- Provide belief view subscription or polling contract.
- Keep `register_belief_key` as key creation, not belief settlement.

## Boundary Requirements

- Belief consumes graph outputs and spine facts.
- Belief publishes belief views and belief signals.
- Belief does not dispatch tasks.
- Belief does not decide goals.
- Belief does not own causal claims.
- Belief does not decide regime identity.
- Agent and execution may hydrate evidence after choosing action, but they do not settle belief from raw facts during planning.

## Nonfunctional Requirements

### Replay

- Every revision is rebuildable from source refs, evidence ids, policy records, comparator engine version, runtime config snapshot, and source cursor.
- Every belief view is rebuildable from current revision, freshness state, contradiction state, observation opportunities, and provenance.

### Determinism

- Same source facts, graph state, policies, comparator engine versions, runtime config snapshots, and source cursors produce same revision outputs.
- Semantic settlement records prompt or policy version and remains provisional by default.

### Concurrency

- Independent belief keys may assess in parallel.
- One belief key has at most one active lease.
- Storm coalescing prevents unbounded comparator workers.

### Audit

- Source refs survive normalization, assignment, revision, view projection, observation projection, and calibration.
- Rejected evidence carries reason when audit policy requires visibility.
- Missing comparator, missing evidence, stale state, and contradiction are explicit.

### Evolution

- Compact comparator output is a sufficient starting shape.
- New belief families are added by runtime configuration, not Rust source changes.
- Priors, posterior distributions, hidden state, inference epochs, calibration, and regime-conditioned prior selection can be added without changing public view boundaries.

## Read With

- [Belief Spec](spec.md)
- [World Model Belief](README.md)
