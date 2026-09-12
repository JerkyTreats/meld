# Graph engine fit and maintenance spike

Slice `graph-engine-fit-v1`. Lifecycle: closed. Readiness: evidence supports architecture assessment; native crash recovery and production adoption remain unqualified. The user's subsequent “Proceed” authorizes the bounded prototype comparison, local dependencies and isolated state needed by its arms, harness measurement, and checkpoint commits and pushes. Production replacement remains unselected. This record defines the evidence exercise, not a selected database architecture.

## Reason and decision

The user identifies two equal concerns: a bespoke implementation could create endless maintenance, while an external graph system could force Meld's semantics into an unsuitable shape. The spike must expose both costs. Its result is permission to make an informed architecture decision, not an automatic preference for the fastest benchmark or the smallest dependency list.

The outcome remains excellent command-line ergonomics that prove continuous reconciliation. A performant default should emerge from the complete path from admitted Events to useful Agent work. An engine choice is only one input to that design.

The [OTel closeout](../../../../../meld-eval/evidence/runtime-otel-v1/CLOSEOUT.md) attributes 107.22 of 124.22 seconds across two passes to graph publication reads. Forty-three scans process 3.59 GB cumulatively. The retained history contains about 83 MB across 74 publication values. These are debug-build observations. The earlier approximately 32 MB syntax publication is a different workload and measures a serialized envelope, not total Graph storage.

Current `GraphRuntime` already advances a durable Event cursor. `TraversalQuery::cut` reloads publication history to select owner revisions; `traverse` reloads it again and constructs temporary object and edge indexes. The shared Sled store exists already. The experiment must distinguish a deficient read projection from a deficient storage engine before attributing the problem to either.

Baseline branches are `feat/event-up-go-wide`: Meld `430a7c8a`, Eval `33448e7`, README `db7885b`, Codebase Semantics `54a798f`. All four remote tips and clean worktrees were verified before this design. The retained qualification and raw trace evidence remain intact.

## Hypotheses and falsifiers

| Hypothesis | Evidence that supports it | Evidence against it |
| --- | --- | --- |
| A small maintained projection on existing storage is enough | Repeated queries avoid history decoding, frozen cuts remain correct, and the maintained code is limited to application indexes and explicit revision selection | The prototype needs a custom transaction engine, general query optimizer, elaborate concurrency control or substantial independent recovery machinery |
| An established engine removes meaningful maintenance | It supplies useful indexing, query or consistency machinery and the adapter preserves native contracts with little duplicate logic | Meld still implements the same machinery beside the engine, or the adapter erases occurrence identity, completeness, provenance or durable cut meaning |
| The dominant cost is an access-pattern defect | Fixed live data with growing history no longer makes unchanged queries scan all retained publication bodies | Cost remains proportional to unrelated history or moves into a hidden conversion, snapshot or owner callback path |

No hypothesis assumes that denser domain semantics or a smaller publication is necessary for the first performance improvement. Hold publication meaning and shape fixed for the engine comparison. Record representation-size opportunities separately for the later architecture assessment.

## Small comparison, explicit semantic contract

Use three arms: the unchanged baseline, one bounded indexed projection on the current engine, and one established external candidate. Survey at most three external candidates through primary documentation and a short contract mapping, then prototype only the strongest fit. An embedded general-purpose database is eligible alongside graph-specific engines. Record exact versions, licensing, maintenance evidence, build dependencies, deployment requirements and rejection reasons. Do not conduct an open-ended database tournament. If none fits, retain the concrete incompatibilities rather than forcing a nominee.

The existing-storage arm may implement owner revision lookup and reusable object and relation indexes. It may not become a new WAL, page manager, transaction engine or general-purpose graph language. The external arm must account for code removed as well as adapter code added. A large dependency is not automatically expensive to maintain, and a small bespoke module is not automatically cheap.

Both arms answer the same native contracts:

