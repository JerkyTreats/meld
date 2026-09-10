# Meld

Meld is a runtime for keeping software intentions true as the world changes. Give it a maintained outcome, the domain knowledge needed to judge that outcome, and authority to act. It observes, decides whether work is needed, carries out authorized changes, and looks again to establish what actually happened.

A useful insight should have a life beyond the conversation that produced it. Meld's ambition is to make expensive reasoning durable: preserve its conclusions, the evidence they depend on and the conditions under which they remain useful. Spend intelligence on understanding and change. Let persistent state and ordinary computation carry that understanding forward.

The native flywheel and command control plane are implemented. The next product acceptance target is **README PDS**: maintain this project's README at the standard of the software it explains. This document is the initial editorial baseline; it is not yet maintained or qualified by that product.

## What Meld maintains

A maintained outcome describes a condition that should hold, such as documentation accurately explaining a changing codebase. It stays meaningful after an individual run finishes. When new evidence arrives, Meld can reassess that condition, preserve a satisfied state or authorize further work.

Product knowledge enters through **Persistent Domain Stewardship**, or PDS. A product supplies exact theory packages, scope, a principal, an Agent assignment and the domain owners needed to observe and act. An Agent carries the installed intention and judges Goals against admitted evidence. Its identity and history persist independently of a model conversation.

The repository contains these native product examples:

| Product | Responsibility | What to read |
| --- | --- | --- |
| Startup | Realize and independently confirm a nonce for the current admission epoch, with no workspace or model provider. | [Startup guide](theory/startup/README.md) |
| Docs freshness | Maintain README existence and configured factual coverage within a bounded source scope. This remains the internal maintenance and harness PDS. | [Docs freshness guide](theory/docs_freshness/README.md) |
| Dependency Security | Reconcile declared dependency-security mitigation using its selected external owner and evidence bindings. | [Security guide](theory/dependency_security_mitigation/README.md) |
| Code Change | Realize declared code changes through the native proposal and execution contracts. | [Code Change guide](theory/code_change/README.md) |

README PDS is a separate production target. Its job is to preserve a useful, accurate and thoughtfully written project introduction over time. The Docs freshness fixture does not establish that editorial capability. Security, Code Change and real-model quality also require their own product qualification; a working Startup loop cannot stand in for those results.

## Try the native loop

From a checkout, build the local executable and put it on this shell's path:

```sh
cargo build --locked --bin meld
export PATH="$PWD/target/debug:$PATH"
meld --version
```

These instructions exercise the checked-out implementation. A local build does not publish a release.

For a fresh configuration, prepare the bundled Startup product and start it:

```sh
meld init
meld runtime start
meld runtime follow
```

Initialization installs and prepares the product without activating work. Repeating it preserves prepared identities. Startup needs no provider credentials or target workspace, so the first run can test the runtime itself.

`runtime start` runs the existing supervisor in the background and waits for native readiness. Repeating start returns the matching live instance. `runtime follow` is an independent observer; Ctrl-C closes the observer and leaves the runtime running.

In another terminal, inspect the result and request orderly shutdown:

```sh
meld runtime startup
meld agent show
meld runtime stop
meld runtime shutdown
```

The Startup account shows the nonce, confirmation and separate Goal judgment. Runtime readiness answers whether the instance opened successfully. Goal evidence answers whether the installed intention was satisfied. Stop waits for native shutdown and release of store ownership; a timeout reports an unresolved outcome.

For a foreground run, use `meld runtime start --foreground`. Ctrl-C then requests native drain. Starting a successor creates a new admission epoch whose confirmation must come from its own evidence.

If an existing configuration is incompatible, init leaves it intact and explains how to select a separate configuration with `--config`. Configured products use their own declarations and packages; init does not silently replace them with Startup.

## Operate through ordinary commands

The command surface exposes native state and control. Use `--config PATH`, `--workspace PATH` and `--assignment NAME` when selection needs to be explicit. These selectors work before or after subcommands. Native inspection commands support `--json` for scripts.

