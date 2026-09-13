# Graph crate feasibility experiment

Slice: `graph-crate-fit-v1`. Lifecycle: closed. Readiness: standalone feasibility evidence accepted; native Meld integration and production selection remain unqualified. Baselines: Meld `03cfd38c`, Meld Eval `fdaeb32`. Working branches: `spike/graph-crate-fit`. The user authorized this experiment and previously authorized natural checkpoint commits and pushes.

## Outcome and limits

Determine whether `terminus-store`, `agdb` or Cozo can preserve Meld's publication and revision semantics while taking over meaningful graph machinery. Use the [Graph contracts](graph_use_case_contracts.md), not the [proposed custom layout](graph_candidate_design.md), as the comparison authority. The Sled-first recommendation is held pending this comparison.

This is an exploratory screen of three disposable adapters owned by Meld Eval. Native runtime integration is the next evidence boundary for a credible survivor; this screen cannot itself qualify a runtime replacement. Each adapter uses its engine's graph or relation operations. It must not hide the entire graph in one JSON blob and then run Meld's existing traversal in memory.

Use captured public Codebase Semantics publications and independent fixed examples. Preserve occurrence identities, full record payload, scopes, provenance, completeness and exact historical membership. No provider calls, domain changes, release publication, production store migration or private-store inspection are needed. Runtime stores remain outside the subject workspace. Explicit local Cargo builds are authorized and separate from harness execution.

## Frozen screen

The harness fixes expected outputs before running adapters: initial publication and retry, same endpoints with distinct occurrences, changed evidence, an incomplete successor, removal and explicit withdrawal, historical selection, structured-address collisions, exact lookup, native graph neighborhood query, clean reopen and interrupted publication. Expected logical outputs come from the input publications, not agreement between databases.

Measure internal operation time separately from process startup and formatted output. Compare a fixed small query before and after unrelated retained history, broad reads and the real captured repository publications. Count adapter-visible row reads and returned payload, identify unavailable internal metrics, record physical footprint and retained revisions. No unexplained counter becomes proof of bounded engine work.

Interruptions before and after engine commit test the adapter's persistence boundary, not native Meld lifecycle. A failed candidate remains evidence. At most one bounded correction follows a semantic or mapping failure before deciding whether further integration is justified. Ordinary compiler/API fixes do not justify redesigning the engine around the experiment.

## Responsibilities and proof

| Responsibility | Mode and owner | Existing route and state | Final disposition and proof |
| --- | --- | --- | --- |
| Canonical Event admission, Graph, Planner and Strategy | retain, Meld | Current runtime and persistent state | Unchanged reference; no replacement authority introduced |
| Fixed comparison fixtures and output oracle | new, Eval | Public captures from `graph-engine-fit-v1` | Capture identities and exact logical comparisons |
| Experimental storage and graph mapping | new, Eval candidate executables | Three external crates, isolated state and pinned lockfile | Disposable named probe route; no runtime registration |
| Capture and resource reporting | extend, Eval | Existing explicit-executable command harness | Reuse capture; report candidate and source identities |
| Candidate selection and maintenance account | new, experiment record | Contract frame and earlier spike evidence | Source-backed mapping, measured results and explicit unknowns |

Affected domains are the existing frozen assessment's Graph, Events, runtime integration, owner publications and Eval. Write scope for this screen is Eval probes, capture/reporting and temporal Meld design. Only Eval behavior changes. No production deletion is expected because this is new evidence machinery, not a runtime replacement.

Gate acceptance requires a reproducible result for each candidate, preserved failures, an explicit account of machinery delegated versus retained, and a justified native-integration decision. Documentation and public captures may expose an early mismatch; unknowns must not be promoted to passes. Native integration is eligible only after the chosen mapping is credible and its remaining scope is concrete.

## Evidence and closeout

