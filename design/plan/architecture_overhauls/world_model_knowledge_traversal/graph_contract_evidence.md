# Graph contract evidence and domain assessment

This is the evidence appendix to [Graph use-case contracts](graph_use_case_contracts.md). Assessment base: Meld `b420a6bc` on `spike/graph-engine-fit`, Meld Codebase Semantics `54a798f`, Meld README `db7885b`, and Meld Eval `9b5d128`. The subject repository remains Gmail Operator `91b6466`. No runtime or subject-repository changes were made for this assessment.

## Concern, scope and confidence

The concern is the work required to publish, retain, select, traverse and explain owner knowledge as Events and consumers accumulate. The scope includes Graph's admission and projection, its real query callers, Planner and completion dependencies, owner transport, and the comparison evidence. It excludes changing domain theory to reduce the number of facts, selecting an engine, implementing a replacement, and the deferred broad lifecycle pass.

User limits remain: “Unless fixing now materially changes the expected recommendation, it seems optimizing current structures should be our priority.” The architecture question remains open because the measured index improvement does not settle representation or whole-runtime query cost. External graph systems have not been ruled out.

Evidence combines a regenerated module inventory, focused production call-site searches, selected source reads, existing native test definitions and the retained engine-spike results. Discovery was bounded to the domain sweep and one level of affected concerns. This is not a complete code audit. Public contracts and directly inspected paths have high confidence; unexercised concurrency, retention, temporal reasoning, generic hydration and crash recovery remain unqualified. Tests cited here were inspected, not rerun for a documentation change.

## Primary evidence map

| Ref | Current evidence | What it establishes |
| --- | --- | --- |
| E01 | [Runtime invariants](../../../../governance/runtime_invariants.md), [Graph requirements](../../../cognitive_architecture/world_model/graph/requirements.md), [Graph specification](../../../cognitive_architecture/world_model/graph/spec.md) | Owner authority, federated traversal, identity, qualified occurrence and cut intent; intended breadth is not implementation evidence |
| E02 | [Graph contracts](../../../../crates/meld-world-model/src/world_state/graph/contracts.rs) and [admission](../../../../crates/meld-world-model/src/world_state/graph/admission.rs) | Full publication operations, owner scopes and routes, deterministic identity, completeness, currentness, receipt and traversal shapes |
| E03 | [Graph query and tests](../../../../crates/meld-world-model/src/world_state/graph/query.rs) and [store](../../../../crates/meld-world-model/src/world_state/graph/store.rs) | Full-history selection and traversal preparation; explicit incomplete supersession; distinct occurrences; conflicting revision rejection |
| E04 | [Graph runtime](../../../../crates/meld-world-model/src/world_state/graph/runtime.rs) and [runtime query facade](../../../../crates/meld-world-model/src/world_state/query_runtime.rs) | Projection flush before cursor advance; bounded actor replay and unbounded query-triggered catch-up |
| E05 | [Graph visibility](../../../../crates/meld-world-model/src/world_state/graph/visibility.rs) and [Curation visibility](../../../../crates/meld-world-model/src/curation/visibility.rs) | Graph-owned publication proof; exact Event, revision and selected-receipt checks; unchanged output may cite earlier evidence |
| E06 | [Planner contracts](../../../../crates/meld-world-model/src/planner/contracts.rs), [assembly](../../../../crates/meld-world-model/src/planner/query.rs), [projection](../../../../crates/meld-world-model/src/planner/projection.rs) | Graph is one part of the reasoning cut; derived evidence must match; exact cut differs from material work basis |
| E07 | [Agent actor](../../../../crates/meld-world-model/src/agent/actor.rs), [Strategy contracts](../../../../crates/meld-world-model/src/strategy/contracts.rs), [Curation actor](../../../../crates/meld-world-model/src/curation/actor.rs) | Currentness, successor planning, explicit completion expectations and waiting on incomplete source cuts |
| E08 | [Owner Graph callback](../../../../src/runtime/owners/graph.rs) and [runtime ports](../../../../src/runtime/ports.rs) | Installed input binding, captured Event frontier, Graph reads and publication-visibility integration |
| E09 | [Repository representation publisher](../../../../../meld-code-semantics/src/owner/representation.rs), [narrow observation publisher](../../../../../meld-code-semantics/src/owner/publication.rs) | External domain-owned lowering to native publications; revision-bound hydration and evidence; distinct broad and narrow paths |
| E10 | [README semantic input](../../../../../meld-readme/owner/src/readme/semantics.rs) and [claims curation](../../../../../meld-readme/owner/src/readme/claims.rs) | Bound Graph query, external category selection, simplified prose input with native read retained, domain-owned claim families |
| E11 | [Workspace publication](../../../../src/workspace/events.rs), [Context publication](../../../../src/context/publication.rs), [Code Change publication](../../../../src/code_change/publication.rs), [Nonce publication](../../../../src/nonce/publication.rs) | Existing native publishers beyond the README experiment; Graph is not a code-semantics-only store |
| E12 | [Branch query](../../../../src/branches/query.rs), [branch runtime](../../../../src/branches/runtime.rs), [causal walk](../../../../src/harness/walk.rs) | Scoped native traversal, retained cut command path, replay integration and exact publication retrieval for explanation |
| E13 | [Event object address](../../../../crates/meld-events/src/events/contracts.rs) | Structured coordinate validation and current ambiguous delimiter key formula |
| E14 | [Engine spike closeout](../../../../../meld-eval/evidence/graph-engine-fit-v1/CLOSEOUT.md) | Six verified fresh-state runs, native parity and comparative costs, trace limits and blocked crash qualification |