- External authorship enters through Events. Graph remains the derived projection authority; the harness never injects records into its private stores.
- Preserve owner and revision identity, source Event provenance, relation occurrence identity and material qualifications. Equal endpoints do not collapse distinct occurrences.
- Select the same eligible revisions at a cut. New incomplete publication, projection lag, missing information and qualified absence remain distinguishable.
- Preserve explicit scope and currentness behavior. Mark intended temporal, branch and perspective semantics that lack present product proof as unproven; a prototype does not silently qualify them.
- Allow a retained cut to be queried after newer publications and after restart. Determine the required retention contract explicitly; a database transaction snapshot alone does not establish durable historical access.
- Preserve bounded traversal results, path lineage and truncation. Index preparation and publication decoding must be measured even when outside traversal bounds.
- Recover without advancing the advertised projection cursor beyond durable indexed data. Duplicate replay must not duplicate semantic publication or relation identity.

The canonical [Graph design](../../../cognitive_architecture/world_model/graph/README.md) supplies the semantic boundary: domains may keep their own stores, and the traversal substrate does not require a universal ontology. A candidate may represent those contracts differently internally, but translation must be explicit and lossless for the exercised products. Reified records and auxiliary indexes count toward its complexity, rather than being concealed as incidental glue.

## Harness exercise and observations

Extend Meld Eval's existing artifact capture, native pass accounts and OTLP receiver. Use isolated Gmail Operator checkouts and fresh external state roots. Build explicitly before measurement. Each arm uses an isolated candidate artifact and exactly one canonical Graph implementation behind the existing command path. Do not install a permanent backend selector or let two implementations write the same product. Backend-only measurements may supplement the journey but cannot replace native command evidence.

Run the same authored domain inputs and query sequence on each arm. Record run-specific ledger identities and compare exact meaning and provenance through an explicit identity mapping where necessary. Do not normalize away a semantic difference. Preserve existing qualification oracles, including their known limits.

| Journey | What it must reveal |
| --- | --- |
| Initial ingest and first query | Admission, validation, indexing, cold-start time and serialized-to-resident expansion |
| Repeated unchanged reads and idle passes | Whether work scales with the queried region or repeatedly with publication history; include status and wake-check costs |
| Change, add and delete one file | Update amplification, completeness, supersession, unaffected identity and eventual visibility through native queries |
| Read an older cut while new data arrives | Version isolation, provenance and whether new incomplete data is misrepresented as current |
| Crash at projection persistence boundaries, then restart | Durable cursor correctness, replay idempotency, recovery time and absence of partial indexes presented as complete |
| Modest growth of retained history, then live data separately | Distinguish history-driven costs from working-set growth; record the exact generated Event sequence |

Run the narrow correctness journey first; do not tune an arm that fails it. For viable arms, use identical build profiles, hardware, input order and observer settings. Report cold and warm observations separately. Use repeated fresh-state comparisons and alternate arm order; do not repeat the previous sequential shared-state comparison as an overhead claim. Finish builds before timing. Use a consistent optimized profile for the comparative run and retain the debug baseline as diagnostic evidence, never as its direct speed denominator.

Capture end-to-end latency and its components, peak resident memory, retained storage size, serialized publication size, decoded bytes, index/update work, query counts, startup and recovery time, and observer/export cost. Include all candidate processes. Wall spans are not CPU samples or disk-I/O measurements. State missing coverage and sampling limitations. Model calls remain zero during the graph-focused probe; a later product regression may exercise the existing README journey and must identify provider time separately.

No production latency SLO is invented here. Before interpreting the comparison, record which command and reconciliation budgets the results would need to satisfy. If those budgets remain undecided, report the measured envelope and scaling rather than claiming production readiness.

## Maintenance and fit account

For each viable arm retain a concrete ownership account: indexing and query code, snapshot and retention logic, recovery obligations, migrations, dependencies, adapter translations, operational processes and failure modes. Name the existing production surfaces it could retire and the machinery Meld would still own. Estimate recurring work with reasons; lines of code alone are insufficient.

Compare semantic correctness first, then product latency and resource cost, then ongoing implementation and operating burden. A faster candidate that loses evidence meaning fails. A correct candidate requiring a bespoke database subsystem triggers reassessment. An external candidate that preserves meaning only by reproducing the current Graph beside it has not demonstrated a maintenance advantage.

Bound execution to the two prototypes and one focused correction per arm. Larger public APIs, services, custom storage machinery or a changed domain grammar become findings requiring reassessment. Missing prototype evidence is an unknown, not a pass or an invitation to keep expanding the spike.

