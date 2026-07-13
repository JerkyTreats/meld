//! Canonical world-model activation inputs and receipts.

use serde::{Deserialize, Serialize};

use crate::agent::AgentCurationRuleConfig;
use crate::belief::{BeliefFamilyConfig, BeliefKey, BranchScope};
use crate::events::DomainObjectRef;
use crate::world_state::graph::PerspectiveKey;

/// Wire schema for legacy embedded directive migration products.
pub const LEGACY_DIRECTIVE_MIGRATION_SCHEMA_VERSION: u32 = 1;

/// Durable user-originated intent referenced by one or more agents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DirectiveRecord {
    /// Stable directive identifier independent of any agent identity.
    pub directive_id: String,
    /// User-originated intent text.
    pub text: String,
}

/// Source-neutral seed-agent values supplied to world-model bootstrap.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeedAgentActivation {
    /// Stable seed agent identifier.
    pub agent_id: String,
    /// Perspective used by the seed agent.
    pub perspective_key: PerspectiveKey,
    /// Subject observed by the seed agent.
    pub subject: DomainObjectRef,
    /// Branch observed by the seed agent.
    pub branch_scope: BranchScope,
    /// Named observation scope for the seed.
    pub observation_scope: String,
    /// Directive referenced by the canonical agent record.
    pub directive_id: String,
    /// Provenance recorded for trusted seed creation.
    pub seed_provenance: String,
}

/// Stable logical identity for migrating one embedded legacy directive.
///
/// W2B bootstrap owns the private compatibility decoder and transactional
/// migration. This contract carries no legacy record shape and performs no
/// persistence mutation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LegacyDirectiveMigrationIdentity {
    /// Compatibility schema interpreted by W2B bootstrap.
    pub schema_version: u32,
    /// Stable product activation that requested the migration.
    pub activation_id: String,
    /// Stable one-shot bootstrap identity reused across retries.
    pub bootstrap_id: String,
    /// Legacy agent whose embedded directive is migrated.
    pub agent_id: String,
    /// Canonical directive identity selected for the migrated record.
    pub directive_id: String,
}

/// Agent field whose configured and legacy values prevented migration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LegacyDirectiveMigrationConflictField {
    /// Stable agent identity.
    AgentId,
    /// Perspective identity and policy.
    PerspectiveKey,
    /// Observed domain object.
    Subject,
    /// Observed branch identity.
    BranchScope,
    /// Named observation scope.
    ObservationScope,
    /// Canonical directive identity.
    DirectiveId,
    /// User-originated directive text.
    DirectiveText,
    /// Trusted seed provenance.
    SeedProvenance,
}

/// Structured diagnostic for a divergent legacy directive migration replay.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LegacyDirectiveMigrationConflict {
    /// Canonical migration identity whose values diverged.
    pub identity: LegacyDirectiveMigrationIdentity,
    /// Semantic field that differed.
    pub field: LegacyDirectiveMigrationConflictField,
    /// BLAKE3 digest of the configured field value.
    pub configured_value_hash: String,
    /// BLAKE3 digest of the legacy field value.
    pub legacy_value_hash: String,
}

/// Durable proof that one legacy embedded directive migration completed.
///
/// W2B bootstrap writes this receipt in the same transaction as the canonical
/// directive and agent records. The hashes characterize the compatibility
/// input and both migration outputs without exposing the private legacy wire
/// shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LegacyDirectiveMigrationReceipt {
    /// Stable receipt key derived from the migration identity.
    pub receipt_id: String,
    /// Canonical logical migration product.
    pub identity: LegacyDirectiveMigrationIdentity,
    /// BLAKE3 digest of the decoded legacy agent record.
    pub legacy_agent_record_hash: String,
    /// BLAKE3 digest of the canonical directive record.
    pub directive_record_hash: String,
    /// BLAKE3 digest of the canonical agent record.
    pub canonical_agent_record_hash: String,
    /// Sequence assigned by W2B bootstrap after transactional completion.
    pub completed_at_seq: u64,
}

/// Durable configured curation rule attributed to one agent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentCurationRuleRecord {
    /// Stable rule identifier.
    pub rule_id: String,
    /// Agent that owns and evaluates the rule.
    pub agent_id: String,
    /// Canonical threshold-rule configuration.
    pub config: AgentCurationRuleConfig,
}

/// Complete owner-scoped input for one world-model product activation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldModelActivationInput {
    /// Hash binding normalized activation content and resolved deployment coordinates.
    pub activation_hash: String,
    /// Stable logical product activation identifier.
    pub activation_id: String,
    /// Stable logical bootstrap identity, independent of config revisions.
    pub bootstrap_id: String,
    /// Belief family semantics supplied as typed data.
    pub belief_family: BeliefFamilyConfig,
    /// Directive that explains why the seed agent exists.
    pub directive: DirectiveRecord,
    /// Seed-agent values that reference the directive by identity.
    pub seed_agent: SeedAgentActivation,
    /// Durable curation rule bound to the seed agent.
    pub curation_rule: AgentCurationRuleRecord,
    /// Exact belief stream that bootstrap must bind as a subscription.
    pub belief_key: BeliefKey,
}

