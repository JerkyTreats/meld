//! Structural handoff from native Agent specification to independently owned products.

use std::sync::Arc;

use meld_world_model::agent::{
    AgentEpochPreparationPort, AgentEpochProducts, AgentEpochSpecification,
};
use meld_world_model::belief::TheoryRevisionRef;
use meld_world_model::curation::{
    CurationJudgmentScope, CurationRuleBinding, CurationSourceBinding, CurationStore,
};
use meld_world_model::error::StorageError;
use meld_world_model::strategy::StrategyTheoryPackage;
use meld_world_model::world_state::graph::contracts::OwnerPublicationScope;
use meld_world_model::world_state::graph::store::TraversalStore;

#[cfg(test)]
mod tests;

#[cfg(unix)]
pub mod code_change;

/// Read-only owner acceptance boundary for one native Agent epoch.
pub trait AgentEpochOwnerPort: Send + Sync {
    fn owner_id(&self) -> &str;

    fn prepare_agent_epoch(
        &self,
        specification: &AgentEpochSpecification,
    ) -> Result<crate::runtime::owners::OwnerAgentEpochProductsV1, String>;
}

impl AgentEpochOwnerPort for crate::runtime::owners::runtime::PreparedOwnerRuntime {
    fn owner_id(&self) -> &str {
        crate::runtime::owners::runtime::PreparedOwnerRuntime::owner_id(self)
    }

    fn prepare_agent_epoch(
        &self,
        specification: &AgentEpochSpecification,
    ) -> Result<crate::runtime::owners::OwnerAgentEpochProductsV1, String> {
        crate::runtime::owners::runtime::PreparedOwnerRuntime::prepare_agent_epoch(
            self,
            specification,
        )
        .map_err(|error| error.to_string())
    }
}

/// Bind owner-selected request inputs through installed native contracts.
pub struct ProductOwnerEpochPreparation {
    owner: Arc<dyn AgentEpochOwnerPort>,
    curation: Arc<CurationStore>,
    traversal: Arc<TraversalStore>,
    template: TheoryRevisionRef,
    strategy: StrategyTheoryPackage,
    catalog: meld_execution::capability::CapabilityCatalog,
    event_watermark: meld_events::EventWatermarkCapability,
}

impl ProductOwnerEpochPreparation {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        owner: Arc<dyn AgentEpochOwnerPort>,
        curation: Arc<CurationStore>,
        traversal: Arc<TraversalStore>,
        template: TheoryRevisionRef,
        strategy: &StrategyTheoryPackage,
        catalog: meld_execution::capability::CapabilityCatalog,
        event_watermark: meld_events::EventWatermarkCapability,
    ) -> Result<Self, StorageError> {
        let revision = curation.resolve_template(&template)?.ok_or_else(|| {
            StorageError::InvalidPath("epoch Curation template is not installed".into())
        })?;
        if revision.template.source_owner_id != owner.owner_id() {
            return Err(StorageError::InvalidPath(
                "epoch Curation template belongs to another owner".into(),
            ));
        }
        Ok(Self {
            owner,
            curation,
            traversal,
            template,
            strategy: strategy.clone(),
            catalog,
            event_watermark,
        })
    }

    fn validate_task_input(&self, input: &meld_lang::TaskInput) -> Result<(), StorageError> {
        input.validate().map_err(StorageError::InvalidPath)?;
        let capability = self
            .strategy
            .capabilities
            .iter()
            .find(|capability| capability.operator.operator_id == input.step_id)
            .ok_or_else(|| {
                StorageError::InvalidPath(
                    "owner epoch input names an operator outside installed Strategy".into(),
                )
            })?;
        let selector = capability
            .operator
            .resolution
            .specific
            .as_ref()
            .ok_or_else(|| {
                StorageError::InvalidPath(
                    "owner epoch input names a floating capability operator".into(),
                )
            })?;
        let contract = self
            .catalog
            .get(&selector.capability_type_id, selector.capability_version)
            .ok_or_else(|| {
                StorageError::InvalidPath(
                    "owner epoch input capability is absent from the installed catalog".into(),
                )
            })?;
        if contract.content_identity() != capability.contract_id
            || contract.owning_domain != self.owner.owner_id()
        {
            return Err(StorageError::InvalidPath(
                "owner epoch input capability differs from the selected owner contract".into(),
            ));
        }
        let slot = contract
            .input_contract
            .iter()
            .find(|slot| slot.slot_id == input.slot_id)
            .ok_or_else(|| {
                StorageError::InvalidPath(
                    "owner epoch input names an undeclared capability slot".into(),
                )
            })?;
        if !slot
            .accepted_artifact_type_ids
            .contains(&input.artifact_type_id)
            || input.schema_version < slot.schema_versions.min
            || input.schema_version > slot.schema_versions.max
            || !capability
                .operator
                .resolution
                .requires_inputs
                .iter()
                .any(|required| {
                    required.artifact_type
                        == meld_lang::Term::ArtifactType(input.artifact_type_id.clone())
                })
        {
            return Err(StorageError::InvalidPath(
                "owner epoch input differs from its exact Strategy and capability slot".into(),
            ));
        }
        Ok(())
    }

    fn validate_source_route(
        &self,
        source: &meld_world_model::curation::CurationSourceBinding,
    ) -> Result<(), StorageError> {
        let Some(expected) = &source.event_source else {
            return Ok(());
        };
        validate_complete_source_route(&self.traversal, self.owner.owner_id(), expected)
    }
}

