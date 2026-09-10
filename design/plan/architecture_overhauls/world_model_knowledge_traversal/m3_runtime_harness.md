# M3 external harness: fast product feedback

Date: 2026-09-10. Status: working approach; implementation not started.

The [program](meld_3_candidate_program.md) owns the outcome, authority and architectural circuit breaker. The harness exists to validate and improve ordinary commands against the compiled flywheel runtime. Its shape follows what exploration needs.

## Small starting point

Use a small external runner that invokes an explicit Meld binary with argument arrays, captures structured output and stderr, and repeats a useful command journey. Direct interactive use of Meld commands is part of exploration. Do not require an SDK, another operator command language, eight separately implemented components, a prescribed evidence bundle or a dedicated reader.

Reuse `meld-eval` where useful. Repository placement and internal organization are routine implementation choices within authorized workspace scope, not prerequisites for resolving the full runtime design. Keep acceptance execution independent of the Meld checkout and private stores. Document the runnable entrypoint once it exists so another agent can reproduce the journey.

The runner accepts a compiled artifact and an output location. It can create isolated configuration/data roots for its own experiment and invoke public init. It does not build implicitly, author domain state or require a repository theory directory. Keep enough artifact and package identity to know what was tested.

## Iterate through the runtime

1. Invoke a public command or sequence against the selected runtime.
2. Observe the native result and identify what is wrong, missing or hard to understand.
3. Improve the command, its existing owner implementation or harness capture as the evidence requires.
4. Rerun the same case and retain the changed result.
5. Use that result to choose the next question.

The first sequence is the [Startup journey](m3_01_installed_startup.md). Grow scenarios from encountered behavior, including useful rejection and recovery paths. A static test runner alone is insufficient: agents must also be able to explore and direct the runtime interactively using the same public commands.

Source inspection and focused tests support diagnosis. Never bypass a difficult command by opening runtime stores, manufacturing Events, stepping actors or duplicating Goal judgment in the harness. An operation missing from the public surface can be exposed through its existing owner. An operation missing because the native architecture cannot perform it invokes the circuit breaker.

## Trustworthy feedback

Capture output incrementally for long-running commands. Keep stdout results distinct from stderr diagnostics. Preserve target, instance, activation and request identities when supplied. Those references prevent an old success or a successor runtime from being mistaken for the requested result.

Use native state to distinguish accepted work from completion, Goal satisfaction from operational progress, and a stopped runtime from unavailable observation. If a reply is lost, inspect the original request or state through public commands before retrying. Repeat only under the native retry contract. A timeout remains pending, unknown or inconclusive until evidence establishes more.

Disconnecting follow stops observation. It does not request shutdown. An automated scenario may clean up its own selected runtime through native controls; attachment alone does not authorize stopping another runtime. Record unresolved cleanup rather than concealing it.

Save enough information for another agent to understand what was requested, what was observed, which artifact produced it and how to repeat it. A transcript and short result may suffice. Add fields, assertions or helpers when a concrete gap makes them useful. If capture fails, say so. Exact filenames, bundle schemas and a separate reader are not acceptance gates.

Commands are the first transport. The runtime shares its unauthenticated local control/status surface with other consumers. Direct harness HTTP support and parity with future graphical clients are not required to prove the command journey.

## Circuit breaker

Follow the [program stop rule](meld_3_candidate_program.md#architectural-circuit-breaker) when a major native architecture gap appears. Preserve the command, artifact, last trustworthy observation and missing native transition. Stop product changes and dependent qualification, then report the finding for workstream reassessment. Do not grow a scheduler, semantic writer or repair framework inside the harness to make the scenario pass.

No external repository files, source behavior, builds or runtime state changed while revising these notes. Actual runtime findings will shape the harness under implementation and build authority.
