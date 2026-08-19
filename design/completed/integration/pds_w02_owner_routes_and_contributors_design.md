# PDS W02 Owner Routes And Contributors Design

Date: 2026-08-18
Status: accepted for authorized implementation
Mapped workstream: `W02`
Scope: owner-published theory handlers and deterministic capability contribution for the current docs package

## Objective

Define the two symmetric publication boundaries required by routed stewardship:

```text
domain theory route publication
+
domain capability contribution
```

The router installs state-free owner meaning. Capability contributors publish exact executable contracts and physical offers. Neither boundary selects an expression or grants authority.

Review note: the current docs Strategy body remains a compatibility input for routed parity. Its embedded exact capabilities, evaluation policy, search bounds, and requested authority are not part of the canonical PDS semantic interface. The [canonical PDS architecture](../../cognitive_architecture/persistent_domain_stewardship.md) records the accepted split between settlement meaning and independently owned Strategy inputs.

## Ownership Rule

Every semantic body remains owned by its current registry domain. The product capability aggregator owns deterministic inventory assembly only.

Adapters may convert an owner revision ref into the common `TheoryRevisionRef`. They may not copy, reinterpret, or persist the body.

## Initial Route Catalog

| Route | Cardinality | Body owner | Existing registry | Runtime consumer |
| --- | --- | --- | --- | --- |
| `world-model.belief-family.v1` | many | belief | `BeliefFamilyRegistry` | belief assessment and projection |
| `world-model.outcome-mapping.v1` | many | belief outcome | `OutcomeMappingRegistryStore` | evidence ingestion |
| `world-model.agent-curation-rule.v1` | many | Agent | `AgentCurationRuleRegistryStore` | Agent runtime |
| `world-model.agent-maintained-condition.v1` | many | Agent | `AgentMaintainedConditionRegistryStore` | maintained-condition evaluation |
| `world-model.strategy-theory.v1` | many | Strategy | `StrategyTheoryRegistryStore` | Strategy construction |
| `execution.capability-contract.v1` | many | execution capability | `CapabilityContractRegistryStore` | compatibility package migration only |
| `execution.authority-policy.v1` | many | execution authority | `AuthorityPolicyRegistryStore` | compatibility package migration only |
| `docs.claim-policy.v1` | at most one | docs | `DocsClaimPolicyRegistryStore` | docs result admission and publication |

Cardinality is package-local. It does not make any route mandatory for all packages.

## Owner Handler Shape

Each owner publishes a thin adapter implementing the `D01` route handler contract.

The adapter sequence is exact:

```text
decode owner schema
→ run existing owner validation
→ install through existing append-only registry command
→ convert exact owner revision ref
→ resolve exact owner revision through existing query
```

Target code locations:

```text
src/docs.rs
src/docs/theory.rs
src/docs/theory/claim_policy.rs
crates/meld-world-model/src/belief/theory.rs
crates/meld-world-model/src/agent/theory.rs
crates/meld-world-model/src/strategy/theory.rs
crates/meld-execution/src/capability/theory.rs
crates/meld-execution/src/authority/theory.rs
```

Each parent module declares its `theory` child directly.

## Common Revision Conversion

The common ref is:

```rust
struct TheoryRevisionRef {
    registry: String,
    id: String,
    content_hash: String,
}
```

Conversion rules:

- `registry` is the stable existing owner registry identity
- `id` is the owner semantic identity already used by exact resolution
- `content_hash` is the existing owner revision content identity
- conversion is lossless and reversible by the route adapter
- a route adapter rejects any ref whose registry does not equal its owned registry

Capability contracts retain their exact selector and `content_identity`. Their common owner ref uses the registry identity plus a stable selector encoding as `id` and the capability `content_identity` as `content_hash`.

Docs claim policy retains `policy_id` and `content_identity` with no rehashing.

## Semantic Link Validation

Theory validates structural component requirements. Owners validate meaning.

The current docs package uses these semantic checks:

