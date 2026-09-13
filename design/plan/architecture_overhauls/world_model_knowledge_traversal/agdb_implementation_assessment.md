# agdb implementation assessment

This is the pre-integration assessment. The subsequent [native delivery](agdb_native_delivery.md) implements the bounded agdb cutover and records current evidence. Statements below about integration not having entered Meld describe the assessment checkpoint.

Status at assessment: agdb is the lead candidate for continued graph-engine investigation. Released 0.13.2 still has a reproduced interruption defect. A disposable corrected build passes the bounded screen; no engine change has entered Meld. This follows the [Cozo assessment](cozo_implementation_assessment.md) and the original [graph-crate experiment](graph_crate_experiment.md).

**The observed agdb failure has a small candidate correction in the engine, so the next investment should preserve the upstream route rather than add recovery machinery to Meld.** Recent maintenance makes that route more credible than it appeared for Cozo. It does not substitute for correctness evidence.

## Current evidence

The [latest release](https://github.com/agnesoft/agdb/releases/tag/v0.13.2) is July 28, 2026. The inspected [main tip](https://github.com/agnesoft/agdb/commit/170b956be92591cb8ce922be180bb93aff0151d7) is September 11, with recent database and server work. The existing comparison already used this latest published release, so upgrading the version alone does not address the failure. The relevant upstream source comparison is retained with the experiment.

Main contains [fsync support merged in August](https://github.com/agnesoft/agdb/pull/1916), with optional commit synchronization. Its outer transaction boundary and forward undo-record iteration remain unchanged in the inspected source. These facts support investigating a corrected upstream revision that includes both concerns. They do not establish main's behavior through execution; the captured runs use the release and its disposable diagnostic variant.

The [minimal harness probe](../../../../../meld-eval/evidence/agdb-implementation-assessment/README.md) removes Meld's publication mapping entirely. It changes two node values inside an agdb transaction, interrupts the process and reopens through agdb. The released engine exposes a partially applied transaction. Ordinary error rollback succeeds. Both mapped and direct-file backends reproduce the distinction, including a panic followed by process exit.

This explains why the earlier publication adapter could expose a new cut and revision guard while keeping the old current head. It was not sufficient to assume that wrapping those writes in the public transaction made their crash boundary atomic. Merely repairing the head on retry would hide one symptom while retaining the engine's partial-transaction behavior.

The diagnostic correction is three changed lines across the public transaction boundary and file recovery. It holds the engine's existing storage transaction open until normal completion and replays undo records in reverse order. It adds no Meld-owned log, recovery store or alternate Graph authority. It was applied only to a disposable copy of the published crate.

| Evidence | Released engine | Corrected disposable engine |
| --- | --- | --- |
| Minimal transaction cases across two backends | 4 of 12 pass | 12 of 12 pass |
| Unchanged publication screen | 68 of 72 pass in the original comparison | 72 of 72 pass in two new runs |
| Corrected-source upstream tests and doctests | Not rerun for comparison | 1,095 pass |

The second corrected publication run occurs after builds finish. Its fixed tiny query remains approximately 0.036–0.039 ms across history growth, and the captured repository neighborhood takes approximately 2.7 ms. This supports continuing the experiment without an observed query-latency regression. It is not an end-to-end runtime or power-loss qualification.

## Fit and remaining boundaries

agdb naturally preserves directed occurrence edges, including parallel edges with different evidence. Rust query objects avoid introducing another public query language. The probe's exact record preservation, indexed revision selection and native graph search remain useful evidence. Its full payload copies still leave historical storage reuse unresolved.

The engine does not take ownership of Meld's Event admission, source routes, completeness, exact visibility, revision cuts or Planner causality. Native integration would replace Graph storage and history reconstruction while retaining those semantic contracts. No WAD or meld-lang change follows from this assessment.

Full traversal remains the next significant semantic question. Meld returns ordered occurrence-qualified paths and an explicit frontier under independent bounds. agdb's generic BFS result does not by itself prove that behavior. Some Meld-owned path and receipt assembly may properly remain; the concern is whether integration can use indexed engine access without rebuilding the whole graph or duplicating a general query engine. Measure what is actually delegated and retired.

Concurrent read behavior also remains open. The embedded database uses mutable access for writes, and its documented shared-instance pattern uses an external read/write lock. The current runtime's query catch-up coupling independently adds waiting. A frozen cut's semantic stability does not guarantee it can read during a publication. Native probes must measure that contention instead of inferring it from the fast sequential query.

Recovery qualification is narrower than a durability guarantee. The diagnostic tests deliberate process exits and normal commit boundaries. The released file backend explicitly relies on OS flushing rather than issuing data synchronization calls; main's newer synchronization controls are not tested here. Power loss, disk errors, interrupted recovery, caught-panic reuse and retention-safe rebuild remain unqualified. The small diagnostic patch does not settle those policies and must not be treated as production-ready source.

The default mapped backend reads the file into a memory buffer. Direct-file storage is available and shares the reproduced recovery issue; memory and query costs need comparison at the expected retained size. Neither a small current dataset nor an engine's graph shape establishes a retention policy.

## Recommended continuation

Keep agdb as the lead candidate. The decisive next boundary is upstream review of the reproduced transaction defect and a supported corrected revision. A [report draft](../../../../../meld-eval/evidence/agdb-implementation-assessment/upstream-issue-draft.md), minimal reproduction, patch and captured evidence are ready. Nothing has been posted externally.

An isolated native parity spike can use a clearly identified experimental revision when authorized, but production integration must not quietly turn this diagnostic patch into a permanent Meld fork. Complete paths and frontier semantics, publication and cursor visibility, reader contention, payload reuse and actual custom-code retirement remain the native acceptance questions.

The earlier circuit breaker still applies if preserving Meld's contract requires a bespoke recovery layer, a database fork without a support path, or a second query authority. Current evidence does not establish that architectural failure: it supports a bounded upstream correction worth pursuing.