## Findings that change the comparison frame

**F01 — History is decoded before the question is narrowed.** Baseline `TraversalQuery::cut` scans admitted publication history to choose owner revisions. `traverse` separately scans through the retained Graph position, selects publications and constructs an in-memory graph before applying bounds. E03 establishes the code path; E14 establishes its measured history sensitivity. An index is justified, but this alone does not decide payload decomposition or retention.

**F02 — Frozen reads inherit catch-up work from the runtime facade.** `WorldModelQueries::cut` and `traverse` both call `GraphRuntime::catch_up` before invoking the query. That method drains replay pages rather than using the bounded actor entrypoint. This is source evidence of unnecessary coupling for a cut already available, not a measured starvation result. The native branch walk uses `TraversalQuery` through a different facade. Its benchmark cannot qualify the full cost of the owner callback path. Contracts U03, U06 and W02 therefore distinguish reaching a captured frontier from reading frozen evidence.

**F03 — Completion visibility is a separate history reader.** `publication_visibility` re-resolves the frozen owner selection and then scans publications through the cut's Event position to find the expected operation and source. Both indexed prototypes focused on cut and traversal access; the visibility scan remains outside that improvement. U07 and W01 require an exact visibility access path. Replacing this with append acknowledgment would break Agent completion meaning.

**F04 — Current object keys can alias valid addresses.** `DomainObjectRef::validate` rejects empty components; `index_key` joins components with `::`. The distinct valid coordinates `['a::b', 'c', 'd']` and `['a', 'b::c', 'd']` both produce `a::b::c::d`. Baseline traversal uses that key for address lookup and visited tracking. The counterexample was evaluated from the source formula; no native corruption probe was run. This violates the intended injective identity requirement and shows why byte parity with current behavior cannot be the sole candidate oracle. A correction needs an encoding and compatibility account wherever affected keys persist, not a Graph-only claim that all callers are fixed.

**F05 — Work reuse already has an identity distinct from the cut.** `PlannerCut::source_basis_id` removes transport watermark and associated Graph identity differences while preserving the selected product account. Receipts also expose owner-authored `work_input_basis_id`. Agent and Strategy retain authority and completed-history checks. This supports G11; it does not establish arbitrary semantic equivalence or license Graph to judge materiality.

**F06 — Cross-owner currentness is not causal agreement.** Graph chooses eligible owner revisions at a shared ledger frontier. Planner separately checks that required Curation evidence matches its selected source and that Belief consumed that evidence. A new source publication can therefore coexist with an older derived product and cause a refusal. A candidate must preserve this useful disagreement rather than treating a complete Graph cut as sufficient readiness. General dependency closure across arbitrary owners is not implemented or proposed by that observation.

