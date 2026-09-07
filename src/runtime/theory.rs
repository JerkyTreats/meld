//! Prepared-product theory resolution and historical receipt reading.

use serde::{Deserialize, Serialize};
use sled::{Db, Tree};
use thiserror::Error;

use crate::config::SelectedStewardshipPackage;
use crate::docs::claim_validation::DocsClaimPolicyRevisionRef;
use meld_events::DomainObjectRef;
use meld_execution::authority::{AuthorityPolicyRevision, AuthorityPolicyRevisionRef};
use meld_execution::capability::CapabilityContractRevisionRef;
use meld_execution::capability::{CapabilityCatalog, CapabilityContractRevision};
use meld_world_model::agent::{AgentCurationRuleRevision, AgentMaintainedConditionRevision};
use meld_world_model::belief::{
    BeliefFamilyRegistry, BeliefFamilyRevision, OutcomeMappingRevision, TheoryRevisionRef,
};
use meld_world_model::strategy::StrategyTheoryRevision;

use crate::docs::claim_validation::DocsClaimPolicyRevision;
use crate::runtime::storage::OpenProductStores;
use crate::theory::{
    InstalledTheoryComponentRef, PreparedActivationClosureV1, ProductCompilationReceiptV1,
};

const TREE_RECEIPTS: &str = "theory_installation_receipts";

/// Historical complete cross-owner installation receipt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TheoryInstallationReceipt {
    /// Content-derived receipt identity.
    pub receipt_id: String,
    /// Stable selected package identities.
    pub selection: SelectedStewardshipPackage,
    /// Exact belief-family revision.
    pub belief_family: TheoryRevisionRef,
    /// Exact curation-rule revision.
    pub curation_rule: TheoryRevisionRef,
    /// Exact standing maintained-condition revision.
    pub maintained_condition: TheoryRevisionRef,
    /// Exact outcome-mapping revision.
    pub outcome_mapping: TheoryRevisionRef,
    /// Exact complete Strategy theory revision.
    pub strategy_theory: TheoryRevisionRef,
    /// Exact executable capability contract revisions.
    pub executable_contracts: Vec<CapabilityContractRevisionRef>,
    /// Exact effective-authority policy revision.
    pub authority_policy: AuthorityPolicyRevisionRef,
    /// Exact Docs policy when the product selects that owner component.
    #[serde(default)]
    pub claim_policy: Option<DocsClaimPolicyRevisionRef>,
    /// Sequence observed when the receipt was first installed.
    pub installed_at_seq: u64,
}

#[derive(Serialize)]
struct ReceiptIdentity<'a> {
    selection: &'a SelectedStewardshipPackage,
    belief_family: &'a TheoryRevisionRef,
    curation_rule: &'a TheoryRevisionRef,
    maintained_condition: &'a TheoryRevisionRef,
    outcome_mapping: &'a TheoryRevisionRef,
    strategy_theory: &'a TheoryRevisionRef,
    executable_contracts: &'a [CapabilityContractRevisionRef],
    authority_policy: &'a AuthorityPolicyRevisionRef,
    claim_policy: &'a Option<DocsClaimPolicyRevisionRef>,
}

impl TheoryInstallationReceipt {
    /// Construct a receipt and derive its exact identity.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        selection: SelectedStewardshipPackage,
        belief_family: TheoryRevisionRef,
        curation_rule: TheoryRevisionRef,
        maintained_condition: TheoryRevisionRef,
        outcome_mapping: TheoryRevisionRef,
        strategy_theory: TheoryRevisionRef,
        mut executable_contracts: Vec<CapabilityContractRevisionRef>,
        authority_policy: AuthorityPolicyRevisionRef,
        claim_policy: impl Into<Option<DocsClaimPolicyRevisionRef>>,
        installed_at_seq: u64,
    ) -> Result<Self, TheoryReceiptError> {
        let claim_policy = claim_policy.into();
        executable_contracts.sort_by(|left, right| {
            left.selector
                .capability_type_id
                .cmp(&right.selector.capability_type_id)
                .then(
                    left.selector
                        .capability_version
                        .cmp(&right.selector.capability_version),
                )
                .then(left.content_identity.cmp(&right.content_identity))
        });
        validate_selection(&selection)?;
        belief_family
            .validate_for_registry("belief_family")
            .map_err(invalid)?;
        curation_rule
            .validate_for_registry("agent_curation_rule")
            .map_err(invalid)?;
        maintained_condition
            .validate_for_registry("agent_maintained_condition")
            .map_err(invalid)?;
        outcome_mapping
            .validate_for_registry("outcome_mapping")
            .map_err(invalid)?;
        strategy_theory
            .validate_for_registry("strategy_theory")
            .map_err(invalid)?;
        if executable_contracts.is_empty() {
            return Err(TheoryReceiptError::Invalid(
                "receipt requires executable contract revisions".to_string(),
            ));
        }
        if authority_policy.policy_id.trim().is_empty()
            || authority_policy.content_hash.trim().is_empty()
        {
            return Err(TheoryReceiptError::Invalid(
                "receipt authority policy reference is incomplete".to_string(),
            ));
        }
        if claim_policy.as_ref().is_some_and(|policy| {
            policy.policy_id.trim().is_empty() || policy.content_identity.trim().is_empty()
        }) {
            return Err(TheoryReceiptError::Invalid(
                "receipt claim policy reference is incomplete".to_string(),
            ));
        }
        let identity = ReceiptIdentity {
            selection: &selection,
            belief_family: &belief_family,
            curation_rule: &curation_rule,
            maintained_condition: &maintained_condition,
            outcome_mapping: &outcome_mapping,
            strategy_theory: &strategy_theory,
            executable_contracts: &executable_contracts,
            authority_policy: &authority_policy,
            claim_policy: &claim_policy,
        };
        let receipt_id = hash(&identity)?;
        Ok(Self {
            receipt_id,
            selection,
            belief_family,
            curation_rule,
            maintained_condition,
            outcome_mapping,
            strategy_theory,
            executable_contracts,
            authority_policy,
            claim_policy,
            installed_at_seq,
        })
    }

    fn verify_identity(&self) -> Result<(), TheoryReceiptError> {
        let identity = ReceiptIdentity {
            selection: &self.selection,
            belief_family: &self.belief_family,
            curation_rule: &self.curation_rule,
            maintained_condition: &self.maintained_condition,
            outcome_mapping: &self.outcome_mapping,
            strategy_theory: &self.strategy_theory,
            executable_contracts: &self.executable_contracts,
            authority_policy: &self.authority_policy,
            claim_policy: &self.claim_policy,
        };
        if hash(&identity)? != self.receipt_id {
            return Err(TheoryReceiptError::CorruptReceipt);
        }
        Ok(())
    }
}

