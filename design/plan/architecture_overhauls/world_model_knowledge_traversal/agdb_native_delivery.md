# Native agdb Graph delivery

Status: implemented and qualified for the bounded native experiment. Baseline `5e9356e2`, branch `feat/agdb-graph`.

The user authorized native agdb integration, the original performance journey, and a full program accounting. Upstream issue and fix preparation is separately delegated, with no Meld context and no publication authorization. Standing checkpoint commit and push authorization remains in force. No release publication is authorized.

The observable increment is the existing Event-to-Graph journey using agdb for canonical publication storage and indexed reads. Owner products, saved cuts, ordered paths, independent bounds, and restart meaning must survive. The original command harness is the acceptance oracle.

This is an experimental native integration with durable-state obligations. A pinned, locally corrected upstream engine is explicit temporary dependency debt. Do not introduce a Meld recovery log or a second publication writer. Stop if correctness requires either. Upstream review, power-loss qualification and broad lifecycle hardening remain separate gates.

## Responsibility and boundary

| Responsibility | Disposition | Successor and proof |
| --- | --- | --- |
| Graph publication persistence | Replace Sled publication bodies and their writer | agdb revision graphs with immutable shared payloads; reconstruct original operations exactly |
| Graph selection and visibility | Replace publication-history scans | Indexed owner/scope headers and exact Event lookup; original cut and visibility tests |
| Startup nonce visibility accounting | Replace retained-publication enumeration | Resolve the known Event only when it is covered by the durable Graph cursor; preserve exact operation equality |
| Graph traversal preparation | Replace whole-history graph assembly | Selected-revision address and native adjacency reads; preserve existing path/frontier assembly |
| Graph cursor and route metadata | Retain current authority | Existing durable publication-before-cursor boundary, with agdb synchronized before the Sled cursor advances |
| Existing Sled publication state | Read-only migration | Import validated retained publications before admitting new Graph writes; no legacy query fallback |
| Event, owners, Belief, Planner, Strategy | Retain contracts | Native journey and focused integration tests; no domain theory enters runtime |
| Runtime storage construction | Extend explicit location | agdb file under the existing external product state root, never the observed workspace |
| Managed stop completion on Linux | Extend physical exit observation | Capture a verified process handle before requesting stop and wait for exit as well as the native shutdown receipt |

The bounded cutover retains the existing serialized cut API and metadata engine. Compact receipts, single-engine metadata colocation, high-degree examined-entry budgets and retention collection are deferred rather than mixed into this engine cutover. This narrows the earlier candidate design into a directly comparable experiment. It does not claim that the entire representation recommendation is complete.

## Evidence and review

Final evidence is retained in the [native harness qualification](../../../../../meld-eval/evidence/agdb-native-v1/README.md). Both final agdb runs and both baseline runs pass all 46 original native checks. Exact per-run Event correspondence and cross-run domain-content comparison pass. The final release binary includes the private revision header, exact startup lookup and normal-stop correction.

| Native measurement | Baseline | agdb | Interpretation |
| --- | --- | --- | --- |
| Narrow traversal after history growth | 58.327 ms | 17.436 ms | Pooled median of six command samples per arm; 3.35 times faster |
| Initial narrow traversal | 19.186 ms | 17.192 ms | Modest improvement before history accumulates |
| Unchanged full traversal | 26.094 ms | 25.835 ms | Essentially unchanged |
| Frozen historical traversal, live | 25.617 ms | 27.228 ms | Small regression in this two-sample comparison |
| Frozen historical traversal after restart | 25.394 ms | 27.570 ms | Small regression in this two-sample comparison |
| Frozen historical traversal, offline | 64.726–166.611 ms | 65.144–66.503 ms | Baseline varies substantially; no stable speedup claim |
| Retained world-model directory | 35.87–35.93 MiB | 13.26–15.44 MiB | Includes other world-model state and metadata; agdb file is 8.90 MiB |
| All retained product state | 128.85–131.45 MiB | 107.23–107.96 MiB | Includes Events, other domains, owner executable and logs |
| Sampled runtime RSS | 365.52–379.36 MiB | 332.32–333.09 MiB | Sampled process accounting, not a precise allocation profile |


The first native qualification passes 41 live checks but fails its immediate offline read after native stop reports completion. The supervisor store had been released while the world-model store remained owned during process teardown. This brings forward only the normal-stop race from lifecycle hardening. The correction observes the verified process through a Linux PID descriptor; it does not change leases, take over an owner, repair stores or add a recovery authority. The corrected candidate passes all 46 original native journey checks before final representation review.

Representation review replaces an intermediate use of an emptied public publication as a stored header with a private revision descriptor. Only explicit reconstruction produces a complete public operation. Final qualification and timing must use this successor candidate.

Broad regression work finds a remaining production history reader in startup accounting. It checks visibility of a known nonce Event by reconstructing every publication. The successor uses the exact Event lookup and the original cursor and operation checks. The earlier long serial suite is retained as an interrupted validation attempt, not a completed pass. Final workspace validation restarts against the corrected caller. Source search then finds no production callers of broad publication reconstruction; its public explanation API and characterization tests remain.

The Rust construction API now requires an explicit Graph file path. The publication mutation method is internal to Graph, with real writes routed through the existing Event reducer. Serialized Event, cut, result and owner callback contracts remain unchanged. Existing Sled publications are read once for migration; newly admitted publications never write that tree. Downgrading an activated product to the old runtime is unsupported because it would ignore newer agdb publications.

This change makes agdb the canonical publication and adjacency authority for native Graph commands while preserving Event admission and existing cut semantics.

## Composed review and acceptance

Logical review confirms one canonical publication writer through Event admission, exact revision reconstruction, current and frozen cut selection, duplicate occurrence identity and ordered bounded traversal. The old Sled tree is a migration source only. Source search finds no production publication-history reconstruction caller. Publication durability precedes cursor advancement; activation refuses a missing successor file. Focused migration, structured identity and maximum Event-position tests pass.

Style assurance retains Graph behavior inside the world-model domain, keeps branch and runtime construction adapters thin, and adds no domain theory to runtime. Upstream engine source is retained with provenance and its existing license. Rust formatting passes. Clippy passes with the pre-existing items-after-test-module lint excluded; that exclusion is explicit rather than a claim of strict repository cleanliness.

Composed acceptance is qualified: the original native command workload passes and its principal history-amplification cost is removed. The 563 root library tests, four binary tests, ten other root tests and 982 non-root workspace tests and doctests pass. Four root integration failures reproduce on the original baseline and remain open. Temporary upstream correction, broad lifecycle qualification and the deferred representation work remain explicit debt. This closes the authorized engine integration experiment, not README production or Meld 3 release qualification.