**F07 — Repeated payload contains both duplication and real new evidence.** Current publications carry hydration and provenance on each object and occurrence. The repository publisher includes scan-specific revision identities and source Event bindings. The indexed prototypes retain full publication history beside rows, and the Sled prototype repeats relation bodies for adjacency directions. Some bytes can be reused; others encode a new observation. Q1 must separate these before promising content deduplication savings.

**F08 — The README consumer requests more structure than its prose view uses.** Its Graph capture retains the native read, while its derived text selects object kinds and categories and renders relations mainly as source, type and target. This is evidence for assessing selection and explanation costs separately. It is not evidence that Graph should discard occurrence qualifications or internalize README categories.

**F09 — The measured size and recovery limits remain narrower than the architecture.** Initial semantic publication size was 453 objects, 577 relations and roughly 1.31 MB as a compact envelope. The upstream 12,184 syntax nodes are not all graph objects in this probe. Whole-product storage was roughly 128–130 MB for baseline and 157–170 MB across indexed arms. These are neither Graph-only payload counts nor proof that semantic compression is required. Native crash restart was blocked by the predecessor lease in every arm, before recovery could be qualified. E14 owns exact values and probe limitations.

## Pass one — regenerated domain sweep

The universe is the current top-level Rust modules and module directories under `src`, the top-level modules in `crates/meld-world-model/src`, the other workspace crates, and the three participating external repositories. Entrypoint and export modules are included so the inventory is explicit. The subject repository is also accounted for. Test modules are evidence, not another semantic owner.

Levels describe a direct relationship to the Graph concern. `none` means no direct behavior needed by this assessment, not that the module is absent from the running program. `partial` means the connection exists but the proposed complete contract or scaling behavior is not fully qualified. A `complete` adapter row claims only its existing routing role.

