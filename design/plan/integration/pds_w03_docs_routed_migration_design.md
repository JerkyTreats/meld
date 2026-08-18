# PDS W03 Docs Routed Migration Design

Date: 2026-08-18
Status: accepted for authorized implementation
Mapped workstream: `W03`
Scope: make routed package installation and owner capability contribution canonical for docs freshness while preserving current semantic and historical behavior

## Objective

Move docs freshness from fixed root slots to the `D01` router and `D02` contributor contracts without changing docs meaning, exact current capability identities, or successful flywheel behavior.

This is an ownership and persistence migration. It is not a docs semantic redesign.

The migration preserves current bodies as characterized compatibility behavior. It does not endorse the current final `stale_probability`, exact five-capability Strategy body, or hidden capability procedure as the PDS semantic boundary. The [canonical PDS architecture](../../cognitive_architecture/persistent_domain_stewardship.md) settles that boundary. A separate authorized runtime refactor must split the compatibility aggregate.

## Compatibility Migration Package

The package identity is:

```text
package id: meld.docs-freshness
manifest schema: 1
initial package version: 1.0.0
```

Authored package root:

```text
theory/docs_freshness/
├── pds-package.json
├── belief_family.docs_freshness.json
├── outcome_interpretation.docs_freshness.json
├── curation_rule.docs_freshness.json
├── maintained_condition.docs_freshness.json
├── strategy_theory.docs_freshness.json
├── authority_policy.docs_workspace_local.json
└── claim_policy.docs-claims-strict-v1.json
```

## Exact Component Index

| Component id | Route | Source | Structural requirements |
| --- | --- | --- | --- |
| `docs-belief-family` | `world-model.belief-family.v1` | authored family file | none |
| `docs-outcome-mapping` | `world-model.outcome-mapping.v1` | authored mapping file | `docs-belief-family` |
| `docs-curation-rule` | `world-model.agent-curation-rule.v1` | authored curation file | `docs-belief-family`, `docs-maintained-condition` |
| `docs-maintained-condition` | `world-model.agent-maintained-condition.v1` | authored condition file | `docs-belief-family` |
| `docs-strategy-theory` | `world-model.strategy-theory.v1` | authored compatibility Strategy file | family plus all five compatibility capability components |
| `docs-authority-policy` | `execution.authority-policy.v1` | authored compatibility authority file | all five compatibility capability components |
| `docs-claim-policy` | `docs.claim-policy.v1` | authored claim-policy file | none |
| `docs-capability-inspect` | `execution.capability-contract.v1` | exact published contract | none |
| `docs-capability-draft` | `execution.capability-contract.v1` | exact published contract | none |
| `docs-capability-validate` | `execution.capability-contract.v1` | exact published contract | `docs-claim-policy` |
| `docs-capability-publish` | `execution.capability-contract.v1` | exact published contract | `docs-claim-policy` |
| `docs-capability-assess` | `execution.capability-contract.v1` | exact published contract | `docs-claim-policy` |

The implementation uses the exact current capability type ids, versions, and content identities from `src/docs/capability.rs`. The migration does not rename or rehash them. These package entries are a parity shim and are not the destination ownership model.

## Canonical Configuration

Canonical config selects an installed package assignment and a separate physical activation.

```toml
[stewardship.assignments.docs_freshness]
package_receipt_id = "exact-package-receipt"
principal_id = "local-user"
agent_id = "docs-freshness-agent"
subject_domain = "workspace_fs"
subject_kind = "node"
subject_id = "workspace-root"
perspective_id = "default"
branch_id = "main"
requested_authority_ref = "exact-request-ref"
principal_grant_ref = "exact-grant-ref"

[stewardship.activations.docs_freshness]
assignment = "docs_freshness"

[stewardship.activations.docs_freshness.bindings.workspace]
kind = "workspace"
ref = "/absolute/workspace"

[stewardship.activations.docs_freshness.bindings.provider]
kind = "provider"
ref = "main-provider"
```

Map keys are configuration-local names. Durable assignment and activation ids are derived under `D04`.

The provider binding is optional when the exact selected capability set contains no provider-backed implementation.

