# WMR-DD-04 Subagent Review Recommendation

Date: 2026-08-22

Review marker: initial integrated design recommendation

Recommender: dedicated read-only subagent `wmr_dd04_integrated_review`

Candidate manifest digest: `70094a79a0b414887311dc100f4b7833cd0bc7efae72fdcc7068251827457512`

Digest reproduction: passed

Recommendation: changes recommended

This recommendation is advisory. It is not Gate Acceptance, next-slice authorization, or commit authority.

## Frozen Finding Set

### WMR-DD-04-F01

Route recovery was not consistently bound to the exact routing-rule revision.

Blocking basis: incomplete restart product under `WMR-DG-04-C06`.

Smallest correction: add the exact versioned routing-rule identity to reconstruction inputs, fences, and restart evidence without choosing storage or topology.

Program-owner disposition: `active-slice defect`, accepted.

### WMR-DD-04-F02

The uncertain-effect identity did not explicitly fence the resolved Capability binding and effect target.

Blocking basis: duplicate-effect and target-drift risk under `WMR-DG-04-C07`.

Smallest correction: bind operation identity and retry to the exact installed binding revision, resolved Capability instance, and effect target. A target change requires a distinct operation or explicit reconciliation.

Program-owner disposition: `active-slice defect`, accepted.

### WMR-DD-04-F03

The grouped details for `WMR-H33` through `WMR-H36` assigned Agent as the single consumer and conflicted with the registry-defined Events, Graph, configured Belief, and Agent consumers.

Blocking basis: incorrect active-path ownership under `WMR-DG-04-C11`.

Smallest correction: give each edge its registry-defined producer, consumer, acceptance position, and restart evidence. Use `WMR-H19` for Agent absorption of a declared direct owner milestone.

Program-owner disposition: `active-slice defect`, accepted.

### WMR-DD-04-F04

The transition ladder presented a universal serial chain across milestones that the design otherwise defined as independent and Plan-selected.

Blocking basis: false active-path dependency under `WMR-DG-04-C09` through `WMR-DG-04-C11`.

Smallest correction: express a partial order and let Agent absorb the exact position declared by the Plan dependency without requiring every later projection.

Program-owner disposition: `active-slice defect`, accepted.

## Passing Areas

The recommender found Execution authority, Task cardinality, admission outcomes, semantic-owner return, docs freshness, dependency-security meaning, lifecycle deferral, and documentation-only scope otherwise sound.

## Verification Boundary

One verification recommendation may assess only `WMR-DD-04-F01` through `WMR-DD-04-F04` and regressions caused by their corrections.

## Verification Recommendation

Corrected candidate manifest digest: `ee6182b00d3d11aec199e1a94ac39e26b2de1aa315f5aa4515b0f26a31fcfd81`

Digest reproduction: passed

Recommendation: passed

| Finding | Verification |
| --- | --- |
| `WMR-DD-04-F01` | passed, reconstruction retains exact installed binding and routing-rule revisions across identity, fence, restart, and successor behavior |
| `WMR-DD-04-F02` | passed, operation identity and retry retain exact resolved Capability instance, binding revision, and effect target |
| `WMR-DD-04-F03` | passed after correction-caused regression repair, each edge retains its registry-defined consumer and Agent absorption uses `WMR-H19` or `WMR-H36` according to the declared milestone |
| `WMR-DD-04-F04` | passed, the transition account is a partial order with independent outcome publication and owner observation branches |

Correction-caused regression set: empty after the bounded wording repair