The circuit breaker is a demonstrated inability to preserve the native Event, cut and reconciliation contracts without a competing authority or substantial new database machinery. Stop and bring that evidence to the whole-Graph assessment. Failure of both arms does not authorize writing a database by elimination.

## Responsibility and policy boundaries

The affected set is World Model, Events, Runtime, native adapters, Telemetry, Eval and the two external domain packages. This is the set to inspect and exercise; it is not a blanket source-edit authorization. The current code grounds are `world_state/graph`, `world_state/query_runtime`, Agent planner assembly, Curation, runtime lifecycle/wake projection, owner transport and the Eval profile/OTLP journey.

| Responsibility | Disposition | Existing route and intended proof |
| --- | --- | --- |
| Graph admission, publication store, query indexes and revision cuts | retain production authority; assess isolated alternatives | `GraphRuntime` and `TraversalStore` through `WorldModelQueries`; prototypes exercise the same native contract independently |
| Event durability and replay | retain | Canonical Event ports and projection cursor; no Event engine replacement or private Graph authorship |
| Agent, Planner, Strategy, Belief and Curation meaning | retain | Existing admitted inputs, immutable planner cuts and owner publications; no new decision authority |
| Runtime scheduling, wake resolution and owner transport | retain, observe | Existing supervisor and owner command paths; expose cost migration without changing planner behavior to improve a score |
| CLI, served reads, initialization and configuration | retain; thin adapter extensions only if a measured capture gap requires them | Existing public commands and prepared selections; no permanent backend abstraction from the spike |
| Telemetry and Eval | extend existing capture when needed | Preserve native timings, OTLP coverage and executable identity; retain rejected runs |
| README and Codebase Semantics | retain domain behavior | Supply the qualified workload through their existing Events and owner interfaces; no README ontology in core |

The top-level source-domain sweep is bounded to the concern. `runtime` owns composition and lifecycle; `events` and `meld-events` publish replay; `meld-world-model` owns Graph and consumes it in Agent, Planner and Curation; `telemetry` observes; `bin`, `cli`, `serve`, `init` and `config` adapt the exercised path. External Eval owns the driver, README consumes and publishes, and Codebase Semantics publishes. Each has a direct integration described above. One-level follow-up concerns are Graph admission/indexing/cuts/recovery, runtime composition/wakes/transport, Event replay/cursor ordering, adapter routing, and measurement integrity.

No direct behavior change is selected for `src/agent`, `branches`, `capability`, `code_change`, `context`, `control`, `execution`, `harness`, `merkle_traversal`, `metadata`, `nonce`, `prompt_context`, `provider`, `session`, `store`, `task`, `theory`, `tree`, `workflow`, `workspace`, `meld-execution` or `meld-lang`. These are explicitly retained consumers or unrelated mechanisms, not additional prototype ownership. Their applicable contracts still constrain the experiment. A newly discovered direct dependency is recorded before expanding scope.

Apply [Runtime Invariants](../../../../governance/runtime_invariants.md) to single authority, intact semantic products, external state placement and recovery. Apply [Contribution Policy](../../../../governance/contribution_policy.md) to source organization, comments, disclosure and commits. Any later replacement must route real callers to one successor and retire superseded authority in that completed change. This spike does not authorize dormant production backends or speculative compatibility layers.

Maturity is exploratory with direct evidence for a narrow multi-owner consumer and a measured performance failure. Confidence is high in the identified repeated-read cost and limited in engine fit, future workload and retention needs. Durable evidence, cut identity and native lifecycle are the obligation floor. No new claims of whole-document README quality or general language adequacy follow from this work.

## Exit and whole-Graph architecture assessment

Exit with a reproducible comparison, native semantic checks, maintenance accounts, fit failures and explicit unknowns. An inconclusive result is valid. Preserve evidence that contradicts a preferred approach. Do not commit prototype machinery into the production path merely because the spike concludes.

The next assessment must consider the Graph as a whole: publication granularity and encoding, Event admission, validation lifetime, revision and historical retention, reusable indexing, query and traversal planning, planner input assembly, wake dependency lookup, owner transport and hydration, memory limits, crash recovery, compaction and observability. Determine where work belongs on ingest, on change and on read. Increasing idle consumers or unrelated retained history should not implicitly multiply full-history reconstruction.