| Domain | Needed integration | Current integration and evidence | Completeness | Non-integration rationale or follow-up |
| --- | --- | --- | --- | --- |
| `src/agent` | none | Product facade | not needed | Agent behavior is assessed in the world-model crate |
| `src/api` | none | General API support | not needed | Graph-specific read adapter is Branches |
| `src/bin` | adapter | Command entrypoints | complete | Preserve entrypoint routing only |
| `src/branches` | adapter | Scope, cut query and replay routing, E12 | partial | Include retained-cut and branch isolation costs |
| `src/capability` | none | Capability realization | not needed | No Graph semantic authority; execution consumes authorized products |
| `src/cli` | adapter | Runtime assembly and command dispatch | complete | Preserve routing; measure actual journey |
| `src/code_change` | publish | Observation publications, E11 | partial | Preserve effect scope and evidence |
| `src/concurrency` | none | Shared execution support | not needed | No separate Graph transaction authority identified |
| `src/config` | none | Configuration | not needed | No new backend selector proposed |
| `src/context` | publish | Frame-head publication, E11 | partial | Preserve revision and withdrawal |
| `src/control` | none | Process command surface | not needed | Generic runtime control reused |
| `src/error` | none | Shared errors | not needed | No separate semantic owner |
| `src/events` | adapter | Event crate facade | complete | Canonical transport assessed in `meld-events` |
| `src/execution` | none | Execution composition | not needed | No new Graph writer or planner |
| `src/harness` | observe | Causal walk and publication retrieval, E12 | partial | Preserve exact explanation references |
| `src/heads` | none | Owner-local heads | not needed | Context publication is the boundary |
| `src/ignore` | none | Source selection | not needed | Domain coverage held fixed |
| `src/init` | adapter | World initialization | complete | Reuse product composition |
| `src/lib` | none | Exports | not needed | No independent runtime responsibility |
| `src/logging` | none | Generic logs | not needed | Telemetry account owns observation concern |
| `src/merkle_traversal` | none | Filesystem traversal | not needed | Owner publishes its result through Events |
| `src/metadata` | none | Local metadata | not needed | No direct Graph contract change |
| `src/nonce` | publish | Scoped nonce publication, E11 | partial | Preserve qualified completion evidence |
| `src/prompt_context` | none | Prompt preparation | not needed | No domain meaning moved into Graph |
| `src/provider` | none | Model calls | not needed | Storage assessment does not change provider policy |
| `src/runtime` | adapter | Event ports, owner callbacks, projection and visibility wiring, E08 | partial | Catch-up and progress work require explicit accounting |
| `src/serve` | none | General serving | not needed | Existing command routing reused |
| `src/session` | none | Session state | not needed | No Graph semantic ownership |
| `src/store` | none | Product storage support | not needed | Graph-specific storage is owned in world state |
| `src/task` | none | Task execution | not needed | Agent and Strategy own the assessed completion dependency |
| `src/telemetry` | observe | Existing trace transport and spike evidence, E14 | partial | Separate operation costs and telemetry overhead |
| `src/theory` | none | Theory support | not needed | Installed Graph route mapping is covered by runtime and Graph |
| `src/tree` | none | Source tree state | not needed | Workspace owns publication |
| `src/types` | none | Shared types | not needed | No separate Graph contract owner |
| `src/views` | none | Local presentation | not needed | Native Graph presentation is covered by Branches |
| `src/workflow` | none | Workflow coordination | not needed | No replacement Strategy or Graph authority proposed |
| `src/workspace` | publish | Tree revision publication, E11 | partial | Keep owner meaning and Event ingress |
| `src/world_state` | adapter | World-model exports | complete | Canonical implementation remains in the crate |
| `meld-world-model/agent` | consume | Source cuts and publication visibility, E07 | partial | Preserve reconciliation and milestone distinctions |
| `meld-world-model/belief` | consume | Evidence settlement used by Planner, E06 | partial | Indirect through existing Curation and Planner contracts; no new Graph writer |
| `meld-world-model/curation` | publish and consume | Source traversal, derived publication and visibility, E05, E07 | partial | Preserve exact source-basis agreement |
| `meld-world-model/lib` | none | Exports | not needed | No separate semantic owner |
| `meld-world-model/lifecycle` | none | Existing lifecycle contracts | not needed | Broad hardening deferred, E14 |
| `meld-world-model/planner` | consume | Complete or refused immutable assembly, E06 | partial | Graph cut is one input, not the full reasoning world |
| `meld-world-model/strategy` | consume | Planner basis and completion expectations, E07 | partial | Preserve independent decomposition and successor decisions |
| `meld-world-model/waiting` | none | Generic dependency declarations | not needed | Existing declarations reused through runtime ports |
| `meld-world-model/world_state` | own | Graph admission, projection, query and proof, E02–E05 | partial | Primary representation and access-path concern |
| `meld-events` | own | Canonical Event transport and object references, E13 | partial | Preserve ledger identity; address-key gap requires an account |
| `meld-execution` | none | Authorized execution | not needed | No Graph semantic change required |
| `meld-lang` | none | Proposition and planning language | not needed | Graph assessment preserves domain meaning and language |
| `meld-code-semantics` | publish | Repository and narrow observation publications, E09 | partial | Hold source meaning fixed; no publication redesign assumed |
| `meld-readme` | consume | Semantic query and claims selection, E10 | partial | Keep domain filters external; account for requested payload |
| `meld-eval` | observe | Command-first comparison and OTel evidence, E14 | partial | Add missing journey evidence only when a candidate is exercised |
| `gmail-operator` | none | Observed source repository | not needed | Read-only workload, no runtime or design ownership |

The affected set is frozen to every non-`none` row above. New consumers discovered during implementation should reopen that boundary assessment; their presence does not silently expand this design into a whole-runtime rewrite.

## Pass two — affected concerns and ownership

This is one level of behavioral decomposition. Postures describe candidate relationships, not authorized implementation work. Evidence references resolve through the map above.

