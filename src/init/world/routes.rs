//! Compiled publication of domain-owned theory routes.
//!
//! This root adapter performs schema decoding and converts existing exact
//! owner revision references into the generic routing reference. Every
//! semantic body is validated, installed, and resolved by its current owner
//! registry through a narrow captured command or query handle.

use std::sync::{Arc, Mutex};

use meld_execution::authority::AuthorityPolicyRegistryStore;
use meld_execution::capability::{
    CapabilityContractRegistryStore, CapabilityContractRevisionRef, CapabilityTypeContract,
};
use meld_lang::{AuthorityPolicy, CapabilityRef};
use meld_world_model::agent::{
    AgentCurationRuleConfig, AgentCurationRuleRegistryStore, AgentMaintainedCondition,
    AgentMaintainedConditionRegistryStore,
};
use meld_world_model::belief::{
    BeliefConfigLoader, BeliefFamilyConfig, BeliefFamilyRegistry, BeliefFamilyRegistryStore,
    ConfiguredOutcomeMappingSet, OutcomeMappingRegistryStore, OutcomeMappingSetConfig,
    TheoryRevisionRef as WorldModelTheoryRevisionRef,
};
use meld_world_model::strategy::{
    validate_strategy_theory_package, StrategyTheoryPackage, StrategyTheoryRegistryStore,
};

use crate::runtime::storage::OpenProductStores;
use crate::theory::{
    no_semantic_links, OwnerRouteDiagnostic, PortBackedTheoryRouteHandler, RouteCardinality,
    TheoryRevisionRef, TheoryRouteCatalog, TheoryRouteContract, TheoryRouteHandler, TheoryRouteId,
    VersionRange,
};

const ROUTE_VERSION: u32 = 1;
const HANDLER_VERSION: u32 = 1;

pub fn current_product_route_catalog(
    stores: &OpenProductStores,
) -> Result<TheoryRouteCatalog, crate::theory::TheoryRouterError> {
    let family_store = Arc::new(Mutex::new(
        BeliefFamilyRegistryStore::new(stores.traversal_store.db().clone()).map_err(|failure| {
            crate::theory::TheoryRouterDiagnostic::new("route_unavailable", failure.to_string())
        })?,
    ));
    let mut handlers: Vec<Arc<dyn TheoryRouteHandler>> = vec![
        belief_family_handler(family_store),
        outcome_mapping_handler(opened(&stores.outcome_mapping_registry)),
        curation_rule_handler(opened(&stores.curation_rule_registry)),
        epistemic_template_handler(opened(&stores.curation_store)),
        graph_owner_event_handler(opened(&stores.traversal_store)),
        maintained_condition_handler(opened(&stores.maintained_condition_registry)),
        strategy_handler(opened(&stores.strategy_theory_registry)),
        capability_contract_handler(opened(&stores.capability_contract_registry)),
        authority_policy_handler(opened(&stores.authority_policy_registry)),
        product_topology_handler(opened(&stores.pds_products)),
    ];
    handlers.extend(stores.owners.route_handlers());
    TheoryRouteCatalog::build(handlers)
}

fn graph_owner_event_handler(
    store: Arc<meld_world_model::world_state::graph::store::TraversalStore>,
) -> Arc<dyn TheoryRouteHandler> {
    use meld_world_model::world_state::graph::admission::{
        GraphOwnerEventRoute, OWNER_EVENT_ROUTE_REGISTRY,
    };
    let validate = Arc::new(|owner_id: &str, bytes: &[u8]| {
        let route: GraphOwnerEventRoute = decode(bytes)?;
        route.validate().map_err(owner_failure)?;
        require_id(owner_id, &route.route_id)
    });
    let install_store = store.clone();
    let install = Arc::new(move |owner_id: &str, bytes: &[u8], _seq| {
        let route: GraphOwnerEventRoute = decode(bytes)?;
        require_id(owner_id, &route.route_id)?;
        install_store
            .install_owner_event_route(&route)
            .map(world_ref)
            .map_err(owner_failure)
    });
    let verify = Arc::new(move |reference: &TheoryRevisionRef| {
        require_registry(reference, OWNER_EVENT_ROUTE_REGISTRY)?;
        let routes = store.owner_event_routes().map_err(owner_failure)?;
        require_found(routes.iter().any(|route| {
            route.revision_ref().is_ok_and(|found| {
                found.id == reference.id && found.content_hash == reference.content_hash
            })
        }))
    });
    Arc::new(PortBackedTheoryRouteHandler::new(
        contract(
            "world-model",
            "graph-owner-event-route",
            RouteCardinality::Many,
        ),
        validate,
        install,
        verify,
        no_semantic_links(),
    ))
}