/// Deterministic identity projection derived by pure activation validation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldModelActivationIdentity {
    /// Root hash binding normalized content and resolved deployment coordinates.
    pub activation_hash: String,
    /// Stable logical product activation identifier.
    pub activation_id: String,
    /// Stable logical bootstrap identity.
    pub bootstrap_id: String,
    /// Canonical BLAKE3 hash of the owner-scoped input.
    pub input_hash: String,
    /// Belief-owned hash of the validated family configuration.
    pub belief_config_hash: String,
}

/// Durable acknowledgement that belief configuration was activated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BeliefActivationReceipt {
    /// Stable belief family identifier.
    pub family_id: String,
    /// Hash of the exact validated family configuration.
    pub config_snapshot_hash: String,
    /// Root hash binding normalized content and resolved deployment coordinates.
    pub activation_hash: String,
    /// Stable logical product activation identifier.
    pub activation_id: String,
}

/// Durable acknowledgement that one-shot world-model bootstrap completed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentBootstrapReceipt {
    /// Stable receipt identity derived from the logical bootstrap id.
    pub receipt_id: String,
    /// Stable logical bootstrap identity.
    pub bootstrap_id: String,
    /// Root hash binding normalized content and resolved deployment coordinates.
    pub activation_hash: String,
    /// Stable logical product activation identifier.
    pub activation_id: String,
    /// Canonical hash of the complete world-model activation input.
    pub input_hash: String,
    /// Activated belief configuration acknowledgement.
    pub belief: BeliefActivationReceipt,
    /// Directive confirmed by bootstrap.
    pub directive_id: String,
    /// Agent confirmed by bootstrap.
    pub agent_id: String,
    /// Curation rule confirmed by bootstrap.
    pub rule_id: String,
    /// Deterministic subscription confirmed by bootstrap.
    pub subscription_id: String,
    /// Agent activation attempt completed by bootstrap.
    pub agent_activation_id: String,
    /// Sequence at which bootstrap completed.
    pub completed_at_seq: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn migration_identity() -> LegacyDirectiveMigrationIdentity {
        LegacyDirectiveMigrationIdentity {
            schema_version: LEGACY_DIRECTIVE_MIGRATION_SCHEMA_VERSION,
            activation_id: "activation.docs_freshness".to_string(),
            bootstrap_id: "bootstrap.docs_freshness".to_string(),
            agent_id: "seed.docs_freshness".to_string(),
            directive_id: "directive.docs_freshness".to_string(),
        }
    }

    #[test]
    fn legacy_migration_identity_has_a_stable_versioned_wire_shape() {
        let identity = migration_identity();
        let encoded = serde_json::to_value(&identity).unwrap();

        assert_eq!(
            encoded,
            serde_json::json!({
                "schema_version": 1,
                "activation_id": "activation.docs_freshness",
                "bootstrap_id": "bootstrap.docs_freshness",
                "agent_id": "seed.docs_freshness",
                "directive_id": "directive.docs_freshness"
            })
        );
        assert_eq!(
            serde_json::from_value::<LegacyDirectiveMigrationIdentity>(encoded).unwrap(),
            identity
        );
    }

    #[test]
    fn legacy_migration_receipt_preserves_identity_and_hash_products() {
        let receipt = LegacyDirectiveMigrationReceipt {
            receipt_id: "legacy-directive-receipt-a".to_string(),
            identity: migration_identity(),
            legacy_agent_record_hash: "a".repeat(64),
            directive_record_hash: "b".repeat(64),
            canonical_agent_record_hash: "c".repeat(64),
            completed_at_seq: 17,
        };
        let encoded = serde_json::to_value(&receipt).unwrap();

        assert_eq!(encoded["identity"]["schema_version"], 1);
        assert_eq!(encoded["legacy_agent_record_hash"], "a".repeat(64));
        assert_eq!(encoded["directive_record_hash"], "b".repeat(64));
        assert_eq!(encoded["canonical_agent_record_hash"], "c".repeat(64));
        assert_eq!(
            serde_json::from_value::<LegacyDirectiveMigrationReceipt>(encoded).unwrap(),
            receipt
        );
    }

    #[test]
    fn legacy_migration_conflict_names_the_divergent_semantic_field() {
        let conflict = LegacyDirectiveMigrationConflict {
            identity: migration_identity(),
            field: LegacyDirectiveMigrationConflictField::DirectiveText,
            configured_value_hash: "d".repeat(64),
            legacy_value_hash: "e".repeat(64),
        };
        let encoded = serde_json::to_value(&conflict).unwrap();

        assert_eq!(encoded["field"], "directive_text");
        assert_eq!(
            serde_json::from_value::<LegacyDirectiveMigrationConflict>(encoded).unwrap(),
            conflict
        );
    }

    #[test]
    fn legacy_migration_contracts_reject_unknown_fields() {
        let mut value = serde_json::to_value(migration_identity()).unwrap();
        value
            .as_object_mut()
            .unwrap()
            .insert("legacy_path".to_string(), serde_json::json!("agents/tree"));

        assert!(serde_json::from_value::<LegacyDirectiveMigrationIdentity>(value).is_err());
    }
}
