# Reconciliation acceptance and remediation

Assessment date: 2026-09-07

Inspected source: `186adaae7a82d8369af5b403dd8ce3ec48aeaffd`

Full architectural acceptance is not established. Installed products have substantial executable evidence, but that evidence does not cover every original architectural obligation or the actual process failure boundary. A remaining root-owned failure classifier also needs correction.

This is a working assessment and remediation sequence under the user's existing implementation authorization. It creates no binding slice, frozen finding set, additional permission step or replacement architecture. The [outcome audit](reconciliation_outcome_audit.md) remains the historical evidence account. Findings may change this sequence when the implementation contradicts it.

## What acceptance means

Acceptance must establish that the intended reconciliation behavior runs through canonical native owners, returns enough evidence for Agent to make the correct next decision, survives the claimed interruptions, and has removed superseded runtime authority. Every applicable original obligation must have delivered evidence or an explicit disposition from the authority that can revise the intended outcome. An assigned future task preserves an obligation; it does not establish full acceptance.

The source baseline is the [world-model architecture](../../../cognitive_architecture/world_model/README.md), the original [Planner, Strategy and Agent design](detailed_design/planner_cut_strategy_plan_and_agent_progression.md), the [original observable outcomes](detailed_design/integrated_product_outcome_evidence_matrix.md), the discovery throughline identified in the audit, and the [runtime invariants](../../../../governance/runtime_invariants.md). Historical program exclusions cannot narrow them.

There are two different acceptance claims. The installed Startup, Docs, Cargo Security and declared code-change products can be accepted against their explicitly declared knowledge and operating limits. Unqualified acceptance of the complete world-model architecture also requires its absent Causation and Regime behavior. Product acceptance must not close that architectural obligation. Conversely, the legal use of an explicitly limited Planner cut is not itself a product defect.

No new permission is needed to correct source or improve proof under the standing mission. Revising away an original outcome would require the appropriate authority; this assessment does not do that.

## Findings that change the acceptance work

| Finding | Evidence and consequence | Required disposition |
| --- | --- | --- |
| Causation and Regime remain absent | The current [crate root](../../../../crates/meld-world-model/src/lib.rs) exposes no native owners for these domains, and source searches find none of their principal semantic products. Their [Causation requirements](../../../cognitive_architecture/world_model/causation/requirements.md) and [Regime requirements](../../../cognitive_architecture/world_model/regime/requirements.md) require replayable behavior, not source-kind labels. | Full world-model acceptance requires native implementation and integration, or an explicit architectural scope revision. Product-only evidence cannot discharge this. |
| Planner can represent source classes without supplying their native meaning | [Planner policy and source positions](../../../../crates/meld-world-model/src/planner/contracts.rs) and [assembly validation](../../../../crates/meld-world-model/src/planner/projection.rs) distinguish required and explicitly omitted sources. They are useful foundations, not causal or regime semantics. | Prove required native revisions are resolved, consumed and invalidated through Planner and Strategy, with changed knowledge causing a production Agent successor. |
| Root still derives terminal failure from diagnostic text | [Claim invocation](../../../../src/runtime/ports.rs) stringifies errors and calls `is_terminal_claimed_failure`, which searches for two marker phrases. A retryable provider message containing a marker can therefore be treated as an owner-declared terminal result. This consequence follows from the inspected control path; a new adversarial regression has not yet been executed. | Replace semantic text matching with typed owner-declared failure and unresolved outcomes through Capability, Task and root translation. Preserve uncertain effects and prove marker-containing transient messages remain unresolved. Remove the superseded classifier. |
| Existing CLI and recovery evidence has a narrower boundary than a process acceptance claim | The inspected [CLI tests](../../../../tests/integration/runtime_cli.rs) invoke `RunContext` in process. [Startup recovery](../../../../src/runtime/assembly/tests/startup_account.rs) and [replacement](../../../../src/runtime/assembly/tests/replacement.rs) reopen stores after explicit flushes and controlled handle drops. | Add actual executable startup, process termination and restart scenarios. Retain the existing deterministic boundary tests; do not relabel them as abrupt-process proof. This inspection does not assert that no subprocess test exists elsewhere. |
| Final combined validation is not yet recorded for this source | The audit records broad historical runs and focused later runs. The latest provider checkpoint records 891 passing tests, not a fresh complete workspace result on the final acceptance source. | Run the complete validation set on the final remediation commit and identify the exact source, commands, results and ignored-test dispositions. |
| The original outcome matrix is not the whole architecture | O01–O34 cover concrete product and runtime observations. The original Plan design also includes prerequisite decomposition, assumptions, unresolved observation paths and causal context. Current [Strategy products](../../../../crates/meld-world-model/src/strategy/contracts.rs) and construction need a clause-level comparison before their completeness can be asserted. | Reconcile these original obligations against actual products and verifier behavior. Record unimplemented behavior separately from missing proof. Do not infer that every unrepresented field requires a new abstraction. |