fn opened<T>(resource: &crate::runtime::storage::ScopedResource<Arc<T>>) -> Arc<T> {
    resource
        .opened()
        .expect("theory route catalog requires the owner's theory store")
        .clone()
}

fn contract(owner: &str, kind: &str, cardinality: RouteCardinality) -> TheoryRouteContract {
    TheoryRouteContract {
        route: TheoryRouteId::new(owner, kind, ROUTE_VERSION),
        accepted_component_schema: VersionRange { min: 1, max: 1 },
        package_cardinality: cardinality,
        handler_contract_version: HANDLER_VERSION,
    }
}

fn belief_family_handler(
    store: Arc<Mutex<BeliefFamilyRegistryStore>>,
) -> Arc<dyn TheoryRouteHandler> {
    let validate = Arc::new(|owner_id: &str, bytes: &[u8]| {
        let body: BeliefFamilyConfig = decode(bytes)?;
        BeliefConfigLoader::snapshot(body.clone()).map_err(owner_failure)?;
        require_id(owner_id, &body.family_id)
    });
    let install_store = store.clone();
    let install = Arc::new(move |owner_id: &str, bytes: &[u8], seq| {
        let body: BeliefFamilyConfig = decode(bytes)?;
        require_id(owner_id, &body.family_id)?;
        let (_, revision) = install_store
            .lock()
            .map_err(|_| owner("owner_lock_failed", "belief registry lock poisoned"))?
            .install(body, seq)
            .map_err(owner_failure)?;
        Ok(world_ref(revision.revision_ref()))
    });
    let verify = Arc::new(move |reference: &TheoryRevisionRef| {
        require_registry(reference, "belief_family")?;
        let found = store
            .lock()
            .map_err(|_| owner("owner_lock_failed", "belief registry lock poisoned"))?
            .resolve(&reference.id, &reference.content_hash)
            .map_err(owner_failure)?;
        require_found(found.is_some())
    });
    Arc::new(PortBackedTheoryRouteHandler::new(
        contract("world-model", "belief-family", RouteCardinality::Many),
        validate,
        install,
        verify,
        no_semantic_links(),
    ))
}

fn outcome_mapping_handler(store: Arc<OutcomeMappingRegistryStore>) -> Arc<dyn TheoryRouteHandler> {
    let validate = Arc::new(|owner_id: &str, bytes: &[u8]| {
        let body: OutcomeMappingSetConfig = decode(bytes)?;
        ConfiguredOutcomeMappingSet::new(body.clone()).map_err(owner_failure)?;
        require_id(owner_id, &body.mapping_id)
    });
    let install_store = store.clone();
    let install = Arc::new(move |owner_id: &str, bytes: &[u8], seq| {
        let body: OutcomeMappingSetConfig = decode(bytes)?;
        require_id(owner_id, &body.mapping_id)?;
        let (_, revision) = install_store.install(body, seq).map_err(owner_failure)?;
        Ok(world_ref(revision.revision_ref()))
    });
    let verify = Arc::new(move |reference: &TheoryRevisionRef| {
        require_registry(reference, "outcome_mapping")?;
        require_found(
            store
                .resolve(&reference.id, &reference.content_hash)
                .map_err(owner_failure)?
                .is_some(),
        )
    });
    Arc::new(PortBackedTheoryRouteHandler::new(
        contract("world-model", "outcome-mapping", RouteCardinality::Many),
        validate,
        install,
        verify,
        no_semantic_links(),
    ))
}

