//! Bind the code-change owner's epoch observation to installed Curation products.

use super::*;

pub struct ProductCodeChangeEpochPreparation {
    curation: Arc<CurationStore>,
    traversal: Arc<TraversalStore>,
    template: TheoryRevisionRef,
    event_watermark: meld_events::EventWatermarkCapability,
}

impl ProductCodeChangeEpochPreparation {
    pub fn new(
        curation: Arc<CurationStore>,
        traversal: Arc<TraversalStore>,
        template: TheoryRevisionRef,
        strategy: &StrategyTheoryPackage,
        event_watermark: meld_events::EventWatermarkCapability,
    ) -> Result<Self, StorageError> {
        let contract = crate::code_change::capability::contract();
        if strategy
            .capabilities
            .iter()
            .filter(|capability| capability.contract_id == contract.content_identity())
            .count()
            != 1
            || strategy
                .snapshot
                .settlement_rules
                .iter()
                .any(|rule| !rule.has_evidence_returns() || rule.requires_effect_visibility())
            || curation.resolve_template(&template)?.is_none()
        {
            return Err(StorageError::InvalidPath("code-change preparation requires the exact materializer and post-Task confirmation".into()));
        }
        Ok(Self {
            curation,
            traversal,
            template,
            event_watermark,
        })
    }
}

impl AgentEpochPreparationPort for ProductCodeChangeEpochPreparation {
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
                "code-change Curation template is outside Agent genesis".into(),
            ));
        }
        let authority = &specification.authority;
        let (scope, root) = if specification.is_prepared_request() {
            crate::code_change::publication::request_observation_source(
                &authority.agent_id,
                &authority.subject,
                &specification.goal_id,
            )
        } else {
            crate::code_change::publication::observation_source(
                &authority.agent_id,
                &authority.subject,
                specification
                    .fence
                    .admission_epoch
                    .as_deref()
                    .ok_or_else(|| StorageError::InvalidPath("code-change epoch absent".into()))?,
            )
        }
        .map_err(StorageError::InvalidPath)?;
        let expected = crate::code_change::publication::graph_route();
        let route = self
            .traversal
            .owner_event_route(&expected.owner_id, &expected.event_type)?
            .filter(|route| route == &expected)
            .ok_or_else(|| {
                StorageError::InvalidPath("exact code-change Graph route is not installed".into())
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
                scope,
                roots: vec![root],
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
            effect_visibility: None,
            specification: specification.clone(),
            observation_subject: rule.rule.expected_object()?,
            task_inputs: vec![],
            curation_rule: rule,
        };
        products.validate()?;
        Ok(products)
    }
}