| Owner validator | Required public inputs | Check |
| --- | --- | --- |
| belief | belief-family and outcome-mapping refs | mapping dimensions are admitted by the selected family |
| Agent | curation-rule, maintained-condition, and belief-family refs | dimension and maintained-condition identities agree |
| Strategy | Strategy and belief-family refs | settlement vocabulary and evidence semantics agree |
| execution authority | authority-policy and assignment inputs later | policy identity is valid state-free theory at install time |
| docs | claim-policy ref | body policy id equals routed component identity under docs rules |

Cross-owner reads use immutable public query ports injected into the validating handler. No handler receives a foreign command port.

For current docs parity only, Strategy to capability validation uses explicit compatibility components in the manifest. Canonical validation instead compares Strategy candidates against the activation-local exact capability snapshot during Strategy problem assembly.

## Route Publication

Every owner exports one constructor returning its handlers. Root product assembly collects constructors without package or expression input.

```rust
trait TheoryRoutePublisher {
    fn owner_domain(&self) -> &str;
    fn handlers(&self) -> Vec<Arc<dyn TheoryRouteHandler>>;
}
```

The product publisher list is a compiled product inventory. It is sorted by owner domain before the immutable catalog is built. Adding a package does not alter this list.

Published capability revisions convert to the common router reference with registry `capability_contract`, owner id `<capability_type_id>.v<capability_version>`, and the exact contract content identity. Route adapters perform this conversion without exposing an owner registry implementation to another domain.

## Capability Publication Contracts

Contract publication and physical implementation offers are separate.

```rust
trait ProductCapabilityContributor: Send + Sync {
    fn owner_domain(&self) -> &str;
    fn published_contracts(&self) -> Vec<CapabilityContractRevision>;
    fn implementation_offers(&self) -> Vec<CapabilityImplementationOffer>;
}

struct CapabilityImplementationOffer {
    contract_ref: CapabilityContractRevisionRef,
    implementation_ref: String,
    required_binding_ids: BTreeSet<String>,
    execution_class: ExecutionClass,
    factory: Arc<dyn CapabilityInvokerFactory>,
}

trait CapabilityInvokerFactory: Send + Sync {
    fn prepare(
        &self,
        request: &ExactCapabilityActivationRequest,
        bindings: &OwnerBindingView,
    ) -> Result<PreparedCapabilityInvoker, CapabilityContributionDiagnostic>;
}
```

`published_contracts` exposes canonical execution-owned revisions. Contributors do not create an alternate contract type.

An implementation offer is physical inventory. Its implementation ref, bindings, and factory do not participate in contract content identity.

## Deterministic Product Inventory

Product aggregation follows this order:

1. Collect every contributor.
2. Sort contributors by owner domain.
3. Sort contracts by capability type, version, and content identity.
4. Deduplicate byte-equivalent refs to the same installed revision.
5. Reject one capability type and version with different content identities.
6. Sort offers by contract ref and implementation ref.
7. Reject duplicate implementation refs for one contract.
8. Freeze an immutable contract inventory and offer inventory.

Several implementations may offer the same exact contract. Selection happens per activation in `D04`. No global executor registry receives every offer.

## Initial Contributors

| Contributor | Contract posture | Binding posture |
| --- | --- | --- |
| docs | five exact current docs contracts | workspace plus provider only for model-backed contracts |
| context | existing context contracts | context store and declared provider inputs only |
| provider | existing provider contracts | selected provider binding only |
| merkle traversal | existing traversal contracts | workspace and traversal inputs |
| workspace | existing workspace contracts | assignment-scoped workspace binding |

The initial docs compatibility package names the five current docs contract refs explicitly to preserve parity. Canonical activation selection is independent of PDS theory. Other published contracts remain product inventory and are absent from an activation draft unless the activation owner selects them.

## Activation Draft Boundary

`D02` defines the contributor output consumed later by `D04`.

