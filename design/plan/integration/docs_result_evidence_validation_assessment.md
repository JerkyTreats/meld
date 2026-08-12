# Docs Result Evidence Validation Assessment

Date: 2026-08-11
Status: implementation assessment

## Concern

Generated READMEs must be accepted only when every asserted claim is accounted for by the bounded source and descendant evidence used by the docs domain. The assessment must remain durable and auditable, feed the existing docs freshness belief, and preserve dynamic Strategy construction from available capabilities without adding docs semantics to generic runtime code.

## Scope

In scope:

- complete claim coverage over candidate README content
- semantic support, contradiction, and unsupported verdicts with evidence quotations
- deterministic validation of literal code, command, URL, and inline-code claims
- bounded revision when a candidate fails validation
- publication of only validated content
- final assessment tied to source and content hashes
- theory mapping from the richer assessment into docs freshness belief evidence

Out of scope:

- CVE domain implementation
- a generic natural-language truth service in Meld core
- legacy docs writer workflows or Methods
- user-interface changes
- automatic materiality wake behavior for later source changes
- claims about truth beyond the admitted repository evidence

Current evidence basis is the committed dynamic docs PDS in `src/docs`, the exact-byte assessment in `src/docs/capability.rs`, the generic Strategy and artifact runtime, the weighted belief comparator, and the retained Gmail Operator live evaluation.

Applicable policy includes domain-first organization, explicit cross-domain contracts, semantic unit preservation, and the Assessment By Domain policy.

## Domain Snapshot

Command:

```sh
find src -maxdepth 1 -type f -name '*.rs' -printf '%f\n' | sed 's/\.rs$//' | sort
```

Output:

```text
agent
api
branches
capability
cli
compat
concurrency
config
context
control
docs
error
events
execution
harness
heads
ignore
init
lib
logging
merkle_traversal
metadata
prompt_context
provider
runtime
serve
session
store
task
telemetry
tree
types
views
workflow
workspace
world_state
```

## Pass One Domain Sweep

