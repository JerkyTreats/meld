# WMR-DD-01 Owner Publication To TraversalCut Detailed Design

Date: 2026-08-21

Status: retrospective correction candidate

Implementation authorization: none

## Actual Problem

Current owner observations can be durably appended and graph materialized, but the path loses essential meaning before later reasoning. Workspace node payload contains path information that a generic graph walk cannot recover from the reached object alone. Graph facts preserve relation provenance, while walk results flatten relation occurrences. Currentness is expressed through a generic flag. A consumer can therefore receive locally valid graph data without one exact, owner-qualified, occurrence-rich knowledge boundary.

The failure is architectural because no isolated owner can repair it. Product owners define observations. Events owns neutral carriage. Graph owns admitted materialization. Traversal owns bounded reads and cut completeness. Later consumers own their own acceptance and judgment.

## Thesis

The first vertical closes when owner-qualified observations can be replayed into an immutable `TraversalCut` without losing identity, occurrence, scope, completeness, or hydration responsibility.

The cut is exact graph input. It is not truth, Belief, a complete `PlannerCut`, Curation completion, or Agent visibility.

## Scope

The active path is:

```text
owner source revision
-> bounded observation set and completeness receipt
-> retryable owner publication operation
-> neutral Event append
-> owner-routed graph admission
-> occurrence-preserving projection
-> bounded traversal against exact owner revision receipts
-> immutable TraversalCut and TraversalResult
```

Workspace and docs provide the primary product pressure. Dependency security provides the dissimilarity check. Curation, Belief, and Planner appear only as declared later consumers.

## Owner Publication Contract

An owner publication preserves four different semantic units:

- an opaque address for each cross-domain object
- an owner-qualified publication that states presence, scope, provenance, and revision
- an independently identified relation occurrence with qualifications
- a bounded completeness receipt for the observed source scope

The publication contract does not require all owner products to become graph nodes. A product opts into traversal only when cross-domain discovery, explanation, dependency reasoning, or later Plan construction requires stable addressing and relation semantics.

Workspace owns physical source, snapshot, node, path, containment, and observation meaning. Docs owns observed document claims and correctness products. Dependency security owns inventory, advisory, assessment, and verification meaning. None of those meanings move into Events or Graph.

### Completeness

Negative reasoning requires a positive completeness product. An observation set names the source revision, included scope, declared exclusions, failures, and terminal status. A missing object or edge inside storage is never enough to establish semantic absence.

For docs freshness, D1 publishes exact observed folders, files, source claims, README claims where present, and the completeness of the inspected source revision. D1 does not author an expected README or assess non-realization. Those are Curation-owned D2 products.

### Retry And Supersession

The owner durably records one publication operation before relying on Event append unless all material needed to recreate it already exists under an authoritative durable source revision. Deterministic reconstruction is valid only when the owner contract also fixes a complete enumeration rule and its revision, requires that enumeration during restart, and derives the same publication-operation and Event record identities from the same source products. If any of those conditions is absent, a durable publication operation or outbox is required.

Retry preserves owner observation identity and Event record identity. Process loss before append cannot erase the fact that publication remains pending.

A new source revision creates successor publications. Withdrawal, replacement, and continued coexistence are explicit owner lifecycle facts. Endpoint equality never implies relation replacement.

## Events Boundary

Events accepts producer-owned envelopes, validates ledger provenance and identity, assigns durable sequence, and replays records without interpreting product grammar.

The producer-owned typed publication batch is authoritative and travels inside the existing opaque producer payload. It contains or references each owner-qualified object publication, independently identified relation occurrence, completeness receipt, and hydration reference. The existing envelope object and relation attachments are bounded routing and graph-index hints. They may repeat addresses, relation families, and endpoints, but they do not define owner publication identity, occurrence identity, qualifications, scope, revision, or completeness.

The Event record identity binds the exact typed publication batch to one owner publication operation. Events persists and replays the payload without interpreting it. A declared Graph admission route later decodes the owner-owned batch and validates it against the neutral envelope hints.

The existing opaque producer payload, `DomainObjectRef`, `EventRelation`, envelope provenance, record identity, append receipt, cursor, and bounded replay are sufficient semantic seams. No new Events grammar is justified.

An append receipt proves only neutral carriage at one ledger position. It does not prove Graph visibility or cut inclusion.

## Graph Admission And Materialization

Graph admission validates that the publishing route is declared, the owner revision and scope are present, relation occurrence identity is stable, and replay is idempotent. It copies graph-readable material while retaining source Event position and owner provenance.