## Current domain sweep

The domain universe is regenerated from root `src/lib.rs` and the four crate roots. Forwarders are counted with their canonical semantic owners. Each current domain is included below; absent required domains are identified in the findings rather than invented as current participants. Completeness here describes acceptance evidence, not a percentage of implemented architecture.

| Current domain or domain group | Needed integration | Current integration and completeness | Acceptance follow-up |
| --- | --- | --- | --- |
| Agent, including root `agent` forwarding | own | Native judgment, requests, authorization, history and lifecycle; partial acceptance | Review initial, changed, failed, refused, historical and maintained-condition paths together |
| Graph and Traversal, including `world_state` and `merkle_traversal` | own, publish | Native occurrence, publication, cut and cursor behavior; partial acceptance | Verify completeness, provenance, stale knowledge and independent replay in the system scenarios |
| Belief and evidence ingestion | own | Native settlement and configured returns; partial acceptance | Prove negative and uncertain evidence cannot become satisfaction through projection |
| Curation | own | Native standing and planned work, rejection, result and publication; partial acceptance | Exercise product-required sequencing, visibility and historical return consumption |
| Planner | own | Exact limited cut assembly and refusal; partial acceptance | Resolve the full source-consumption gap and verify currentness from the same owner selections |
| Strategy | own | Pure mixed Plan construction, verification and successor history; partial acceptance | Compare the entire original closure grammar and implement material missing behavior |
| Execution and `task` | own | Native admission, network, sharing, dispatch and publication; partial acceptance | Correct typed failure transport and verify independent discharge and uncertain-effect recovery |
| Capability and provider | own, publish | Exact executable bindings and provider response classification; partial acceptance | Preserve owner failure decisions intact through invocation and recovery |
| Events | own | Native append, identities, cursor and outbox contracts; partial acceptance | Verify actual process interruption between durable production and consumption |
| workspace, tree, branches, heads | publish | Native source and destination observation; partial acceptance | Verify source changes, owner return and external runtime-state roots |
| Docs | own, publish | Repair, no-action, confirmation and provider failure recovery; partial acceptance | Run the ordinary executable route with repeated change and interruption |
| dependency security | own, publish | Declared inventory, advisory, mitigation and independent verification; partial acceptance | Prove the installed product's full path and retain its explicit advisory-source limit |
| nonce | own, publish | Native Startup effect and epoch attribution; partial acceptance | Use as the minimal real-process lifecycle and replay proof |
| code change | own, publish | Declared proposals and recovered materialization on supported platforms; partial acceptance | Prove exact proposal, effect recovery and separate returned evidence through the executable |
| theory and init | own | Product compilation, native installation and inert preparation; partial acceptance | Verify clean installation and physical replacement without inherited test-only state |
| runtime | adapter, own | Structural composition and lifecycle aggregate; partial acceptance | Remove terminal text classification and re-audit every retained semantic conversion |
| CLI, API, control, serve | adapter, observe | Entrypoints, requests and inspection; partial acceptance | Demonstrate real process use and truthful explanation of the first missing owner position |
| context, prompt context, views, session | consume, adapter | Manual authoring and retained context support; partial acceptance | Retain their demonstrated responsibilities and check no superseded Plan or publication writer returns |
| harness | observe | Composed proofs and controlled failure injection; partial acceptance | Keep it out of product authority and distinguish each injected boundary from ordinary operation |
| workflow | none | Retained historical reading and explicit refusal of retired execution; not needed for native reconciliation | Recheck callers and constructible exports as retirement evidence, not migration back to Workflow |
| compat, concurrency, config, error, ignore, logging, metadata, store, telemetry, types | none | Supporting facilities, not independent reconciliation authorities; not needed for new semantic integration | Retain regression coverage. `error` carries the typed-failure remediation as a supporting contract. |
| Meld Language | publish | Shared propositions, bindings, authority and compositions; partial acceptance | Check that closure and uncertainty remain domain-owned rather than added as consumer-specific language rules |

