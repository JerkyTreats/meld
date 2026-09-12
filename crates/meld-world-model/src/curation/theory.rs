//! Portable epistemic rules grounded by Curation for an assigned Agent.

use std::collections::BTreeMap;

use meld_events::DomainObjectRef;
use serde::{Deserialize, Serialize};

use super::{
    stable_identity, CurationJudgmentScope, CurationRealizationRule, StandingCurationRule,
};
use crate::belief::TheoryRevisionRef;
use crate::error::StorageError;
use crate::world_state::graph::contracts::{
    OwnerPublicationScope, TraversalBounds, TraversalDirection,
};

pub const CURATION_TEMPLATE_REGISTRY_ID: &str = "epistemic_curation_template";

/// Portable owner-qualified selection within the assignment's observation scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CurationObjectSelection {
    AssignedSubject,
    SourceRoot,
    Exact(DomainObjectRef),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CurationRealizationTemplate {
    pub observed_object: CurationObjectSelection,
    #[serde(default)]
    pub required_qualifications: BTreeMap<String, String>,
    pub realized_relation_type: String,
    pub not_realized_relation_type: String,
}

/// Installed semantic content independent of physical Agent and subject assignment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CurationRuleTemplate {
    #[serde(
        default,
        skip_serializing_if = "super::CurationSelectionPosture::is_standing"
    )]
    pub selection_posture: super::CurationSelectionPosture,
    pub rule_id: String,
    pub source_owner_id: String,
    pub traversal_direction: TraversalDirection,
    pub bounds: TraversalBounds,
    pub expected_object_kind: String,
    pub expected_object_key: String,
    pub relation_type: String,
    pub output_policy_revision: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub realization: Option<CurationRealizationTemplate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coverage: Option<super::CurationCoverageRule>,
}

/// Owner-issued selection of an independently scoped observation source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CurationSourceBinding {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_source: Option<crate::world_state::graph::admission::OwnerEventSourceRef>,
    pub scope: OwnerPublicationScope,
    pub roots: Vec<DomainObjectRef>,
}

/// Structural assignment supplied to the semantic owner at preparation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CurationRuleBinding {
    pub agent_id: String,
    pub subject: DomainObjectRef,
    pub scope: OwnerPublicationScope,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CurationTemplateRevision {
    pub template: CurationRuleTemplate,
    pub content_hash: String,
    pub installed_at_seq: u64,
}

impl CurationRuleTemplate {
    pub fn validate(&self) -> Result<(), StorageError> {
        let subject = DomainObjectRef::new(&self.source_owner_id, "template-subject", "validation")
            .map_err(|error| StorageError::InvalidPath(error.to_string()))?;
        self.ground(&CurationRuleBinding {
            agent_id: "template-validation".into(),
            subject,
            scope: OwnerPublicationScope {
                scope_id: "validation".into(),
                branch_id: None,
                perspective_id: None,
                valid_at: None,
            },
        })
        .map(|_| ())
    }

    pub fn ground(
        &self,
        binding: &CurationRuleBinding,
    ) -> Result<StandingCurationRule, StorageError> {
        self.ground_with_source(binding, None)
    }

    /// Ground source coverage independently from the Agent's exact judgment context.
    pub fn ground_for_source(
        &self,
        binding: &CurationRuleBinding,
        source: &CurationSourceBinding,
        judgment: &CurationJudgmentScope,
    ) -> Result<StandingCurationRule, StorageError> {
        judgment.validate()?;
        if judgment.subject != binding.subject
            || binding.scope.branch_id.as_deref() != Some(judgment.branch_scope.branch_id.as_str())
            || binding.scope.perspective_id.as_deref()
                != Some(judgment.perspective.perspective_id.as_str())
        {
            return Err(StorageError::InvalidPath(
                "Curation source binding has a foreign judgment context".into(),
            ));
        }
        self.ground_with_source(binding, Some((source, judgment)))
    }

    fn ground_with_source(
        &self,
        binding: &CurationRuleBinding,
        source: Option<(&CurationSourceBinding, &CurationJudgmentScope)>,
    ) -> Result<StandingCurationRule, StorageError> {
        let scope = source
            .map(|(source, _)| &source.scope)
            .unwrap_or(&binding.scope);
        let roots = source
            .map(|(source, _)| source.roots.clone())
            .unwrap_or_else(|| vec![binding.subject.clone()]);
        if roots.is_empty() {
            return Err(StorageError::InvalidPath(
                "Curation source has no roots".into(),
            ));
        }
        if self.expected_object_key.trim().is_empty() {
            return Err(StorageError::InvalidPath(
                "Curation expected object key is empty".into(),
            ));
        }
        let expected_object_id = stable_identity(
            "curation-assigned-expectation-v1",
            &(
                &self.rule_id,
                &self.expected_object_key,
                &binding.subject,
                &binding.scope,
            ),
        )?;
        let expected_object_id = match source {
            Some((source, judgment)) => stable_identity(
                "curation-source-expectation-v1",
                &(&expected_object_id, source, judgment),
            )?,
            None => expected_object_id,
        };
        let mut rule = StandingCurationRule {
            selection_posture: self.selection_posture,
            source_event_route: source.and_then(|(source, _)| source.event_source.clone()),
            judgment_scope: source.map(|(_, judgment)| judgment.clone()),
            rule_id: self.rule_id.clone(),
            agent_id: binding.agent_id.clone(),
            source_owner_id: self.source_owner_id.clone(),
            scope: scope.clone(),
            publication_scope: source
                .map(|(source, _)| {
                    let mut output = binding.scope.clone();
                    output.scope_id = stable_identity(
                        "curation-output-scope-v1",
                        &(
                            &binding.agent_id,
                            &self.rule_id,
                            &binding.subject,
                            &binding.scope,
                            &source.scope,
                        ),
                    )?;
                    Ok::<_, StorageError>(output)
                })
                .transpose()?,
            roots: roots.clone(),
            traversal_direction: self.traversal_direction,
            bounds: self.bounds.clone(),
            expected_object_kind: self.expected_object_kind.clone(),
            expected_object_id,
            relation_type: self.relation_type.clone(),
            output_policy_revision: self.output_policy_revision.clone(),
            coverage: self.coverage.clone(),
            realization: self
                .realization
                .as_ref()
                .map(|template| CurationRealizationRule {
                    observed_object: match &template.observed_object {
                        CurationObjectSelection::AssignedSubject => binding.subject.clone(),
                        CurationObjectSelection::SourceRoot => roots[0].clone(),
                        CurationObjectSelection::Exact(object) => object.clone(),
                    },
                    required_qualifications: template.required_qualifications.clone(),
                    realized_relation_type: template.realized_relation_type.clone(),
                    not_realized_relation_type: template.not_realized_relation_type.clone(),
                }),
        };
        if let Some(realization) = &rule.realization {
            if !rule.roots.contains(&realization.observed_object) {
                rule.roots.push(realization.observed_object.clone());
            }
        }
        rule.validate()?;
        Ok(rule)
    }
}

impl CurationTemplateRevision {
    pub fn validate(&self) -> Result<(), StorageError> {
        self.template.validate()?;
        if stable_identity("curation-rule-template-v1", &self.template)? != self.content_hash {
            return Err(StorageError::InvalidPath(
                "Curation template identity differs from its body".into(),
            ));
        }
        Ok(())
    }

    pub fn revision_ref(&self) -> TheoryRevisionRef {
        TheoryRevisionRef {
            registry: CURATION_TEMPLATE_REGISTRY_ID.into(),
            id: self.template.rule_id.clone(),
            content_hash: self.content_hash.clone(),
        }
    }
}
