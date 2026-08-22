# WMR-DD-01 Retrospective Assurance Recommendation

Date: 2026-08-22

Review marker: independent retrospective integrated-design review

Recommender: dedicated read-only subagent `wmr_dd01_retrospective_review`

Historical candidate commit: `17c4933d0a4ea43069e3c63b6585305dc2069045`

Historical candidate tree: `14a479c9c511a892e5feab3b6063155dca286af5`

Integrated-review manifest digest: `f2efca7d2269e39322cdeac0aa854875259f371995c43c92e7de7ef381315c78`

Gate-candidate manifest digest: `41c8dbff632906f9863647a677f5e65d406196cdc1e24bb0b0ab177f6f805888`

Digest reproduction: passed for both historical manifests

Initial recommendation: changes required before independent reaffirmation

This recommendation audits the exact historical commit tree. It does not rewrite the original review, issue Gate Acceptance, or authorize later work.

## Frozen Finding Set

### WMR-DD-01-RA-F01

Classification: `active-slice defect`

Severity: high

Blocking basis: `WMR-DG-01-C01`, `WMR-DG-01-C04`, and `WMR-H01` through `WMR-H03` do not establish how occurrence-rich owner publications survive neutral Event carriage.

Historical evidence: the candidate defines independently identified qualified occurrences in `detailed_design/owner_publication_to_traversal_cut.md` lines 40 through 45, but names only bare Event graph attachments and an opaque payload seam at lines 63 through 85. The code evidence at `crates/meld-events/src/events/contracts.rs` lines 84 through 93 carries relation family and endpoints but no occurrence identity or qualifications.

Smallest correction: declare the producer-owned typed payload as semantic authority, classify neutral object and relation attachments as routing hints, and make owner-routed Graph admission consume and preserve the typed publication batch.

Program-owner disposition: accepted.

### WMR-DD-01-RA-F02

Classification: `active-slice defect`

Severity: high

Blocking basis: `WMR-DG-01-C06`, `WMR-DG-01-C10`, and the producer precondition for `WMR-H05` require owner observation completeness to reach the cut.

Historical evidence: the candidate defines observation-set completeness but does not name its receipt among Graph-preserved material, the cut contract, result contract, or hydration table. The missing-docs proof nevertheless claims that Traversal returns declared exclusions and a complete source revision.

Smallest correction: carry exact completeness receipt identity, bounded scope, exclusions, failures, and terminal status through owner publication, Graph lineage, `TraversalCut`, and `TraversalResult`.

Program-owner disposition: accepted.

### WMR-DD-01-RA-F03

Classification: `active-slice defect`

Severity: medium

Blocking basis: `WMR-DG-01-C03`, `WMR-DG-01-C08`, and `WMR-H01` require an independently durable producer position and restart source.

Historical evidence: the semantic ledger requires a durable operation or outbox, while the detailed design also permits deterministic reconstruction without fixing an authoritative durable revision, complete enumeration rule, or mandatory restart scan.

Smallest correction: require a durable operation or outbox unless the operation derives from an authoritative durable source revision through a complete versioned enumeration rule that restart must execute.

Program-owner disposition: accepted.

### WMR-DD-01-RA-F04

Classification: `evidence correction`

Severity: medium

Blocking basis: `WMR-DG-01-C04` requires direct workspace and dependency-security lineage examples proving that equal endpoints do not erase distinct occurrences.

Historical evidence: the candidate states the abstract invariant but does not trace equal-endpoint occurrences through owner revision, Event positions, Graph facts, cut selection, and result inclusion.

Smallest correction: add bounded cross-domain occurrence-lineage specimens with explicit identities and positions.

Program-owner disposition: accepted.

### WMR-DD-01-RA-F05

Classification: `evidence correction`

Severity: low

Blocking basis: nonblocking artifact-state inconsistency.

Historical evidence: the applied commit and Gate Receipt claim completion while the worker packet and design artifacts still describe an active candidate or proposed edge set.

Smallest correction: mark the historical worker packet complete and make the accepted or corrective assurance state explicit on current artifacts.

Program-owner disposition: accepted as a nonblocking coherence correction.

## Correction Boundary

One documentation-only correction cycle may address `WMR-DD-01-RA-F01` through `WMR-DD-01-RA-F05`. It may not change Rust shapes, add stores, amend the frozen gate, close later consumers, or authorize implementation.

One verification recommendation may assess only the frozen findings and regressions caused by their corrections.

## Verification Recommendation

Corrected integrated candidate digest: `753046de7e4775b0a75db01c049593ec37febd897889382941bd844fbf8e0931`

Digest reproduction: passed

Recommendation: passed

| Finding | Verification |
| --- | --- |
| `WMR-DD-01-RA-F01` | passed, the typed owner publication batch is authoritative while neutral Event attachments remain routing hints |
| `WMR-DD-01-RA-F02` | passed, owner completeness receipts now cross publication, Graph, cut, result, and hydration boundaries |
| `WMR-DD-01-RA-F03` | passed, publication is durable directly or recoverable only from authoritative durable state through mandatory deterministic enumeration |
| `WMR-DD-01-RA-F04` | passed, two domain specimens preserve distinct occurrences over equal endpoints through Event, Graph, and cut positions |
| `WMR-DD-01-RA-F05` | passed, historical worker and current shared-ledger states no longer claim an active DD-01 slice |

Correction-caused regression set: empty.

Residual nonblocking wording risk: the dependency-security specimen says two Graph facts retain both assessments even though it defines one assessment and two owner occurrences. The surrounding cells preserve and return both occurrence identities, so the reviewer found no reopened criterion or blocking ambiguity.

The final integrity check covered only the later shared-ledger reconciliation. It reproduced the digest above and confirmed that all five findings remain resolved.