That assessment produces an evidence-backed architecture for performance by default, including operating budgets, tradeoffs, canonical ownership, storage choice, migration and retirement scope, and harness proof. It updates `design/cognitive_architecture` only once the intended design is accepted. This temporal spike record does not preempt evergreen design or authorize implementation of the successor.

Design review: the native path, semantic gates and maintenance comparison are specified; independent prototypes have no production cutover authority. The initial design was accepted and pushed as `5d137f65`. Execution is now authorized. The bounded comparison is complete. Six final journeys pass native semantic checks and verified cross-run input accounting; indexed Sled removes the measured history penalty without a new database subsystem. SQLite preserves the exercised contracts but has not demonstrated a maintenance advantage in this adapter. All crash probes are blocked before recovery readiness by the predecessor runtime lease. The production successor remains a proposal.

Commit effect: If applied, this commit records a bounded comparison that exposes both bespoke maintenance cost and external engine semantic mismatch before selecting Meld's Graph architecture. Working branches isolate the evidence ledger, indexed prototype and SQLite prototype. No production cutover is authorized by completion of the spike.

## External shortlist and selected arm

SQLite through rusqlite 0.40.2 is selected for the external prototype. SQLite provides maintained transactional tables and indexes, accepts opaque owner identifiers and qualified relation records, and embeds in the existing command process. The adapter still owns explicit historical revision rows and Meld traversal ordering; those costs must remain visible. The bundled SQLite version will be recorded from the built artifact. SQLite is public domain; rusqlite is MIT licensed and adds a bundled C build. [SQLite durability](https://www.sqlite.org/transactional.html), [isolation](https://www.sqlite.org/isolation.html), [release history](https://sqlite.org/changes.html), [rusqlite](https://docs.rs/rusqlite/0.40.2/rusqlite/).

Redb 4.2.0 is a plausible embedded alternative with transactions and MVCC, but its key-value interface leaves graph indexing and query work with Meld. It is not selected because this arm should test a greater delegation of indexing work than another KV engine. Its recent releases, pure Rust implementation and MIT/Apache-2.0 licensing remain useful comparison facts. [Redb](https://docs.rs/crate/redb/4.2.0).

Oxigraph 0.5.11 is the graph-native shortlist candidate. It offers an embedded Rust RDF/SPARQL store backed by RocksDB under MIT/Apache-2.0 licensing. Mapping occurrence identities, qualifications and historical owner cuts would require explicit RDF records and named-graph conventions. This is possible, not disproved, but introduces a second query representation without an observed need for SPARQL. It is not selected for this bounded prototype; this decision does not establish that graph-specific engines cannot fit Meld. [Oxigraph](https://docs.rs/crate/oxigraph/0.5.11), [architecture](https://github.com/oxigraph/oxigraph/wiki/Architecture).


## Execution closeout

The [evidence closeout](../../../../../meld-eval/evidence/graph-engine-fit-v1/CLOSEOUT.md) retains six final release journeys, three diagnostic trace journeys, six failed crash-recovery probes, calibration failures, exact artifact identities and dependency locks. All normal final journeys pass 46 checks. Seven Graph tests pass in each arm and 28 Eval tests pass. Distinct occurrence identity, incomplete supersession and retained cuts after clean restart are preserved. Immediate crash replay remains unqualified because a prior owner lease prevents native restart in every arm, including baseline.

The [whole-Graph assessment](graph_architecture_assessment.md) recommends a maintained versioned read projection on existing Sled, with explicit work placement for admission, cuts, traversal, wake checks, planner inputs, payload reuse, retention and recovery. The recommendation does not select a production cutover. The failure gate is native recovery, not a demonstrated inability to represent Meld semantics without inventing a database.

Prototype checkpoints are Meld `cff89b22` on `spike/graph-engine-fit`, indexed Sled `b6ae182d` on `spike/graph-indexed`, and SQLite `8097bb9d` on `spike/graph-sqlite`. All three have been pushed. Eval `9b5d128` on `spike/graph-engine-fit` retains the harness, final comparison and failed recovery evidence. The shared public command changes expose retained cuts and permit bounded full-publication Event append. The experimental failpoints stay isolated on spike branches. The production baseline branch and the two domain packages are unchanged.
