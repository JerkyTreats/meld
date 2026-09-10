# M3 control resource and operation matrix

Date: 2026-09-10. Supporting source assessment and working hypotheses for the [command reference](m3_command_reference.md). This matrix is a navigation aid for runtime exploration, not a completeness gate or a mandate to implement every cell. The [program](meld_3_candidate_program.md) owns the feedback method and architectural circuit breaker.

## Purpose and grammar

Every public operation needs a discoverable target, a bounded result, an owning contract and truthful side effects. CRUD completeness is not the goal: Events are immutable, Goal judgment belongs to Agent, and runtime stop is a lifecycle transition rather than deletion.

Use resource families with list/show for collections and records, status for a composed current account, and explicit verbs for domain actions. Do not implement a generic `runtime list SUBJECT` parser with an undocumented universe of subject types. Use `runtime list` for known instances and explicit participant/action resources for their different identities.

CLI and the local runtime surface expose the same existing domain behavior without another scheduler or writer. Future TUI and dashboard consumers create no acceptance requirements now.

Completeness serves excellent command-line ergonomics and proof of the flywheel runtime. Each operation or prerequisite must improve that journey or enable its proof. The same unauthenticated loopback interface supplies startup readiness, control and observation; no separate private launch channel or control credential is required.

## Identity and discovery boundaries

| Resource | Identity and discovery boundary | Canonical authority |
| --- | --- | --- |
| Installation/preparation | Selected config and declared product/package identities. Never enumerate arbitrary directories as owned installations. | Init authors; Config resolves; Theory and native genesis owners prepare. |
| Runtime instance | Product root plus supervisor instance identity. Enumerate known instances reachable from the selected configuration's declared roots; optional assignment narrows this set. | Existing supervisor instance/lease records and verified live endpoint. |
| Runtime participant | Runtime ID within an exact selected instance; distinct from OS processes and publication-owner IDs. | Supervisor and participant lifecycle contracts. |
| Operational action | Existing actor/report identity and position within selected product/instance. | Actor reports and shared production projections. |
| Control request | Request key plus operation, product and exact instance. | Supervisor-owned acceptance and shutdown records. |
| Native Agent | Agent ID within selected product. | Native Agent store/contracts, not profile registry. |
| Reconciliation request | Agent plus request key and canonical request ID. | Native Agent request and completion records. |
| Goal, decision, Task, evidence and Belief revision | Typed domain references carried by Agent views, Events and causal traces. Tasks retain network and instance identity. | Respective domain stores through public reads. |
| Publication owner | Owner ID in selected product; declarations and observed publications are distinct populations. | Package declarations and admitted owner publications. |
| Publication scope | Owner plus full scope identity; branch/perspective/time qualifiers cannot be discarded. | Owner publication contracts and Graph projection. |
| Published object/relation | Domain object reference within a qualified publication cut; relation occurrence identity is distinct from an object. | Publishing owner; Graph retains projection/provenance. |
| Graph branch | Catalog branch ID and validated associated store/product identity; not a Git branch. | Branch catalog and Graph query runtime. |
| Event ledger/record | Ledger ID and sequence, with domain/object/stream references. | Events. |
| Command session | Session identity within the selected ledger; discover from record references. | Event session records. |
| Workspace source/snapshot | Configured physical binding plus observed snapshot/root identity. | Workspace scan/publication and lifecycle owner. |
| Ignore rule | Selected workspace plus explicit rule and origin. | Workspace inclusion policy; imported .gitignore rules retain their source ownership. |
| Provider configuration | Provider name within selected config registry. | Provider configuration service. |

Runtime enumeration is a bounded inventory of known records, not a system-wide process census. Return live-verified, retained-stopped or unknown/unreachable states with observation time. A configured product with no instance is an explicit unstarted target, not a fabricated runtime instance. Inaccessible roots appear as gaps. Unregistered custom configurations are outside coverage. Listing must not create stores.

Default runtime operations resolve the configured assignment and verify its live instance. An exceptional `--instance ID` fences selection within that scope; it does not grant discovery outside it. An ambiguity reports candidates. Start cannot accept an old instance as a request to resurrect that process.

## Resource × operation matrix

“Extend” identifies a possible public gap in existing behavior. “Move” identifies possible adapter/caller migration. Exercise the relevant journey before expanding either proposal. No row implies executable candidate proof or authority to repair a missing semantic architecture.