| Question or action | Command |
| --- | --- |
| Which instance is running, and what history remains? | `meld runtime list` |
| How are the instance and its participants doing? | `meld runtime status` |
| What have the runtime actors reported? | `meld runtime actions --limit 20` |
| How do I resume observation? | `meld runtime follow --after SEQUENCE` |
| Which native Agents exist? | `meld agent list` |
| What intention, Goals, plans and judgments does an Agent hold? | `meld agent show AGENT_ID` |
| What evidence explains a Goal? | `meld runtime trace goal:GOAL_ID` |
| Why is Task completion waiting? | `meld runtime why task-completion` |
| What is arriving on the Event spine? | `meld event tail --follow` |
| How do I replace the running instance cleanly? | `meld runtime restart` |

An eligible product can accept an explicit reconciliation request:

```sh
meld agent request AGENT_ID --request-key review-1
meld agent request-status review-1 AGENT_ID
```

Repeating a key returns the same request. Acceptance records a request for judgment; it does not itself create a Goal, authorize a Task or prove completion. Startup rejects these requests because its admission lifecycle initiates its work. Docs supports them.

Use an exact instance identity with `runtime stop --instance INSTANCE_ID` when a script must fence its target. Retained shutdown records remain readable through `runtime shutdown --instance INSTANCE_ID`. Reports and Events expose their own continuation positions and retention gaps.

The local control surface uses an unauthenticated loopback listener. It shares readiness and observation with operational control. See the [command reference](design/plan/architecture_overhauls/world_model_knowledge_traversal/m3_command_reference.md) for trace subjects, wait explanations and full command semantics.

## How the flywheel closes

Meld separates a proposed action, permission to perform it, the result of execution and evidence that the intention now holds. Each has a different owner.

1. **Observe.** Sensory and product owners capture external state and publish owner-defined observations through Events.
2. **Understand.** The world model admits evidence, settles Beliefs and makes relevant knowledge available through Graph, Traversal and Curation.
3. **Judge and plan.** Agent assesses the maintained intention. Strategy constructs a causal Plan where a mismatch calls for work. Agent owns authorization.
4. **Act.** Execution admits complete authorized Tasks, coordinates them through one Task Network and invokes bound Capabilities. Plans may also contain epistemic work owned by Curation.
5. **Confirm.** Product owners observe the result. Returned evidence can revise Beliefs and support an independent Agent Goal judgment.

A successful write is an operational result. A correct README is a semantic judgment about the written document and current evidence. Keeping those separate lets Meld explain incomplete work and continue from durable history without treating a successful subprocess as proof of the desired outcome.

An already-correct state can satisfy an intention without a repair Task. A changed observation can invalidate an earlier basis and lead to further work. The runtime remains responsible for lifecycle, readiness and drain while the domains retain these decisions.

## Intelligence that lasts

Models contribute interpretation and authored proposals where the product needs semantic reasoning. Meld retains exact policy identities, observations, judgments and the evidence relationships used by the native loop. In the Docs owner, semantic reports also retain provider execution provenance, including request and response identities and reported model information.

That creates an opportunity to amortize reasoning. Existing Docs source judgments can be reused when the captured source and policy are unchanged, including when only the README changes. Ordinary computation checks identities, validates citations and advances the durable runtime. A changed basis calls for fresh assessment under the applicable contract.

The extent of that reuse matters. Whole-capture invalidation can still cause expensive reassessment. Reliable maintenance by a smaller model, economical invalidation across repository changes and preservation of editorial quality are product questions to demonstrate. Stored judgment preserves an answer and its basis; it does not make an incorrect answer true or give a CPU an independent understanding of prose.

README PDS will test this ambition on Meld itself. Begin with a strong document and an explicit account of its audience, structure, evidence and voice. Maintain that work through change, preserving what remains good. The intended voice is precise and welcoming, with room for the quiet pleasure of a system doing what its user meant.

## Architecture and ownership

The [cognitive architecture](design/cognitive_architecture/README.md) describes the intended domain contracts. Current source is organized around these responsibilities:

