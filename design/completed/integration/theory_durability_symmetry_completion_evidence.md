# Theory Durability Symmetry Completion Evidence

Date: 2026-08-12
Status: complete
Scope: Theory Elevation Step 1

## Installed Inventory

| Revision unit | Owner registry | Exact reference |
| --- | --- | --- |
| Belief family | `BeliefFamilyRegistryStore` | `TheoryRevisionRef` |
| Curation rule | `AgentCurationRuleRegistryStore` | `TheoryRevisionRef` |
| Outcome mapping | `OutcomeMappingRegistryStore` | `TheoryRevisionRef` |
| Strategy theory | `StrategyTheoryRegistryStore` | `TheoryRevisionRef` |
| Executable contract | `CapabilityContractRegistryStore` | `CapabilityContractRevisionRef` |
| Claim policy | `DocsClaimPolicyRegistryStore` | `DocsClaimPolicyRevisionRef` |

Root owns `TheoryInstallationReceiptStore`. It stores reference sets and one active receipt head per selected package. It owns no semantic body.

## Activation And Historical Isolation

`receipt_activation_preserves_an_already_resolved_historical_image` installs an incomplete owner set with no active receipt, activates receipt A, activates receipt B with a new curation revision, and proves all of the following:

- source owner writes alone do not activate an image
- executable reference order does not alter receipt identity
- a snapshot already resolved through receipt A stays byte exact after B activates
- receipt A reloads after B through `ResolvedStewardshipTheory::resolve_receipt`
- both curation revisions remain exactly resolvable

`stewardship_receipts_activate_routes_and_preserve_a_b_lineage` runs the product initialization and runtime assembly path. It persists an A curation decision and Strategy authorization, activates B under the same selected package, reopens assembly, admits a new observation, and persists B lineage. The test proves the current head selects B while A remains exactly resolvable and A records retain A references.

No executable contract body changes between those receipts. The separate execution owner test rejects changed content under the same type and version, enforcing the required version increment rule.

## Durable Lineage

The exercised durable path is:

```text
TheoryInstallationReceipt
  -> AgentRecord curation_rule_revision
  -> AgentCurationDecision curation_rule_revision
  -> StrategyAuthorization strategy_theory_revision
  -> ExecutionStrategyAuthorization Strategy id and content hash
  -> capability_contract_ids
  -> task lineage
  -> outcome policy_identity
  -> PromotedEvidenceRecord outcome_mapping_revision
  -> EvidenceItem outcome_mapping_revision
  -> BeliefRevision theory_revision
```

Authorization identity includes the exact Strategy revision. Promoted evidence identity includes the exact mapping content hash. Planning skips only an already materialized exact authorized graph and otherwise revalidates executable content identities against the receipt-built catalog.

## Failure Evidence

Focused tests cover an absent selected receipt, an active head naming a missing receipt, a receipt naming an absent owner revision, capability version drift, authored identity mismatch, and source provisioning without activation. These failures remain unresolved bindings or typed owner failures and create no fallback semantic runtime.

Production composition no longer reads authored theory files. World initialization is the only production loader of those compatibility inputs.

## Compatibility Surfaces

The belief-family current head remains for pre-receipt callers. Elevated actors use receipt-selected exact revisions.

Loose fields in `StewardshipTheoryBindings` remain for harness fixtures under an explicit compatibility removal note. Production CLI composition supplies no loose semantic bodies.

Legacy Agent records with embedded curation bodies remain readable. World initialization migrates them to `curation_rule_revision` and clears the embedded body.

## Quality Gates

The implementation passes the required gates:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

The assembled receipt integration additionally proves the dynamic five-capability Strategy graph still lowers without a Method or workflow route, exact Agent lineage persists, a newer receipt does not reinterpret prior records, and the bounded runtime returns to quiet passes without retrying an already materialized historical authorization against a newer planner frame.