| Resource | Discover/inspect | Direct/change | Observe | Current foundation and change hypothesis |
| --- | --- | --- | --- | --- |
| Installation | Help/version; init reports selected setup or incompatibility | Init; bounded migration only for a demonstrated later need | Preparation results | Ordered preparation exists. Expose simple bootstrap and preserve incompatible state with an actionable fresh setup. |
| Runtime instances | runtime list/status | start, stop; restart composes both | follow | Foreground supervisor/status exist. Extend inventory, detached readiness through public status and control receipts. |
| Participants | runtime participant list/show | No individual start/stop in candidate | Shared follow/status | Existing runtime status rows expose leases/health. Extend public identity-preserving presentation; do not bypass topology with participant CRUD. |
| Actions | runtime action list | None | follow and trace | Actor reports exist. Move existing action inspection; bounded filtering, instance identity and pagination need verification. |
| Control requests | runtime request show KEY | Created by stop; repeat same key | Status/follow show drain | Extend supervisor-owned durable request lookup. Lookup never submits a request. |
| Native Agents | agent list/show | agent request | Agent view, runtime follow/trace | Native state exists; current profile CLI is not this API. Replace adapter and verify bounded enumeration. |
| Reconciliation requests | agent request-status KEY | agent request with stable key | Native completion evidence | Request storage/intake exists. Extend read-only lookup and lost-response handling. |
| Goals/decisions/Tasks/evidence/revisions | Typed references from Agent/show, Events and trace | Through native domain authority only | Trace/why/follow | Reuse readers where present. No blanket CRUD and no speculative family for every internal record. Missing trace resolvers return named gaps. |
| Publication owners | world owner list/show | Through package/owner preparation, not Graph editing | Publication state and provenance | Extend declared/observed enumeration and public owner read. |
| Publication scopes | world scope list/show | Through owner publication | Coverage, revision, exclusions/failures | Typed scopes exist. Extend enumeration and complete scope selection. |
| Published objects/relations | world object list/show; world walk | Owning domain operations only | Bounded traversal at an explicit cut | Traversal/publication contracts exist. Extend browsing; do not fabricate arbitrary object CRUD or implicit hydration. |
| Graph branches | world branch list | attach, discover; conversion deferred to a concrete user transition | world status/walk | Assess existing catalog operations through a useful journey; discover writes registration. Native store compatibility remains to prove. |
| Events | event status/tail/trace/flow | No raw append/update/delete command | tail --follow | Existing ledger reads. Preserve live-owner access, pagination, gaps and retention semantics. |
| Sessions | event session ID; discover IDs in event records | Authored by operations | Event timeline | Existing inspection. Do not invent session creation/deletion to fill cells. |
| Workspace | workspace status/validate | scan | Snapshot/publication progress | Shared scan exists. Narrow status; extend live-owner routing and publication receipts. |
| Ignore policy | workspace ignore list | add/remove | Report next-observation effect | Existing helpers; extend origin-aware results and truthful failures. No hidden rescan. |
| Providers | provider list/show/status/validate | create/edit/remove; test performs network diagnostics | Diagnostic result | CRUD/diagnostics exist. Simplify flags, separate local validation from network testing, verify noninteractive input and redaction. |

## Side effects, result semantics and coverage

Reads do not prepare products, register branches, create config/store directories, hydrate remote payloads or submit reconciliation. A read may be unavailable while a live owner is unreachable; it must not open a competing writer. Filesystem inspection is explicit under workspace validate/scan rather than hidden in routine status.

State changes report target, request identity where asynchronous, acceptance, durable completion and remaining obligations. Stop requests native drain; it does not delete product state. Scan can finish indexing before downstream projection or Goal judgment. Provider test performs network calls; local validate does not.

Lists are bounded and carry a continuation position when truncated. Streams carry source/instance identity, cursor, freshness and gap information. An empty result is scoped observation, not universal absence. No full-table scan should be disguised by merely truncating rendered output.

Do not normalize all failures into false or not-found. Preserve distinctions needed to explain uninitialized, absent, ambiguous, inaccessible, stale, partial, rejected and pending/unknown results. Refine representations through exercised cases and focused checks rather than blocking feedback on a complete taxonomy.

Provider operations illustrate actual CRUD fit. World publication and lifecycle operations illustrate where it is inappropriate. Avoid duplicated diagnostic choices: provider validate checks local configuration; provider test owns connectivity and configured model checks. Supply useful defaults and permit overrides where they improve real command use.

## Questions to exercise

1. Add runtime list with the explicit configured-root discovery boundary. Current per-product discovery files alone cannot provide a complete installation inventory.
2. Expose participant list/show separately from instance list. Existing status already distinguishes supervisor instance from runtime rows.
3. Replace runtime actions with runtime action list. Do not add action show until stable record lookup is established.
4. Add runtime request show KEY so a lost stop response can be resolved without resubmitting. Retain Agent request lookup under Agent.
5. Preserve full World scope identity. A plain scope string may be ambiguous across branch/perspective/time; inspection should return a qualified selector usable by subsequent commands. Settle its encoding while implementing and exercising that journey; do not drop qualifiers or force users to construct it.
6. Remove network flags from provider validate. Provider test owns network diagnostics; its outcome is not proof of an actual product/model contribution.
7. Keep the workspace reductions and exclusions from the [workspace pass](m3_workspace_assessment.md).

This assessment does not add package authoring, Agent intention editing, arbitrary Task submission, Event CRUD, manual Goal satisfaction, product deletion or general artifact GC. Those cells remain absent because no candidate user contract justifies them.

## Source evidence and next work

The [harness notes](m3_runtime_harness.md) use this matrix to suggest useful questions. Capture and repair observed command gaps without private store access. If the gap is a missing native semantic capability rather than access to existing behavior, apply the program circuit breaker before proposing new components.

The [current parser](../../../../src/cli/parse.rs) establishes existing command families and provider diagnostics. [Runtime tooling](../../../../src/runtime/tooling.rs) distinguishes instance identity from participant runtime rows and exposes Startup/reconciliation/status. [Serve discovery](../../../../src/serve/discovery.rs) is a per-product locator, not a global inventory. The [Agent store](../../../../crates/meld-world-model/src/agent/store.rs) contains reconciliation request enumeration and completion reads. [Graph contracts](../../../../crates/meld-world-model/src/world_state/graph/contracts.rs) retain qualified scopes and bounded traversal. [Causal walking](../../../../src/harness/walk.rs) reports unresolved/out-of-scope references rather than guaranteeing every domain resolver.

Source foundations were inspected; the proposed surface has not been qualified through the runtime. M3 labels group backlog concerns, not implementation dependencies. Use the next observed gap to choose which operation to exercise and update this assessment when the evidence changes.

For the operation being changed, identify its existing owner and actual caller path, make the smallest correction, and rerun it through the harness. Retire a superseded authority within the completed replacement. Ordinary schema and adapter choices belong inside that iteration. A complete matrix, generic CRUD framework or new flywheel architecture is not a prerequisite or deliverable.
