# Command surface grounded in current functionality

Status: recommendation, 2026-09-09. No CLI implementation changed.

The public tree below is superseded by [the flywheel-first recommendation](flywheel_command_surface.md). This document remains the complete handler inventory and old-to-new mapping considered before separating native operation from compatibility tooling.

## Scope and recommendation

Expose existing behavior with one clear public home per operation. Rename and route commands without adding lifecycle decisions, domain writers, generalized world queries, automatic product configuration, or new diagnostic derivations.

The inventory follows every explicit command variant in [CLI parsing](../../../../src/cli/parse.rs), its [dispatch route](../../../../src/cli/route.rs), and the corresponding domain handlers. It also identifies existing served read products that merit CLI adapters. This is source verification, not execution testing of every operation.

Use `init`, `run`, and `status` as the main lifecycle entry points. Use `agent`, `world`, `event`, `runtime`, `workspace`, `provider`, `context`, and `workflow` as task families. `workflow` is historical inspection only. `runtime` holds detailed operational inspection, while top-level `status` remains the existing supervisor snapshot rather than claiming newly aggregated whole-product health.

## Complete recommended tree

The following is proposed syntax, not commands available in the installed binary. Entries marked `adapter` have existing read implementations but need a new CLI binding. Other entries route existing CLI operations.

```text
meld
  init                         initialize default profiles and prompts
    product                    install and prepare a declared native product
  run                          foreground runtime, including orderly interruption
  status                       runtime supervisor status

  agent
    request                    request reconciliation of an installed intention
    profile
      status                   profile validation and prompt-path status
      list                     configured Reader and Writer profiles
      show                     one configured profile
      create                   create a profile
      edit                     edit a profile
      remove                   remove a profile
      validate                 validate one profile or all profiles
      prompt
        show                   show a profile's prompt
        edit                   edit a profile's prompt

  world
    graph
      status                   graph readiness across selected branches
      owner-walk               bounded traversal of exact owner publications

  event
    status                     ledger health, retention, drops, and consumer lag
    tail                       read records, optionally follow
    trace                      causal references for object, stream, or sequence
    session                    reconstruct a command session timeline
    flow                       event flow over a bounded trailing window

  runtime
    startup-account            inspect the native Startup nonce path
    actions                    recent actor reports or latest for one actor; adapter
    flow                       existing user projection of the runtime loop; adapter
    trace                      existing cross-domain thread walk; adapter
    why                        existing absent-record eligibility walk; adapter
    projection
      parent                   scoped boundary changes and anomalies; adapter
      subagent                 scoped event diff and latest actor reports; adapter

  workspace
    status                     workspace tree and stored context coverage
    overview                   existing workspace, profile, and provider overview
    scan                       build or rebuild the filesystem tree
    watch                      foreground compatibility filesystem watcher
    validate                   validate workspace integrity
    ignore                     list or add ignored paths
    delete                     logically tombstone indexed nodes
    restore                    restore tombstoned nodes
    list-deleted               list tombstoned nodes
    compact                    purge eligible tombstoned records
    flush                      remove workspace runtime state, preserving logs
    branches
      status                   branch registry and migration state
      discover                 discover dormant branches
      attach                   attach an explicit workspace branch path
      migrate                  migrate registered branches

  provider
    status                     provider status, optionally connectivity
    list                       configured providers
    show                       one provider configuration
    create                     create provider configuration
    edit                       edit provider configuration
    remove                     remove provider configuration
    validate                   configuration and selected diagnostic checks
    test                       explicit provider connectivity check

  context
    get                        retrieve existing stored context frames

  workflow
    list                       resolved historical Workflow profiles
    inspect                    inspect one historical profile
    validate                   validate the historical profile registry

  help                         command and subcommand usage
```

The separate profile level under Agent prevents configuration CRUD from claiming to create or delete a native world-model Agent. The current implementation exposes native reconciliation requests but does not provide an equivalent native Agent CRUD command family. This recommendation does not fill that gap with invented behavior.

## Exact disposition of existing commands

