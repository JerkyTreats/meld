# Meld native command reference

This reference describes the command surface exercised by the delivery loop. The [program](meld_3_candidate_program.md) owns the outcome and circuit breaker. Further resource proposals in the [matrix](m3_control_matrix.md) remain exploration prompts, not a mandate to build unused commands.

## First use and selection

`meld`, `meld -h` and command-family help do not initialize state. `meld --version` identifies the executable. `meld init` prepares bundled Startup without a workspace, provider or repository checkout. Repeating init preserves native identities. Incompatible older configuration is preserved and the error explains how to select a separate configuration.

Use `--config PATH`, `--workspace PATH` and `--assignment NAME` when selection needs to be explicit. These selectors work before or after subcommands. Native inspection commands support `--json`; existing commands also retain their demonstrated `--format json` spelling. Help lists diagnostic overrides where available.

An explicitly configured product uses `meld init --package PATH`. This invokes the same native preparation pipeline as the advanced `world init` adapter. Initialization does not activate work.

## Runtime

| Command | Meaning |
| --- | --- |
| `meld runtime start` | Launch the selected native supervisor as a detached process and verify readiness through its public surface. A repeated start returns the existing matching live instance. |
| `meld runtime start --foreground` | Run the same supervisor attached to the terminal. Ctrl-C requests native drain. The existing `runtime run` spelling remains an adapter for demonstrated scripts and hosts. |
| `meld runtime list` | Read known instances of the selected assignment, distinguishing the verified live identity from retained history. This is not a machine-wide process inventory. |
| `meld runtime status` | Read instance and participant health. `--runtime-id ID` narrows participant rows. Live status reads current native records through the owner; offline status describes retained records. |
| `meld runtime stop` | Signal the exact discovered instance and wait for its native shutdown record and released store ownership. `--instance ID` fences an explicit target. |
| `meld runtime shutdown` | Read the native shutdown result without submitting a stop. `--instance ID` selects retained history. |
| `meld runtime restart` | Finish native stop before starting a successor. An unresolved or failed stop prevents restart. |
| `meld runtime follow` | Follow native actor reports without owning runtime lifetime. `--after N` resumes a report position. Ctrl-C ends observation. |
| `meld runtime actions` | Read retained actor reports with `--limit N` and `--after N`. Results report missing retained history and continuation. |
| `meld runtime startup [AGENT]` | Inspect native nonce, confirmation and Goal evidence. `--generation ID`, `--epoch ID` and `--nonce ID` select retained evidence. |
| `meld runtime trace SUBJECT` | Walk existing cross-domain citations with `--limit N`. Unresolved references and traversal bounds remain visible. |
| `meld runtime why KIND [SUBJECT]` | Explain native waits for `task-completion`, `task-admission`, `belief-revision` or `evidence`. |

Trace subjects are `event:SEQ`, `publication:SEQ`, `evidence:ID`, `belief:ID`, `decision:ID`, `goal:ID`, or `task NETWORK TASK`. Identity suffixes remain opaque, including additional colons.

Stop acknowledgement means received, not completed. The existing native shutdown identity supplies repeat-safe exact-instance stop and retained lookup; no second control-request store is needed. A deadline reports pending or unknown outcome rather than success. Readiness timeout reports the unresolved instance and PID for inspection. No timeout authorizes force killing or a competing successor.

The loopback control surface is unauthenticated. Public status supplies readiness, identity verification and observation; there is no private launch channel or credential exchange. Operational control routes are `/v1/runtime/status`, `/v1/runtime/instances`, `/v1/runtime/stop` and `/v1/runtime/shutdown`. Report paging uses `/v1/reports/actions`.

## Native Agents

| Command | Meaning |
| --- | --- |
| `meld agent list` | Page native genesis identities with `--limit N` and `--after ID`. |
| `meld agent show [AGENT]` | Read the installed intention, Goals, current admitted plans, authorizations and separate native judgments. |
| `meld agent request [AGENT]` | Request reconciliation of an eligible installed intention. `--request-key KEY` repeats the same request; a key is generated when omitted. |
| `meld agent request-status KEY [AGENT]` | Read acceptance and completion without creating or repeating a request. |

An omitted Agent resolves only when unambiguous. Startup's admission-epoch intention rejects explicit requests; its native lifecycle produces observations. Docs supports explicit reconciliation, including repeated requests and retained completion reads.

`agent list` and `agent show` now mean native Agents. The former profile command family moved intact to `meld profile`. Profile editing never creates native genesis or supplies Goal evidence. Native reads share `/v1/agents/query`; request intake shares `/v1/agents/reconciliation_requests` with the demonstrated `runtime request` adapter.

## Events

`event status`, `event tail`, `event flow`, `event trace` and `event session` read the live owner when it holds the stores. Existing renderers and native Event contracts remain shared with offline reads.

`event tail --follow` observes independently. `--after SEQ` resumes the native cursor and `--limit N` bounds a page. Retention gaps remain errors or explicit coverage, never inferred continuity. Event traffic and runtime action progress are separate observations.

## Delivery boundary

The completed native control journey covers preparation, launch, inspection, observation, request intake, native completion, drain and restart. Startup and deterministic Docs replay provide product evidence. Security, Code Change, real-model quality, full legacy CLI modernization and release qualification remain separate work. Existing provider and workspace commands are not newly qualified by these journeys. No new semantic owner, scheduler or planner was introduced to fill a command inventory.