| Domain | Needed integration | Current integration | Completeness | Evidence | Non-integration rationale | Follow-up |
| --- | --- | --- | --- | --- | --- | --- |
| `agent` | `consume` | Agent curation consumes the docs freshness belief and authorizes dynamic Strategy candidates. | `complete` | [docs PDS](/home/jerkytreats/meld/src/docs/pds.rs) |  | Reuse unchanged. |
| `api` | `none` | No result-validation API is required. | `not needed` | [API facade](/home/jerkytreats/meld/src/api.rs) | Validation is internal capability work. | none |
| `branches` | `none` | The current docs target remains one bound workspace scope. | `not needed` | [branches domain](/home/jerkytreats/meld/src/branches.rs) | No branch federation or branch-local assessment is introduced. | none |
| `capability` | `publish` | Generic contracts already support typed multi-input and typed output slots. | `complete` | [capability contracts](/home/jerkytreats/meld/crates/meld-execution/src/capability/contracts.rs) |  | Reuse unchanged. |
| `cli` | `none` | No new command or output format is required. | `not needed` | [CLI route](/home/jerkytreats/meld/src/cli/route.rs) | Existing runtime command drives the flywheel. | none |
| `compat` | `none` | No persisted compatibility migration is introduced. | `not needed` | [compat domain](/home/jerkytreats/meld/src/compat.rs) | Evaluation artifacts are new docs-domain products. | none |
| `concurrency` | `none` | Existing queued provider execution and leases remain sufficient. | `not needed` | [runtime contracts](/home/jerkytreats/meld/src/runtime/contracts.rs) | No new coordination primitive is needed. | none |
| `config` | `none` | Provider and stewardship selection remain unchanged. | `not needed` | [configuration](/home/jerkytreats/meld/src/config.rs) | Validation policy is PDS-owned theory rather than operator configuration. | none |
| `context` | `none` | The verifier uses bounded docs evidence directly. | `not needed` | [generation context](/home/jerkytreats/meld/src/context.rs) | No frame or context-query contract changes. | none |
| `control` | `none` | No control plan or node-outcome state changes. | `not needed` | [control domain](/home/jerkytreats/meld/src/control.rs) | Strategy and task-network execution already own orchestration. | none |
| `docs` | `own` | Docs owns evidence collection, generation, publication, and byte assessment but not semantic claim validation. | `partial` | [docs capabilities](/home/jerkytreats/meld/src/docs/capability.rs) |  | Add claim assessment and bounded revision behavior. |
| `error` | `publish` | Existing errors lacked a generic terminal capability signal for deterministic policy exhaustion. | `partial` | [execution errors](/home/jerkytreats/meld/crates/meld-execution/src/error.rs) |  | Add one generic marker without docs semantics. |
| `events` | `publish` | Execution success outcomes already preserve artifact records for evidence interpretation. | `complete` | [outcome interpretation](/home/jerkytreats/meld/crates/meld-world-model/src/belief/outcome/interpretation.rs) |  | Reuse canonical artifact publication. |
| `execution` | `consume` | Dynamic Strategy lowering connects capabilities by typed artifacts but selected the same shared producer more than once for a multi-input DAG. | `partial` | [Strategy search](/home/jerkytreats/meld/crates/meld-world-model/src/strategy/search.rs) |  | Reuse a selected producer generically across artifact requirements. |
| `harness` | `none` | The live evaluation can observe runtime output without harness contract changes. | `not needed` | [harness domain](/home/jerkytreats/meld/src/harness.rs) | Validation does not add a harness semantic. | none |
| `heads` | `none` | Legacy head indexes are unrelated. | `not needed` | [heads domain](/home/jerkytreats/meld/src/heads.rs) | No legacy context head behavior is used. | none |
| `ignore` | `none` | Existing docs scope filtering remains unchanged. | `not needed` | [docs scope inspection](/home/jerkytreats/meld/src/docs/capability.rs) | Claim validation consumes the already admitted evidence bundle. | none |
| `init` | `adapter` | Theory source provisioning installs the docs belief and outcome mapping. | `complete` | [theory provisioning](/home/jerkytreats/meld/src/init/world/source.rs) |  | Update authored theory bodies only. |
| `lib` | `none` | Existing docs export remains sufficient. | `not needed` | [crate surface](/home/jerkytreats/meld/src/lib.rs) | New products remain inside the exported docs domain. | none |
| `logging` | `none` | Provider and task logs already expose calls and failures. | `not needed` | [logging domain](/home/jerkytreats/meld/src/logging.rs) | Logs are not assessment authority. | none |
| `merkle_traversal` | `none` | Claim validation uses the docs evidence artifact rather than tree traversal. | `not needed` | [traversal domain](/home/jerkytreats/meld/src/merkle_traversal.rs) | No traversal behavior changes. | none |
| `metadata` | `none` | No frame metadata contract changes. | `not needed` | [metadata domain](/home/jerkytreats/meld/src/metadata.rs) | Claim provenance lives in docs artifacts. | none |
| `prompt_context` | `none` | Verifier prompts are capability-local provider messages. | `not needed` | [prompt context](/home/jerkytreats/meld/src/prompt_context.rs) | No prompt artifact lineage contract is changed. | none |
| `provider` | `consume` | Existing provider execution accepts bounded messages and returns completion content. | `complete` | [provider execution](/home/jerkytreats/meld/src/provider.rs) |  | Reuse unchanged for constrained verdicts and revisions. |
| `runtime` | `adapter` | Root assembly composes the docs PDS catalog and executor registry, while task dispatch classified every capability error as retryable. | `partial` | [runtime ports](/home/jerkytreats/meld/src/runtime/ports.rs) |  | Recognize the generic terminal marker without docs semantics. |
| `serve` | `none` | No loopback route is required. | `not needed` | [serve routes](/home/jerkytreats/meld/src/serve.rs) | Runtime execution remains in process. | none |
| `session` | `none` | Validation truth is durable artifact state, not command lifecycle. | `not needed` | [session domain](/home/jerkytreats/meld/src/session.rs) | No session semantics change. | none |
| `store` | `none` | Existing artifact and world-model stores persist the new products. | `not needed` | [store domain](/home/jerkytreats/meld/src/store.rs) | No storage primitive or schema owner changes. | none |
| `task` | `consume` | Task networks carry typed evidence, patch, validation, and receipt artifacts. | `complete` | [task domain](/home/jerkytreats/meld/src/task.rs) |  | Reuse generic artifact lineage. |
| `telemetry` | `none` | Existing request and task events are sufficient for observation. | `not needed` | [telemetry domain](/home/jerkytreats/meld/src/telemetry.rs) | Telemetry must not determine claim truth. | none |
| `tree` | `none` | The verifier receives rendered source evidence. | `not needed` | [tree domain](/home/jerkytreats/meld/src/tree.rs) | No tree representation changes. | none |
| `types` | `none` | No shared root type is required. | `not needed` | [types](/home/jerkytreats/meld/src/types.rs) | Claim products are docs-owned. | none |
| `views` | `none` | No presentation projection is requested. | `not needed` | [views domain](/home/jerkytreats/meld/src/views.rs) | Audit artifacts remain available through existing event and artifact observation. | none |
| `workflow` | `none` | The docs PDS continues with no Method or workflow route. | `not needed` | [dynamic docs PDS](/home/jerkytreats/meld/src/docs/pds.rs) | Adding a verifier must only change capability availability and artifact constraints. | none |
| `workspace` | `consume` | Docs inspection and publication read and write the bound repository. | `complete` | [docs publication](/home/jerkytreats/meld/src/docs/capability.rs) |  | Publish only a validated patch artifact. |
| `world_state` | `consume` | Outcome mapping promotes docs assessment fields into belief evidence. | `partial` | [docs outcome theory](/home/jerkytreats/meld/theory/docs_freshness/outcome_interpretation.docs_freshness.json) |  | Map the validation-aware stale probability and retain assessment provenance. |

