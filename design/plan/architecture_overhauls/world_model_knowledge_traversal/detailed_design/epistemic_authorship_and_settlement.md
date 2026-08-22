# Epistemic Authorship And Settlement Detailed Design

Date: 2026-08-21

Slice: `WMR-DD-02`

Status: accepted design product with retrospective clarification

Implementation authorization: none

## Decision

Curation is one world-model semantic owner with two invocation contexts. Standing Curation applies an installed Agent specification to admitted knowledge. Strategy-planned Curation accepts an exact Agent-authorized Plan product. Both contexts use the same bounded operation, terminal result, authorship, publication, currentness, and replay semantics.

The vertical closes through independently visible owner positions:

```text
exact initiating authority plus immutable TraversalCut
-> Curation acceptance
-> bounded Curation authorship
-> terminal Curation result
-> Event append
-> Graph projection visibility
-> configured Belief revision when an evidence route exists
```

Agent acceptance and complete `PlannerCut` assembly are declared downstream positions. They do not need to exist for this design vertical to be coherent.

## Owner Boundaries

| Concern | Owner | Design rule |
| --- | --- | --- |
| observed entity or claim existence | workspace, docs, dependency security, or other semantic source | Curation may cite but cannot impersonate the owner |
| structural discovery | Graph and Traversal | returns occurrence-rich bounded material under one immutable cut |
| expected entities and Agent-scoped epistemic connections | Curation | authors only installed Curation vocabulary under exact perspective |
| operation authority | Agent specification for standing work, Agent Plan authorization for planned work | initiating authority remains visible in every accepted operation |
| neutral carriage and ordering | Events | assigns durable sequence without interpreting Curation meaning |
| structural materialization | Graph | admits and projects published objects and relation occurrences without settling truth |
| evidence admission and settlement | Belief | applies installed mappings and comparators to produce immutable revisions |
| Plan progression and Goal satisfaction | Agent | consumes later owner results and never equates operation completion with satisfaction |
| reasoning-cut assembly | Planner | later binds exact Graph, Belief, and other owner revisions |

Traversal does not author knowledge. Curation does not mutate Graph or Belief storage. Events does not validate epistemic grammar. Belief does not inherit every Curation result automatically. Agent does not become the Curation executor.

## Bounded Operation Contract

A complete operation must make the following semantic dimensions exact without prescribing a runtime representation:

- invocation context and initiating authority identity
- Agent, perspective, branch, and activation generation
- standing rule revision or planned operation revision
- exact subject and root identities
- accepted `TraversalCut` and owner-hydrated input revisions
- direction, relation vocabulary, owner filters, frontier, and resource bounds
- Curation-owned output vocabulary and foreign-owner proposal limits
- preconditions and owner currentness expectations
- completion, abstention, incompleteness, conflict, rejection, and failure meaning
- idempotency, publication, supersession, and result-visibility policy

The complete write set must be known before effects, or derivable through an owner-approved bounded query whose exact cut and frontier become part of the result. A live graph read, ambient Agent state, process-local authorization, or latest-record lookup cannot complete the contract.

## One Authority Across Two Invocations

Standing and planned Curation differ only in how an operation becomes eligible.

Standing selection begins with an installed Agent specification and Curation rule. It may establish expected state before a Goal exists. This bootstrap role is essential for producing an explicit expected-versus-observed mismatch.

Planned selection begins with an exact Agent authorization for a Strategy Plan product. Curation validates the authorization envelope against its own operation grammar. `WMR-DD-02` defines this consumer acceptance boundary but does not define who stores or progresses the Plan. That producer closes in `WMR-DD-03`.

After admission, both contexts share one execution path and one terminal result grammar. Preadmission rejection uses one shared acceptance-rejection receipt grammar. A consumer can therefore interpret Curation outputs without knowing which scheduler or intake transport caused the work.

## Acceptance And Terminality

Curation first records one durable acceptance decision under the current authority and generation fence. The decision either admits the operation or rejects it before admission. Admission is not execution and is not semantic completion.

A preadmission `rejected` decision is an observable terminal intake outcome with an exact rejection receipt. It is not an accepted-operation result, does not create semantic publications, and does not enter `WMR-H08` terminal-result publication.