fn curation_rule_handler(
    store: Arc<AgentCurationRuleRegistryStore>,
) -> Arc<dyn TheoryRouteHandler> {
    let validate = Arc::new(|_owner_id: &str, bytes: &[u8]| {
        let body: AgentCurationRuleConfig = decode(bytes)?;
        body.validate().map_err(owner_failure)
    });
    let install_store = store.clone();
    let install = Arc::new(move |owner_id: &str, bytes: &[u8], seq| {
        let body: AgentCurationRuleConfig = decode(bytes)?;
        let (_, revision) = install_store
            .install(owner_id, body, seq)
            .map_err(owner_failure)?;
        Ok(world_ref(revision.revision_ref()))
    });
    let verify = Arc::new(move |reference: &TheoryRevisionRef| {
        require_registry(reference, "agent_curation_rule")?;
        require_found(
            store
                .resolve(&reference.id, &reference.content_hash)
                .map_err(owner_failure)?
                .is_some(),
        )
    });
    Arc::new(PortBackedTheoryRouteHandler::new(
        contract("world-model", "agent-curation-rule", RouteCardinality::Many),
        validate,
        install,
        verify,
        no_semantic_links(),
    ))
}

fn epistemic_template_handler(
    store: Arc<meld_world_model::CurationStore>,
) -> Arc<dyn TheoryRouteHandler> {
    let validate = Arc::new(|owner_id: &str, bytes: &[u8]| {
        let body: meld_world_model::curation::CurationRuleTemplate = decode(bytes)?;
        body.validate().map_err(owner_failure)?;
        require_id(owner_id, &body.rule_id)
    });
    let install_store = store.clone();
    let install = Arc::new(move |owner_id: &str, bytes: &[u8], seq| {
        let body: meld_world_model::curation::CurationRuleTemplate = decode(bytes)?;
        require_id(owner_id, &body.rule_id)?;
        Ok(world_ref(
            install_store
                .install_template(body, seq)
                .map_err(owner_failure)?
                .revision_ref(),
        ))
    });
    let verify = Arc::new(move |reference: &TheoryRevisionRef| {
        require_registry(
            reference,
            meld_world_model::curation::CURATION_TEMPLATE_REGISTRY_ID,
        )?;
        require_found(
            store
                .resolve_template(&WorldModelTheoryRevisionRef {
                    registry: reference.registry.clone(),
                    id: reference.id.clone(),
                    content_hash: reference.content_hash.clone(),
                })
                .map_err(owner_failure)?
                .is_some(),
        )
    });
    Arc::new(PortBackedTheoryRouteHandler::new(
        contract(
            "world-model",
            "epistemic-curation-rule",
            RouteCardinality::Many,
        ),
        validate,
        install,
        verify,
        no_semantic_links(),
    ))
}

fn maintained_condition_handler(
    store: Arc<AgentMaintainedConditionRegistryStore>,
) -> Arc<dyn TheoryRouteHandler> {
    let validate = Arc::new(|owner_id: &str, bytes: &[u8]| {
        let body: AgentMaintainedCondition = decode(bytes)?;
        body.validate().map_err(owner_failure)?;
        require_id(owner_id, &body.condition_id)
    });
    let install_store = store.clone();
    let install = Arc::new(move |owner_id: &str, bytes: &[u8], seq| {
        let body: AgentMaintainedCondition = decode(bytes)?;
        require_id(owner_id, &body.condition_id)?;
        let (_, revision) = install_store.install(body, seq).map_err(owner_failure)?;
        Ok(world_ref(revision.revision_ref()))
    });
    let verify = Arc::new(move |reference: &TheoryRevisionRef| {
        require_registry(reference, "agent_maintained_condition")?;
        require_found(
            store
                .resolve(&reference.id, &reference.content_hash)
                .map_err(owner_failure)?
                .is_some(),
        )
    });
    Arc::new(PortBackedTheoryRouteHandler::new(
        contract(
            "world-model",
            "agent-maintained-condition",
            RouteCardinality::Many,
        ),
        validate,
        install,
        verify,
        no_semantic_links(),
    ))
}

