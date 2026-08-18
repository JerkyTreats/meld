use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use meld_execution::capability::{
    CapabilityContractRevision, CapabilityContractRevisionRef, CapabilityInvoker,
};

use crate::error::ApiError;
use crate::execution::ExecutionRuntimeContext;

use super::{CapabilityCatalog, CapabilityExecutorRegistry, ExecutionClass};

pub type PreparedCapabilityInvoker =
    Arc<dyn CapabilityInvoker<Error = ApiError, ExecutionApi = dyn ExecutionRuntimeContext>>;

#[derive(Debug, Clone, Default)]
pub struct OwnerBindingView {
    values: BTreeMap<String, String>,
}

impl OwnerBindingView {
    pub fn new(values: BTreeMap<String, String>) -> Self {
        Self { values }
    }

    pub fn get(&self, binding_id: &str) -> Option<&str> {
        self.values.get(binding_id).map(String::as_str)
    }

    pub fn contains(&self, binding_id: &str) -> bool {
        self.values.contains_key(binding_id)
    }
}

#[derive(Debug, Clone)]
pub struct ExactCapabilityActivationRequest {
    pub assignment_id: String,
    pub activation_id: String,
    pub selected_contracts: Vec<CapabilityContractRevisionRef>,
    pub selected_implementations: BTreeMap<CapabilityContractRevisionRef, String>,
}

pub struct CapabilityFactoryRequest<'a> {
    pub assignment_id: &'a str,
    pub activation_id: &'a str,
    pub contract_ref: &'a CapabilityContractRevisionRef,
}

pub trait CapabilityInvokerFactory: Send + Sync {
    fn prepare(
        &self,
        request: &CapabilityFactoryRequest<'_>,
        bindings: &OwnerBindingView,
    ) -> Result<PreparedCapabilityInvoker, CapabilityContributionDiagnostic>;
}

#[derive(Clone)]
pub struct CapabilityImplementationOffer {
    pub contract_ref: CapabilityContractRevisionRef,
    pub implementation_ref: String,
    pub required_binding_ids: BTreeSet<String>,
    pub execution_class: ExecutionClass,
    pub factory: Arc<dyn CapabilityInvokerFactory>,
}

pub trait ProductCapabilityContributor: Send + Sync {
    fn owner_domain(&self) -> &str;
    fn published_contracts(&self) -> Vec<CapabilityContractRevision>;
    fn implementation_offers(&self) -> Vec<CapabilityImplementationOffer>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityContributionReceipt {
    pub owner_domain: String,
    pub contract_ref: CapabilityContractRevisionRef,
    pub implementation_ref: String,
}

pub struct PreparedCapabilityClosure {
    pub assignment_id: String,
    pub activation_id: String,
    pub contracts: CapabilityCatalog,
    pub invokers: CapabilityExecutorRegistry,
    pub contribution_receipts: Vec<CapabilityContributionReceipt>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{code}: {message}")]
pub struct CapabilityContributionDiagnostic {
    pub code: String,
    pub message: String,
    pub owner_domain: Option<String>,
    pub contract_ref: Option<CapabilityContractRevisionRef>,
    pub implementation_ref: Option<String>,
    pub assignment_id: Option<String>,
    pub activation_id: Option<String>,
}

impl CapabilityContributionDiagnostic {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            owner_domain: None,
            contract_ref: None,
            implementation_ref: None,
            assignment_id: None,
            activation_id: None,
        }
    }
}

pub struct ProductCapabilityInventory {
    contracts: BTreeMap<(String, u32), CapabilityContractRevision>,
    offers: BTreeMap<(CapabilityContractRevisionRef, String), CapabilityImplementationOffer>,
    offer_owners: BTreeMap<(CapabilityContractRevisionRef, String), String>,
}

