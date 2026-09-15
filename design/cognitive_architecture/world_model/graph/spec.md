# Graph And Traversal Specification

## Core Products

`GraphObjectPublication` binds an opaque object address to owner, product type, presence state, scope, provenance, and revision.

`RelationOccurrencePublication` binds one stable occurrence identity to source, relation type, target, qualifications, scope, provenance, currentness, and owner revision.

`TraversalCut` names the immutable source boundary across participating owners.

`TraversalQuery` names roots, relation policy, direction, scope, currentness policy, cut, and resource bounds.

`TraversalResult` returns reached object publications, relation occurrences, paths, frontier, truncation state, and exact cut identity.

## Identity Rules

Object address identity is owner-issued. Publication identity additionally includes owner revision and scope. Relation occurrence identity is independent from endpoint equality. Traversal result identity derives from the normalized query and exact cut.

## Presence And Currentness

Presence is an explicit state such as present, expected, absent, withdrawn, or archived. Currentness is evaluated under an owner policy and scope. No generic `current_only` flag may erase the policy or scope used to decide currentness.

## Federation

Each owner supplies a traversal projection or publication reducer that conforms to the shared query and result protocol. Federation joins results by opaque address while preserving owner, occurrence, scope, and revision.

A federated cut contains one exact revision receipt per participating owner. Missing owner receipts make the cut incomplete rather than silently current.

## Curation Path

Curation emits typed publications through Events. Graph admission validates owner route, schema, occurrence identity, scope, and idempotency, then advances the graph projection. A Curation result may report unchanged when the requested publication already exists at the required revision.

## Planner Path

Planner issues bounded traversal queries, retains occurrence-rich results, and freezes them with belief, causal, regime, Capability, directive, and policy revisions. Strategy consumes only that immutable planner cut.

## Publication Storage And Query Work

Each admitted revision has independent membership over immutable object and relation content and exact evidence bindings. Equal content can share physical storage without equating observations, source revisions or relation occurrences. Graph reconstructs the intact owner publication for explanation. Ordinary selection and traversal do not require reconstruction of retained publication history.

Owner and full-scope selection locate revision headers. Exact Event positions locate admitted publications. Structured addresses locate nodes in selected revisions, and native directed adjacency supplies candidate occurrences. Graph assembles deterministic paths, independent result bounds and explicit frontier under its public contract. Storage keys do not define semantic equivalence.

A frozen cut reads its retained revisions without catching up to unrelated new Events. New currentness selection requires the requested Event frontier. Publication contents become durable before Graph advances its canonical projection cursor. External knowledge still enters through Events, and semantic ownership remains with the publishing domain.

## Uncertain Storage Handles

A failed publication mutation or durability operation quarantines its physical database handle. Every shared caller observes the same quarantine under the database lock. Reads, subsequent mutations and synchronization reject while the handle is quarantined. A panic during mutation also closes access; unwinding does not imply rollback succeeded. Preflight domain validation can reject without changing handle health.

Quarantine does not attempt a compensating commit or synchronize uncertain state. During teardown a storage guard prevents engine optimization from finalizing the uncertain handle; the physical backend retains responsibility for replaying its recovery log. All holders must release the handle before a fresh physical open delegates recovery to the storage engine. Recovery failure remains an error. Graph resumes from its durable Event position only after reopening succeeds. Publication replay is idempotent, so an operation whose durability acknowledgment failed can be encountered again without inventing a second semantic publication.

A storage error is an uncertain outcome, not proof that the attempted write was absent. Consumers must not acknowledge a new projection position or obtain new graph evidence through the failed handle. Already returned immutable cuts retain their original evidence boundary.