fn strategy_handler(store: Arc<StrategyTheoryRegistryStore>) -> Arc<dyn TheoryRouteHandler> {
    let validate = Arc::new(|owner_id: &str, bytes: &[u8]| {
        let body: StrategyTheoryPackage = decode(bytes)?;
        validate_strategy_theory_package(&body).map_err(owner_failure)?;
        require_id(owner_id, &body.snapshot.theory_id)
    });
    let install_store = store.clone();
    let install = Arc::new(move |owner_id: &str, bytes: &[u8], seq| {
        let body: StrategyTheoryPackage = decode(bytes)?;
        require_id(owner_id, &body.snapshot.theory_id)?;
        let (_, revision) = install_store.install(body, seq).map_err(owner_failure)?;
        Ok(world_ref(revision.revision_ref()))
    });
    let verify = Arc::new(move |reference: &TheoryRevisionRef| {
        require_registry(reference, "strategy_theory")?;
        require_found(
            store
                .resolve(&reference.id, &reference.content_hash)
                .map_err(owner_failure)?
                .is_some(),
        )
    });
    Arc::new(PortBackedTheoryRouteHandler::new(
        contract("world-model", "strategy-theory", RouteCardinality::Many),
        validate,
        install,
        verify,
        no_semantic_links(),
    ))
}

fn capability_contract_handler(
    store: Arc<CapabilityContractRegistryStore>,
) -> Arc<dyn TheoryRouteHandler> {
    let validate = Arc::new(|owner_id: &str, bytes: &[u8]| {
        let body: CapabilityTypeContract = decode(bytes)?;
        body.validate().map_err(owner_failure)?;
        require_id(owner_id, &capability_owner_id(&body))
    });
    let install_store = store.clone();
    let install = Arc::new(move |owner_id: &str, bytes: &[u8], seq| {
        let body: CapabilityTypeContract = decode(bytes)?;
        require_id(owner_id, &capability_owner_id(&body))?;
        let (_, revision) = install_store.install(body, seq).map_err(owner_failure)?;
        let reference = revision.revision_ref();
        Ok(TheoryRevisionRef {
            registry: "capability_contract".to_string(),
            id: capability_ref_id(&reference.selector),
            content_hash: reference.content_identity,
        })
    });
    let verify = Arc::new(move |reference: &TheoryRevisionRef| {
        require_registry(reference, "capability_contract")?;
        let selector = parse_capability_ref_id(&reference.id)?;
        let exact = CapabilityContractRevisionRef {
            selector,
            content_identity: reference.content_hash.clone(),
        };
        require_found(store.resolve(&exact).map_err(owner_failure)?.is_some())
    });
    Arc::new(PortBackedTheoryRouteHandler::new(
        contract("execution", "capability-contract", RouteCardinality::Many),
        validate,
        install,
        verify,
        no_semantic_links(),
    ))
}

fn authority_policy_handler(
    store: Arc<AuthorityPolicyRegistryStore>,
) -> Arc<dyn TheoryRouteHandler> {
    let validate = Arc::new(|owner_id: &str, bytes: &[u8]| {
        let body: AuthorityPolicy = decode(bytes)?;
        body.validate().map_err(owner_failure)?;
        require_id(owner_id, &body.policy_id)
    });
    let install_store = store.clone();
    let install = Arc::new(move |owner_id: &str, bytes: &[u8], seq| {
        let body: AuthorityPolicy = decode(bytes)?;
        require_id(owner_id, &body.policy_id)?;
        let (_, revision) = install_store.install(body, seq).map_err(owner_failure)?;
        let reference = revision.revision_ref();
        Ok(TheoryRevisionRef {
            registry: "authority_policy".to_string(),
            id: reference.policy_id,
            content_hash: reference.content_hash,
        })
    });
    let verify = Arc::new(move |reference: &TheoryRevisionRef| {
        require_registry(reference, "authority_policy")?;
        require_found(
            store
                .resolve(&reference.id, &reference.content_hash)
                .map_err(owner_failure)?
                .is_some(),
        )
    });
    Arc::new(PortBackedTheoryRouteHandler::new(
        contract("execution", "authority-policy", RouteCardinality::Many),
        validate,
        install,
        verify,
        no_semantic_links(),
    ))
}

fn world_ref(reference: WorldModelTheoryRevisionRef) -> TheoryRevisionRef {
    TheoryRevisionRef {
        registry: reference.registry,
        id: reference.id,
        content_hash: reference.content_hash,
    }
}

