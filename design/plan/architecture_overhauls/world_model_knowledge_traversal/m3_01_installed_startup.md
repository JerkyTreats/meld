# M3-01: first Startup journey

Date: 2026-09-10. Source reference: `f2af45c8` on `feat/meld-3-candidate`.
Status: active delivery following design commit `06fe52fc`. Native first-use Startup verified through the external harness; managed control remains open. Local builds and checks are routine; publication and release actions require explicit authorization.

The [program](meld_3_candidate_program.md) owns the outcome, iteration method, authority and architectural circuit breaker. These notes describe the next useful experiment, not a complete implementation specification.

## Journey

A user initializes bundled Startup, starts the native runtime, observes nonce publication and independent Goal judgment, exercises its request boundary, stops the runtime and inspects retained evidence. A subsequent start must expose the new activation rather than reuse historical success. Startup requires neither a provider nor a manufactured workspace.

The starting command hypothesis is:

```sh
meld
meld init
meld runtime start
meld runtime status
meld runtime follow
meld runtime startup
meld runtime stop
```

Startup is admission-epoch scoped: lifecycle initiates its work, and the existing native Agent contract rejects explicit reconciliation requests. The harness checks that supported rejection and its explanation. Deduplication of accepted requests belongs to a product whose native contract supports them.

Exercise the commands through the [external harness](m3_runtime_harness.md) and direct interactive use. Improve command behavior and harness capture as gaps appear. The [command reference](m3_command_reference.md) suggests a consistent vocabulary; actual runtime findings determine the necessary surface.

Begin with whatever part of this journey the identifiable compiled artifact supports. If bootstrap or a command is missing, make the smallest enabling change through existing owners and run the journey again. Do not implement every supporting proposal before the first feedback.

## Behavior that makes the proof useful

Help explains the next action without creating product state. Init prepares a usable default through the existing config, theory installer and native genesis path. Bundle the Startup bytes and preset needed to remove manual copying and repository paths from first use. Preserve explicit configured products and report preparation failures with an actionable next step. Repeating preparation must preserve native identities rather than manufacture another Agent or success.

Start uses the existing supervisor and returns a truthful readiness result. Managed operation permits independent commands; foreground operation uses the same runtime when a terminal owns its lifetime. The launcher checks the shared public status surface. The local loopback interface needs no authentication, separate private launch channel or handshake. Product/instance identity and exclusive store ownership keep selection and effects correct.

Status and follow expose enough native evidence to understand active work, waiting, failures and completion. Goal judgment remains independent of command completion, nonce publication and outstanding operational obligations. Show current versus retained evidence and report unavailable observation honestly. An unreachable listener with occupied stores is not proof that the runtime stopped.

Stop asks the native supervisor to drain. A disconnected observer does not stop the runtime. An interrupted foreground runtime requests drain. Explain whether stop completed, is still pending or has an unknown outcome. Use existing lifecycle records and request identities to make retries and lost responses understandable; settle the concrete wire details while exercising these cases. Never silently retarget a request to a successor instance. Restart can compose completed stop and start without a new transaction authority.

Reads through the live owner do not create competing store writers. Offline inspection does not initialize or reconcile. Adapter and observation changes expose existing semantics; a major gap in those semantics invokes the program circuit breaker.

## Simple setup and migration

Use existing external storage/configuration conventions where they suffice. Make the fresh default usable without requiring a new installation metadata system or a storage-layout overhaul. Record binary and package identities needed to explain the proof.

Preserve older data. Reuse it only where compatibility is demonstrated; otherwise report the incompatibility and provide a supported separate-configuration setup for fresh Startup. That path must be achievable through documented commands rather than manual TOML construction. Do not move, delete or reinterpret old histories as part of proving first use.

Automated archival, migration journals and semantic conversion are not M3-01 prerequisites. If a concrete existing-user journey later needs a simple conversion, bound it to the identified format and preserve the source. Design recovery before introducing actual data movement. The program's M3-05 backlog retains that user-transition concern.

## Feedback and completion

Run init, start, observation, reconciliation, stop and reopen against an explicit artifact outside the Meld checkout. Capture command results, nonce and independent Goal evidence, current activation and retained history. As commands become usable, exercise repetition, observer reconnect, lost replies, concurrent start and interrupted drain to expose concrete correctness gaps. A deadline records uncertainty, not invented success or failure.

