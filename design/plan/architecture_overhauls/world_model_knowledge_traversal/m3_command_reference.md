# Meld command reference

Working command vocabulary, not a frozen delivery checklist or an implemented interface. The [program](meld_3_candidate_program.md) selects work through fast harness feedback. Keep and refine spellings that help operate existing native capabilities; remove or defer proposals that do not serve an exercised journey. A command proposal does not authorize inventing its underlying semantic component.

The interface serves excellent command-line ergonomics that prove the flywheel runtime. Its local control surface is raw but complete: commands and external consumers share public operations, status and observation. Start verifies readiness through that same surface. There is no separate private launch channel or control credential requirement; loopback control is unauthenticated. Exact instance targeting remains part of command correctness.

## Invocation and common options

`meld` and `meld -h` show the same concise human help. Invoking a command family without a subcommand shows that family's help. Help never initializes state.

| Option | Meaning |
| --- | --- |
| `-h, --help` | Show usage and available subcommands. |
| `--version` | Show the executable version. |
| `--json` | Emit machine-readable output for commands that also have a human view. Streaming commands emit newline-delimited JSON. |
| `--config PATH` | Use an explicit configuration. |
| `--assignment NAME` | Select an assignment when automatic selection is ambiguous. |
| `--workspace PATH` | Select a workspace for workspace-scoped operations. |
| `-v, --verbose` | Include diagnostic logging on stderr. |
| `-q, --quiet` | Suppress routine progress and informational logging; preserve requested result output and failures. |

Common options are accepted before or after the subcommand. Normal operation needs none. Default selection uses the configured active target, or the sole eligible target. Ambiguity reports available choices.

There is no `--format` option. Machine-only interfaces emit JSON without a format selector. Human commands use concise text by default and support the shared `--json` switch.

Configuration supplies useful defaults. Optional command overrides for diagnostics, budgets or operational settings are appropriate when they help an actual journey. Keep normal invocation simple without forcing configuration edits for temporary choices.

Collection-list commands share `--limit N` and `--after CURSOR` for bounded paging. Defaults return a useful first page, with continuation and coverage in the result. These shared collection options are not repeated as unique options below.

## Initialization

| Command | Behavior | Unique options |
| --- | --- | --- |
| `meld init` | Initialize or resume native preparation. A fresh installation selects bundled Startup. Existing incompatible state is reported without modification. | `--package PATH` selects package input for an explicitly configured product. |

For incompatible existing data, init reports the issue and explains a supported separate-configuration setup for fresh Startup while preserving old state. A migration command is deferred until a concrete user transition needs a bounded conversion; automated archival is not a first-use requirement.

No stage numbers, genesis-stage selectors or overwrite flags are public initialization options. Package preparation reports named failures and resumes completed work automatically.

## Runtime

Runtime commands other than start/list accept `--instance ID` when an exact instance must be selected within the configured target. Normally selection is automatic. Historical instances support reads; they cannot be controlled as live processes.

| Command | Behavior | Unique options |
| --- | --- | --- |
| `meld runtime list` | List known supervisor instances within the selected configuration's declared product roots. Report unstarted targets, inaccessible roots and unverified liveness separately. An explicit assignment narrows discovery. | None. |
| `meld runtime start` | Start the selected native runtime as a managed process and return after verified readiness. Report an already-running matching instance without launching another. | `--foreground` keeps the same runtime attached for terminals and process hosts. |
| `meld runtime stop` | Request orderly shutdown of the selected instance and wait for its result within the configured control deadline. | `--request-key KEY` repeats an exact control request; generated when omitted. |
| `meld runtime restart` | Complete orderly stop before starting a successor. A failed or unresolved stop prevents another start. | None. |
| `meld runtime status` | Show selected runtime, admission, observer availability, current work, outstanding obligations and evidence freshness. | None. |
| `meld runtime follow` | Follow runtime progress. Disconnecting stops observation without stopping the runtime. | `--after CURSOR` resumes an existing observation position; unavailable history is reported. |
| `meld runtime startup [AGENT]` | Inspect Startup nonce, confirmation and Goal evidence. This does not fire a nonce. | `--generation ID`, `--epoch ID`, `--nonce ID` select retained evidence. |
| `meld runtime participant list` | List participant runtime IDs and their readiness, lease and health within the selected instance. | None. |
| `meld runtime participant show ID` | Inspect one participant's readiness, waiting state and retained lifecycle evidence. | None. |
| `meld runtime action list` | Inspect retained actor operation reports within the selected instance. | `--actor ID` filters an actor. |
| `meld runtime request show KEY` | Read a lifecycle control request's recorded acceptance, drain outcome and outstanding obligations. Never submits or repeats the request. | None. |
| `meld runtime trace SUBJECT` | Follow the existing cross-domain causal thread. | `--limit N` bounds traversal. |
| `meld runtime why KIND [SUBJECT]` | Explain an absent Task completion, Task admission, Belief revision or evidence record from native waiting declarations. | None. |