Canonical config contains no belief-family, mapping, curation-rule, maintained-condition, Strategy, authority-policy, claim-policy, or capability-contract fields. During parity migration those exact compatibility identities remain in the package receipt. The destination activation obtains capability identity independently.

## Legacy Input Lowering

The migration accepts these current forms through one compatibility adapter:

- `stewardship.docs_freshness`
- fixed entries under `stewardship.declarations`

Both lower to:

1. One deterministic synthetic manifest with the exact component index above.
2. One package install request.
3. One assignment declaration.
4. One physical activation declaration.

The synthetic manifest reads the exact selected current owner bodies and current exact published capability refs. It uses the normal router. It never invokes `CompleteTheoryInstall` or writes a fixed receipt.

The synthetic package version is `compat-v1`. Its package content hash is derived normally from exact bodies and refs. The legacy declaration label does not participate.

Dependency security and every later package are forbidden from using this adapter.

## Runtime Owner Resolution

Root resolves a `ResolvedPdsPackage` containing only route-addressed refs.

Each runtime consumer requests its owned refs:

| Consumer | Route view | Resolution owner |
| --- | --- | --- |
| belief assessment | belief family | world-model belief |
| evidence ingestion | outcome mapping | world-model belief outcome |
| Agent runtime | curation rule and maintained condition | world-model Agent |
| Strategy runtime | Strategy theory | world-model Strategy |
| planning and dispatch | capability contracts and authority policy | execution |
| docs validation and capability activation | claim policy | docs |

Root never reconstructs a monolithic `ResolvedStewardshipTheory` for new receipts.

## Capability Cutover

Current direct behavior:

```text
published_product_contracts
→ docs published_contracts only
→ activate_exact_capabilities
→ docs register_exact_contracts
```

Canonical behavior:

```text
compatibility package exact capability refs
→ D02 product contract and offer inventory
→ assignment-local exact offer selection
→ contributor factory preparation
→ exact prepared capability closure
→ D04 activation publication
```

Docs owns all five invoker factories. Root knows only contributor and exact capability contracts.

## Generic Receipt Write Cutover

Migration states are delivery states, not a persisted runtime enum.

| State | New install writer | Runtime reader | Fixed writer | Direct docs activation |
| --- | --- | --- | --- | --- |
| characterized | fixed | fixed | canonical | canonical |
| parallel proof | fixed production, generic test | both in tests | canonical | canonical |
| router canonical | generic | generic plus historical fixed decoder | disabled | disabled |
| historical only | generic | generic plus historical fixed decoder | absent | absent |
| compatibility retired | generic | generic | absent | absent |

Cutover to router canonical is one code change after `D00` parity and all router tests pass. After this point no production path writes a fixed receipt.

Rollback to a fixed writer is permitted only before router canonical cutover. After generic receipts are written, recovery uses a forward fix or a binary that reads generic receipts. It never forks owner meaning by resuming fixed writes.

## Historical Fixed Receipt Decoder

The decoder is read-only.

```rust
trait HistoricalTheoryReceiptDecoder {
    fn decode(
        &self,
        fixed: &TheoryInstallationReceipt,
    ) -> Result<ResolvedHistoricalPdsPackage, HistoricalReceiptDiagnostic>;
}
```

It maps fixed fields to the exact routes from `D02` and carries every current exact ref intact. It does not reinstall, recanonicalize, or read current authored files.

The synthetic historical package identity is derived from:

```text
compat namespace
fixed receipt id
sorted route-addressed exact owner refs
```

The decoder verifies the original fixed receipt identity first. A missing owner revision fails exactly as it does today.

## Parity Matrix

The routed path is compared with `DocsParitySnapshotV1` from `D00`.

| Dimension | Required relationship |
| --- | --- |
| input meaning | equivalent after lowering |
| owner revisions | exact same content identities |
| capability contracts | exact same five type, version, and content identities |
| claim policy | exact same owner revision |
| Strategy closure | exact same selectors and contract identities |
| deterministic artifacts | exact same content hashes |
| provider-backed artifacts | same scripted-provider semantic hashes |
| canonical events | same semantic event sequence and payload hashes |
| evidence | same classes, sources, theory refs, and values |
| belief and maintained condition | same semantic transitions and final state |
| Goal, task, and outcome | same semantic transitions and lineage |
| reopen | exact package and owner refs remain resolvable |
| lifecycle | equivalent current behavior before `W06` extension |

