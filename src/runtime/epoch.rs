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