| Domain concern | Owner | Current ground and required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- |
| Admission, revision identity and projection progress | World State Graph | Validate Events once per admitted operation and retain exact owner account | extend existing | Another writer or cursor ahead of durable projection | E02–E04 |
| Current selection, frozen traversal and publication visibility | World State Graph | Three consumer questions need selective access under one authority | extend existing | Optimizing one reader while leaving another history scan | E03–E05 |
| Retained membership, explanation and representation version | World State Graph | Preserve immutable cuts while allowing physical reuse | extend existing | Rebuild or collection changes historical meaning | E02, E03, E12 |
| Object addressing and replay transport | Meld Events | Reuse ledger authority; account for unambiguous key encoding | extend existing | A private Graph fix leaves other persisted key consumers ambiguous | E13 |
| Owner transport, query catch-up and dependency progress | Runtime | Bound reads by their declared need and preserve installed owner bindings | extend existing | Query latency depends on unrelated future Events | E04, E08 |
| Input assembly and product basis | Planner | Validate Graph, Belief, Curation and authority together | reuse unchanged | Treating Graph completeness as planning readiness | E06 |
| Belief settlement and evidence agreement | Belief | Continue consuming domain evidence through existing paths | reuse unchanged | Graph starts making judgments about truth or materiality | E06 |
| Evidence selection, publication and completion visibility | Curation | Preserve current source and exact output correspondence | reuse unchanged | Append success substituted for Graph visibility | E05, E07 |
| Successor coordination and milestone acceptance | Agent | Keep frozen history and react to new material information | reuse unchanged | Graph introduces cancellation or action authorization policy | E07 |
| Decomposition and successor judgment | Strategy | Consume Planner basis and authorized completion conditions | reuse unchanged | Storage selection becomes a prescribed plan | E07 |
| Tree publication | Workspace | Keep full owner revision and source meaning | reuse unchanged | Storage optimization changes coverage | E11 |
| Head publication | Context | Keep current and withdrawn products qualified | reuse unchanged | Old membership silently remains current | E11 |
| Effect observation publication | Code Change | Keep issuer, effect scope and revision evidence | reuse unchanged | Wrong output satisfies a completion expectation | E11 |
| Nonce observation publication | Nonce | Keep singleton scope and explicit product identity | reuse unchanged | Generic existence mistaken for expected effect | E11 |
| Repository semantic publication | Meld Codebase Semantics | Keep broad representation and evidence fixed | reuse unchanged | Fewer facts disguise structural amplification | E09 |
| Semantic selection and prose shaping | Meld README | Keep category and claim theory external | reuse unchanged | README vocabulary leaks into Graph | E10 |
| Branch selection and retained-cut routing | Branches | Delegate to canonical query authority and preserve per-ledger context | adapter only | Aggregation implies nonexistent global atomicity | E12 |
| Causal explanation | Native Harness | Resolve exact publication and evidence references read-only | reuse unchanged | Historical walk hydrates a current replacement | E12 |
| Command entry and initialization | Bin, CLI, Init | Keep ordinary composition and public commands | adapter only | Benchmark bypasses the actual runtime facade | E08, E12 |
| Export compatibility | Events and World State facades | Forward to canonical crate authorities | adapter only | Parallel semantic implementations | E01, E02 |
| Cost observation and comparative capture | Telemetry and Meld Eval | Distinguish operation paths, byte categories and missing evidence | extend existing | Instrumentation overhead or private probes mistaken for product performance | E14 |

## Ownership synthesis and unresolved scope

Graph owns the largest behavior change under consideration: durable reusable representation and selective reads. Events owns identity and transport contracts. Runtime owns query integration and progress scheduling. Owner publishers remain responsible for meaning, Planner for assembling compatible knowledge, and Strategy for deciding how to reconcile. An external database would not inherit those domain responsibilities merely by storing their records.

The operating path includes publishers, Graph, Events, runtime composition, Planner, Curation, Belief, Agent, Strategy and read adapters. That is much broader than the likely behavior-change set: Graph, the runtime query boundary, and any demonstrated shared identity correction. The likely implementation units are therefore conditional on a selected design, not every domain in the operating path. This assessment's actual write scope is documentation under `design/plan` only.

No new ontology, generic scheduler, universal Graph query language or domain-theory migration is established as necessary. The smallest connective issues already visible are selective publication visibility, a frozen-read path whose preparation respects its cut, and collision-free structured key handling. Whether those fit a compact projection correction or justify broader representation work depends on Q1–Q4 and the component size account.

Unresolved policies remain in the [contract document](graph_use_case_contracts.md#what-is-fixed-and-what-remains-to-decide). Existing implementation and the prior spike support assessing candidates against that frame. They do not yet establish retention safety, sustained multi-owner concurrency, native crash recovery or a production architecture winner.