```rust
struct ExactCapabilityActivationRequest {
    assignment_id: String,
    activation_id: String,
    selected_contracts: Vec<CapabilityContractRevisionRef>,
    selected_implementations: BTreeMap<CapabilityContractRevisionRef, String>,
}

struct PreparedCapabilityClosure {
    assignment_id: String,
    activation_id: String,
    contracts: CapabilityCatalog,
    invokers: CapabilityExecutorRegistry,
    contribution_receipts: Vec<CapabilityContributionReceipt>,
}
```

Preparation rejects a missing selected contract, missing selected offer, exact content mismatch, missing binding, duplicate invoker, extra invoker, or factory output that claims another contract.

The closure contains exactly the selected contract set. It remains inert until `D04` publishes the activation generation.

## Provider Optionality

Provider is not a universal stewardship binding.

- a contributor declares provider binding only for an offer that requires it
- an activation selecting no provider-backed offer needs no provider
- absence of a provider offer cannot fail an unrelated deterministic activation
- provider implementation or credential changes do not alter contract identity

The current `PhysicalBinding.provider_id` and direct root provider construction remain compatibility inputs until `D03` and `D04` replace them.

## Diagnostics

Stable contribution codes include:

- `capability_contract_conflict`
- `capability_implementation_conflict`
- `selected_contract_missing`
- `selected_implementation_missing`
- `selected_contract_identity_mismatch`
- `selected_binding_missing`
- `prepared_invoker_duplicate`
- `prepared_invoker_extra`
- `prepared_invoker_identity_mismatch`

Diagnostics identify owner domain, contract ref, implementation ref when present, and assignment and activation ids during preparation.

## Verification Design

Focused tests:

```text
belief::theory::tests::handler_reuses_exact_family_and_mapping_registries
agent::theory::tests::handler_reuses_exact_curation_and_condition_registries
strategy::theory::tests::handler_rejects_unresolved_capability_selector
capability::theory::tests::handler_round_trips_exact_contract_revision
authority::theory::tests::handler_round_trips_exact_policy_revision
docs::theory::tests::handler_rejects_policy_id_mismatch
theory::registry::tests::current_docs_routes_publish_without_duplicates
capability::tests::product_inventory_is_deterministic
capability::tests::different_content_for_one_selector_is_rejected
capability::tests::identical_contract_with_several_offers_is_valid
capability::tests::unselected_contract_and_invoker_are_absent_from_draft
capability::tests::provider_offer_is_optional_for_deterministic_selection
```

Static checks:

```sh
rg -n "docs_freshness|dependency_security" src/capability.rs src/theory.rs src/theory
rg -n "OpenProductStores" src/docs/theory.rs crates/meld-world-model/src/*/theory.rs crates/meld-execution/src/*/theory.rs
rg -n "crate::docs::capability::published_contracts" src/capability.rs
```

Expected results are empty after cutover of the corresponding seam.

## Acceptance Criteria

- all eight current docs routes publish through their owners
- each handler uses the current validator and append-only registry
- common refs resolve back to exact owner revisions
- semantic linking uses public queries only
- product capability aggregation contains no expression dispatch
- exact current docs contract identities are unchanged
- several physical offers may implement one exact contract
- an unselected capability and invoker are absent from an activation draft
- deterministic selections require no provider

## Rejection Criteria

Reject implementation if it:

- copies owner bodies into theory storage
- gives a handler unrestricted product stores
- gives a handler foreign command authority
- derives capability components implicitly from Strategy
- selects contributors by expression name
- registers every physical offer into one mutable global executor registry
- changes contract identity because implementation or provider changes
- makes provider mandatory for every package

## Decision Ledger

- Route publication and capability contribution are separate symmetric boundaries.
- The initial catalog has eight exact route ids.
- Existing owner registries remain canonical.
- Generic revision conversion is lossless and owner-verified.
- Contracts deduplicate by exact revision while physical offers remain distinct.
- Capability selection and invoker closure are assignment-local.

## Completion Condition

`D02` is delivered. `W02` may implement owner routes and contributors without further cross-domain design decisions.