Every admitted operation reaches exactly one terminal result identity. Its terminal classes are `applied`, `unchanged`, `abstained`, `incomplete`, `conflicted`, and `failed` as defined in the [transition ledger](epistemic_operation_transition_ledger.md).

`unchanged` is first-class success when the requested Curation-owned state already holds under the exact cut. It names the existing semantic products so consumers can distinguish closure from work that never ran.

`incomplete` is bounded evidence. It must name the frontier, exclusions, source failures, and exact cut. It cannot be upgraded to absence or correctness.

Preadmission `rejected`, postadmission `conflicted`, and postadmission `failed` are observable at their exact Curation positions. They do not publish the desired semantic assertion and do not silently retry under the same operation identity.

No terminal class universally satisfies a Goal. Agent owns that later judgment.

## Semantic Authorship

Curation may author:

- expected entities distinct from observed entities
- requirement, materiality, relevance, comparison, realization-assessment, and coverage-assessment occurrences inside installed Curation vocabulary
- bounded non-realization assessments over complete declared observation scope
- withdrawals and supersession products for Curation-owned material
- proposals addressed to a foreign semantic owner when Curation cannot own the requested meaning

Every shared product retains Curation authority, Agent perspective, branch, source-cut lineage, rule revision, provenance, and owner currentness meaning.

Curation may not assert physical presence on behalf of workspace, claim extraction on behalf of docs, advisory authority on behalf of a security source, verification truth on behalf of a verification owner, or belief confidence on behalf of Belief.

Generic graph coordinates and relation names are structural attachments. Typed Curation hydration remains necessary to recover perspective, bound, policy, currentness, and completion meaning.

## Publication And Visibility Barriers

Terminal state is durable in Curation before Curation reports publication complete. Semantic effects and the terminal result are appended idempotently through Events with deterministic producer identities.

The following barriers are independent:

| Barrier | Evidence | What it does not prove |
| --- | --- | --- |
| Curation rejection | exact acceptance-rejection receipt is durable | operation admission, terminal result, or semantic effect |
| Curation terminal | exact result identity and disposition are durable | Event append or downstream visibility |
| Event durable | append receipt names ledger identity and sequence | Graph materialization or semantic admission |
| Graph visible | projection position is at or beyond every required result sequence | Belief evidence or currentness judgment |
| Belief settled | installed route produced an exact immutable revision | Agent delivery, Plan progression, or Goal satisfaction |
| Agent accepted | later Agent position records result consumption | complete `PlannerCut` or Goal satisfaction |

An applied result may carry several semantic publications. Graph visibility requires the projection position to cover all Event sequences named by that result, not merely the terminal Event.

## Configured Belief Settlement

Belief consumption is explicit and selective. A Curation result becomes eligible only when an installed evidence route names the result class or semantic product, target belief key, perspective, mapping revision, and relevant source dependency.

The route may consume the Event publication directly or use owner-hydrated Graph material, but it must preserve the exact Curation result and source-cut lineage. Graph reachability alone is not evidence. An unmapped result may become Graph-visible and remain intentionally absent from Belief.

When a relevant Curation or owner revision changes, Belief must have a durable basis for selecting the affected key again. This design requires the dependency relationship and resulting revision position but leaves the implementation choice between existing Event mapping and a Belief-owned dependency index to a later implementation program.

Belief commits domain state before advancing its own consumer position. The resulting revision is immutable and distinct from the Curation result. `WMR-DD-03` decides how that revision enters an exact `PlannerCut` and how Agent consumes it.

## Replay, Feedback, And Currentness

The operation identity freezes every semantic input. Replay of the same accepted operation returns the same terminal result identity and reuses deterministic Event publication identities.

Curation outputs can appear in later Traversal cuts. This is a controlled feedback path, not implicit recursion.

- the accepted operation never expands its input cut after acceptance
- self-produced products are inputs only when the installed rule names them
- reaching the same requested state returns `unchanged`
- a successor cut creates successor work only when a declared dependency changed
- Event replay and Graph catch-up never grant fresh authority
- output publication is finite under the declared bound