| Area | Responsibility |
| --- | --- |
| [Meld language](crates/meld-lang/) | Shared values such as Goals, Tasks, Capabilities, predicates and compositions. Domain owners define their meaning and validity. |
| [Events](crates/meld-events/) | Durable records, append and replay contracts. |
| [World model](crates/meld-world-model/) | Evidence, Belief, Graph, Traversal, Curation, Strategy and Agent reconciliation. |
| [Execution](crates/meld-execution/) | Authorized Task intake, Task Network coordination and executable work. |
| [Product theory](src/theory/) | Exact package installation, product compilation receipts and activation material. |
| [Runtime](src/runtime/) | Owner composition, lifecycle, operational control and inspection. |
| [External owners](owners/) | Product-specific observations, semantic judgments and capability implementations. |

`meld-lang` supplies shared vocabulary. It does not need to encode every concept a product can care about. Installed Docs theory already carries natural-language instructions and structured response contracts, interpreted by its semantic owner.

PDS packages are immutable product material. Assignments bind exact package revisions to an Agent; activation realizes a particular generation. Runtime observations and judgments evolve separately. This keeps a product's selected theory distinct from the history of what it has learned and done.

The [runtime invariants](governance/runtime_invariants.md) require one canonical authority for each semantic responsibility. Adapters may expose owner contracts; they must not become a second planner, writer or source of success evidence.

## Configure a workspace product

Docs and Dependency Security use separately selected owner executables. Build the workspace binaries with `cargo build --workspace --bins`, then follow the [external owner setup](owners/README.md) and the relevant product guide. Owner selection binds an executable by its exact BLAKE3 digest alongside the declared physical grants.

Product declarations select the target, principal, Agent, theory and required bindings. Docs also needs a configured provider that supports its JSON Schema response contracts. An installed policy determines observation scope, judgment instructions, acceptance rules and permitted repair behavior.

Keep runtime state outside the target workspace:

```toml
[system.storage]
product_root = "/absolute/path/outside/workspace/meld-product"
```

Meld uses platform configuration, data and state directories, including XDG locations on Linux. `--config` selects an explicit configuration. Logging is enabled by default; `--log-file PATH` selects a destination and `--quiet` disables logging. Targeted edits to workspace files are product effects; stores, leases and runtime logs belong outside that workspace.

A revised package does not silently rewrite an existing Agent's genesis. Follow the product guide for a separate assignment when changing theory. Preserve prior records so historical results remain attributable to the policy that produced them.

## Existing context tooling

Meld also retains filesystem and stored-context commands, including `scan`, `workspace validate`, `context get` and `context generate`. The filesystem tooling uses Merkle identities; stored context frames preserve content-addressed history.

Legacy Reader and Writer configuration lives under `meld profile`. Those profiles are distinct from native Agents. Legacy Workflow execution is retired, and old profile output does not establish native Goal satisfaction. Use the product guides for executable stewardship and command help for the retained context tools.

## Develop through the product

The delivery loop is to exercise a compiled command journey, inspect native evidence, repair the smallest observed gap and run the journey again. Unit and integration tests explain and protect behavior; the external journey establishes that the composed product is usable.

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
```

The sibling [Meld Eval harness](../meld-eval/README.md#command-journeys) accepts an explicit compiled binary, isolates its environment and captures public commands without opening domain stores or building implicitly:

```sh
cd ../meld-eval
PYTHONPATH=src python -m meld_eval.cli first-use \
  --meld-bin /absolute/path/to/meld --output /tmp/meld-eval
```

First-use, managed-control and deterministic Docs journeys have local evidence in the [delivery record](design/plan/architecture_overhauls/world_model_knowledge_traversal/m3_01_installed_startup.md#native-command-control-closeout). A deterministic provider proves the exercised transport and runtime behavior; real-model reasoning and writing need their own evaluation.

The [Meld 3 candidate program](design/plan/architecture_overhauls/world_model_knowledge_traversal/meld_3_candidate_program.md) records acceptance and remaining work. Major gaps in the core reconciliation contracts trigger reassessment of the workstream. Local builds and checks are routine; release publication is a separately authorized action.

## License

MIT OR Apache-2.0, as declared in [Cargo.toml](Cargo.toml).
