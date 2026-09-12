# OTel attribution of reconciliation cost

Slice `runtime-otel-v1`. Lifecycle: closed. Readiness: accepted for attribution. Baselines: Meld `fd2d11af`, Eval `ef75eb5`, README `c497cca`; all verified on origin. The user authorizes OTel instrumentation, harness measurement, and checkpoint commit and push.

The preceding capture locates expensive reconciliation and status projection but cannot attribute their internals. This first-slice instrumentation must produce standard OTLP spans through real runtime commands, connect host and external owner work, and explain slow calls with named child operations. The harness retains export evidence and existing native timing reports on a disposable product. No source edit or model call is required for the primary measurement.

The domain sweep follows current `src` domains and the world-model crate. Telemetry and logging own export and subscriber integration. Runtime owns bounded passes, lifecycle projection and owner transport. World Model supplies observational spans around queries, authority and reconciliation. Eval owns capture and analysis. CLI is an adapter. README and Codebase Semantics reuse the shared owner server and rebuild to carry context. Agent, capability, context, execution, events, theory, workspace, storage, provider, session, task, tree, workflow, branches, controls, views, heads, metadata, nonce, ignore, prompt context, code change and Merkle traversal retain semantic behavior. Their work is observed only where already on the measured call path.

| Responsibility | Mode and current route | Change and proof |
| --- | --- | --- |
| Export | extend Telemetry and logging subscriber | Opt-in OTel SDK and OTLP export, independent log filtering, graceful flush; disabled and failed export cannot gate domain work |
| Runtime timing | extend existing pass and actor calls | Nested spans around actors, status projection and wake probes; compare with retained command timings |
| Owner transport | extend existing command and callback envelopes | Optional trace context only; unchanged grants and products; propagation verified through public owner execution |
| World Model reads | extend existing functions | Instrument current authority and query paths without adding caches, alternate queries or semantic writers |
| Harness | extend Eval | Local OTLP capture, span hierarchy and exclusive-time analysis, dropped or missing coverage visible |
| External packages | retain domain theory and owner behavior | Rebuild linked server and update explicit binary selections in the disposable measurement product |

Native Events and Graph identity remain authoritative. Trace context is diagnostic metadata and does not participate in authored knowledge, content identities, deduplication or planning. Per-operation traces are bounded; durable Event-to-work span links are deferred because this measurement follows synchronous pass and owner execution. No permanent collector service, observability platform deployment, release, new domain theory or runtime optimization is included.

The policy obligations are domain-first placement, one canonical runtime authority, XDG state isolation, and preservation of existing product qualification. The user authorization covers the OTel dependencies, optional wire metadata and harness-local receiver. Cross-process integration and retained native report compatibility form the obligation floor. Existing timings remain independent evidence. Stop and reassess if instrumentation requires a new semantic authority or changes domain outcomes.

Acceptance requires standard exported spans, real host-owner-parent relationships, attributable slow operations, clean native stop, unchanged target README and no provider requests. Check export overhead against an untraced run using the same candidate. Record workload differences and sampling limits rather than treating a small comparison as a performance guarantee. Logical review, style review and final acceptance will be recorded here against the measured candidate. Commit effect: If applied, this commit exposes the internal cost of the existing reconciliation loop through standard traces and retained harness evidence. Further performance repairs require the next selected slice.

## Acceptance evidence

The [Eval closeout](../../../../../meld-eval/evidence/runtime-otel-v1/CLOSEOUT.md) retains the native profiles, OTLP capture, command receipts and measured executable hashes. Two traced passes took 124.22 seconds, including 107.22 seconds of graph publication reads. Forty-three scans processed 3.59 GB of serialized publication values, including repeated history. Decoding and validation account for 106.86 seconds. Status projection drives 22 of those scans through lifecycle wake probes. This is concrete repeated processing in the canonical Graph read path, not evidence of a model or semantic-conversion bottleneck.

All 28 actor calls, two runtime passes and 74 runtime owner requests have corresponding exported spans. The capture contains 7,737 spans, 137 cross-process edges and no missing parents. Thirty-four short-lived command-process requests lack owner child exports; the harness exposes that limitation separately. The native runtime coverage used for attribution is complete at the measured boundaries. Full internal span delivery is not asserted.

Both tracing-off and tracing-on cases stopped cleanly with zero provider requests and an unchanged target README. Their work and drain alignment differ, so the comparison does not prove an overhead bound. The initial activation mismatch was retained and resolved through `world init`, with unchanged theory and genesis. No direct knowledge authorship or semantic authority was added.

Logical review: accepted. Existing domain calls and grants remain authoritative; trace context affects no decisions or products. Independent native timings agree with span boundaries. Optional owner envelope fields are source-breaking for exhaustive construction and require rebuilt old strict readers when tracing is enabled. Compatibility is explicit, with no duplicate authority or negotiation layer.

Style review: accepted. Instrumentation lives in existing domains, logging retains one subscriber authority, adapters delegate, no `mod.rs` was added, and docs distinguish wall-time attribution from CPU measurement. Comments explain context and lifecycle intent. The compressed evidence retains the complete capture without adding a permanent collector.

Verification: builds passed for Meld and both owners; 10 owner, 48 supervisor, 7 logging, 7 Graph and 27 Eval tests passed. Gate judgment: accept this observability slice and retain the existing semantic qualification boundary. Further performance work should target repeated publication materialization while preserving validated, frozen Graph cuts. No performance repair is included here. No architectural circuit breaker was reached.
