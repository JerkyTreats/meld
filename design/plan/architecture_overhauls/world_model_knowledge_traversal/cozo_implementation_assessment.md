# Cozo implementation assessment

Status: assessed on `spike/graph-crate-fit`, following Meld `b204b164` and Eval `9f545a8`. Native integration of the pinned release is on hold. This assessment supersedes the immediate promotion recommendation in the [graph-crate experiment](graph_crate_experiment.md), while preserving its narrower measured results.

**Cozo offers useful query machinery, but the released dependency and the prototype mapping are not yet a dependable foundation for Meld's Graph.** The next decision is whether a supportable Cozo version can satisfy the missing contracts without making Meld maintain a database fork. Implementing a full native adapter first would put that decision too late.

## New evidence that changes the decision

The [follow-up harness evidence](../../../../../meld-eval/evidence/cozo-implementation-assessment/README.md) uses the unchanged Cozo 0.7.6 dependency and a separate explicitly built diagnostic executable. Two fresh stores reproduce silent omission of the inclusive lower-bound row in both a numeric range and a scoped revision range. Exact lookup of the omitted row succeeds.

For keys 0 through 9, the query below should return keys 3 through 6. It returns only 4 through 6:

```text
?[k,v] := *plain{k,v}, k >= 3, k < 7
```

