# Flywheel command surface

Status: revised recommendation, 2026-09-09. Supersedes the public tree in [the complete command inventory](command_surface_recommendation.md). No executable behavior or command spelling changed.

## Selection rule

The primary command surface prepares, runs, requests work from, or explains a native product. Supporting tools remain when a current product or operator task consumes their behavior. The existence of a handler is not sufficient reason to make it prominent.

This is a refinement of the source-backed inventory, not a request to delete compatibility code or implement additional lifecycle behavior. Existing runtime composition loads some compatibility stores and registries. That physical dependency does not establish a user requirement to create Reader or Writer profiles before running a native product.

## Primary surface

```text
meld init                         Prepare a declared native product
meld run                          Run the foreground native runtime
meld status                       Inspect supervisor state

meld agent request                Request reconciliation of an installed intent

meld world graph status           Inspect graph readiness across branch scope
meld world graph owner-walk       Traverse exact owner publications

meld event status                 Inspect ledger health and consumer lag
meld event tail                   Read or follow durable records
meld event trace                  Inspect ledger causal references
meld event session                Inspect a command session
meld event flow                   Inspect recent event flow

meld runtime startup-account      Inspect Startup nonce and Goal evidence
meld runtime actions              Inspect retained actor reports
meld runtime flow                 Inspect the existing user projection
meld runtime trace                Walk an existing cross-domain thread
meld runtime why                  Explain supported absent-record cases
meld runtime projection parent    Inspect scoped boundary changes and anomalies
meld runtime projection subagent  Inspect scoped event changes and actor reports
```

`init` routes the existing `world init` operation. Preserve its path, package source, stage selection, and format controls. It installs and prepares an already declared product; it does not start execution. Move the old default-profile initialization out of the primary entry point. This revises the previous proposal, which retained profile initialization as bare `init` and added `init product`.

This move does not solve automatic bootstrap. The existing preparation contract requires a configured declaration and available package source. Automatically authoring the declaration or selecting a product would be additional behavior. Help must explain those actual prerequisites rather than promise that a bare invocation makes any fresh install runnable.

`run`, `status`, and `agent request` route existing runtime commands. Status retains its current supervisor meaning, not a newly invented aggregate health judgment. A reconciliation request accepts an idempotent request against an installed intention; it does not construct a new arbitrary intention from prose.

`world` gets the existing branch graph queries. Their implementation remains with the current branch federation and Graph contracts. No speculative Belief, Goal, or Agent CRUD is added.

`runtime startup-account` already exists. The remaining six diagnostic leaves expose existing served report, projection, and walk contracts through new CLI adapters. They introduce no new diagnostic derivations. Preserve bounds, session fences, evidence identities, and explicit gaps. The eligibility walk supports Task completion, Task admission, Belief revision, and evidence absence, rather than arbitrary natural-language diagnosis.

Events and runtime reports remain distinct. Event trace follows ledger references; runtime trace follows the existing cross-domain thread. Event flow summarizes ledger traffic; runtime flow renders the existing operational projection. Neither substitutes for the component-observance work in [the architecture assessment](observability_command_lifecycle_assessment.md).

## Supporting surface

| Family | Recommended leaves | Demonstrated reason to retain |
| --- | --- | --- |
| `provider` | `list`, `show`, `status`, `create`, `edit`, `remove`, `validate`, `test` | Native Docs selects a configured provider. Host execution resolves its configuration and forwards authorized requests. Provider maintenance and diagnostics support that product directly. Startup and declared Code Change do not require one. |
| `workspace branches` | `status`, `discover`, `attach`, `migrate` | Existing registry and migration operations support branch selection and federated graph access. They are workspace administration, not native Agent decisions. |
| `workspace` | `status`, `scan`, `validate`, `ignore`, `delete`, `restore`, `list-deleted`, `compact` | Implemented operations manage indexed filesystem state and its stored context. Retain as advanced workspace administration, with exact indexed-state semantics. This assessment does not establish that every native product requires them or that scan is a universal prerequisite. |
| `workspace flush` | Existing destructive operation with target, `--dry-run`, and `--yes` | Explicit operator removal of workspace runtime state. Preserve the dedicated safety checks and entry route. It is not a normal lifecycle transition or a method of making a stalled product healthy. |