fn validate_complete_source_route(
    traversal: &TraversalStore,
    owner_id: &str,
    expected: &meld_world_model::world_state::graph::admission::OwnerEventSourceRef,
) -> Result<(), StorageError> {
    expected.validate()?;
    let installed = traversal.owner_event_routes()?.into_iter().any(|route| {
        route.complete_event_source
            && route.owner_id == owner_id
            && route.source_ref().is_ok_and(|actual| &actual == expected)
    });
    if !installed {
        return Err(StorageError::InvalidPath(
            "owner epoch source route is not installed as a complete Event source for the selected owner"
                .into(),
        ));
    }
    Ok(())
}

impl AgentEpochPreparationPort for ProductOwnerEpochPreparation {
    fn prepare(
        &self,
        specification: &AgentEpochSpecification,
    ) -> Result<AgentEpochProducts, StorageError> {
        specification.validate()?;
        if !specification
            .genesis
            .installed_owner_revisions
            .contains(&self.template)
        {
            return Err(StorageError::InvalidPath(
                "epoch Curation template is outside Agent genesis".into(),
            ));
        }
        let prepared = self
            .owner
            .prepare_agent_epoch(specification)
            .map_err(StorageError::InvalidPath)?;
        if prepared.specification_id != specification.specification_id {
            return Err(StorageError::InvalidPath(
                "owner epoch products belong to another specification".into(),
            ));
        }
        if let Some(expected) = &prepared.effect_visibility {
            expected.validate()?;
            if expected.owner_id != self.owner.owner_id() {
                return Err(StorageError::InvalidPath(
                    "owner epoch effect visibility belongs to another owner".into(),
                ));
            }
        }
        self.validate_source_route(&prepared.curation_source)?;
        for input in &prepared.task_inputs {
            self.validate_task_input(input)?;
        }
        let authority = &specification.authority;
        let rule = self.curation.prepare_rule_for_source(
            &self.template,
            &CurationRuleBinding {
                agent_id: authority.agent_id.clone(),
                subject: authority.subject.clone(),
                scope: OwnerPublicationScope {
                    scope_id: specification.genesis.registration.observation_scope.clone(),
                    branch_id: Some(authority.branch_scope.branch_id.clone()),
                    perspective_id: Some(authority.perspective.perspective_id.clone()),
                    valid_at: None,
                },
            },
            &prepared.curation_source,
            &CurationJudgmentScope {
                subject: authority.subject.clone(),
                perspective: authority.perspective.clone(),
                branch_scope: authority.branch_scope.clone(),
            },
            self.event_watermark
                .snapshot()
                .map_err(|error| StorageError::InvalidPath(error.to_string()))?
                .committed_seq,
        )?;
        let products = AgentEpochProducts {
            effect_visibility: prepared.effect_visibility,
            specification: specification.clone(),
            observation_subject: rule.rule.expected_object()?,
            task_inputs: prepared.task_inputs,
            curation_rule: rule,
        };
        products.validate()?;
        Ok(products)
    }
}

/// Bind an exact installed nonce Capability and Curation template. All semantic
/// products are constructed by their native owners; this port only passes refs.
pub struct ProductNonceEpochPreparation {
    curation: Arc<CurationStore>,
    traversal: Arc<TraversalStore>,
    template: TheoryRevisionRef,
    step_id: String,
    event_watermark: meld_events::EventWatermarkCapability,
}