A missing public signal calls for a bounded owner read or clearer result. A harness capture problem calls for a harness correction. A bug implementing an existing native contract calls for a focused repair and rerun. If progress requires inventing flywheel components or changing fundamental semantic ownership, stop and reassess under the [architectural circuit breaker](meld_3_candidate_program.md#architectural-circuit-breaker).

When replacing a command or runtime entrypoint, identify the actual caller path, route it through the existing canonical owner and retire the superseded authority in the completed change. Do this for the path being changed, not every speculative future command.

Keep the iteration record short: artifact and command, expected behavior, observation, correction, rerun result and next question. No mandatory receipt files, harness framework or complete future schema is needed. This journey is proved when ordinary commands and external replay establish the native Startup loop and its lifecycle honestly. Subsequent product journeys extend that evidence; Startup alone does not qualify all advertised products.

## Iteration record

Design baseline: `06fe52fc`. The first implementation checkpoint is recorded on `feat/meld-3-candidate`. Local Cargo builds/checks are routine; the earlier pause was a misinterpretation of the user's release-publication guard and is superseded.

The first external probe found bare help exiting two and version unsupported. Both now use Clap before logging or assembly. Native init then exposed the next gap: old Reader/Writer setup reported success while public Startup inspection had no compiled product or native Agent. Init now authors fresh configuration, materializes the bundled package and invokes the existing native preparation pipeline. It preserves configured products, refuses incompatible older configuration with an actionable separate setup, and holds a preparation lock through genesis.

The superseded profile/prompt bootstrap APIs, formatter and exclusive tests are removed. `init --force` and `init --list` are retired. Existing `world init` remains an advanced adapter into the same native pipeline, exercised by configured-product regressions; it is not another preparation authority. The external legacy fixture runner now invokes native init with its explicitly configured package.

Runtime exploration proved native Startup and corrected an assumption in the proposed journey: admission-epoch work is initiated by lifecycle, so this product rejects explicit reconciliation requests. The first live test exposed that the CLI hid the rejection behind generic retry advice. It now reports the HTTP status and native reason, while transport failures remain unknown outcomes. The harness verifies the rejection rather than changing Agent semantics to force acceptance. Accepted-request deduplication remains a question for an eligible product.

Final standalone artifact SHA-256: `bc778e286079b21e1554b17d3cbe898c5efe44b931ec96644f826b311daf5892`. The copied [executable](../../../../../.local/state/meld-eval/m3/candidates/bc778e286079b21e1554b17d3cbe898c5efe44b931ec96644f826b311daf5892/meld) and [source-state hashes](../../../../../.local/state/meld-eval/m3/candidates/bc778e286079b21e1554b17d3cbe898c5efe44b931ec96644f826b311daf5892/source-state.json) identify the locally built candidate without replacing the user's installed Meld.

The [final report](../../../../../.local/state/meld-eval/m3/first-use-p62erqig/report.json) passes all 22 checks. The [command transcript](../../../../../.local/state/meld-eval/m3/first-use-p62erqig/commands.jsonl) and incremental runtime output establish help/version, repeated preparation with identical receipts, no initialization activation, live nonce/confirmation/Goal evidence, truthful request rejection, Ctrl-C drain, retained history, and distinct generations, epochs and nonces on restart. Execution used a binary and working directory outside the checkout, with isolated HOME/XDG roots and no package source directory supplied.

Validation: four native-init integration tests cover fresh/repeated preparation, old-config preservation with explicit fresh setup, preparation-lock refusal/resume and package corruption refusal. Fourteen runtime CLI regressions and fifteen progress/observability regressions pass. Four existing external harness tests pass. Strict library/binary Clippy, Rust formatting and diff whitespace checks pass. Local logs are retained beside the final report.

Logical review: the native owners retain preparation and runtime meaning; the old bootstrap authority is removed and callers migrated. Style review: preparation behavior lives under init, with thin adapters and no new domain store or semantic component. Composed local proof: the bounded first-use journey passes; no architectural breaker has tripped. Managed start/stop/follow, broader command cutover, accepted requests for eligible products, and advertised product/release qualification remain open.

The local CI workflow now requires manual `publish_release` opt-in on `master` before its release job can run. YAML parsing confirms the input defaults to false. This proposed guard is not active remotely until merged. No push, release dispatch, tag or publication occurred. The user authorized implementation commits at natural checkpoints and continued delivery until completion or the architectural breaker. The next runtime question is how to expose managed control through the existing supervisor without changing its semantics.