The current producer allowlist is evidence of an implementation seam, not a semantic owner registry. Detailed design requires deliberate owner publication contracts rather than domain ids embedded as universal graph policy.

Graph materialization preserves:

- object address and owner publication identity
- relation occurrence identity independent from endpoints
- observation-set completeness receipt identity, source scope, terminal status, exclusions, and failure references
- source Event identity and sequence
- valid and transaction time where supplied
- branch and perspective qualification
- owner currentness lifecycle
- source product and hydration reference

Graph does not infer truth, absence, correctness, evidence eligibility, or universal currentness.

## TraversalCut

`TraversalCut` is owned by the Graph and Traversal boundary. It selects one exact revision receipt and the required observation-set completeness receipts for every participating owner required by a query.

A cut records the owner set, exact owner revisions, observation-set completeness receipt identities and statuses, projection positions, temporal and branch scope, perspective, currentness policy, and cut completeness. Missing, failed, or nonterminal required completeness receipts produce an incomplete cut. Optional owners remain optional only when the query policy declares them so.

`TraversalQuery` supplies roots, relation families, direction, scope, owner policy, cut selection, and resource bounds. `TraversalResult` returns exact object publications, relation occurrences, required completeness receipt references with their bounded scope and terminal status, paths, provenance, frontier, truncation state, and the cut identity.

Equal normalized queries over equal cuts produce equal result identity. A changed owner revision, scope, policy, or projection version produces a successor cut or result.

Resource truncation is explicit. It can establish that the bounded query ended, not that the unexplored graph contains nothing relevant.

## Hydration Boundary

A traversal result must let a later consumer identify what was reached without parsing an opaque address or interpreting arbitrary Event payload.

Hydration remains owner-correct:

| Product | Semantic owner | Cut-carried material | Deferred read when needed |
| --- | --- | --- | --- |
| workspace folder or file | workspace | address, kind, path-safe display identity, source revision, provenance | exact workspace snapshot record |
| source claim | docs | claim identity, subject reference, owner revision, provenance | docs-owned claim product |
| observed README claim | docs | claim identity, README reference, source revision, provenance | docs-owned observation product |
| observation-set completeness receipt | workspace, docs, or dependency security | receipt identity, exact source scope, terminal status, exclusions, and failure references | owner completeness product and exact excluded or failed source records |
| dependency inventory | dependency security | inventory identity, manifest and lockfile references, owner revision | owner inventory record |
| advisory or assessment | dependency security | semantic product reference, qualification, owner revision | owner advisory or assessment record |

The cut may carry bounded projection values needed for discovery and explanation. Material qualifications remain intact owner products rather than flattened edge labels.

## Handoff Closure

### WMR-H01

The producer position is either a durable owner publication operation or a deterministic operation reconstructed from an authoritative durable source revision through a complete versioned enumeration rule that restart must execute. Events accepts the same stable record identity and exact typed publication batch on retry and returns an exact ledger position.

Closure requires the owner completeness account, retry identity, late-source fence, and restart source. Best-effort append alone does not close this edge.

### WMR-H02

The producer position is the Event record at exact ledger sequence. The consumer position is the Graph cursor through that sequence after projection writes and derived outbox effects are durable.

Closure requires owner-route validation, occurrence preservation, retention handling, idempotent replay, and a truthful blocked state for invalid publications.

### WMR-H03

The producer positions are exact owner graph revision receipts and projection positions. The consumer position is a bounded traversal result tied to one complete or explicitly incomplete cut.

Closure requires explicit owner participation, scope, currentness policy, frontier, truncation state, hydration ownership, deterministic identity, and restart reproducibility.

## Deferred Handoffs

`WMR-H04` supplies exact graph input to later `PlannerCut` assembly. Planner acceptance and multi-source cut identity close in `WMR-DD-03`.

`WMR-H05` supplies a bounded source cut to standing Curation. Operation identity, authority validation, durable acceptance, waits, wakes, and publication close in `WMR-DD-02`.

`WMR-H06` supplies exact graph products to configured Belief. Mapping, eligibility, reassessment wake, revision visibility, and Agent delivery close in `WMR-DD-02` and `WMR-DD-03`.

None of these deferred edges contributes readiness or quiescence evidence in D1.

## Product Proof

### Already-Correct Docs

