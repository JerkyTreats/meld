//! Root-owned complete theory installation receipts.

use serde::{Deserialize, Serialize};
use sled::{Db, Tree};
use thiserror::Error;

use crate::config::SelectedStewardshipPackage;
use crate::docs::claim_validation::DocsClaimPolicyRevisionRef;
use meld_execution::capability::CapabilityContractRevisionRef;
use meld_execution::capability::{CapabilityCatalog, CapabilityContractRevision};
use meld_world_model::agent::{AgentCurationRuleRevision, AgentMaintainedConditionRevision};
use meld_world_model::belief::{
    BeliefFamilyRegistry, BeliefFamilyRevision, OutcomeMappingRevision, TheoryRevisionRef,
};
use meld_world_model::strategy::StrategyTheoryRevision;

use crate::docs::claim_validation::DocsClaimPolicyRevision;
use crate::runtime::storage::OpenProductStores;

const TREE_RECEIPTS: &str = "theory_installation_receipts";
const TREE_CURRENT: &str = "theory_installation_current";

/// One complete cross-owner installation receipt.
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
    /// Exact docs claim-policy revision.
    pub claim_policy: DocsClaimPolicyRevisionRef,
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
    claim_policy: &'a DocsClaimPolicyRevisionRef,
}

impl TheoryInstallationReceipt {
    /// Construct a receipt and derive its exact identity.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        selection: SelectedStewardshipPackage,
        belief_family: TheoryRevisionRef,
        curation_rule: TheoryRevisionRef,
        maintained_condition: TheoryRevisionRef,
        outcome_mapping: TheoryRevisionRef,
        strategy_theory: TheoryRevisionRef,
        mut executable_contracts: Vec<CapabilityContractRevisionRef>,
        claim_policy: DocsClaimPolicyRevisionRef,
        installed_at_seq: u64,
    ) -> Result<Self, TheoryReceiptError> {
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
        if claim_policy.policy_id.trim().is_empty()
            || claim_policy.content_identity.trim().is_empty()
        {
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
            claim_policy,
            installed_at_seq,
        })
    }

    /// Derive the stable activation key for the selected identities.
    pub fn selection_key(&self) -> Result<String, TheoryReceiptError> {
        selection_key(&self.selection)
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
    /// No complete active receipt exists for a selection.
    #[error("theory_image_not_installed")]
    NotInstalled,
    /// An active head cites a missing append-only receipt.
    #[error("theory_revision_missing: active receipt is absent")]
    MissingReceipt,
    /// Stored receipt bytes do not match the receipt identity.
    #[error("theory_revision_corrupt: receipt identity mismatch")]
    CorruptReceipt,
    /// Physical persistence or serialization failed.
    #[error("theory receipt storage failed: {0}")]
    Storage(String),
}

/// Append-only receipt store with one activation head per selected package.
#[derive(Clone)]
pub struct TheoryInstallationReceiptStore {
    db: Db,
    receipts: Tree,
    current: Tree,
}

/// Immutable exact semantic image frozen for one runtime composition.
#[derive(Debug, Clone)]
pub struct ResolvedStewardshipTheory {
    /// Complete receipt that selected every exact body.
    pub receipt: TheoryInstallationReceipt,
    /// Exact belief-family revision.
    pub belief_family: BeliefFamilyRevision,
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
    /// Exact docs claim-policy revision.
    pub claim_policy: DocsClaimPolicyRevision,
}

impl ResolvedStewardshipTheory {
    /// Resolve the active receipt and every exact owner body once.
    pub fn resolve(
        stores: &OpenProductStores,
        selection: &SelectedStewardshipPackage,
    ) -> Result<Self, TheoryResolutionError> {
        let receipt = stores
            .theory_receipts
            .current(selection)
            .map_err(TheoryResolutionError::Receipt)?;
        Self::resolve_exact(stores, selection, receipt)
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
        Self::resolve_exact(stores, &selection, receipt)
    }