Curation owns currentness for its own products. It uses explicit validity, withdrawal, or supersession semantics tied to rule and source revisions. Event time, Event order, endpoint equality, and Traversal `current_only` cannot replace owner policy.

A failed standing operation remains terminal under its existing identity. A successor requires a named semantic trigger such as renewed authority, an explicit retry generation, or a changed declared input. A deadline may wake eligibility evaluation, but it cannot silently retry the failed identity. Runtime implementation cannot begin until the Curation contract selects and records the permitted successor trigger.

## Lifecycle Account

Curation readiness means that the semantic owner can validate initiating authority and consume the exact bounded source cut. It does not mean that planned authorization already exists or that Graph and Belief are caught up.

Curation waits on named structural conditions such as a missing owner revision, unavailable cut, uninstalled rule, unresolved authorization, or exhausted eligible work. Wake references are the exact new rule, source, projection, authorization, or deadline positions that can change eligibility.

Every accepted operation is fenced by Agent, perspective, branch, activation generation, rule or operation revision, source cut, and authority lineage. Late work can finish only under the result policy of that fence and cannot silently publish as current for a successor generation.

Restart begins from Curation acceptance and terminal positions, then reconciles Event append, Graph projection, and configured Belief positions independently. Replayed transport does not repeat semantic work when the terminal identity already exists.

Curation-local quiescence requires every accepted operation to be terminal and no eligible work through the observed selection positions. It says nothing about Graph, Belief, Agent, the full activation, or safe retirement. Those aggregate claims close in `WMR-DD-06`.

## Product Proof

### Already-correct README

The installed rule identifies governed folder F and expected README F under one Agent perspective. The accepted `TraversalCut` includes complete workspace scope, the observed README revision, addressable source and README claims, and existing Curation-owned requirement and coverage products.

Curation validates that the requested state already holds and emits `unchanged`, citing the exact semantic products. If products are missing but derivable from existing admitted evidence, it emits `applied` and publishes them. Either result can become Graph-visible and, when configured, Belief-settled. No Goal, Strategy Plan, Task, publication receipt from a write pipeline, or Execution outcome is required.

### Missing or incorrect README

Curation authors expected README F and requirement relationships without asserting a physical file. It evaluates realization only against a complete workspace observation scope carried by the accepted cut.

If no observed README realizes the expectation, Curation publishes a positive bounded non-realization assessment naming the snapshot, governed scope, exclusions, failures, and expected entity. It does not infer absence from a missing edge.

If a README exists but claim coverage is incomplete, Curation publishes only the requirement, materiality, and bounded comparison products its installed vocabulary owns. Whether that mismatch warrants a Goal and what causal plan should follow remain Agent and Strategy concerns in `WMR-DD-03`.

### Dependency security dissimilarity

The exact cut may contain inventory observations, advisory publications, owner assessments, and verification results. Curation may author Agent-scoped relevance, required-comparison, or bounded coverage relationships among those products.

It cannot turn advisory mention into installed-package presence, replace the assessment owner's disposition, or claim verification completion from graph reachability. A result that lacks one required owner revision terminates `incomplete` rather than manufacturing a foreign-owner assertion.

## Downstream Handoff

`WMR-DD-03` may rely on:

- one constructible bounded operation grammar shared by standing and planned invocation
- exact Curation acceptance and terminal result identities
- explicit result, Graph, and configured Belief visibility milestones
- Curation-owned expected entities, mismatches, and assessments with exact perspective and source lineage
- a declared Agent acceptance position that remains unimplemented and undesigned here

`WMR-DD-03` must still define planned authorization production, Plan product progression, the exact milestone that satisfies each causal dependency, Agent result acceptance, and complete `PlannerCut` assembly.

## Non-Goals

- runtime types, APIs, schemas, storage placement, actors, or scheduling
- a new Event grammar or generic command protocol
- a universal graph ontology or evidence route
- Strategy Plan shape, Agent Plan store, or progression algorithm
- Execution intake or returned observation
- PDS compilation, root runtime composition, activation supervision, or retirement
- compatibility with legacy Workflow
