# M3 workspace assessment

Date: 2026-09-10. Source inspection on the candidate worktree. Design recommendations only; no build or runtime test was performed.

This is supporting evidence for the [program's runtime exploration](meld_3_candidate_program.md), not a fixed implementation checklist. Apply dispositions to exercised workspace journeys. Major gaps in native behavior invoke the program circuit breaker; command completeness does not authorize inventing missing components.

## Judgment

Workspace remains a live source domain. Its candidate command surface should describe the selected filesystem source, refresh its indexed snapshot, inspect integrity and manage inclusion rules. The previous reference wrongly promoted nearly every older maintenance command into the candidate.

World exposes published knowledge; Workspace manages the physical source and observation policy that can produce that knowledge. Runtime owns process and participant lifecycle. Keeping those distinctions permits the same operations through CLI, harness and later graphical clients.

## Current production path

The [scan core](../../../../src/workspace/scan.rs) is shared by CLI and the [Execution workspace port](../../../../src/execution/ports.rs). It builds the filtered tree, writes node records, returns snapshot/root identities and emits publication candidates when supplied a session. An unchanged tree can still return an owner publication candidate. Scanned/up-to-date is therefore distinct from published/projected/acted-upon.

The [CLI scan service](../../../../src/workspace/commands.rs) routes into that core and durably emits owner publications through its progress adapter; other telemetry is best effort. The core does not append canonical Events itself. Preserve this separation while moving transport into the live owner. Scan success cannot mean that an Agent has judged a Goal satisfied.

The [passive workspace lifecycle](../../../../src/workspace/lifecycle.rs) owns readiness, wait, fence and stop evidence and participates in [supervisor assembly](../../../../src/runtime/supervisor/entrypoint.rs). Docs and Code Change [topologies](../../../../theory/docs_freshness/product_topology.json) include workspace.source. This is direct production integration, not merely test scaffolding. It does not establish that a continuous filesystem watcher runs inside the native supervisor.

Startup has no required workspace source. Commands requiring a workspace should report no workspace binding without creating one.

## Recommended surface

| Candidate command | Required behavior | Disposition |
| --- | --- | --- |
| `meld workspace status` | Show selected source path, retained snapshot/root, last observation and publication state, inclusion policy and freshness boundary. | Retain and simplify. Remove profile/frame coverage from ordinary source health. |
| `meld workspace scan` | Observe the selected source through the canonical scan contract and report index/publication outcomes separately. | Move existing top-level scan here. Keep exceptional `--rebuild` for the existing force policy. |
| `meld workspace validate` | Explicitly inspect current filesystem/index consistency and report the limits of the check. | Retain a narrowed real integrity check. Do not report whole-product validity. |
| `meld workspace ignore list` | Show effective rules and their origin. | Replace overloaded list/add invocation. |
| `meld workspace ignore add PATH` | Add an explicit workspace exclusion. | Retain existing addition with `--dry-run`. |
| `meld workspace ignore remove PATH` | Remove an explicit Meld exclusion; explain if another rule still excludes the path. | Expose existing removal helper through a proper result contract, with `--dry-run`. |

Removing a rule does not resurrect historical tombstoned records or change source files. Inclusion changes take effect on the next observation; commands should say so. They must not silently trigger product work. Imported .gitignore rules remain owned by that file.

The [current status implementation](../../../../src/workspace/section.rs) computes a filesystem root and combines node counts with Reader/Writer frame coverage. Repeated status can therefore perform substantial filesystem work. Candidate status should use retained observation and report age/unknown freshness. Explicit validate or scan performs fresh inspection. This is a required behavioral change, not a claim that today's status is inexpensive.

## Retire, relocate or defer

| Existing or previously proposed surface | Finding | Candidate disposition |
| --- | --- | --- |
| Delete/hide, restore, list-deleted | Directly tombstones/restores node and frame-head records. Delete also adds an ignore entry. Restore discards ignore-removal errors but reports removal. These methods do not establish a native owner-publication transition. | Remove from the proposed ordinary command surface. Inclusion control belongs to ignore; historical storage repair needs a separately justified contract. Do not route hide into the existing writer under a friendlier name. |
| Compact | Purges nodes, frame heads/blobs and prompt artifacts through ContextApi. | Defer public retention control until owner references and retained evidence are assessed. No automatic GC is introduced by deleting the command. |
| Frames | Retrieval of older stored context is real, but does not describe workspace source health. | Exclude from the workspace contract. A demonstrated retained-history consumer may justify an owner-specific reader during M3-03; do not promise a replacement family now. |
| Reset / dangerous flush | Flush targets include the entire configured product runtime root, plus node/frame/artifact and fallback roots. The service does not acquire a native shutdown/ownership fence. | Remove the proposed workspace reset. Product destruction requires its own explicit lifecycle/storage design. Fresh setup preserves old history and is not a reset alias. |
| Branch status, attach, discover, migrate | BranchRuntime owns a global catalog and graph-store discovery/migration. Attach registers a dormant branch; discover mutates the catalog. These are not Git checkout operations. | Put useful graph-branch discovery/attachment under World. Defer conversion to a concrete existing-user transition; preserve incompatible state meanwhile. |
| Watch | A separate blocking filesystem watcher builds/updates the tree and publishes snapshot facts. It has its own loop and loads profile registry configuration. | Do not rename it runtime start. If an exercised journey needs continuous observation, assess whether existing behavior can be integrated without new semantic architecture. A fundamental gap invokes the breaker; do not promise a new watcher component to fill a command cell. Retire superseded paths within any completed cutover. |
| Unified status | Aggregates workspace, old Agent registry and providers. | Split through native runtime/agent/provider and narrowed workspace views. |
| CI helper API | validate_workspace returns unconditional valid; report and diff methods contain placeholder results. | Never count these as qualification evidence or create public commands for them. Audit/remove unused exports and dependent tests during implementation. |

The [maintenance implementations](../../../../src/workspace/commands.rs), [flush service](../../../../src/workspace/danger.rs), [branch runtime](../../../../src/branches/runtime.rs), [watch loop](../../../../src/workspace/watch/runtime.rs) and [CI helpers](../../../../src/workspace/ci.rs) establish these findings. Public library exports and tests are callers too; removal requires their disposition. This pass recommends command removal, not an unreviewed deletion of every supporting store operation.

Branch navigation remains grounded functionality, but native compatibility is not proved by its old catalog existing. Proposed World branch commands must validate selected product/store identity. Branch discover retains its explicit registration side effect; it must not be presented as an ordinary read.

## Lifecycle and storage consequences

All live workspace mutations must route to the process that owns the product stores. The current command service accepts ContextApi and directly mutates its stores; a live HTTP route and supervisor admission fence are required for the candidate. The existence of passive-source lifecycle receipts alone does not prove those CLI mutations are fenced.

A scan is explicit source observation, not Agent work authorization. Its publication may subsequently cause native reconciliation. Report accepted scan, node-store completion, durable publication and projection progress separately; on lost replies preserve request identity and unknown outcomes. An offline mutation must acquire exclusive ownership and the correct publication authority, or refuse with a start instruction. Do not launch a second store writer as a convenience.

Status and ignore listing must not initialize paths. Current ignore helpers resolve through XDG functions that may create directories, so making reads side-effect-free requires implementation work when this journey is exercised. Ignore policy currently lives in workspace-derived data and syncs a marked .gitignore block. Preserve both; inventory them if a later bounded migration actually moves or converts this state.

Initialization prepares configured workspace products through their existing binding and package contracts. It should not globally scan every workspace or attach every discovered old store. A user with only Startup should not encounter workspace requirements.

## Workstream effect and acceptance

M3-01 needs accurate optional workspace binding and ownership-aware targeting. It leaves old workspace data and policy intact; automated migration and the maintenance cutover do not precede Startup proof.

M3-02, M3-03 and M3-05 identify related backlog concerns for observation, command consistency and existing-user transitions. Select concrete work through harness findings. These labels neither impose a fixed sequence nor authorize missing native components or general conversion machinery.

After explicit build authorization, evidence must show: no workspace requirement for Startup; a workspace-bound product can scan and publish through the canonical path; status distinguishes retained observation from a fresh filesystem check; ignore changes have truthful effects; live ownership blocks competing writers; interruption preserves index/publication distinctions; and retired commands no longer reach old writers. Filesystem-watch parity is required only if continuous observation is retained as a candidate claim, and must not be inferred from the passive participant.

This is a source-grounded disposition pass. Exact live scan request wiring, historical-reader demand, native branch compatibility and continuous-observation scope still need implementation-slice resolution. No source behavior, user data or running process changed.