impl ProductCapabilityInventory {
    pub fn assemble(
        mut contributors: Vec<Arc<dyn ProductCapabilityContributor>>,
    ) -> Result<Self, CapabilityContributionDiagnostic> {
        contributors.sort_by(|left, right| left.owner_domain().cmp(right.owner_domain()));
        let mut contracts = BTreeMap::new();
        let mut offers = BTreeMap::new();
        let mut offer_owners = BTreeMap::new();
        for contributor in contributors {
            let owner = contributor.owner_domain().to_string();
            if owner.trim().is_empty() {
                return Err(CapabilityContributionDiagnostic::new(
                    "capability_contract_conflict",
                    "contributor owner domain is empty",
                ));
            }
            let mut published = contributor.published_contracts();
            published.sort_by(|left, right| {
                left.contract
                    .capability_type_id
                    .cmp(&right.contract.capability_type_id)
                    .then(
                        left.contract
                            .capability_version
                            .cmp(&right.contract.capability_version),
                    )
                    .then(left.content_identity.cmp(&right.content_identity))
            });
            for revision in published {
                revision.contract.validate().map_err(|failure| {
                    CapabilityContributionDiagnostic::new(
                        "capability_contract_conflict",
                        failure.to_string(),
                    )
                })?;
                if revision.content_identity != revision.contract.content_identity() {
                    return Err(CapabilityContributionDiagnostic::new(
                        "capability_contract_conflict",
                        "published revision content identity differs from its contract",
                    ));
                }
                let key = (
                    revision.contract.capability_type_id.clone(),
                    revision.contract.capability_version,
                );
                if let Some(existing) = contracts.get(&key) {
                    if existing != &revision {
                        return Err(CapabilityContributionDiagnostic::new(
                            "capability_contract_conflict",
                            format!(
                                "selector '{}' version {} has different content",
                                key.0, key.1
                            ),
                        ));
                    }
                } else {
                    contracts.insert(key, revision);
                }
            }
            let mut published_offers = contributor.implementation_offers();
            published_offers.sort_by(|left, right| {
                left.contract_ref
                    .cmp(&right.contract_ref)
                    .then(left.implementation_ref.cmp(&right.implementation_ref))
            });
            for offer in published_offers {
                let selector = (
                    offer.contract_ref.selector.capability_type_id.clone(),
                    offer.contract_ref.selector.capability_version,
                );
                let Some(contract) = contracts.get(&selector) else {
                    return Err(CapabilityContributionDiagnostic::new(
                        "capability_contract_conflict",
                        "offer cites an unpublished contract",
                    ));
                };
                if contract.content_identity != offer.contract_ref.content_identity {
                    return Err(CapabilityContributionDiagnostic::new(
                        "capability_contract_conflict",
                        "offer cites a different exact contract identity",
                    ));
                }
                let key = (offer.contract_ref.clone(), offer.implementation_ref.clone());
                if offers.insert(key.clone(), offer).is_some() {
                    return Err(CapabilityContributionDiagnostic::new(
                        "capability_implementation_conflict",
                        "duplicate implementation ref for exact contract",
                    ));
                }
                offer_owners.insert(key, owner.clone());
            }
        }
        Ok(Self {
            contracts,
            offers,
            offer_owners,
        })
    }

    pub fn contracts(&self) -> impl Iterator<Item = &CapabilityContractRevision> {
        self.contracts.values()
    }

    pub fn unique_implementation_ref(
        &self,
        contract_ref: &CapabilityContractRevisionRef,
    ) -> Result<String, CapabilityContributionDiagnostic> {
        let mut matches = self
            .offers
            .keys()
            .filter(|key| &key.0 == contract_ref)
            .map(|key| key.1.clone());
        let Some(implementation_ref) = matches.next() else {
            return Err(CapabilityContributionDiagnostic::new(
                "selected_implementation_missing",
                "exact contract has no published implementation",
            ));
        };
        if matches.next().is_some() {
            return Err(CapabilityContributionDiagnostic::new(
                "selected_implementation_ambiguous",
                "exact contract has several implementations and requires physical selection",
            ));
        }
        Ok(implementation_ref)
    }