    fn resolve_exact(
        stores: &OpenProductStores,
        selection: &SelectedStewardshipPackage,
        receipt: TheoryInstallationReceipt,
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
        let claim_policy = stores
            .claim_policy_registry
            .resolve(&receipt.claim_policy)
            .map_err(owner_error)?
            .ok_or_else(|| missing("docs claim policy"))?;
        let resolved = Self {
            receipt,
            belief_family,
            curation_rule,
            maintained_condition,
            outcome_mapping,
            strategy_theory,
            executable_contracts,
            claim_policy,
        };
        resolved.validate(selection)?;
        Ok(resolved)
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
    ) -> Result<(), TheoryResolutionError> {
        if &self.receipt.selection != selection
            || self.belief_family.revision_ref() != self.receipt.belief_family
            || self.curation_rule.revision_ref() != self.receipt.curation_rule
            || self.maintained_condition.revision_ref() != self.receipt.maintained_condition
            || self.outcome_mapping.revision_ref() != self.receipt.outcome_mapping
            || self.strategy_theory.revision_ref() != self.receipt.strategy_theory
            || self.claim_policy.revision_ref() != self.receipt.claim_policy
        {
            return Err(TheoryResolutionError::Inconsistent(
                "resolved owner revision does not match its receipt reference".to_string(),
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
                .any(|dimension| dimension != &self.belief_family.config.dimension_id)
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

/// Stable exact image resolution failures.
#[derive(Debug, Error)]
pub enum TheoryResolutionError {
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
    /// Open the receipt trees in the shared theory database.
    pub fn new(db: Db) -> Result<Self, TheoryReceiptError> {
        Ok(Self {
            receipts: db.open_tree(TREE_RECEIPTS).map_err(to_storage)?,
            current: db.open_tree(TREE_CURRENT).map_err(to_storage)?,
            db,
        })
    }

    /// Append and activate one complete receipt idempotently.
    pub fn install(&self, receipt: TheoryInstallationReceipt) -> Result<bool, TheoryReceiptError> {
        receipt.verify_identity()?;
        let selection_key = receipt.selection_key()?;
        let encoded = serde_json::to_vec(&receipt)
            .map_err(|error| TheoryReceiptError::Storage(error.to_string()))?;
        let changed = self
            .current
            .get(selection_key.as_bytes())
            .map_err(to_storage)?
            .as_deref()
            != Some(receipt.receipt_id.as_bytes());
        if self
            .receipts
            .get(receipt.receipt_id.as_bytes())
            .map_err(to_storage)?
            .is_none()
        {
            self.receipts
                .insert(receipt.receipt_id.as_bytes(), encoded)
                .map_err(to_storage)?;
        }
        if changed {
            self.current
                .insert(selection_key.as_bytes(), receipt.receipt_id.as_bytes())
                .map_err(to_storage)?;
        }
        self.db.flush().map_err(to_storage)?;
        Ok(changed)
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

    /// Resolve the active complete receipt for a selected package.
    pub fn current(
        &self,
        selection: &SelectedStewardshipPackage,
    ) -> Result<TheoryInstallationReceipt, TheoryReceiptError> {
        let key = selection_key(selection)?;
        let Some(raw) = self.current.get(key.as_bytes()).map_err(to_storage)? else {
            return Err(TheoryReceiptError::NotInstalled);
        };
        let receipt_id = std::str::from_utf8(&raw)
            .map_err(|error| TheoryReceiptError::Storage(error.to_string()))?;
        self.resolve(receipt_id)?
            .ok_or(TheoryReceiptError::MissingReceipt)
    }
}

fn validate_selection(selection: &SelectedStewardshipPackage) -> Result<(), TheoryReceiptError> {
    for value in [
        &selection.expression,
        &selection.belief_family_id,
        &selection.evidence_mapping_id,
        &selection.curation_rule_id,
        &selection.maintained_condition_id,
        &selection.strategy_theory_id,
        &selection.claim_policy_id,
    ] {
        if value.trim().is_empty() {
            return Err(TheoryReceiptError::Invalid(
                "selected stewardship package identities must not be empty".to_string(),
            ));
        }
    }
    Ok(())
}

fn selection_key(selection: &SelectedStewardshipPackage) -> Result<String, TheoryReceiptError> {
    validate_selection(selection)?;
    hash(selection)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::docs::capability::published_contracts;
    use crate::docs::claim_validation::DocsClaimPolicy;
    use crate::runtime::storage::ProductStorageLayout;
    use meld_world_model::agent::{AgentCurationRuleConfig, AgentMaintainedCondition};
    use meld_world_model::belief::{
        BeliefFamilyConfig, BeliefFamilyRegistryStore, OutcomeMappingSetConfig,
    };
    use meld_world_model::strategy::StrategyTheoryPackage;

    fn selection() -> SelectedStewardshipPackage {
        SelectedStewardshipPackage {
            expression: "docs_freshness".to_string(),
            belief_family_id: "docs_freshness".to_string(),
            evidence_mapping_id: "docs_freshness_outcome_interpretation_v1".to_string(),
            curation_rule_id: "docs_freshness".to_string(),
            maintained_condition_id: "docs_freshness".to_string(),
            strategy_theory_id: "docs_freshness".to_string(),
            claim_policy_id: "docs-claims-strict-v1".to_string(),
        }
    }

    #[test]
    fn receipt_activation_preserves_an_already_resolved_historical_image() {
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
        let policy: DocsClaimPolicy = serde_json::from_str(include_str!(
            "../../theory/docs_freshness/claim_policy.docs-claims-strict-v1.json"
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
        let (_, policy_revision) = stores.claim_policy_registry.install(policy, 10).unwrap();
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

        assert!(matches!(
            stores.theory_receipts.current(&selection()),
            Err(TheoryReceiptError::NotInstalled)
        ));

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
            policy_revision.revision_ref(),
            99,
        )
        .unwrap();
        assert_eq!(reordered.receipt_id, receipt_a.receipt_id);
        stores.theory_receipts.install(receipt_a.clone()).unwrap();
        let resolved_a = ResolvedStewardshipTheory::resolve(&stores, &selection()).unwrap();

        let mut curation_b_body = curation;
        curation_b_body.threshold = 0.91;
        let (_, curation_b) = stores
            .curation_rule_registry
            .install("docs_freshness", curation_b_body, 20)
            .unwrap();
        let mut condition_b_body = maintained_condition;
        condition_b_body.desired =
            meld_lang::Condition::Above(meld_lang::Term::Literal(meld_lang::Literal::Number(0.91)));
        condition_b_body.desired_summary = "confidence>0.91".to_string();
        let (_, condition_b) = stores
            .maintained_condition_registry
            .install(condition_b_body, 20)
            .unwrap();
        let receipt_b = TheoryInstallationReceipt::new(
            selection(),
            family_revision.revision_ref(),
            curation_b.revision_ref(),
            condition_b.revision_ref(),
            mapping_revision.revision_ref(),
            strategy_revision.revision_ref(),
            capability_revisions
                .iter()
                .map(CapabilityContractRevision::revision_ref)
                .collect(),
            policy_revision.revision_ref(),
            20,
        )
        .unwrap();
        stores.theory_receipts.install(receipt_b.clone()).unwrap();
        let resolved_b = ResolvedStewardshipTheory::resolve(&stores, &selection()).unwrap();
        let reloaded_a =
            ResolvedStewardshipTheory::resolve_receipt(&stores, &receipt_a.receipt_id).unwrap();

        assert_eq!(resolved_a.receipt.receipt_id, receipt_a.receipt_id);
        assert_eq!(reloaded_a.curation_rule, resolved_a.curation_rule);
        assert_eq!(reloaded_a.strategy_theory, resolved_a.strategy_theory);
        assert_eq!(resolved_b.receipt.receipt_id, receipt_b.receipt_id);
        assert_ne!(
            resolved_a.curation_rule.content_hash,
            resolved_b.curation_rule.content_hash
        );
        assert_ne!(
            resolved_a.maintained_condition.content_hash,
            resolved_b.maintained_condition.content_hash
        );
        assert_eq!(
            reloaded_a.maintained_condition,
            resolved_a.maintained_condition
        );
        assert_eq!(
            resolved_a.curation_rule.rule.threshold,
            curation_a.rule.threshold
        );
        assert_eq!(
            stores
                .theory_receipts
                .resolve(&receipt_a.receipt_id)
                .unwrap(),
            Some(receipt_a)
        );
        assert!(stores
            .curation_rule_registry
            .resolve("docs_freshness", &curation_a.content_hash)
            .unwrap()
            .is_some());
    }

    #[test]
    fn active_head_to_missing_receipt_has_a_stable_failure() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let store = TheoryInstallationReceiptStore::new(db).unwrap();
        store
            .current
            .insert(selection_key(&selection()).unwrap(), b"absent".as_slice())
            .unwrap();

        assert!(matches!(
            store.current(&selection()),
            Err(TheoryReceiptError::MissingReceipt)
        ));
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
            DocsClaimPolicyRevisionRef {
                policy_id: "docs-claims-strict-v1".to_string(),
                content_identity: "missing-policy".to_string(),
            },
            1,
        )
        .unwrap();
        stores.theory_receipts.install(receipt).unwrap();

        assert!(matches!(
            ResolvedStewardshipTheory::resolve(&stores, &selection()),
            Err(TheoryResolutionError::Missing(message)) if message.contains("belief family")
        ));
    }
}