/// Stable receipt persistence and resolution failures.
#[derive(Debug, Error)]
pub enum TheoryReceiptError {
    /// A receipt or selection violated its structural contract.
    #[error("theory_image_inconsistent: {0}")]
    Invalid(String),
    /// Stored receipt bytes do not match the receipt identity.
    #[error("theory_revision_corrupt: receipt identity mismatch")]
    CorruptReceipt,
    /// Physical persistence or serialization failed.
    #[error("theory receipt storage failed: {0}")]
    Storage(String),
}

/// Read-only access to historical root-owned installation receipts.
#[derive(Clone)]
pub struct TheoryInstallationReceiptStore {
    receipts: Tree,
}

/// Immutable exact semantic image frozen for one runtime composition.
#[derive(Debug, Clone)]
pub struct ResolvedStewardshipTheory {
    /// Exact product compilation that selected the live semantic image.
    pub product_compilation_receipt_id: Option<String>,
    /// Exact PDS package receipts named by that compilation.
    pub package_receipt_ids: Vec<String>,
    /// In-memory owner revision view derived from exact PDS records.
    /// Historical resolution may read the old stored shape by explicit id.
    pub receipt: TheoryInstallationReceipt,
    /// Prepared closure that anchored live runtime hydration.
    pub prepared_closure: Option<PreparedActivationClosureV1>,
    /// Exact belief-family revision.
    pub belief_family: BeliefFamilyRevision,
    /// Every exact belief family selected by the compiled product.
    pub belief_families: Vec<BeliefFamilyRevision>,
    /// Exact curation-rule revision.
    pub curation_rule: AgentCurationRuleRevision,
    /// Exact standing maintained-condition revision.
    pub maintained_condition: AgentMaintainedConditionRevision,
    /// Exact outcome-mapping revision.
    pub outcome_mapping: OutcomeMappingRevision,
    /// Exact complete Strategy theory revision.
    pub strategy_theory: StrategyTheoryRevision,
    /// Exact executable capability contract revisions.
    pub executable_contracts: Vec<CapabilityContractRevision>,
    /// Exact effective-authority policy revision.
    pub authority_policy: AuthorityPolicyRevision,
    /// Exact Docs policy when the product selects that owner component.
    pub claim_policy: Option<DocsClaimPolicyRevision>,
}

#[derive(Clone)]
pub enum PreparedCurationSelection {
    Installed(Box<meld_world_model::StandingCurationRuleRevision>),
    Epoch(Box<meld_world_model::curation::CurationTemplateRevision>),
}

impl ResolvedStewardshipTheory {
    /// Validate this exact image against one physical activation binding.
    pub fn validate_activation(
        &self,
        selection: &SelectedStewardshipPackage,
        subject: &DomainObjectRef,
    ) -> Result<(), TheoryResolutionError> {
        self.validate(selection, Some(subject))
    }