The same defect omits revision `003` from a scoped range beginning at `003`. Upstream [PR 286](https://github.com/cozodb/cozo/pull/286) corrects range-bound construction in stored relation joins. The pinned source still has the pre-fix expression. This is directly relevant to the range access paths a native revision index would introduce, though it does not prove every possible Event-frontier query is affected. The original screen used equality-selected revisions and did not cover this case.

The dependency also has a concrete integration defect already handled in the probe: `MultiTransaction::commit` treats a received error result as success. The probe reads the public result channel directly. This assessment does not claim to have induced an actual failed engine commit. The source defect and the observed range defect are separate findings.

Upstream maintenance needs explicit consideration. GitHub currently reports [v0.7.6](https://github.com/cozodb/cozo/releases/tag/v0.7.6), published December 11, 2023, as the latest release. The latest default-branch commit is [481af058](https://github.com/cozodb/cozo/commit/481af058abac9444ea8c9c52c78f096ed4b5bfc4), dated December 4, 2024. The repository is not archived. These facts do not prove abandonment, but they do not support assuming timely releases or upstream fixes for Meld's future failures. A merged fix is not present merely because Cargo selects the latest published version.

## What the implementation really delegates

The [current adapter](../../../../../meld-eval/probes/graph-crates/src/cz.rs) uses Cozo's relation encoding, indexed access, recursive Datalog evaluation and transactions. Its SQLite backend stores encoded tuples in a single key/value table; SQLite is the persistence substrate, while Cozo supplies the relation and query layer. It is not a collection of ordinary SQL graph tables with SQLite evaluating the Datalog.

This is useful delegation. The current Meld [query](../../../../crates/meld-world-model/src/world_state/graph/query.rs) prepares a complete in-memory graph before answering a bounded question. Cozo demonstrates that relationship expansion can occur in the engine with indexed revision selection. The original probe's stable history test remains evidence for that mapping, under its serialized workload.

The adapter still copies the full payload of every object and occurrence for every revision. Relation type, qualifications and evidence remain in JSON bodies rather than selective query columns. Its approximately 25.27 MB repository store therefore does not settle the representation problem. Shared immutable content, evidence bindings and revision membership remain an independent design choice. Cozo can plausibly represent them as keyed relations, but that layout has not been implemented or measured.

CozoScript would remain a private Graph implementation language. This assessment proposes no changes to meld-lang, no database queries in WAD domain theory, and no replacement of Strategy planning with database rules. Cozo would answer selected Graph questions; Meld would continue deciding what admitted knowledge means and which actions are justified.

## Contract gaps in the prototype

| Concern | Current Cozo probe | Native Meld requirement and assessment |
| --- | --- | --- |
| Identity and admission | Sequence-only publication key, synthetic shape validation, external retry guard | Preserve ledger identity, exact Event reference, installed source route and typed operation validation. Reuse canonical admission rather than accepting external database writes. |
| Historical selection | Exact saved sequence or current head for one scope | Select each eligible owner revision at the captured Event frontier, accounting for projection progress and source coverage. A current-head lookup is insufficient. |
| Incompleteness | Stores the newest incomplete header | Native cut assembly must reject its use as complete and preserve optional-owner and qualified-empty behavior. Merely retaining the status field does not implement that decision. |
| Paths and bounds | Single-root outgoing reachability and one output limit | Preserve multiple roots, direction, filters, occurrence-qualified paths, stable order, independent bounds and an unexplored frontier. Native equivalence is unproved. |
| Atomic mutation | Writes rows and heads in one transaction; guard and head decisions are read before it | Serialize the complete admission decision or move checks into the same transaction. Keep publication visibility and durable projection progress coherent across interruption. |
| Concurrent readers | Sequential processes in the original screen | The SQLite backend holds an exclusive lock for writes. A frozen reader of unchanged data waits during that write. Sustained latency and fairness remain unmeasured. |
| Initialization and retained data | Creates schemas only if no relations exist | Partial initialization, schema version, reopen validation, old-cut retention and migration need explicit behavior. An arbitrary nonempty database is not proof of a complete schema. |
| Work and observability | Returned records and elapsed time | Output limit is not an examined-work budget. Expose actual query plan, cancellation behavior, contention and work exhaustion through the harness. |

The path gap is substantial. Cozo's probe carries address and depth during recursion, then emits a set of occurrences. Meld's current traversal constructs ordered path witnesses while applying bounds and records why exploration stopped. Merely adding path arrays to the Datalog rule could enumerate many more paths or change which paths survive a bound. The next test must compare full native results on cycles, parallel evidence and diamond-shaped graphs; reachability agreement is not enough.

Cozo [documents](https://docs.cozodb.org/en/dev/queries.html) output limits and query timeouts. Those facilities may help implement cancellation, but neither establishes Meld's independent object, occurrence, path or physical-work accounting. Adding a large result limit to an expensive recursive query does not qualify bounded work.

## Continuous reconciliation and storage ownership

The pinned SQLite backend obtains a shared lock for a read transaction and an exclusive lock for a write transaction. The diagnostic starts a reader of unchanged data while holding a write transaction open. It cannot finish during the 300 ms observation window, then completes correctly after commit in both runs. This is blocking, not evidence of incorrect data or sustained starvation. The backend's concurrency contract must be assessed at the actual publication size and frequency.

This is particularly relevant to Meld because sensors publish while Agents consume retained evidence. A frozen cut removes semantic dependence on new writes; it does not automatically remove the engine lock dependency. Moving to another Cozo backend changes the experiment and introduces a different dependency and operational account. It should not be treated as a free configuration fix.

The existing [runtime facade](../../../../crates/meld-world-model/src/world_state/query_runtime.rs) also catches up Graph before both cut selection and traversal. A faster store does not remove that coupling. Native work must distinguish reaching a requested Event frontier from reading a cut already durable. Otherwise continuous arrival can still dominate query latency even if Cozo's query itself is fast.

The replacement scope is concrete:

| Runtime responsibility | Proposed disposition |
| --- | --- |
| Event transport and owner publication grammar | Retain. No external knowledge-graph authoring interface. |
| Graph admission and source-route eligibility | Retain semantic ownership and validation; route accepted projections into the successor store. |
| Full publication values and history scans in `TraversalStore` | Replace for Graph with selected revision membership and indexed reads. Preserve exact reconstruction for explicit broad export. |
| In-memory reconstruction in `TraversalQuery` | Replace with engine queries only if full traversal parity and actual reduction in custom traversal work are demonstrated. |
| Publication visibility's history scan | Replace with exact indexed proof lookup under the same Graph authority. |
| Graph-owned cursor, route coverage and replay metadata | Give the successor one coherent durable ownership account. Do not leave a new payload store paired casually with an old authoritative cursor. |
| Runtime composition and query catch-up | Bind one canonical Graph store under external state storage and separate frozen reads from unnecessary catch-up. |
| Planner, Agent, Curation and Strategy | Preserve public Graph contracts and existing semantic decisions. |

The [Graph cursor](../../../../crates/meld-world-model/src/world_state/graph/cursor.rs) currently reaches directly into Sled through `TraversalStore::db`. Consequently native integration is more than replacing the approximately 190-line probe. It touches persistence, cursor recovery, read selection, visibility and runtime composition. It does not justify a rewrite of the cognitive architecture or of non-Graph Sled owners.

Historical compatibility also needs a bounded answer. Replaying Events can construct a new projection only if the necessary Events and source-route account remain available. It cannot silently discard saved cuts that users can still query. Any temporary comparison implementation remains isolated; completed production source must retain one Graph writer and one query authority.

## Recommendation and next evidence boundary

Hold implementation against released Cozo 0.7.6. The immediate promotion from the first screen was too broad given the newly reproduced range defect and the dependency's maintenance history. The engine remains a technically plausible candidate, not a selected dependency.

If Cozo stays in contention, first identify a supportable pinned source or distribution containing the range correction and confirm how further fixes will arrive. Then qualify complete path and frontier semantics and reader behavior under continuous publications before committing to native integration. Merely patching the demonstrated range expression locally would resolve one bug without answering the user's maintenance concern.

The stop condition is a need to maintain a query engine fork, rebuild most traversal decisions beside Cozo, or weaken the Graph contracts to fit its query behavior. In that case, return to an explicit indexed representation on an established storage substrate and compare the amount of application code actually owned. Do not respond by writing a new database.

No production implementation is authorized or performed by this assessment. Work completed here is source analysis, two captured diagnostic runs and a corrected recommendation. The original comparison's measured passes remain intact; its native-promotion judgment is superseded by this assessment.