Package receipt ids differ because the canonical receipt schema differs. The parity record links fixed receipt id to generic receipt id and requires identical exact owner refs.

## Compatibility Removal Gates

| Compatibility seam | Removal gate |
| --- | --- |
| legacy docs config table | supported deployment migration complete and legacy lowering tests removed by explicit policy decision |
| fixed canonical declaration fields | all product config uses assignment and activation forms |
| `CompleteTheoryInstall` docs bundle | owner routes are canonical and no production caller remains |
| fixed receipt writer | router canonical cutover complete |
| historical fixed receipt decoder | supported data window intentionally ends under compatibility policy |
| `ResolvedStewardshipTheory` new-receipt path | every new runtime consumer resolves owner routes |
| docs-only `published_product_contracts` | `D02` product aggregation canonical |
| direct docs exact registration | assignment-local contributor activation canonical |
| `src/docs/pds.rs` direct composition | routed five-capability characterization is enduring |

Every retained seam carries a local compatibility comment and no-new-callers test.

## Failure And Recovery

- synthetic manifest failure changes no owner state before router validation completes
- generic package install failure leaves no selectable receipt
- owner partial revisions remain inert and retry-safe
- activation failure does not fall back to direct docs registration
- generic receipt corruption fails closed
- historical decoder failure does not reinstall or consult current heads
- parity mismatch blocks cutover and reports the exact snapshot field

## Verification Design

Focused tests:

```text
docs::theory::tests::canonical_manifest_contains_every_current_owner_body
docs::theory::tests::legacy_forms_lower_to_one_synthetic_package
docs::theory::tests::synthetic_package_uses_router_only
runtime::theory::tests::generic_docs_package_resolves_owner_scoped_views
runtime::theory::tests::fixed_receipt_decodes_without_reinstall
runtime::assembly::tests::docs_activation_uses_contributors_without_direct_registration
runtime::assembly::tests::provider_free_docs_selection_needs_no_provider
tests::pds_w03_docs_routed_parity::routed_snapshot_matches_fixed_oracle
tests::pds_w03_docs_routed_parity::new_writes_are_generic_only
tests::pds_w03_docs_routed_parity::historical_fixed_receipt_survives_new_install
```

Static checks after canonical cutover:

```sh
rg -n "DocsClaimPolicy|DocsClaimPolicyRevisionRef" src/runtime src/init/world src/config
rg -n "crate::docs::capability::register_exact_contracts" src/runtime src/init src/capability.rs
rg -n "claim_policy_id" src/runtime src/config/stewardship src/init/world
rg -n "docs_freshness" crates/meld-events crates/meld-execution/src crates/meld-lang/src
```

Only explicitly allowlisted compatibility paths may match the first and third searches. The second and fourth searches must be empty.

## Acceptance Criteria

- canonical and legacy inputs reach the same router path
- new docs installations write only generic package receipts
- exact current owner and capability content identities are preserved
- routed and fixed semantic snapshots pass the parity matrix
- root config and runtime contain no canonical docs component slots
- root contains no direct docs capability registration
- deterministic docs activation is representable without provider
- historical fixed receipts resolve without reinstall or current-head lookup
- no compatibility shim gains a new caller

## Rejection Criteria

Reject implementation if it:

- retains the fixed installation path as fallback
- changes docs content identity without a version decision
- copies docs policy into a generic receipt slot
- recomputes historical refs from current source
- resumes fixed writes after generic cutover
- compares only terminal output while ignoring lineage
- removes a compatibility seam before its gate is observed

## Decision Ledger

- The canonical package id is `meld.docs-freshness`.
- The compatibility package explicitly contains all seven current theory bodies and five exact capability refs.
- Both legacy inputs lower through one synthetic router package.
- Generic receipt cutover disables the fixed writer and direct activation together.
- Historical compatibility is read-only and exact.
- Rollback to fixed writes ends at generic cutover.

## Completion Condition

`D03` is delivered. `W03` may implement the routed docs migration without further cross-domain design decisions.