| Current command or family | Recommended destination | Existing authority and scope |
| --- | --- | --- |
| `init` | `init` | [Init handler](../../../../src/init/tooling.rs) calls initialization preview or default agent and prompt initialization. Preserve `--list` and `--force`. |
| `world init` | `init product` | [World initialization](../../../../src/init/world/tooling.rs) resolves an existing declaration, installs its package, prepares Agent activation, and seeds initial evidence. Preserve target path, stage selection, package source, and output format. |
| `runtime run` | `run` | [Runtime handler](../../../../src/runtime/tooling.rs) starts the foreground supervisor. Preserve tick, duration, identity, and restart controls. Preserve production dispatch binding in the CLI route. |
| `runtime status` | `status` | Same supervisor read, runtime filters, and live-process bypass. This is not a Goal-success report. |
| `runtime request` | `agent request` | Same native Agent reconciliation intake and idempotent request key. Preserve live-process routing. |
| `runtime startup-account` | unchanged | Same native projection, explicit Agent identity, exact generation and epoch selection, nonce selection, and inspection fence. Does not fire a nonce or claim whole-system health. |
| `agent status/list/show/create/edit/remove/validate` | `agent profile` with the same leaves | [Agent handler](../../../../src/agent/tooling.rs) operates on configured profiles. |
| `agent prompt show/edit` | `agent profile prompt show/edit` | Same prompt-file operations. |
| `branches graph-status` | `world graph status` | [Branch handler](../../../../src/branches/tooling.rs) invokes the federated graph readiness query. Preserve branch scope. |
| `branches graph-owner-walk` | `world graph owner-walk` | Same bounded owner-publication traversal, product Event position, owner scope, object identity, direction, relation filters, and limits. No arbitrary belief or semantic search implied. |
| `branches status/discover/attach/migrate` | `workspace branches` with the same leaves | Same branch registry operations. Preserve the early binary route that avoids ordinary workspace assembly where applicable. |
| `event status/tail/trace/session/flow` | unchanged | [Event handler](../../../../src/events/tooling.rs) delegates to existing ledger capabilities. Preserve cursors, limits, subjects, and session identity. |
| top-level `status` | `workspace overview` | Preserve the existing combined workspace, profile, and provider result, filters, and optional connectivity checks. Do not silently discard it when promoting runtime status. |
| `scan` | `workspace scan` | Same filesystem scan and force behavior. |
| `watch` | `workspace watch` | Same filesystem watcher; help must reflect actual process behavior. |
| top-level `validate` | `workspace validate` | Consolidate with the existing workspace validation operation rather than maintaining two public spellings. |
| all existing `workspace` leaves | unchanged | [Workspace handler](../../../../src/workspace/tooling.rs) retains status, validate, ignore, delete, restore, compact, and list-deleted. Preserve logical deletion semantics and dry-run controls. |
| `danger flush` | `workspace flush` | Preserve the dedicated destructive entry route, exact workspace target, dry-run behavior, and confirmation requirement. This removes runtime state, not workspace source files. |
| all eight `provider` leaves | unchanged | [Provider handler](../../../../src/provider/tooling.rs) retains configuration and diagnostic operations. Do not collapse test into validate and lose its distinct options. |
| `context get` | unchanged | [Context handler](../../../../src/context/tooling.rs) retrieves stored frames with node or path targeting, filters, ordering, metadata, and output controls. |
| `context generate/regenerate` | remove from supported help | Both return retirement errors. The handler explicitly states that a replacement generation package is not yet provided. They are not working capabilities to resurface. |
| `workflow list/inspect/validate` | unchanged, labeled historical | [Workflow handler](../../../../src/workflow/tooling.rs) retains useful inspection of persisted configuration. |
| `workflow execute` | remove from supported help | Deliberately returns a retirement error. Do not redirect to unrelated execution while implying behavioral equivalence. |

Old working spellings should either be removed as disclosed CLI changes or retained only as justified forwarding syntax into the same handler. Existing retirement errors may remain as explicit migration diagnostics without advertising them as runnable features.

## Existing read capabilities worth exposing

These commands add CLI parsing and rendering, not new domain behavior. The [served routes](../../../../src/serve/routes.rs) already expose each underlying operation. Live adapters should use that existing process; any offline mode must retain existing session fences and safe store access rather than claiming unverified parity.

| Proposed command | Existing read contract | Limits to retain |
| --- | --- | --- |
| `runtime actions` | `/v1/reports/recent_actions` and `/v1/reports/latest_for_runtime` | Bounded retained reports and exact runtime ID for the latest-report variant. |
| `runtime flow` | `/v1/projections/user` | Existing topology, coupling status, and bounded queue observations. No new diagnosis or live in-flight claim. |
| `runtime trace` | `/v1/walks/thread` | Existing typed thread subjects, bounded traversal, unresolved references, and evidence identities. |
| `runtime why` | `/v1/walks/eligibility` | Supported absence kinds are Task completion, Task admission, Belief revision, and evidence. This is not free-form question answering. |
| `runtime projection parent` | `/v1/projections/parent` | Existing delegation scope, event watermark, and anomaly derivation. |
| `runtime projection subagent` | `/v1/projections/subagent` | Existing delegation scope, event watermark, and actor reports. |

Do not add an unrestricted `event append` merely because the HTTP substrate has append routes. Consumer transport operations and internal capabilities do not automatically warrant end-user commands. Likewise, ledger identity, watermark, and subscription polling support status and following; they do not need separate top-level verbs.

## What this scope cannot honestly promise

First-time default setup and native product preparation are existing but separate operations. Grouping them under `init` improves discovery without inventing a bootstrap workflow. `init product` still needs a valid declaration and package source. Automatic product selection, declaration creation, and a bare `init` that guarantees a runnable native product require additional behavior and should not be smuggled into a rename.

The proposed `status` initially means exactly the existing supervisor status. Joining all product readiness, Goal state, observation freshness, and operational blockers into a new unified report belongs to the dedicated observability pass.

There is no existing user-facing remote stop, attach, background native service installation, OTel exporter, observation-coverage report, generic world-model CRUD, or replacement Context generation package established by this inventory. Foreground interruption already exists. Do not advertise additional lifecycle verbs without their corresponding contracts.

The bulk of the command changes are parser, help, routing, and documentation work. Existing live-command bypasses, production dispatch binding, destructive entry routing, and graph Event-position capture are behaviorally significant and must survive regrouping. Adapter work for served diagnostics and correcting misleading existing options are distinct from simple renames.

## Handler review findings

Sol independently checked all Agent profile, provider, workspace, watch, and destructive command handlers. The review confirmed implemented profile and provider operations and the distinction between configured profiles and native Agent records. Provider model, timeout, connectivity, and credential-status flags reach the corresponding implementations.

Watch needs explicit qualification. The route discards `--foreground`, and the current handler runs in the foreground despite advertising a default background daemon. It also performs compatibility frame maintenance for configured profiles; Workflow-bound profiles are skipped. It is not native Agent reconciliation or a replacement for the retired direct Context generation commands.

Source inspection found that [the watch loop](../../../../src/workspace/watch/runtime.rs) appends raw events whenever [the batcher](../../../../src/workspace/watch/events.rs) returns false, including paths it ignored and events it intended to debounce. Therefore help should not promise correct suppression behavior without a separate fix. The recommendation retains access to the existing watcher while identifying these defects; no watch fix or new daemon lifecycle is included in this assessment.