fn capability_owner_id(contract: &CapabilityTypeContract) -> String {
    format!(
        "{}.v{}",
        contract.capability_type_id, contract.capability_version
    )
}

fn capability_ref_id(reference: &CapabilityRef) -> String {
    format!(
        "{}.v{}",
        reference.capability_type_id, reference.capability_version
    )
}

fn parse_capability_ref_id(value: &str) -> Result<CapabilityRef, OwnerRouteDiagnostic> {
    let (type_id, version) = value
        .rsplit_once(".v")
        .ok_or_else(|| owner("owner_ref_invalid", "capability ref id has no version"))?;
    Ok(CapabilityRef {
        capability_type_id: type_id.to_string(),
        capability_version: version
            .parse()
            .map_err(|_| owner("owner_ref_invalid", "capability version is invalid"))?,
    })
}

fn decode<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, OwnerRouteDiagnostic> {
    serde_json::from_slice(bytes).map_err(owner_failure)
}

fn require_id(expected: &str, actual: &str) -> Result<(), OwnerRouteDiagnostic> {
    if expected == actual {
        Ok(())
    } else {
        Err(owner(
            "owner_id_mismatch",
            format!("routed owner id '{expected}' differs from body id '{actual}'"),
        ))
    }
}

fn require_registry(
    reference: &TheoryRevisionRef,
    expected: &str,
) -> Result<(), OwnerRouteDiagnostic> {
    if reference.registry == expected {
        Ok(())
    } else {
        Err(owner(
            "owner_ref_invalid",
            format!(
                "owner ref registry '{}' differs from '{expected}'",
                reference.registry
            ),
        ))
    }
}

fn require_found(found: bool) -> Result<(), OwnerRouteDiagnostic> {
    if found {
        Ok(())
    } else {
        Err(owner(
            "owner_revision_missing",
            "exact owner revision is absent",
        ))
    }
}

fn owner(code: impl Into<String>, message: impl Into<String>) -> OwnerRouteDiagnostic {
    OwnerRouteDiagnostic::new(code, message)
}

fn owner_failure(failure: impl ToString) -> OwnerRouteDiagnostic {
    owner("owner_contract_invalid", failure.to_string())
}

fn product_topology_handler(
    store: Arc<crate::theory::PdsProductStore>,
) -> Arc<dyn TheoryRouteHandler> {
    use crate::theory::ProductTopologyV1;
    let install = store.clone();
    Arc::new(PortBackedTheoryRouteHandler::new(
        contract("runtime", "product-topology", RouteCardinality::Many),
        Arc::new(|id, bytes| {
            let topology: ProductTopologyV1 = decode(bytes)?;
            require_id(id, &topology.topology_id)?;
            topology.validate().map_err(owner_failure)
        }),
        Arc::new(move |id, bytes, _| {
            let topology: ProductTopologyV1 = decode(bytes)?;
            require_id(id, &topology.topology_id)?;
            install.install_topology(&topology).map_err(owner_failure)
        }),
        Arc::new(move |reference| {
            require_registry(reference, crate::theory::PRODUCT_TOPOLOGY_REGISTRY)?;
            require_found(store.topology(reference).map_err(owner_failure)?.is_some())
        }),
        no_semantic_links(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::storage::ProductStorageLayout;

    #[test]
    fn current_owner_routes_publish_without_duplicates() {
        let root = tempfile::tempdir().unwrap();
        let stores =
            OpenProductStores::open(&ProductStorageLayout::from_root(root.path())).unwrap();
        let catalog = current_product_route_catalog(&stores).unwrap();
        let routes: Vec<String> = catalog.routes().map(TheoryRouteId::key).collect();
        assert_eq!(routes.len(), 10);
        assert_eq!(
            routes,
            vec![
                "execution.authority-policy.v1",
                "execution.capability-contract.v1",
                "runtime.product-topology.v1",
                "world-model.agent-curation-rule.v1",
                "world-model.agent-maintained-condition.v1",
                "world-model.belief-family.v1",
                "world-model.epistemic-curation-rule.v1",
                "world-model.graph-owner-event-route.v1",
                "world-model.outcome-mapping.v1",
                "world-model.strategy-theory.v1",
            ]
        );
    }
}