impl ProductNonceEpochPreparation {
    pub fn new(
        curation: Arc<CurationStore>,
        traversal: Arc<TraversalStore>,
        template: TheoryRevisionRef,
        strategy: &StrategyTheoryPackage,
        event_watermark: meld_events::EventWatermarkCapability,
    ) -> Result<Self, StorageError> {
        let contract = crate::nonce::capability::contract();
        let selected: Vec<_> = strategy
            .capabilities
            .iter()
            .filter(|capability| {
                capability.contract_id == contract.content_identity()
                    && capability
                        .operator
                        .resolution
                        .specific
                        .as_ref()
                        .is_some_and(|reference| {
                            reference.capability_type_id == contract.capability_type_id
                                && reference.capability_version == contract.capability_version
                        })
            })
            .collect();
        let [selected] = selected.as_slice() else {
            return Err(StorageError::InvalidPath(
                "epoch preparation requires one exact installed nonce operator".into(),
            ));
        };
        if !selected
            .operator
            .resolution
            .requires_inputs
            .iter()
            .any(|input| {
                input.required
                    && input.artifact_type
                        == meld_lang::Term::ArtifactType(crate::nonce::capability::REQUEST.into())
            })
        {
            return Err(StorageError::InvalidPath(
                "nonce operator does not declare its frozen request input".into(),
            ));
        }
        if curation.resolve_template(&template)?.is_none() {
            return Err(StorageError::InvalidPath(
                "epoch Curation template is not installed".into(),
            ));
        }
        Ok(Self {
            curation,
            traversal,
            template,
            step_id: selected.operator.operator_id.clone(),
            event_watermark,
        })
    }
}

impl AgentEpochPreparationPort for ProductNonceEpochPreparation {
    fn prepare(
        &self,
        specification: &AgentEpochSpecification,
    ) -> Result<AgentEpochProducts, StorageError> {
        specification.validate()?;
        if !specification
            .genesis
            .installed_owner_revisions
            .contains(&self.template)
        {
            return Err(StorageError::InvalidPath(
                "epoch Curation template is outside Agent genesis".into(),
            ));
        }
        let authority = &specification.authority;
        let request = crate::nonce::NonceRequest::new(
            authority.agent_id.clone(),
            authority.subject.clone(),
            specification.effect_correlations(),
            specification.fence.admission_epoch.clone().ok_or_else(|| {
                StorageError::InvalidPath("nonce epoch has no admission identity".into())
            })?,
        )
        .map_err(|error| StorageError::InvalidPath(error.to_string()))?;
        let route = self
            .traversal
            .owner_event_route(crate::nonce::OWNER_ID, crate::nonce::EVENT_TYPE)?
            .filter(|route| route == &crate::nonce::graph_route())
            .ok_or_else(|| {
                StorageError::InvalidPath("exact nonce Graph route is not installed".into())
            })?;
        let rule = self.curation.prepare_rule_for_source(
            &self.template,
            &CurationRuleBinding {
                agent_id: authority.agent_id.clone(),
                subject: authority.subject.clone(),
                scope: OwnerPublicationScope {
                    scope_id: specification.genesis.registration.observation_scope.clone(),
                    branch_id: Some(authority.branch_scope.branch_id.clone()),
                    perspective_id: Some(authority.perspective.perspective_id.clone()),
                    valid_at: None,
                },
            },
            &CurationSourceBinding {
                scope: request.publication_scope(),
                roots: vec![request
                    .object_ref()
                    .map_err(|error| StorageError::InvalidPath(error.to_string()))?],
                event_source: Some(route.source_ref()?),
            },
            &CurationJudgmentScope {
                subject: authority.subject.clone(),
                perspective: authority.perspective.clone(),
                branch_scope: authority.branch_scope.clone(),
            },
            self.event_watermark
                .snapshot()
                .map_err(|error| StorageError::InvalidPath(error.to_string()))?
                .committed_seq,
        )?;
        let products = AgentEpochProducts {
            effect_visibility: Some(meld_world_model::world_state::graph::contracts::OwnerPublicationExpectation::from_operation(
                &request.publication().map_err(|error| StorageError::InvalidPath(error.to_string()))?
            )?),
            specification: specification.clone(),
            observation_subject: rule.rule.expected_object()?,
            task_inputs: vec![meld_lang::TaskInput {
                step_id: self.step_id.clone(),
                slot_id: crate::nonce::capability::REQUEST.into(),
                artifact_type_id: crate::nonce::capability::REQUEST.into(),
                schema_version: 1,
                content: serde_json::to_value(request)
                    .map_err(|error| StorageError::InvalidPath(error.to_string()))?,
            }],
            curation_rule: rule,
        };
        products.validate()?;
        Ok(products)
    }
}
