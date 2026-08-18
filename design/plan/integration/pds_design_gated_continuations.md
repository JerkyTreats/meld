# PDS Design-Gated Continuations

Date: 2026-08-16
Status: not authorized for implementation
Scope: preserve the explicit authorization boundary after `W06` and state the design work required before any later PDS continuation may return to an implementation plan

The predecessor `W00` through `W06` program is closed as superseded at a compatibility checkpoint. This register remains historical scope control. The successor [PDS Boundary Program](pds_boundary_program.md) authorizes none of the continuation work below unless one item is explicitly moved into an active successor phase.

## Authority

No scope in this document is authorized for implementation.

The closed predecessor delivery plan is [PDS Authorized Implementation Workstreams](pds_implementation_workstreams.md). `W06` did not pass. Closing that plan and authorizing the PDS Boundary Program does not implicitly authorize this document, any former later workstream, or any implementation needed to explore them.

The labels `W07` through `W10` are retained only for continuity with prior discussion. They are not queued work.

## W07 — Dependency-Security Completion

Status: requires real domain and product design before implementation authorization

The authorized `W05` and `W06` scope is deliberately bounded to:

- state-free dependency-security policy installation
- read-only inventory, advisory, coverage, assessment, and verification products
- explicit unknown, stale, conflicted, clean-within-coverage, and violated states
- owner admission before canonical publication
- local and serialized adapter contract proof
- generic lifecycle, readiness, wake, recovery, and late-result fencing

That authorization does not establish a complete dependency-security product.

Before any broader implementation is considered, the design must settle:

- the real maintained subject and responsibility boundary
- authoritative dependency inventory and resolved-component identity
- advisory source selection, source revision, coverage, and freshness semantics
- bounded negative evidence and what clean can truthfully mean
- scanner, resolver, hosted service, and persistent monitor roles
- external runtime trust and result-admission requirements
- remediation-feasibility meaning
- ownership of repository mutation and pull-request behavior
- how a security fact becomes input to another steward without becoming a command
- authority, approval, and effect boundaries for dependency changes
- subsequent verification and resolution evidence
- retention, privacy, credential, and external source obligations
- migration and compatibility behavior for security policy revisions

Required review before reauthorization:

- domain-first dependency-security architecture
- external integration and source-truth analysis
- end-to-end evidence and authority trace
- lifecycle and failure review against real adapter choices
- cross-domain review of workspace, events, world model, execution, and developer behavior
- explicit comparison with simpler scanner, monitor, and policy-controller baselines

The design must end in a new implementation proposal with a bounded first deliverable. The prior concept of completing a broad second expression is not sufficient authorization.

## W08 — Settled Strategy Replay

Status: requires a full design audit and necessity proof

The proposed replay change currently has the smell of unnecessary optimization. Exact owner revisions, package lineage, recorded Agent authorization, realized Composition, and historical resolution may already provide the correctness properties the product needs.

No new Strategy settlement store, replay mode, candidate reuse path, or replay-specific lineage may be implemented until an audit answers:

- what concrete user or runtime problem candidate regeneration causes
- whether the problem is correctness, latency, cost, determinism, availability, or inspection
- whether exact historical explanation already solves the actual need
- whether ordinary recomputation is safer and simpler
- whether a narrow cache would be sufficient
- when a recorded candidate remains semantically eligible
- whether replay would bypass new evidence, authority, restrictions, or current-world checks
- how invalidation works across theory, package, assignment, activation, and source changes
- whether replay preserves Agent authority rather than reusing a stale decision
- which domain owns any durable settlement product
- what measurable threshold would justify the added persistence and control path

The audit must cover world-model Strategy, Agent judgment, execution admission, planning, events, persistence, historical resolution, and operator inspection.

Reauthorization requires a reviewed design that demonstrates a correctness need or a measured operational need. Performance intuition alone is insufficient.

## W09 — Canonical Declaration And Customer Profile

Status: not authorized and transitively design-gated

This work depends on a proven stable target above generic package attachment. The bounded authorized work through `W06` may provide useful evidence, but it does not authorize declaration schema, profile schema, compiler, editor, semantic diff, or approval-surface implementation.

Any future design must be initiated separately after the package and assignment contracts from `W00` through `W06` are reviewed in operation.

## W10 — Stewardship Projection And Upgrade

Status: not authorized and transitively design-gated

Projection, episode coordination, and package upgrade remain separate product and architecture questions. They must not be bundled into runtime completion or introduced as scaffolding for future use.

Any future proposal must first establish:

- the concrete user query or coordination need
- which current domain records already answer it
- why a derived view is insufficient if authoritative episode state is proposed
- the exact upgrade behavior required by a real package lifecycle
- compatibility and historical interpretation obligations

## Reauthorization Rule

Implementation beyond `W06` requires all of the following:

1. Completion evidence for the authorized workstreams.
2. A new design artifact for the specific continuation.
3. Review of that design against current code and domain ownership.
4. Explicit user authorization naming the continuation that may enter implementation planning.

Authorization for one continuation does not authorize the others.