Frozen affected-domain set:

```text
agent
capability
docs
error
events
execution
init
provider
runtime
task
workspace
world_state
```

## Pass Two Affected-Domain Decomposition

| Domain concern | Owner | Current ground | Required relationship | Change posture | Boundary risk | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| Goal curation | `agent` | Curates divergence and authorizes a recommended Strategy. | Consume the validation-aware belief without understanding claim semantics. | `reuse unchanged` | Claim verdicts must not leak into Agent internals. | [agent strategy](/home/jerkytreats/meld/crates/meld-world-model/src/agent/strategy.rs) |
| Satisfaction judgment | `agent` | Uses planner-safe belief projections. | Remain driven by the docs freshness threshold. | `reuse unchanged` | Assessment existence must not substitute for confidence. | [agent runtime](/home/jerkytreats/meld/crates/meld-world-model/src/agent/runtime.rs) |
| Capability contracts | `capability` | Typed slots, effects, execution class, and physical binding identity. | Carry evidence plus draft into a validation capability and a validated patch out. | `reuse unchanged` | Avoid a docs-specific generic contract variant. | [contracts](/home/jerkytreats/meld/crates/meld-execution/src/capability/contracts.rs) |
| Capability invocation | `capability` | Validates supplied typed inputs and records artifacts. | Preserve both input lineages and the validation artifact. | `reuse unchanged` | Slot identity must remain exact. | [invocation](/home/jerkytreats/meld/crates/meld-execution/src/capability/invocation.rs) |
| Evidence admission | `docs` | Builds bounded direct and descendant evidence. | Define the complete evidence boundary used by claim verdicts. | `extend existing` | A verifier must not consult hidden context unavailable to generation. | [scope inspection](/home/jerkytreats/meld/src/docs/capability.rs) |
| Claim extraction | `docs` | No complete final-claim inventory exists. | Atomize every non-empty README assertion into stable claim identities. | `new local behavior` | Model-selected claims could omit hallucinations, so extraction must be deterministic. | [docs capability](/home/jerkytreats/meld/src/docs/capability.rs) |
| Claim comparison | `docs` | Prompt guidance is not an assessment product. | Produce supported, unsupported, and contradicted verdicts with exact evidence quotations. | `new local behavior` | The provider verdict is probabilistic and must carry reliability and auditable support. | [claim validation](/home/jerkytreats/meld/src/docs/claim_validation.rs) |
| Claim scope guards | `docs` | A low-power verifier can cite one true clause while accepting a broader claim. | Require exact literal admission, explicit universal scope, and citation coverage for every independent clause. | `new local behavior` | Conservative lexical guards can remove valid paraphrase. | [claim validation](/home/jerkytreats/meld/src/docs/claim_validation.rs) |
| Literal verification | `docs` | Generation asks for exact identifiers but does not verify them. | Reject invented code blocks, URLs, and inline-code literals absent from evidence. | `new local behavior` | Overly broad literal checks could reject valid Markdown notation. | [README prompt](/home/jerkytreats/meld/src/docs/capability.rs) |
| Bounded revision | `docs` | Draft generation retries only context-limit failures. | Revise failed candidates using the explicit claim report within a PDS-owned bound. | `new local behavior` | Repeated identical invalid output must terminate truthfully. | [generation retry](/home/jerkytreats/meld/src/docs/capability.rs) |
| Publication | `docs` | Publishes raw patch sets. | Accept only validation-bearing patch sets and preserve content hashes. | `extend existing` | Assessment and published bytes must not diverge. | [publication](/home/jerkytreats/meld/src/docs/capability.rs) |
| Final assessment | `docs` | Checks source fingerprint and exact published bytes. | Include claim validation identity and weighted risk in the final evidence product. | `extend existing` | A successful write must not imply semantic freshness. | [final assessment](/home/jerkytreats/meld/src/docs/capability.rs) |
| PDS capability publication | `docs` | Publishes four atomic capabilities and lets Strategy close their artifact chain. | Publish claim validation as another atomic capability with a PDS-owned policy. | `extend existing` | Do not encode an authored workflow or Method. | [docs PDS](/home/jerkytreats/meld/src/docs/pds.rs) |
| Canonical task outcome | `events` | Carries all artifact records in execution success events. | Carry validated patch and final assessment unchanged. | `reuse unchanged` | Array position must remain non-semantic. | [outcome mapping](/home/jerkytreats/meld/crates/meld-world-model/src/belief/outcome/interpretation.rs) |
| Strategy construction | `execution` | Backward search selected a producer independently for each required artifact. | Reuse one already selected producer when it satisfies another artifact in the same generic DAG. | `extend generic behavior` | The fix must remain artifact-driven and contain no docs names or ordering. | [Strategy search](/home/jerkytreats/meld/crates/meld-world-model/src/strategy/search.rs) |
| Terminal capability outcome | `error` | Deterministic capability rejection otherwise reentered retry forever. | Mark policy-exhausted capability failure as terminal through a generic execution contract. | `extend generic behavior` | Provider transport and storage failures must remain retryable. | [execution error](/home/jerkytreats/meld/crates/meld-execution/src/error.rs) |
| Theory provisioning | `init` | Installs selected authored belief and outcome bodies. | Provision updated docs evidence fields through the existing path. | `adapter only` | Init must not interpret validation semantics. | [world source](/home/jerkytreats/meld/src/init/world/source.rs) |
| Completion execution | `provider` | Executes provider requests with bounded time and durable request logs. | Run constrained claim verdict and revision prompts. | `reuse unchanged` | Generation and verification share a model, so assessment reliability must remain explicit. | [provider executor](/home/jerkytreats/meld/src/provider/executor.rs) |
| Product assembly | `runtime` | Composes the docs PDS registry and Strategy into generic actors. | Receive the expanded catalog without docs-aware branches. | `adapter only` | Root must not name claim-validation steps. | [runtime assembly](/home/jerkytreats/meld/src/runtime/assembly.rs) |
| Task failure classification | `runtime` | Dispatch retried all capability failures. | Settle only the generic terminal capability signal as a final task outcome. | `extend generic behavior` | The adapter must not recognize docs capability identities. | [runtime ports](/home/jerkytreats/meld/src/runtime/ports.rs) |
| Artifact readiness | `task` | Requires typed input artifacts before capability dispatch. | Require evidence and draft artifacts for validation and validated output for publication. | `reuse unchanged` | Readiness must follow contracts rather than step order. | [task readiness](/home/jerkytreats/meld/crates/meld-execution/src/task/readiness.rs) |
| Source observation | `workspace` | Supplies the bound repository to docs inspection. | Remain the source of admitted local evidence. | `reuse unchanged` | The verifier must not redefine workspace truth. | [workspace domain](/home/jerkytreats/meld/src/workspace.rs) |
| Publication target | `workspace` | Receives atomic README writes through docs behavior. | Receive only validated bytes. | `reuse unchanged` | No generic workspace write policy should become docs-specific. | [docs publication](/home/jerkytreats/meld/src/docs/capability.rs) |
| Evidence interpretation | `world_state` | Selects the final assessment artifact and promotes scalar evidence. | Promote validation-aware stale probability with source and assessment lineage. | `extend existing` | Intermediate draft or validation artifacts must not settle freshness. | [outcome theory](/home/jerkytreats/meld/theory/docs_freshness/outcome_interpretation.docs_freshness.json) |
| Belief comparison | `world_state` | Combines normalized scalar evidence using installed family theory. | Consume the docs-domain aggregate without learning claim semantics. | `reuse unchanged` | Weighted claim evaluation must happen before generic Bayesian comparison. | [belief comparator](/home/jerkytreats/meld/crates/meld-world-model/src/belief/comparator.rs) |