1. Workspace publishes a complete folder and file observation set plus completeness receipt WCR under source revision S.
2. Docs publishes addressable observed source claims, observed README claims, and completeness receipt DCR under its exact owner revision.
3. Events assigns durable positions to those owner publications.
4. Graph materializes exact objects and relation occurrences through the required Event positions.
5. Traversal selects the required workspace and docs owner revision receipts plus WCR and DCR into cut C.
6. A bounded result reaches the folder, observed README, source claims, and README claims with exact occurrence, completeness, and hydration lineage.
7. The trace ends at observed graph input. Later Curation and Belief may establish correctness without requiring D1 to draft, publish, or execute a Task.

This proves that the first vertical no longer makes executable work a prerequisite for obtaining the observed inputs needed by epistemic closure.

### Missing Or Incorrect Docs

1. Workspace publishes a complete observed source scope and the files actually present.
2. Docs publishes observed claims for products that exist and exact completeness receipt DCR for its inspection.
3. Traversal returns the folder, source files, observed claims, DCR, declared exclusions, and complete source revision without inventing a missing README node or correctness verdict.
4. The result provides sufficient provenance and hydration for D2 Curation to author an expected README and assess non-realization or mismatch against the exact observed cut.

D1 proves observed completeness. It does not prove expectedness, absence, mismatch, or correctness.

### Dependency Security

1. Dependency security publishes an inventory and completeness receipt SCR bound to exact manifest and lockfile observations.
2. Advisory, assessment, and verification products retain their owner identities and qualifications rather than becoming docs-shaped claims.
3. Events carries the publications neutrally.
4. Graph preserves the references and occurrence lineage.
5. Traversal returns the exact security products and SCR allowed by query policy under one cut.

This proves that the shared boundary is owner-neutral and not a docs-specific ontology.

### Equal-Endpoint Occurrence Lineage Specimens

These specimens are semantic identities, not proposed Rust fields or storage keys.

| Trace | Owner occurrences with equal endpoints | Neutral Event positions | Graph facts | Cut inclusion |
| --- | --- | --- | --- | --- |
| workspace and docs | `docs-occurrence-17` and `docs-occurrence-18` both connect observed README R to source claim K under docs revision D7, while one qualifies direct coverage and the other qualifies example coverage | the typed batches are carried at E41 and E42 with the same endpoint hints and distinct occurrence identities | G41 and G42 retain both owner occurrences, qualifications, D7, payload identity, and hydration references | cut C9 selects docs revision D7 and completeness receipt DCR7, and the result returns both occurrences rather than collapsing them by endpoints |
| dependency security | `security-occurrence-31` and `security-occurrence-32` both connect assessment A to advisory V under security revision S12, while one qualifies production applicability and the other qualifies a development-only exclusion | the typed batches are carried at E73 and E74 with distinct publication identities | G73 and G74 retain both assessments, qualifications, S12, inventory lineage, and hydration references | cut C12 selects security revision S12 and completeness receipt SCR12, and the result returns both occurrences under the requested policy |

The object and relation attachments on each Event can be equal in these specimens. The producer-owned typed payload, source Event position, and Graph fact preserve the semantic distinctions.

### Lag And Restart

If Events is durable through N while Graph is at N minus one, the Event position is valid and the graph consumer remains behind. If one required owner revision or completeness receipt is missing, failed, or nonterminal, the cut is incomplete. If a process restarts, the durable publication operation or authoritative source revision plus mandatory enumeration rule, Event cursor, graph projection state, owner receipts, normalized query, and cut identity are sufficient to resume or reproduce the result.

No state borrows another owner's cursor.

## Sacred Seams

- Events remains a neutral durable carrier.
- Graph and Traversal remain structural read substrate.
- product owners retain meaning, presence, and currentness policy.
- Curation authors expected entities and epistemic relations later.
- Belief admits evidence later.
- Planner assembles the complete reasoning cut later.
- lifecycle aggregates receipts later without interpreting semantics.
- legacy Workflow is not a compatibility target.

## Downstream Preconditions

After Gate Acceptance, `WMR-DD-02` may rely on exact owner publications, complete observed source scopes, occurrence-rich traversal results, deterministic cuts, and declared hydration routes.

`WMR-DD-03` may later rely on the same graph cut as one exact input to complete `PlannerCut` assembly. It must still define owner-specific inputs from Belief, Causation, Regime, directive context, Capability catalog, authority, scope, and projection policy.

## Unresolved Later-Phase Questions

- exact Curation operation and standing-work acceptance products
- Belief eligibility and wake behavior after graph-only change
- complete `PlannerCut` assembly and `WorldModelView` terminology retirement
- implementation storage and API shapes
- activation-wide participant, wait, wake, safe-point, and retirement aggregation

These questions do not prevent D1 from closing and may not be answered by expanding its authority.
