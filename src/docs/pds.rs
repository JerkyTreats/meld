//! Regression fixture for the retired hand-composed docs PDS image.
//!
//! Production composition lowers declarations through installed receipts.
//! These tests retain the original direct search and lowering proof while
//! using the same owner activation contracts as production.

use meld_events::DomainObjectRef;
use meld_world_model::{AgentStrategyRuntimeConfig, StrategyTheoryPackage};

use crate::capability::{CapabilityCatalog, CapabilityExecutorRegistry, CapabilityTypeContract};
use crate::docs::capability::DocsCapabilityConfig;
use crate::docs::claim_validation::DocsClaimPolicy;
use crate::error::ApiError;

#[derive(Clone)]
pub struct DocsPdsRuntime {
    pub catalog: CapabilityCatalog,
    pub registry: CapabilityExecutorRegistry,
    pub strategy: AgentStrategyRuntimeConfig,
    pub requested_dimensions: Vec<String>,
}

#[cfg(test)]
pub fn compose(config: DocsCapabilityConfig) -> Result<DocsPdsRuntime, ApiError> {
    let package = serde_json::from_str(include_str!(
        "../../theory/docs_freshness/strategy_theory.docs_freshness.json"
    ))
    .map_err(|error| ApiError::ConfigError(error.to_string()))?;
    let claim_policy = serde_json::from_str(include_str!(
        "../../theory/docs_freshness/claim_policy.docs-claims-strict-v1.json"
    ))
    .map_err(|error| ApiError::ConfigError(error.to_string()))?;
    compose_with_theory(
        config,
        package,
        claim_policy,
        crate::docs::capability::published_contracts(),
    )
}

/// Compose docs executors against one exact receipt-resolved theory image.
pub fn compose_with_theory(
    config: DocsCapabilityConfig,
    package: StrategyTheoryPackage,
    claim_policy: DocsClaimPolicy,
    exact_contracts: Vec<CapabilityTypeContract>,
) -> Result<DocsPdsRuntime, ApiError> {
    let subject = DomainObjectRef::new("workspace_fs", "node", config.subject_id.clone())?;
    let mut catalog = CapabilityCatalog::new();
    let mut registry = CapabilityExecutorRegistry::new();
    crate::docs::capability::register_exact_contracts(
        config.clone(),
        claim_policy,
        &exact_contracts,
        &mut catalog,
        &mut registry,
    )?;
    if catalog.iter().count() != exact_contracts.len() {
        return Err(ApiError::ConfigError(
            "receipt executable contract set is incomplete for docs runtime".to_string(),
        ));
    }
    for capability in &package.capabilities {
        let specific = capability
            .operator
            .resolution
            .specific
            .as_ref()
            .ok_or_else(|| {
                ApiError::ConfigError(format!(
                    "strategy capability '{}' is not pinned to an executable contract",
                    capability.operator.operator_id
                ))
            })?;
        let contract = catalog
            .get(&specific.capability_type_id, specific.capability_version)
            .ok_or_else(|| {
                ApiError::ConfigError(format!(
                    "strategy capability '{}' version '{}' is absent from the receipt catalog",
                    specific.capability_type_id, specific.capability_version
                ))
            })?;
        if contract.content_identity() != capability.contract_id {
            return Err(ApiError::ConfigError(format!(
                "strategy capability contract identity drift for '{}' version '{}'",
                specific.capability_type_id, specific.capability_version
            )));
        }
    }
    let requested_dimensions = package.requested_dimensions.clone();
    let strategy =
        AgentStrategyRuntimeConfig::activate_installed(package, subject, config.agent_id.clone())
            .map_err(|error| ApiError::ConfigError(error.to_string()))?;
    Ok(DocsPdsRuntime {
        catalog,
        registry,
        strategy,
        requested_dimensions,
    })
}