Trace subjects use `event:SEQ`, `publication:SEQ`, `evidence:ID`, `belief:ID`, `decision:ID`, `goal:ID`, or `task NETWORK TASK`. KIND is `task-completion`, `task-admission`, `belief-revision`, or `evidence`. For colon-prefixed subjects, the complete suffix is the opaque identity; parsers do not split additional colons inside it. Task subjects take two separate identity arguments. Structured HTTP requests retain typed fields.

Start and stop target the selected product and exact live instance discovered at request time. A changed instance produces a conflict rather than controlling its successor accidentally. Deadline expiry reports a pending or unknown outcome and the request identity; it does not imply successful shutdown.

Runtime listing covers configured roots, not all processes on the machine or unregistered configurations. Instance IDs and participant IDs are different targets. Retained records and discovery files alone do not establish liveness.

## Agent

These commands address native Agents, not Reader/Writer configuration files.

| Command | Behavior | Unique options |
| --- | --- | --- |
| `meld agent list` | List native Agents in the selected product. | None. |
| `meld agent show [AGENT]` | Show an Agent's maintained intention, current state, Goal disposition and outstanding work. | None. |
| `meld agent request [AGENT]` | Request reconciliation of an installed intention. Does not invent an intention or guarantee another effect. | `--request-key KEY` repeats the same request; generated when omitted. |
| `meld agent request-status KEY [AGENT]` | Inspect acceptance and completion of an existing reconciliation request without creating it. | None. |

Omitted Agent identity resolves only when unambiguous. New Agent genesis occurs through product initialization; there is no profile-based `agent create`.

## World

| Command | Behavior | Unique options |
| --- | --- | --- |
| `meld world branch list` | Show registered graph branches, attachment and migration state. These are not Git branches. | None. |
| `meld world branch attach PATH` | Register the graph branch associated with an explicit workspace path after identity validation. Does not check out source or activate a product. | None. |
| `meld world branch discover` | Discover existing branch data and register recovered entries in the catalog. This command writes registration state. | None. |
| `meld world status` | Show Graph readiness across selected branches. | `--branch ID` selects an explicit branch and is repeatable. |
| `meld world owner list` | List publication owners visible in the selected product, distinguishing declared owners from owners with observed publications. | None. |
| `meld world owner show OWNER` | Show an owner's identity, publication state and available scopes. | None. |
| `meld world scope list OWNER` | List known publication scopes for an owner. | None. |
| `meld world scope show OWNER SCOPE` | Show scope identity, revision, coverage, exclusions and failures. | None. |
| `meld world object list OWNER SCOPE` | List published object references in an owner scope. | `--limit N` bounds the page; `--after CURSOR` continues it. |
| `meld world object show OWNER SCOPE OBJECT` | Show a published object, its provenance, revision and available hydration reference. OBJECT uses `DOMAIN::KIND::ID`. | None. |
| `meld world walk OWNER SCOPE OBJECT` | Traverse exact owner publications within the selected scope. OBJECT uses `DOMAIN::KIND::ID`. | `--branch ID`; `--direction incoming\|outgoing\|both`; repeatable `--relation TYPE`; `--depth N`; `--limit N`. |