    /// Resolve every live semantic body from one current prepared product.
    pub fn resolve_prepared_product(
        stores: &OpenProductStores,
        selection: &SelectedStewardshipPackage,
        subject: &DomainObjectRef,
    ) -> Result<Self, TheoryResolutionError> {
        let head = stores
            .pds_products
            .prepared_head(&selection.expression)
            .map_err(|failure| TheoryResolutionError::Inconsistent(failure.to_string()))?
            .ok_or(TheoryResolutionError::NotPrepared)?;
        let closure = stores
            .pds_products
            .prepared_closure(&head.prepared_id)
            .map_err(|failure| TheoryResolutionError::Inconsistent(failure.to_string()))?
            .ok_or_else(|| missing("prepared product closure"))?;
        if closure.assignment.assignment_id != head.assignment_id
            || closure.assignment.principal_id != selection.principal_id
            || &closure.assignment.subject != subject
        {
            return Err(TheoryResolutionError::Inconsistent(
                "prepared product assignment differs from the physical stewardship binding"
                    .to_string(),
            ));
        }
        let compilation = stores
            .pds_products
            .compilation(&closure.assignment.product_compilation_receipt_id)
            .map_err(|failure| TheoryResolutionError::Inconsistent(failure.to_string()))?
            .ok_or_else(|| missing("product compilation"))?;
        let declaration = stores
            .pds_products
            .declaration(&closure.assignment.product_revision_id)
            .map_err(|failure| TheoryResolutionError::Inconsistent(failure.to_string()))?
            .ok_or_else(|| missing("product declaration"))?;
        if declaration.product_id != selection.expression
            || compilation.product_revision_id != declaration.product_revision_id
        {
            return Err(TheoryResolutionError::Inconsistent(
                "prepared product does not resolve its declared compilation".to_string(),
            ));
        }
        let package_receipts = compilation
            .package_receipt_ids
            .iter()
            .map(|receipt_id| {
                stores
                    .pds_packages
                    .resolve_receipt(receipt_id)
                    .map_err(|failure| TheoryResolutionError::Inconsistent(failure.to_string()))?
                    .ok_or_else(|| missing("compiled PDS package receipt"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let rebuilt = ProductCompilationReceiptV1::compile(
            &declaration,
            package_receipts,
            compilation.compiled_at_seq,
        )
        .map_err(|failure| TheoryResolutionError::Inconsistent(failure.to_string()))?;
        if rebuilt != compilation {
            return Err(TheoryResolutionError::Inconsistent(
                "prepared product compilation differs from its exact package receipts".to_string(),
            ));
        }
        let receipt = receipt_from_components(
            selection,
            &compilation.installed_owner_revisions,
            compilation.compiled_at_seq,
        )?;
        Self::resolve_exact_with_lineage(
            stores,
            selection,
            Some(subject),
            receipt,
            Some(compilation.compilation_receipt_id),
            compilation.package_receipt_ids,
            Some(closure),
        )
    }

    /// Resolve one historical receipt and every body it pinned.
    pub fn resolve_receipt(
        stores: &OpenProductStores,
        receipt_id: &str,
    ) -> Result<Self, TheoryResolutionError> {
        let receipt = stores
            .theory_receipts
            .resolve(receipt_id)
            .map_err(TheoryResolutionError::Receipt)?
            .ok_or_else(|| missing("installation receipt"))?;
        let selection = receipt.selection.clone();
        Self::resolve_exact_with_lineage(stores, &selection, None, receipt, None, Vec::new(), None)
    }

    /// Resolve one routed package receipt into the existing owner runtime view.
    pub fn resolve_pds_receipt(
        stores: &OpenProductStores,
        selection: &SelectedStewardshipPackage,
        subject: &DomainObjectRef,
        package_receipt_id: &str,
    ) -> Result<Self, TheoryResolutionError> {
        let package = stores
            .pds_packages
            .resolve_receipt(package_receipt_id)
            .map_err(|failure| TheoryResolutionError::Inconsistent(failure.to_string()))?
            .ok_or_else(|| missing("PDS package receipt"))?;
        let compatibility_receipt =
            receipt_from_components(selection, &package.components, package.installed_at_seq)?;
        Self::resolve_exact_with_lineage(
            stores,
            selection,
            Some(subject),
            compatibility_receipt,
            None,
            vec![package_receipt_id.to_string()],
            None,
        )
    }

    fn resolve_exact_with_lineage(
        stores: &OpenProductStores,
        selection: &SelectedStewardshipPackage,
        expected_subject: Option<&DomainObjectRef>,
        receipt: TheoryInstallationReceipt,
        product_compilation_receipt_id: Option<String>,
        package_receipt_ids: Vec<String>,
        prepared_closure: Option<PreparedActivationClosureV1>,
    ) -> Result<Self, TheoryResolutionError> {
        let belief_family = stores
            .belief_family_registry
            .resolve(
                &receipt.belief_family.id,
                &receipt.belief_family.content_hash,
            )
            .map_err(owner_error)?
            .ok_or_else(|| missing("belief family"))?;
        let curation_rule = stores
            .curation_rule_registry
            .resolve(
                &receipt.curation_rule.id,
                &receipt.curation_rule.content_hash,
            )
            .map_err(owner_error)?
            .ok_or_else(|| missing("curation rule"))?;
        let maintained_condition = stores
            .maintained_condition_registry
            .resolve(
                &receipt.maintained_condition.id,
                &receipt.maintained_condition.content_hash,
            )
            .map_err(owner_error)?
            .ok_or_else(|| missing("maintained condition"))?;
        let outcome_mapping = stores
            .outcome_mapping_registry
            .resolve(
                &receipt.outcome_mapping.id,
                &receipt.outcome_mapping.content_hash,
            )
            .map_err(owner_error)?
            .ok_or_else(|| missing("outcome mapping"))?;
        let strategy_theory = stores
            .strategy_theory_registry
            .resolve(
                &receipt.strategy_theory.id,
                &receipt.strategy_theory.content_hash,
            )
            .map_err(owner_error)?
            .ok_or_else(|| missing("Strategy theory"))?;
        let executable_contracts = receipt
            .executable_contracts
            .iter()
            .map(|reference| {
                stores
                    .capability_contract_registry
                    .resolve(reference)
                    .map_err(owner_error)?
                    .ok_or_else(|| missing("executable capability contract"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let authority_policy = stores
            .authority_policy_registry
            .resolve(
                &receipt.authority_policy.policy_id,
                &receipt.authority_policy.content_hash,
            )
            .map_err(owner_error)?
            .ok_or_else(|| missing("authority policy"))?;
        let claim_policy = receipt
            .claim_policy
            .as_ref()
            .map(|reference| {
                stores
                    .claim_policy_registry
                    .resolve(reference)
                    .map_err(owner_error)?
                    .ok_or_else(|| missing("docs claim policy"))
            })
            .transpose()?;
        let mut families = std::collections::BTreeMap::from([(
            belief_family.family_id.clone(),
            belief_family.clone(),
        )]);
        for id in &package_receipt_ids {
            let package = stores
                .pds_packages
                .resolve_receipt(id)
                .map_err(owner_error)?
                .ok_or_else(|| missing("compiled package families"))?;
            for component in package.components.iter().filter(|component| {
                component.route.owner_domain == "world-model"
                    && component.route.component_kind == "belief-family"
            }) {
                let reference = &component.owner_revision;
                let family = stores
                    .belief_family_registry
                    .resolve(&reference.id, &reference.content_hash)
                    .map_err(owner_error)?
                    .ok_or_else(|| missing("compiled belief family"))?;
                if families
                    .get(&family.family_id)
                    .is_some_and(|prior| prior != &family)
                {
                    return Err(TheoryResolutionError::Inconsistent(
                        "compiled families contain conflicting revisions".into(),
                    ));
                }
                families.insert(family.family_id.clone(), family);
            }
        }
        let belief_families = families.into_values().collect();
        let resolved = Self {
            product_compilation_receipt_id,
            package_receipt_ids,
            receipt,
            prepared_closure,
            belief_family,
            belief_families,
            curation_rule,
            maintained_condition,
            outcome_mapping,
            strategy_theory,
            executable_contracts,
            authority_policy,
            claim_policy,
        };
        resolved.validate(selection, expected_subject)?;
        Ok(resolved)
    }

    /// Resolve the native rule named by the prepared Agent genesis receipt.
    pub fn native_curation_selection(
        &self,
        stores: &OpenProductStores,
        agent: &meld_world_model::agent::AgentRecord,
    ) -> Result<Option<PreparedCurationSelection>, TheoryResolutionError> {
        let epoch = self.maintained_condition.condition.observation_scope
            != meld_world_model::agent::AgentObservationScope::AssignedSubject;
        let Some(closure) = &self.prepared_closure else {
            if epoch {
                return Err(missing("epoch observation prepared closure"));
            }
            return stores
                .curation_store
                .active_rule(&agent.agent_id)
                .map(|rule| rule.map(|rule| PreparedCurationSelection::Installed(Box::new(rule))))
                .map_err(owner_error);
        };
        let compilation = stores
            .pds_products
            .compilation(&closure.assignment.product_compilation_receipt_id)
            .map_err(owner_error)?
            .ok_or_else(|| missing("prepared product compilation"))?;
        let templates: Vec<_> = compilation
            .installed_owner_revisions
            .iter()
            .filter(|component| {
                component.owner_revision.registry
                    == meld_world_model::curation::CURATION_TEMPLATE_REGISTRY_ID
            })
            .collect();
        if templates.is_empty() {
            if epoch {
                return Err(missing("epoch observation Curation template"));
            }
            // Existing packages predate native Curation installation. Their explicit
            // active selection remains readable until those products are migrated.
            return stores
                .curation_store
                .active_rule(&agent.agent_id)
                .map(|rule| rule.map(|rule| PreparedCurationSelection::Installed(Box::new(rule))))
                .map_err(owner_error);
        }
        if templates.len() != 1 {
            return Err(TheoryResolutionError::Inconsistent(
                "this Curation participant requires one exact assigned template".into(),
            ));
        }
        let template = &templates[0].owner_revision;
        let template_ref = TheoryRevisionRef {
            registry: template.registry.clone(),
            id: template.id.clone(),
            content_hash: template.content_hash.clone(),
        };
        let receipts = stores
            .agent_store
            .genesis_receipts_for_assignment(&closure.assignment.assignment_id)
            .map_err(owner_error)?;
        let genesis: Vec<_> = receipts
            .iter()
            .filter(|receipt| {
                receipt.agent_id == agent.agent_id
                    && receipt.product_compilation_receipt_id == compilation.compilation_receipt_id
            })
            .collect();
        if genesis.len() != 1 {
            return Err(missing("exact Agent genesis Curation binding"));
        }
        if epoch {
            if !genesis[0].installed_owner_revisions.contains(&template_ref) {
                return Err(missing("epoch genesis Curation template"));
            }
            let template = stores
                .curation_store
                .resolve_template(&template_ref)
                .map_err(owner_error)?
                .ok_or_else(|| missing("installed epoch Curation template"))?;
            return Ok(Some(PreparedCurationSelection::Epoch(Box::new(template))));
        }
        let rules: Vec<_> = genesis[0]
            .installed_owner_revisions
            .iter()
            .filter(|reference| {
                reference.registry == meld_world_model::curation::CURATION_RULE_REGISTRY_ID
            })
            .collect();
        if rules.len() != 1 || !genesis[0].installed_owner_revisions.contains(&template_ref) {
            return Err(missing("prepared native Curation revision"));
        }
        let binding = meld_world_model::curation::CurationRuleBinding {
            agent_id: agent.agent_id.clone(),
            subject: agent.subject.clone(),
            scope: meld_world_model::world_state::graph::contracts::OwnerPublicationScope {
                scope_id: agent.subject.object_id.clone(),
                branch_id: Some(agent.branch_scope.branch_id.clone()),
                perspective_id: Some(agent.perspective_key.perspective_id.clone()),
                valid_at: None,
            },
        };
        let template = stores
            .curation_store
            .resolve_template(&template_ref)
            .map_err(owner_error)?
            .ok_or_else(|| missing("installed Curation template"))?;
        let rule = match assigned_curation_source(&template.template, &binding)
            .map_err(TheoryResolutionError::Inconsistent)?
        {
            Some(source) => stores.curation_store.resolve_source_bound_rule(
                &template_ref,
                &binding,
                &source,
                &meld_world_model::curation::CurationJudgmentScope {
                    subject: agent.subject.clone(),
                    perspective: agent.perspective_key.clone(),
                    branch_scope: agent.branch_scope.clone(),
                },
                rules[0],
            ),
            None => stores
                .curation_store
                .resolve_bound_rule(&template_ref, &binding, rules[0]),
        }
        .map_err(owner_error)?;
        Ok(Some(PreparedCurationSelection::Installed(Box::new(rule))))
    }

    /// Build the exact in-memory execution catalog selected by the receipt.
    pub fn capability_catalog(&self) -> Result<CapabilityCatalog, TheoryResolutionError> {
        let mut catalog = CapabilityCatalog::new();
        for revision in &self.executable_contracts {
            catalog
                .register(revision.contract.clone())
                .map_err(owner_error)?;
        }
        Ok(catalog)
    }

    fn validate(
        &self,
        selection: &SelectedStewardshipPackage,
        expected_subject: Option<&DomainObjectRef>,
    ) -> Result<(), TheoryResolutionError> {
        if &self.receipt.selection != selection
            || self.belief_family.revision_ref() != self.receipt.belief_family
            || self.curation_rule.revision_ref() != self.receipt.curation_rule
            || self.maintained_condition.revision_ref() != self.receipt.maintained_condition
            || self.outcome_mapping.revision_ref() != self.receipt.outcome_mapping
            || self.strategy_theory.revision_ref() != self.receipt.strategy_theory
            || self.authority_policy.revision_ref() != self.receipt.authority_policy
            || self
                .claim_policy
                .as_ref()
                .map(|policy| policy.revision_ref())
                != self.receipt.claim_policy
        {
            return Err(TheoryResolutionError::Inconsistent(
                "resolved owner revision does not match its receipt reference".to_string(),
            ));
        }
        if self.receipt.belief_family.id != selection.belief_family_id
            || self.receipt.curation_rule.id != selection.curation_rule_id
            || self.receipt.maintained_condition.id != selection.maintained_condition_id
            || self.receipt.outcome_mapping.id != selection.evidence_mapping_id
            || self.receipt.strategy_theory.id != selection.strategy_theory_id
            || self.receipt.authority_policy.policy_id != selection.authority_policy_id
            || self
                .receipt
                .claim_policy
                .as_ref()
                .map_or("", |policy| policy.policy_id.as_str())
                != selection.claim_policy_id
        {
            return Err(TheoryResolutionError::Inconsistent(
                "prepared owner revisions differ from the physical stewardship selection"
                    .to_string(),
            ));
        }
        if self.authority_policy.policy.policy_id != selection.authority_policy_id
            || self.authority_policy.policy.principal_id != selection.principal_id
            || expected_subject
                .is_some_and(|subject| &self.authority_policy.policy.subject != subject)
        {
            return Err(TheoryResolutionError::Inconsistent(
                "resolved authority policy does not match the selected policy, principal, and subject"
                    .to_string(),
            ));
        }
        if self.curation_rule.rule.dimension_id != self.belief_family.config.dimension_id
            || self.maintained_condition.condition.dimension_id
                != self.belief_family.config.dimension_id
            || self.curation_rule.rule.maintained_condition_id.as_deref()
                != Some(self.maintained_condition.condition.condition_id.as_str())
            || self
                .strategy_theory
                .package
                .requested_dimensions
                .iter()
                .any(|dimension| {
                    self.belief_families
                        .iter()
                        .filter(|family| &family.config.dimension_id == dimension)
                        .count()
                        != 1
                })
        {
            return Err(TheoryResolutionError::Inconsistent(
                "resolved cross-domain projection dimensions disagree".to_string(),
            ));
        }
        for capability in &self.strategy_theory.package.capabilities {
            let Some(selector) = capability.operator.resolution.specific.as_ref() else {
                return Err(TheoryResolutionError::Inconsistent(
                    "Strategy capability has no exact selector".to_string(),
                ));
            };
            if !self.executable_contracts.iter().any(|revision| {
                revision.contract.capability_type_id == selector.capability_type_id
                    && revision.contract.capability_version == selector.capability_version
                    && revision.content_identity == capability.contract_id
            }) {
                return Err(TheoryResolutionError::Inconsistent(format!(
                    "Strategy capability '{}' does not resolve through the receipt",
                    capability.operator.operator_id
                )));
            }
        }
        Ok(())
    }
}

fn receipt_from_components(
    selection: &SelectedStewardshipPackage,
    components: &[InstalledTheoryComponentRef],
    installed_at_seq: u64,
) -> Result<TheoryInstallationReceipt, TheoryResolutionError> {
    let route = |owner: &str, kind: &str| crate::theory::TheoryRouteId::new(owner, kind, 1);
    let one = |wanted: crate::theory::TheoryRouteId| {
        let mut matches = components
            .iter()
            .filter(|component| component.route == wanted);
        let first = matches
            .next()
            .ok_or_else(|| missing("compiled owner component"))?;
        if matches.next().is_some() {
            return Err(TheoryResolutionError::Inconsistent(format!(
                "route '{}' requires one compiled runtime component",
                wanted.key()
            )));
        }
        Ok(first.owner_revision.clone())
    };
    let world_ref = |reference: crate::theory::TheoryRevisionRef| TheoryRevisionRef {
        registry: reference.registry,
        id: reference.id,
        content_hash: reference.content_hash,
    };
    let selected_families: Vec<_> = components
        .iter()
        .filter(|component| {
            component.route == route("world-model", "belief-family")
                && component.owner_revision.id == selection.belief_family_id
        })
        .collect();
    if selected_families.len() != 1 {
        return Err(TheoryResolutionError::Inconsistent(
            "selected Agent observation family requires one exact compiled revision".into(),
        ));
    }
    let belief_family = world_ref(selected_families[0].owner_revision.clone());
    let curation_rule = world_ref(one(route("world-model", "agent-curation-rule"))?);
    let maintained_condition = world_ref(one(route("world-model", "agent-maintained-condition"))?);
    let outcome_mapping = world_ref(one(route("world-model", "outcome-mapping"))?);
    let strategy_theory = world_ref(one(route("world-model", "strategy-theory"))?);
    let authority = one(route("execution", "authority-policy"))?;
    let claim = if selection.claim_policy_id.is_empty() {
        None
    } else {
        Some(one(route("docs", "claim-policy"))?)
    };
    let mut executable_contracts = Vec::new();
    for component in components
        .iter()
        .filter(|component| component.route == route("execution", "capability-contract"))
    {
        let (type_id, version) =
            component
                .owner_revision
                .id
                .rsplit_once(".v")
                .ok_or_else(|| {
                    TheoryResolutionError::Inconsistent("invalid capability owner ref".to_string())
                })?;
        executable_contracts.push(CapabilityContractRevisionRef {
            selector: meld_lang::CapabilityRef {
                capability_type_id: type_id.to_string(),
                capability_version: version.parse().map_err(|_| {
                    TheoryResolutionError::Inconsistent(
                        "invalid capability owner ref version".to_string(),
                    )
                })?,
            },
            content_identity: component.owner_revision.content_hash.clone(),
        });
    }
    TheoryInstallationReceipt::new(
        selection.clone(),
        belief_family,
        curation_rule,
        maintained_condition,
        outcome_mapping,
        strategy_theory,
        executable_contracts,
        AuthorityPolicyRevisionRef {
            policy_id: authority.id,
            content_hash: authority.content_hash,
        },
        claim.map(|claim| DocsClaimPolicyRevisionRef {
            policy_id: claim.id,
            content_identity: claim.content_hash,
        }),
        installed_at_seq,
    )
    .map_err(|failure| TheoryResolutionError::Inconsistent(failure.to_string()))
}

/// Stable exact image resolution failures.
#[derive(Debug, Error)]
pub enum TheoryResolutionError {
    /// No prepared product head exists for the selected product.
    #[error("theory_image_not_installed: prepared product head is absent")]
    NotPrepared,
    /// Receipt selection or persistence failed.
    #[error("{0}")]
    Receipt(TheoryReceiptError),
    /// An exact referenced owner revision is absent.
    #[error("theory_revision_missing: {0}")]
    Missing(String),
    /// An owner rejected or could not read stored content.
    #[error("theory_revision_corrupt: {0}")]
    Corrupt(String),
    /// Exact bodies disagree across domain boundaries.
    #[error("theory_image_inconsistent: {0}")]
    Inconsistent(String),
}

impl TheoryInstallationReceiptStore {
    /// Open the historical receipt tree in the shared theory database.
    pub(crate) fn new(db: Db) -> Result<Self, TheoryReceiptError> {
        Ok(Self {
            receipts: db.open_tree(TREE_RECEIPTS).map_err(to_storage)?,
        })
    }

    /// Resolve one historical receipt by exact identity.
    pub fn resolve(
        &self,
        receipt_id: &str,
    ) -> Result<Option<TheoryInstallationReceipt>, TheoryReceiptError> {
        let Some(raw) = self
            .receipts
            .get(receipt_id.as_bytes())
            .map_err(to_storage)?
        else {
            return Ok(None);
        };
        let receipt: TheoryInstallationReceipt = serde_json::from_slice(&raw)
            .map_err(|error| TheoryReceiptError::Storage(error.to_string()))?;
        receipt.verify_identity()?;
        if receipt.receipt_id != receipt_id {
            return Err(TheoryReceiptError::CorruptReceipt);
        }
        Ok(Some(receipt))
    }
}

fn validate_selection(selection: &SelectedStewardshipPackage) -> Result<(), TheoryReceiptError> {
    for value in [
        &selection.expression,
        &selection.principal_id,
        &selection.belief_family_id,
        &selection.evidence_mapping_id,
        &selection.curation_rule_id,
        &selection.maintained_condition_id,
        &selection.strategy_theory_id,
        &selection.authority_policy_id,
    ] {
        if value.trim().is_empty() {
            return Err(TheoryReceiptError::Invalid(
                "selected stewardship package identities must not be empty".to_string(),
            ));
        }
    }
    Ok(())
}

fn hash<T: Serialize>(value: &T) -> Result<String, TheoryReceiptError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| TheoryReceiptError::Storage(error.to_string()))?;
    Ok(blake3::hash(&bytes).to_hex().to_string())
}

fn invalid(error: impl ToString) -> TheoryReceiptError {
    TheoryReceiptError::Invalid(error.to_string())
}

fn to_storage(error: sled::Error) -> TheoryReceiptError {
    TheoryReceiptError::Storage(error.to_string())
}

fn missing(kind: &str) -> TheoryResolutionError {
    TheoryResolutionError::Missing(format!("receipt cites absent {kind} revision"))
}

fn owner_error(error: impl ToString) -> TheoryResolutionError {
    TheoryResolutionError::Corrupt(error.to_string())
}

/// Composition selects an owner adapter; the owner constructs its semantic source address.
pub(crate) fn assigned_curation_source(
    template: &meld_world_model::curation::CurationRuleTemplate,
    binding: &meld_world_model::curation::CurationRuleBinding,
) -> Result<Option<meld_world_model::curation::CurationSourceBinding>, String> {
    if template.source_owner_id == binding.subject.domain_id {
        return Ok(None);
    }
    match template.source_owner_id.as_str() {
        crate::docs::publication::OWNER_ID => {
            crate::docs::publication::curation_source(binding).map(Some)
        }
        crate::dependency_security::publication::OWNER => {
            crate::dependency_security::condition::curation_source(template, binding).map(Some)
        }
        owner => Err(format!("no assigned Curation source adapter for '{owner}'")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::docs::capability::published_contracts;
    use crate::docs::claim_validation::DocsClaimPolicy;
    use crate::runtime::storage::ProductStorageLayout;
    use meld_lang::AuthorityPolicy;
    use meld_world_model::agent::{AgentCurationRuleConfig, AgentMaintainedCondition};
    use meld_world_model::belief::{
        BeliefFamilyConfig, BeliefFamilyRegistryStore, OutcomeMappingSetConfig,
    };
    use meld_world_model::strategy::StrategyTheoryPackage;

    fn selection() -> SelectedStewardshipPackage {
        SelectedStewardshipPackage {
            expression: "docs_freshness".to_string(),
            principal_id: "workspace-owner".to_string(),
            belief_family_id: "docs_freshness".to_string(),
            evidence_mapping_id: "docs_freshness_outcome_interpretation_v1".to_string(),
            curation_rule_id: "docs_freshness".to_string(),
            maintained_condition_id: "docs_freshness".to_string(),
            strategy_theory_id: "docs_freshness".to_string(),
            authority_policy_id: "docs_workspace_local".to_string(),
            claim_policy_id: "docs-claims-strict-v1".to_string(),
        }
    }

    #[test]
    fn explicit_historical_receipt_read_preserves_its_frozen_image() {
        let temp = tempfile::tempdir().unwrap();
        let stores =
            OpenProductStores::open(&ProductStorageLayout::from_root(temp.path())).unwrap();
        let mut families =
            BeliefFamilyRegistryStore::new(stores.traversal_store.db().clone()).unwrap();
        let family: BeliefFamilyConfig = serde_json::from_str(include_str!(
            "../../theory/docs_freshness/belief_family.docs_freshness.json"
        ))
        .unwrap();
        let mapping: OutcomeMappingSetConfig = serde_json::from_str(include_str!(
            "../../theory/docs_freshness/outcome_interpretation.docs_freshness.json"
        ))
        .unwrap();
        let curation: AgentCurationRuleConfig = serde_json::from_str(include_str!(
            "../../theory/docs_freshness/curation_rule.docs_freshness.json"
        ))
        .unwrap();
        let maintained_condition: AgentMaintainedCondition = serde_json::from_str(include_str!(
            "../../theory/docs_freshness/maintained_condition.docs_freshness.json"
        ))
        .unwrap();
        let strategy: StrategyTheoryPackage = serde_json::from_str(include_str!(
            "../../theory/docs_freshness/strategy_theory.docs_freshness.json"
        ))
        .unwrap();
        let claim_policy: DocsClaimPolicy = serde_json::from_str(include_str!(
            "../../theory/docs_freshness/claim_policy.docs-claims-strict-v1.json"
        ))
        .unwrap();
        let authority_policy: AuthorityPolicy = serde_json::from_str(include_str!(
            "../../theory/docs_freshness/authority_policy.docs_workspace_local.json"
        ))
        .unwrap();

        let (_, family_revision) = families.install(family, 10).unwrap();
        let (_, mapping_revision) = stores
            .outcome_mapping_registry
            .install(mapping, 10)
            .unwrap();
        let (_, strategy_revision) = stores
            .strategy_theory_registry
            .install(strategy, 10)
            .unwrap();
        let (_, authority_revision) = stores
            .authority_policy_registry
            .install(authority_policy, 10)
            .unwrap();
        let (_, policy_revision) = stores
            .claim_policy_registry
            .install(claim_policy, 10)
            .unwrap();
        let capability_revisions = published_contracts()
            .into_iter()
            .map(|contract| {
                stores
                    .capability_contract_registry
                    .install(contract, 10)
                    .unwrap()
                    .1
            })
            .collect::<Vec<_>>();
        let (_, curation_a) = stores
            .curation_rule_registry
            .install("docs_freshness", curation.clone(), 10)
            .unwrap();
        let (_, condition_revision) = stores
            .maintained_condition_registry
            .install(maintained_condition.clone(), 10)
            .unwrap();

        let receipt_a = TheoryInstallationReceipt::new(
            selection(),
            family_revision.revision_ref(),
            curation_a.revision_ref(),
            condition_revision.revision_ref(),
            mapping_revision.revision_ref(),
            strategy_revision.revision_ref(),
            capability_revisions
                .iter()
                .map(CapabilityContractRevision::revision_ref)
                .collect(),
            authority_revision.revision_ref(),
            policy_revision.revision_ref(),
            10,
        )
        .unwrap();
        let mut reversed_contracts = receipt_a.executable_contracts.clone();
        reversed_contracts.reverse();
        let reordered = TheoryInstallationReceipt::new(
            selection(),
            family_revision.revision_ref(),
            curation_a.revision_ref(),
            condition_revision.revision_ref(),
            mapping_revision.revision_ref(),
            strategy_revision.revision_ref(),
            reversed_contracts,
            authority_revision.revision_ref(),
            policy_revision.revision_ref(),
            99,
        )
        .unwrap();
        assert_eq!(reordered.receipt_id, receipt_a.receipt_id);
        stores
            .theory_receipts
            .receipts
            .insert(
                receipt_a.receipt_id.as_bytes(),
                serde_json::to_vec(&receipt_a).unwrap(),
            )
            .unwrap();
        let resolved =
            ResolvedStewardshipTheory::resolve_receipt(&stores, &receipt_a.receipt_id).unwrap();

        assert_eq!(resolved.receipt, receipt_a);
        assert_eq!(resolved.curation_rule, curation_a);
        assert_eq!(resolved.maintained_condition, condition_revision);
        assert!(resolved.product_compilation_receipt_id.is_none());
        assert!(resolved.package_receipt_ids.is_empty());
        assert_eq!(
            stores
                .theory_receipts
                .resolve(&resolved.receipt.receipt_id)
                .unwrap(),
            Some(resolved.receipt)
        );
    }

    #[test]
    fn receipt_with_an_absent_owner_revision_fails_exact_resolution() {
        let temp = tempfile::tempdir().unwrap();
        let stores =
            OpenProductStores::open(&ProductStorageLayout::from_root(temp.path())).unwrap();
        let receipt = TheoryInstallationReceipt::new(
            selection(),
            TheoryRevisionRef {
                registry: "belief_family".to_string(),
                id: "docs_freshness".to_string(),
                content_hash: "missing-family".to_string(),
            },
            TheoryRevisionRef {
                registry: "agent_curation_rule".to_string(),
                id: "docs_freshness".to_string(),
                content_hash: "missing-curation".to_string(),
            },
            TheoryRevisionRef {
                registry: "agent_maintained_condition".to_string(),
                id: "docs_freshness".to_string(),
                content_hash: "missing-condition".to_string(),
            },
            TheoryRevisionRef {
                registry: "outcome_mapping".to_string(),
                id: "docs_freshness_outcome_interpretation_v1".to_string(),
                content_hash: "missing-mapping".to_string(),
            },
            TheoryRevisionRef {
                registry: "strategy_theory".to_string(),
                id: "docs_freshness".to_string(),
                content_hash: "missing-strategy".to_string(),
            },
            vec![CapabilityContractRevisionRef {
                selector: meld_lang::CapabilityRef {
                    capability_type_id: "docs.inspect_scope".to_string(),
                    capability_version: 1,
                },
                content_identity: "missing-contract".to_string(),
            }],
            AuthorityPolicyRevisionRef {
                policy_id: "docs_workspace_local".to_string(),
                content_hash: "missing-authority-policy".to_string(),
            },
            DocsClaimPolicyRevisionRef {
                policy_id: "docs-claims-strict-v1".to_string(),
                content_identity: "missing-policy".to_string(),
            },
            1,
        )
        .unwrap();
        stores
            .theory_receipts
            .receipts
            .insert(
                receipt.receipt_id.as_bytes(),
                serde_json::to_vec(&receipt).unwrap(),
            )
            .unwrap();

        assert!(matches!(
            ResolvedStewardshipTheory::resolve_receipt(&stores, &receipt.receipt_id),
            Err(TheoryResolutionError::Missing(message)) if message.contains("belief family")
        ));
    }
    #[test]
    fn security_package_resolves_its_selected_family_without_a_docs_policy() {
        let temp = tempfile::tempdir().unwrap();
        let stores =
            OpenProductStores::open(&ProductStorageLayout::from_root(temp.path())).unwrap();
        let package = crate::dependency_security::theory::install_package(
            &stores,
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("theory/dependency_security"),
            1,
        )
        .unwrap();
        let selected = SelectedStewardshipPackage {
            expression: "dependency_security_fixture".into(),
            principal_id: "workspace-owner".into(),
            belief_family_id: "dependency_security_posture".into(),
            evidence_mapping_id: "dependency_security_outcome_mapping_v1".into(),
            curation_rule_id: "dependency_security_posture".into(),
            maintained_condition_id: "dependency_security_posture".into(),
            strategy_theory_id: "dependency_security_fixture".into(),
            authority_policy_id: "dependency_security_fixture_read_only".into(),
            claim_policy_id: String::new(),
        };
        let subject = DomainObjectRef::new("workspace_fs", "node", "dependency-graph").unwrap();
        let resolved = ResolvedStewardshipTheory::resolve_pds_receipt(
            &stores,
            &selected,
            &subject,
            &package.receipt_id,
        )
        .unwrap();
        assert_eq!(
            resolved.belief_family.config.family_id,
            "dependency_security_posture"
        );
        assert!(resolved.claim_policy.is_none());
        assert_eq!(resolved.executable_contracts.len(), 4);
        assert_eq!(
            package
                .components
                .iter()
                .filter(|component| component.route.component_kind == "belief-family")
                .count(),
            2
        );
        // The primary observation selection cannot silently drift to the first family in the package.
        assert_eq!(
            resolved
                .belief_families
                .iter()
                .map(|family| family.family_id.as_str())
                .collect::<Vec<_>>(),
            vec![
                "dependency_security_coverage",
                "dependency_security_posture"
            ]
        );
        let mut foreign = selected;
        foreign.belief_family_id = "missing-observation-family".into();
        assert!(ResolvedStewardshipTheory::resolve_pds_receipt(
            &stores,
            &foreign,
            &subject,
            &package.receipt_id
        )
        .is_err());
    }
}