Provider and workspace families belong in supported help with their dependency and administration roles stated. They should not appear as mandatory steps in every product's quick start. Native Startup requires neither workspace scanning nor a model provider.

## Compatibility disposition

These operations have no demonstrated role as primary native flywheel commands in the inspected routes. Retaining access to existing records or workflows is different from endorsing them for new native product setup.

| Existing behavior | Retention basis | Recommendation |
| --- | --- | --- |
| Default profile and prompt initialization | It creates configuration consumed by compatibility frame tooling. It does not prepare native Agent intent. | Remove from the primary `init` meaning. Preserve only with the retained compatibility tooling, not as a native setup step. |
| Reader/Writer profile list, show, status, validation, and prompt reads | Existing TOML and prompt files can need inspection. Context retrieval can use a profile's Workflow binding to resolve a frame type. | Retain as explicitly legacy profile inspection. Remove the proposed `agent profile` subtree from the native Agent family. |
| Profile create, edit, remove, and prompt editing | They maintain the compatibility watcher and frame tooling. No direct requirement was found in the native Docs provider path or native initialization route. | Keep only while that compatibility workflow is intentionally supported. Do not present them as native Agent administration. This assessment does not authorize their removal. |
| `context get` | Reads stored frames, including historical results and configured frame-type selection. | Retain for historical and compatibility data access. Clearly distinguish stored code context from live runtime context. |
| Workflow list, inspect, validate | Existing profile files remain inspectable; context selection may reference their frame types. | Retain historical inspection. No execution capability is implied. |
| Filesystem watch | It performs foreground filesystem maintenance and compatibility frame work. It is not the native reconciliation supervisor. | Exclude from the primary flywheel path. Retention as a supported compatibility operation also requires an explicit disposition of its known ignore/debounce defects and false daemon help. |
| Combined workspace/profile/provider status | Existing aggregate report concerns the older configuration surface. | Retain only as a compatibility overview or advanced workspace report. Do not use it as the native runtime status. |
| Direct Context generate/regenerate and Workflow execute | Handlers deliberately return retirement errors; no replacement Context generation package is supplied. | Keep out of supported executable help. Existing explicit retirement errors may remain for migration guidance. |

If compatibility syntax must be regrouped, one advanced `legacy` family can forward these operations to their existing owners. Do not expose both a new primary profile family and a legacy copy. The exact legacy spelling and retirement date are migration choices, not required native runtime behavior. No working command is deleted by this document.

## Source grounding and limits

- [Native initialization](../../../../src/init/world/tooling.rs) resolves the declaration, installed package, and native preparation pipeline. [Default initialization](../../../../src/init.rs) creates profiles, prompts, and their validation result.
- [Native Docs guide](../../../../theory/docs_freshness/README.md) selects a provider through its declaration. [Host route binding](../../../../src/cli/route.rs) binds the owner provider to the existing API; [provider execution](../../../../src/provider/executor.rs) resolves provider configuration. [Owner provider callbacks](../../../../src/runtime/owners/provider.rs) enforce the native operation's selected scope.
- [Startup](../../../../theory/startup/README.md) needs no workspace or model provider; [Code Change](../../../../theory/code_change/README.md) acquires a declared proposal without a model provider.
- [CLI assembly](../../../../src/cli/runtime_assembly.rs) still loads both profile and provider registries. Removing that assembly dependency is not part of a command rename.
- [Branch tooling](../../../../src/branches/tooling.rs) supplies registry operations and Graph queries. [Workspace tooling](../../../../src/workspace/tooling.rs) supplies indexed-state administration.
- [Context tooling](../../../../src/context/tooling.rs) provides stored retrieval, optional Workflow-derived frame-type selection, and explicit refusal of direct generation. [Watch runtime](../../../../src/workspace/watch/runtime.rs) still performs compatibility frame work.
- [Served routes](../../../../src/serve/routes.rs) expose the operational read products recommended for CLI adapters.

Confidence is high in the listed command routing and direct consumers, based on current source and the prior handler inventory. This is not a complete repository-wide reachability or deletion-safety proof. Source changes, command execution tests, automatic bootstrap, improved runtime presentation, and instrumentation coverage remain outside this assessment.

The resulting recommendation separates native operation, necessary product support, and compatibility retention. Most public regrouping remains parser, routing, and documentation work. It does not require representing every internal domain as a command family or adding missing product behavior to make the tree look complete.
