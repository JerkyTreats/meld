# Local candidate setup

Commit authorization update: the user explicitly requested committing R5 and Startup, then creating the Meld 3 candidate branch. This supersedes the earlier no-commit statements below for this completed slice. Existing validation evidence is retained; no new crate build is authorized. The local setup is accepted for this commit, not as official candidate qualification.

Slice `local-meld3-startup`, baseline `cd2144b4` plus the existing uncommitted R5 candidate. The user authorizes a separate local `meld3` installation, initialization and visible nonce healthcheck. This is a bounded local setup and correction slice, not R6 freeze acceptance or commit authorization.

Readiness: build-ready. Maturity: first slice. Existing native Startup proof supplies the runtime semantics; actual operator commands exposed an inspection gap. Normal startup can satisfy its Goal, but default offline inspection selects only a current generation and loses the closed history. The previous three-second example also used the default one-second tick interval, which did not allow enough passes to prove completion.

Owned changes are a local launcher, pinned candidate binary and Startup theory, isolated persistent configuration and state, Startup inspection and its regression, current runtime diagnostic rendering, and the directly affected operator guide. The launcher only selects executable and environment; native Meld retains initialization, runtime scheduling and semantic truth. Existing installed `meld` and its configuration remain intact.

| Responsibility | Mode | Disposition and proof |
| --- | --- | --- |
| Local executable and environment selection | New | Thin `meld3` launcher forwards native arguments to a pinned binary with dedicated XDG roots. Verify PATH resolution and installed binary identity |
| Startup generation inspection | Extend | Retain the canonical Startup account reader; prefer explicit or current generation, then latest retained generation when stopped. Historical admission remains marked stale. Verify active, stopped and reopened behavior |
| Dispatch diagnostic rendering | Extend | Suppress the captured unresolved-route message only after the canonical route slot is bound. Leave every other diagnostic and required-route enforcement intact |
| Initialization and nonce execution | Retain | Use native `world init` and `runtime run`. Prove nonce publication, Graph visibility, confirmation, Belief and Goal satisfaction through ordinary executable commands |

Runtime Invariants apply: no parallel planner, actor, Event authority or semantic writer. Inspection remains read-only. No stores, protocols, services, automatic migration or background daemon are introduced. Existing R5 changes remain user-owned and preserved. No commit, installation over `meld`, or full-program acceptance is authorized.

Completed: local initialization, foreground progress, live and stopped account inspection, and restart with a new epoch are demonstrated. Two ordinary executable runs independently reached `GoalSatisfied: Available`, with distinct nonces. The second used bare `meld3 runtime run` with default timing and logging. Both stopped accounts are readable in a new process, retain their nonce and satisfaction, and mark closed admission stale. Repeating native initialization after shutdown succeeds. The stale dispatch warning is absent from the running profile.

Installed candidate SHA-256 is `98e22a5578ef84ec5461c9185e98028ba593c0d40b802ca125f9411b873c246d`, matching the current debug binary. The unchanged installed `meld` SHA-256 is `e773662065a0fdffb9e88a884aef50f9a6c6cd362a976cc1606e41ca79d8b5a5`. The local launcher, config, bundled Startup theory and command guide are retained under the user's `meld3` directories. Source changes remain uncommitted and the package remains version-stamped `2.7.0`.

Logical review: satisfied. A bounded read-only Sol review confirmed explicit/current/history selection priority, currentness markings and canonical dispatch-state checks. Style Assurance: satisfied. The focused native account regression passed, covering stopped history and a newer successor, and strict root library/binary Clippy passed. Formatting, patch whitespace and launcher syntax checks pass. Composed judgment: accepted for this local setup after ordinary live, shutdown, offline, restart and repeated-init proofs. Test and build logs plus CLI outputs are retained at `/home/jerkytreats/.local/state/meld3/setup`.

The earlier comparison overstated the failure of the candidate: the three-tick run did not prove completion, and the default stopped reader hid history. Subsequent native records established satisfaction with enough runtime passes. These corrections preserve R5's bounded Docs and Security evidence; they do not establish R6 freeze acceptance. No background process remains running. Further CLI redesign, release versioning and full-program validation require their own authorized scope.