## Synthesis

The docs domain owns admitted evidence, claim extraction, claim verdicts, bounded revision, publication eligibility, and the validation-aware assessment. PDS theory owns claim weights, acceptance thresholds, revision bounds, capability availability, and the prospective evidence route.

Provider execution, generic capabilities, task readiness, event publication, belief comparison, workspace access, and Agent curation remain unchanged runtime participants. Strategy search gains generic shared-producer reuse. Execution errors and runtime task dispatch gain a generic terminal capability boundary. Root assembly and init remain adapters.

The smallest missing connective behavior is one docs-owned capability that consumes the evidence bundle and draft patch set, produces a validation-bearing patch set, and fails truthfully when bounded revision cannot satisfy the PDS claim policy.

The principal boundary risks are incomplete claim extraction, unsupported claims mislabeled by the same model that generated them, one citation supporting only part of a compound claim, accidental publication before validation, and weighted averages hiding critical contradictions. Complete deterministic claim coverage, exact evidence quotations, literal and scope guards, strict contradiction thresholds, and content-hash fencing constrain those risks.

No integration is added to workflow, CLI, API, context, telemetry, storage primitives, or legacy compatibility paths.

## Implementation Scope Separation

Runtime path domains:

```text
agent capability docs error events execution init provider runtime task workspace world_state
```

Domains with changed behavior:

```text
docs error execution runtime world_state theory
```

Likely file scope:

```text
src/docs/capability.rs
src/docs/claim_validation.rs
src/docs/pds.rs
crates/meld-execution/src/error.rs
crates/meld-world-model/src/strategy/search.rs
src/runtime/ports.rs
theory/docs_freshness/outcome_interpretation.docs_freshness.json
docs capability and PDS tests
generic Strategy and terminal outcome tests
integration theory and runtime tests
```

## Unresolved Questions

The first implementation uses the same configured provider for generation and semantic verification. Its verdicts therefore remain calibrated evidence rather than absolute truth. A future independent verifier may raise reliability without changing the artifact or belief boundary.

Documentation coverage obligations remain outside this workstream. This assessment verifies every claim the final makes. A later extension may also verify that all PDS-required claims are present.