Discovery proceeds from owner to scope to object without requiring users to invent identifiers. Listing and inspection report their observation boundary and completeness; an empty observed list is not proof that an owner has no objects. Published scopes are not assumed to form a filesystem tree or to be globally unique. Object inspection does not implicitly invoke an owner or fetch an unavailable payload. Walking is bounded and reports truncation and source positions. World commands do not perform initialization or fabricate absent knowledge.

SCOPE accepts the qualified selector returned by scope discovery. It preserves branch, perspective and temporal qualifiers when present. A bare scope name works only when unambiguous; otherwise the result supplies qualified choices.

## Event

| Command | Behavior | Unique options |
| --- | --- | --- |
| `meld event status` | Show ledger identity, watermark, retention, drops and consumer lag. | None. |
| `meld event tail` | Read recent ledger records. | `--follow` continues observation; `--after SEQ` resumes; `--limit N` bounds each page. |
| `meld event trace SUBJECT` | Inspect causal ledger references. SUBJECT is `record:SEQ`, `object:DOMAIN::KIND::ID`, or `stream:DOMAIN::STREAM`. | None. |
| `meld event session ID` | Inspect one command session's timeline. | None. |
| `meld event flow` | Summarize recent ledger traffic. | `--limit N` bounds the trailing event window. |

Event observation uses the running process when it owns the stores. Following never acquires a competing database writer. Event traffic and runtime progress are separate observations.

## Provider

| Command | Behavior | Unique options |
| --- | --- | --- |
| `meld provider list` | List configured providers. | None. |
| `meld provider show NAME` | Show configuration with credentials redacted. | None. |
| `meld provider status` | Report configuration readiness without making an inference request. | None. |
| `meld provider create [NAME]` | Interactively create provider configuration. | `--from FILE` supplies validated noninteractive configuration. |
| `meld provider edit NAME` | Edit provider configuration using the configured editor, then validate. | `--from FILE` supplies a replacement configuration. |
| `meld provider remove NAME` | Remove provider configuration after confirmation. | `--yes` confirms noninteractively. |
| `meld provider validate NAME` | Validate local configuration without network requests. | None. |
| `meld provider test NAME` | Test connectivity and model availability using configured defaults and deadline. Report unsupported checks explicitly. | `--model NAME` selects a model override for this test. |

Interactive commands refuse to prompt without a terminal and identify the noninteractive alternative. Credentials are never returned by show or status.

## Workspace

These commands address the selected physical source and its indexed snapshot. A product without a workspace binding reports that absence.

| Command | Behavior | Unique options |
| --- | --- | --- |
| `meld workspace status` | Show source binding, retained snapshot, last observation/publication and freshness boundary. Does not rescan to claim current filesystem freshness. | None. |
| `meld workspace scan` | Refresh the source index and report snapshot and publication outcomes separately. | `--rebuild` forces reconstruction. |
| `meld workspace validate` | Inspect current filesystem/index consistency and report the check's coverage. Does not claim whole-product integrity. | None. |
| `meld workspace ignore list` | Show effective inclusion rules and their origins. | None. |
| `meld workspace ignore add PATH` | Add an explicit Meld exclusion for subsequent observation. | `--dry-run` previews the change. |
| `meld workspace ignore remove PATH` | Remove an explicit Meld exclusion and report whether another rule still excludes the path. | `--dry-run` previews the change. |

Ignore changes do not modify source files, resurrect historical records or trigger a scan. Imported .gitignore rules remain owned by that file. Live mutations use the runtime's owning interface; they never acquire a competing database writer.

## Results

Read commands return immediately with a result or a typed error. Follow commands run until disconnected. Control commands return acceptance and completion state, with a request identity whenever work remains pending.

The starting exit-code proposal is zero for a successful completed command, one for an operational failure, two for invalid usage or configuration, and three when the operation remains pending or its outcome is unknown at the wait deadline. The result must say whether acceptance was actually observed. Follow interruption exits successfully without implying runtime shutdown. Errors go to stderr and use structured error output under `--json`. Refine these details through actual command and harness use rather than making a complete future wire schema a prerequisite.

The runtime's local HTTP surface uses the same existing domain behavior as the CLI. Target identities preserve command correctness. A second harness transport or parity proof for hypothetical graphical clients is not required for command qualification.