The [retained Eval results](../../../../../meld-eval/evidence/graph-crate-fit-v1/README.md) contain the mapping account, measurements, correction history, unknowns and next-candidate decision. The [manifest](../../../../../meld-eval/evidence/graph-crate-fit-v1/manifest.json) identifies the source, lockfile, binary and evidence. The final executable digest is `720f056d6454ac03c352a19719154226d987b50a67a26021f7fb2acafa27dc07`.

Two fresh release runs use identical frozen inputs and reverse candidate order. Cozo passes 58 of 58 checks in each; corrected Terminus passes 86 of 86; agdb passes 68 of 72, with the same four interrupted-publication failures. Portable Cozo replay from the compressed fixture also passes all 58 checks and reproduces the input digest. Counts differ with failpoint topology and are not a ranking of contract coverage.

Cozo earns a bounded native parity spike. It delegates recursive query evaluation and transactional indexed storage while preserving the exercised semantics. Its fixed old query remains about 0.34–0.44 ms after 100 revisions, and its captured repository neighborhood takes about 5.9–6.2 ms. This is not a performance win over the existing optimized native prototypes, whose command and runtime boundaries differ.

agdb's interrupted cut write leaves a visible revision and guard with an old head; replay can return success without advancing that head. No missing old cut or corrupt payload was observed. Correcting this requires an explicit publication or recovery mapping before native investment. A failed pre-commit rollback check alone is not proof of corruption, because unacknowledged completed operations may be tolerable.

Terminus's first mapping failed because independent cut, guard and head labels were not one publication boundary. Its one bounded semantic correction uses an immutable catalogue with one engine label as visibility authority. The corrected mapping passes the interruption cases, but the old tiny query grows from 7–11 ms to 112–118 ms after 100 revisions. Engine rollup exists but is unqualified here. The unrolled mapping does not satisfy the intended history-scaling shape.

The screen answers whether useful graph-shaped delegation is plausible: yes, particularly Cozo's native relation queries. It does not show that an engine removes Meld's revision, coverage, provenance or causal-cut responsibilities. Payload sharing, complete occurrence-qualified paths, independent physical work budgets, full ledger and route selection, concurrent operation and native lifecycle remain open. No candidate becomes the production authority in this slice.

### Assurance and disposition

Logical Review: satisfied for the frozen standalone screen. Independent expected outputs come from captured operations and explicit synthetic fixtures. Failures remain visible; a partial report cannot pass. Runtime owners and source authority are unchanged. The one semantic mapping correction is confined to Terminus catalogue publication. agdb's remaining recovery mismatch is a candidate result rather than hidden behind repair machinery. Cozo API and query batching corrections preserve the same expected output. Full runtime Graph parity is explicitly not claimed.

Style Assurance: satisfied for the final source identified by the manifest. Probe code remains isolated under Meld Eval, with pinned dependencies and `publish = false`. Rust formatting and Clippy pass with warnings denied. All 33 Eval tests pass, including the fixed-oracle and incomplete-report checks. The portable fixture path additionally passes a real Cozo run. Documentation separates measured engine work, adapter work and unavailable counters. Existing domain assessment remains applicable; no production replacement or retirement is claimed.

Gate Acceptance: accepted for the experiment deliverables. Every candidate has retained commands, outputs or explicit bulk-output hashes, exact source publications, a mapping account and a disposition. Failed checks are preserved rather than counted as native acceptance. The source-to-adapter-to-oracle boundary is demonstrated. Complete native command parity, retention policy and production migration are forbidden substitutes for these narrower results and remain unqualified.

Product acceptance: the feasibility screen and its evidence are complete. Architectural completeness: Cozo's mapping is credible enough for native comparison, not a settled Graph architecture. Commit authority: existing user authorization covers natural checkpoints and pushes on both working branches. Next-slice scope: recommend one canonical native Cozo Graph implementation against the existing reference, with complete public traversal, Event visibility, replay, reuse cost and custom-code retirement as deciding evidence. This record does not perform that integration or authorize release publication.

Commit effect: If applied, this commit records a graph-crate comparison and advances Cozo to a proposed native parity spike without changing Meld runtime behavior.