The affected acceptance set is exactly the non-`none` rows, plus the retirement check on Workflow. This does not authorize treating every domain as an implementation edit. The absence of Causation and Regime expands the architectural remediation set explicitly.

## One level of affected concerns

| Concern and owner | Current ground | Required relationship and change posture |
| --- | --- | --- |
| Observation in workspace, Docs, Security, nonce and code change | Existing native producers and product proofs | Reuse existing owners; extend only where actual executable scenarios expose missing production behavior |
| Durable transport and graph consumption in Events, Graph and Traversal | Native ledgers, publications and replay | Reuse existing authority; add process-boundary evidence and fix any reproduced persistence defect |
| Epistemic authorship and settlement in Curation and Belief | Separate operation, publication, visibility and belief returns | Reuse existing contracts; audit adverse returns and independence of satisfaction |
| Reasoning context in Planner | Exact limited source cuts | Extend native source consumption for required causal and regime products |
| Closure and verification in Strategy | Mixed Plans, bounded construction and predecessor history | Compare original prerequisite and unresolved-condition semantics, then extend existing construction and verifier where needed |
| Judgment, requests and progression in Agent | Native currentness, authorization and history | Reuse canonical actor; demonstrate new semantic source invalidation and real-process recovery |
| Operational realization in Capability, provider, Task and Execution | Native dispatch with a text-based terminal bridge | Extend typed return contracts and delete text-based semantic classification |
| Installation in theory and init | Complete preparation and activation inputs | Reuse existing compilation; bind any new owners through the same installation route |
| Participation in native owners and runtime | Owner lifecycle receipts and structural aggregation | Reuse native evidence; any new owner must derive its own checkpoints and safe points |
| Inspection in CLI, API, control and serve | Runtime accounts and native queries | Adapter-only extension where a required missing position is not inspectable |
| Context support and shared language | Existing manual paths and domain-neutral values | Reuse unchanged unless a concrete caller or contract correction requires an edit |
| Acceptance evidence in harness | Existing production composition and boundary controls | Extend process-based proof; never manufacture domain decisions to obtain a passing result |

Confirmed immediate source write scope is the Capability and Task error transport, supporting error contract, root claim translation and corresponding tests. Causation and Regime are new native implementation scope, with consumer changes in Planner, Strategy and Agent and installation changes as required. Other source writes depend on a reproduced defect or an established missing original behavior. Runtime participation alone is not write scope.

## Remediation into acceptance