    pub fn prepare(
        &self,
        mut request: ExactCapabilityActivationRequest,
        bindings: &OwnerBindingView,
    ) -> Result<PreparedCapabilityClosure, CapabilityContributionDiagnostic> {
        request.selected_contracts.sort();
        request.selected_contracts.dedup();
        let selected: BTreeSet<_> = request.selected_contracts.iter().cloned().collect();
        if request
            .selected_implementations
            .keys()
            .any(|key| !selected.contains(key))
        {
            return Err(diagnostic_for(
                "prepared_invoker_extra",
                "implementation selected for an unselected contract",
                &request,
            ));
        }
        let mut catalog = CapabilityCatalog::new();
        let mut invokers = CapabilityExecutorRegistry::new();
        let mut receipts = Vec::new();
        for exact in &request.selected_contracts {
            let selector = (
                exact.selector.capability_type_id.clone(),
                exact.selector.capability_version,
            );
            let Some(contract) = self.contracts.get(&selector) else {
                return Err(diagnostic_for(
                    "selected_contract_missing",
                    "selected contract is absent from product inventory",
                    &request,
                ));
            };
            if contract.content_identity != exact.content_identity {
                return Err(diagnostic_for(
                    "selected_contract_identity_mismatch",
                    "selected contract content identity differs",
                    &request,
                ));
            }
            let Some(implementation_ref) = request.selected_implementations.get(exact) else {
                return Err(diagnostic_for(
                    "selected_implementation_missing",
                    "selected contract has no implementation selection",
                    &request,
                ));
            };
            let key = (exact.clone(), implementation_ref.clone());
            let Some(offer) = self.offers.get(&key) else {
                return Err(diagnostic_for(
                    "selected_implementation_missing",
                    "selected implementation is absent from product inventory",
                    &request,
                ));
            };
            if let Some(missing) = offer
                .required_binding_ids
                .iter()
                .find(|binding_id| !bindings.contains(binding_id))
            {
                return Err(diagnostic_for(
                    "selected_binding_missing",
                    format!("selected implementation requires binding '{missing}'"),
                    &request,
                ));
            }
            let invoker = offer.factory.prepare(
                &CapabilityFactoryRequest {
                    assignment_id: &request.assignment_id,
                    activation_id: &request.activation_id,
                    contract_ref: exact,
                },
                bindings,
            )?;
            let published = invoker.contract();
            if published.capability_type_id != exact.selector.capability_type_id
                || published.capability_version != exact.selector.capability_version
                || published.content_identity() != exact.content_identity
            {
                return Err(diagnostic_for(
                    "prepared_invoker_identity_mismatch",
                    "factory returned an invoker for another exact contract",
                    &request,
                ));
            }
            invokers
                .register_arc(&mut catalog, invoker)
                .map_err(|failure| {
                    diagnostic_for("prepared_invoker_duplicate", failure.to_string(), &request)
                })?;
            receipts.push(CapabilityContributionReceipt {
                owner_domain: self.offer_owners[&key].clone(),
                contract_ref: exact.clone(),
                implementation_ref: implementation_ref.clone(),
            });
        }
        Ok(PreparedCapabilityClosure {
            assignment_id: request.assignment_id,
            activation_id: request.activation_id,
            contracts: catalog,
            invokers,
            contribution_receipts: receipts,
        })
    }
}

impl crate::theory::PublishedComponentResolver for ProductCapabilityInventory {
    fn resolve(
        &self,
        publisher: &str,
        id: &str,
        content_identity: &str,
    ) -> Result<Vec<u8>, crate::theory::TheoryRouterError> {
        if publisher != "execution.capability-contract.v1" {
            return Err(crate::theory::TheoryRouterDiagnostic::new(
                "package_source_invalid",
                format!("unknown compiled publisher '{publisher}'"),
            )
            .into());
        }
        let (type_id, version) = id.rsplit_once(".v").ok_or_else(|| {
            crate::theory::TheoryRouterError::from(crate::theory::TheoryRouterDiagnostic::new(
                "package_source_invalid",
                "published capability id has no version",
            ))
        })?;
        let version: u32 = version.parse().map_err(|_| {
            crate::theory::TheoryRouterError::from(crate::theory::TheoryRouterDiagnostic::new(
                "package_source_invalid",
                "published capability version is invalid",
            ))
        })?;
        let revision = self
            .contracts
            .get(&(type_id.to_string(), version))
            .ok_or_else(|| {
                crate::theory::TheoryRouterError::from(crate::theory::TheoryRouterDiagnostic::new(
                    "package_source_invalid",
                    "published capability is absent",
                ))
            })?;
        if revision.content_identity != content_identity {
            return Err(crate::theory::TheoryRouterDiagnostic::new(
                "package_identity_mismatch",
                "published capability exact identity differs",
            )
            .into());
        }
        serde_json::to_vec(&revision.contract).map_err(|failure| {
            crate::theory::TheoryRouterError::from(crate::theory::TheoryRouterDiagnostic::new(
                "package_source_invalid",
                failure.to_string(),
            ))
        })
    }
}

fn diagnostic_for(
    code: impl Into<String>,
    message: impl Into<String>,
    request: &ExactCapabilityActivationRequest,
) -> CapabilityContributionDiagnostic {
    let mut diagnostic = CapabilityContributionDiagnostic::new(code, message);
    diagnostic.assignment_id = Some(request.assignment_id.clone());
    diagnostic.activation_id = Some(request.activation_id.clone());
    diagnostic
}