1. Reconcile original obligations before declaring architectural completion. Extend the existing outcome projection with the applicable original Plan, causal and regime requirements. For each record, name the observable behavior, current native source, strongest actual proof, missing behavior or proof, semantic owner and closure condition. Preserve original wording where a temporary restriction would otherwise replace it. This is one current account, not a new packet hierarchy.
2. Correct the confirmed terminal-classification defect. First reproduce a transient response whose message contains a terminal marker. Carry typed terminal versus unresolved owner results through ordinary invocation and recovery; remove substring inference. Re-run definite rejection, transient retry and uncertain-effect recovery together.
3. Complete required native knowledge behavior. Causation must retain interventions and measured outcomes, identification blockers, mechanisms, assumptions, uncertainty and replayable summaries under versioned methods. Regime must retain source-qualified signals, continuation versus break reasoning, posterior and segment history, mixture, sensitivity and stress summaries under versioned methods. Implement their detailed requirements through meaningful native paths, not empty products or blanket provisional labels. Resolve dependency cycles through explicit source revisions and inspectable absence. Connect revisions to Planner and Strategy, and prove a change invalidates old eligibility and produces an Agent successor without replaying completed effects. A single thin demonstration is the starting point, not acceptance of every requirement in those domains.
4. Close any remaining Plan-construction discrepancy found in the original-obligation comparison. In particular, establish what happens when a needed condition is not initially true, information is incomplete, or a required mechanism becomes invalid. A truthful refusal is correct where closure is impossible; it cannot substitute for a required constructible observation or prerequisite path.
5. Exercise ordinary executable product use. Start from isolated user configuration and external product state. Run Startup; Docs absent, wrong and already correct; changed and stale Security inputs with declared mitigation; and an exact declared code-change request. Use production installation, request intake, provider protocol and supervision. Retain exact before-and-after artifacts and owner records. Scripted providers and advisory sources are acceptable deterministic inputs, with their limits stated.
6. Exercise cross-cutting failures through that same executable. Cover source change before authorization, change while work is admitted, delayed visibility, missed notification, rejection, returned failure, uncertain callback, competing compatible admissions, generation replacement and process termination. At minimum terminate once after an acknowledged durable producer commit before consumption and once after a workspace effect before its consumer return. Restart through ordinary startup and prove preserved attribution, no duplicate effect and eventual correct disposition. Do not extend this claim to hardware power-loss durability without a corresponding experiment.
7. Review and validate one final source. Re-audit canonical ownership and constructible legacy exports against original invariants, allowing newly found contradictions to reopen the relevant implementation. Run the complete suite and final executable scenarios on the same source. After a correction, repeat its causal regression and affected scenarios; obtain the final complete run after all source changes are finished. Commit and push natural verified checkpoints under the user's standing authorization.

The native knowledge work can begin while executable acceptance infrastructure is prepared. Final full architectural acceptance depends on both. No code implementation or test run was performed as part of this assessment.

## Final acceptance evidence

The acceptance result should be concise enough to inspect: one final commit and build identity; the completed original-obligation projection; reproducible scenario commands and retained native positions; canonical responsibility and retirement findings; test results; and explicit product, platform and fault-model limits. A review should challenge the final source without a frozen finding set. Findings are closed by corrected behavior and proof, not by editing their wording to fit current source.

The core regression commands are:

```bash
cargo fmt --all -- --check
cargo test --workspace --lib --bins --tests --examples -- --test-threads=1
cargo test --workspace --doc
cargo clippy --workspace --all-targets -- -D warnings -A clippy::result_large_err
git diff --check
```

Record the rationale for the established large-error Clippy allowance. The three currently ignored integration tests require an external Gmail operator environment and are outside these declared reconciliation products; identify that disposition rather than silently counting them as passed. Actual executable and interruption scenarios supplement these commands. The final claim must identify whether its binary was built in debug or release mode and must match the tested configuration.

Full acceptance requires no unresolved required behavior, no demonstrated ownership contradiction, no required failure scenario without evidence, and no omitted requirement hidden behind a declared-complete domain. Narrow product acceptance may be useful earlier, but its name and limits must remain explicit. New unrelated research or speculative features do not enter acceptance merely because they appear elsewhere in the repository.

## Confidence and unresolved work

This assessment used a bounded breadth-first inspection of current public roots, original design, Planner and Strategy contracts, root failure translation, representative product and recovery tests, and recent verification history. It did not independently rerun historical tests or exhaustively review every domain implementation. Source inspection establishes the absent principal causal and regime products and the terminal marker classifier. The broader source-closure audit, executable scenario inventory, exact missing Plan behaviors, method-level Causation and Regime implementation details, and final validation remain work to execute. The assessment must not be cited as their successful review.
