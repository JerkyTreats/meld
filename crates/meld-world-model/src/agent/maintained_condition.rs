//! Agent-owned standing maintained conditions and exact revisions.

use std::io;

use meld_lang::{Condition, GoalPriority, Proposition, Term};
use serde::{Deserialize, Serialize};
use sled::{Db, Tree};

use crate::belief::{TheoryInstallDisposition, TheoryRevisionRef};
use crate::error::StorageError;
use crate::events::DomainObjectRef;

const REGISTRY_ID: &str = "agent_maintained_condition";
const TREE_REVISIONS: &str = "agent_maintained_condition_registry_revisions";

/// Reusable desired state that remains authoritative across Goal lifecycles.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentMaintainedCondition {
    /// Stable maintained-condition identity.
    pub condition_id: String,
    /// Belief dimension evaluated for the grounded subject.
    pub dimension_id: String,
    /// Desired comparison over the current projected value.
    pub desired: Condition,
    /// Priority copied into transient Goals caused by a breach.
    pub goal_priority: GoalPriority,
    /// Human-readable desired state copied into Goal provenance.
    pub desired_summary: String,
}

impl AgentMaintainedCondition {
    /// Validate stable identity and ground desired-state semantics.
    pub fn validate(&self) -> Result<(), StorageError> {
        require_non_empty("maintained condition id", &self.condition_id)?;
        require_non_empty("maintained condition dimension id", &self.dimension_id)?;
        require_non_empty(
            "maintained condition desired summary",
            &self.desired_summary,
        )?;
        if let Some(issue) = self.desired.grounding_issue() {
            return Err(StorageError::InvalidPath(format!(
                "maintained condition desired comparison must be ground: {issue}"
            )));
        }
        Ok(())
    }

    /// Ground the reusable desired state against one concrete subject.
    pub fn target_for(&self, subject: DomainObjectRef) -> Result<Proposition, StorageError> {
        self.validate()?;
        Ok(Proposition::Holds {
            subject: Term::Object(subject),
            dimension: Term::Dimension(self.dimension_id.clone()),
            condition: self.desired.clone(),
        })
    }

    /// Lower one legacy threshold rule into an ephemeral compatibility body.
    // TODO compat-shim: remove after every Agent binds an exact maintained
    // condition revision and threshold-only characterization remains green.
    pub fn from_legacy_rule(rule: &super::AgentCurationRuleConfig) -> Result<Self, StorageError> {
        rule.validate()?;
        Ok(Self {
            condition_id: format!("compat-threshold::{}", rule.dimension_id),
            dimension_id: rule.dimension_id.clone(),
            desired: rule.target_condition(),
            goal_priority: GoalPriority {
                urgency: rule.priority_urgency,
                cost_ceiling: None,
            },
            desired_summary: rule.desired_summary.clone(),
        })
    }
}

/// Maintained condition bound to an Agent for exact runtime activation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentMaintainedConditionBinding {
    /// Intact installed condition body.
    pub condition: AgentMaintainedCondition,
    /// Exact owner revision selected by the complete receipt.
    pub revision: TheoryRevisionRef,
}

impl AgentMaintainedConditionBinding {
    /// Build and validate an exact binding.
    pub fn for_revision(
        condition: AgentMaintainedCondition,
        revision: TheoryRevisionRef,
    ) -> Result<Self, StorageError> {
        let binding = Self {
            condition,
            revision,
        };
        binding.validate()?;
        Ok(binding)
    }

    /// Validate body identity and exact owner reference.
    pub fn validate(&self) -> Result<(), StorageError> {
        self.condition.validate()?;
        self.revision.validate_for_registry(REGISTRY_ID)?;
        if self.revision.id != self.condition.condition_id {
            return Err(StorageError::InvalidPath(
                "maintained condition revision id does not match its body".to_string(),
            ));
        }
        if self.revision.content_hash != hash_body(&self.condition)? {
            return Err(StorageError::InvalidPath(
                "maintained condition revision hash does not match its body".to_string(),
            ));
        }
        Ok(())
    }
}

/// One append-only maintained-condition revision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentMaintainedConditionRevision {
    /// Stable condition identity.
    pub condition_id: String,
    /// Canonical condition content hash.
    pub content_hash: String,
    /// Intact condition body.
    pub condition: AgentMaintainedCondition,
    /// Sequence observed on first installation.
    pub installed_at_seq: u64,
}

impl AgentMaintainedConditionRevision {
    /// Return the exact world-model reference.
    pub fn revision_ref(&self) -> TheoryRevisionRef {
        TheoryRevisionRef {
            registry: REGISTRY_ID.to_string(),
            id: self.condition_id.clone(),
            content_hash: self.content_hash.clone(),
        }
    }

    /// Return a validated runtime binding for this exact revision.
    pub fn binding(&self) -> Result<AgentMaintainedConditionBinding, StorageError> {
        AgentMaintainedConditionBinding::for_revision(self.condition.clone(), self.revision_ref())
    }
}

/// Append-only Agent-owned maintained-condition registry.
#[derive(Clone)]
pub struct AgentMaintainedConditionRegistryStore {
    db: Db,
    revisions: Tree,
}

impl AgentMaintainedConditionRegistryStore {
    /// Open the Agent-owned registry tree.
    pub fn new(db: Db) -> Result<Self, StorageError> {
        Ok(Self {
            revisions: db.open_tree(TREE_REVISIONS).map_err(to_storage_io)?,
            db,
        })
    }

    /// Install or reuse one exact revision.
    pub fn install(
        &self,
        condition: AgentMaintainedCondition,
        installed_at_seq: u64,
    ) -> Result<(TheoryInstallDisposition, AgentMaintainedConditionRevision), StorageError> {
        condition.validate()?;
        let content_hash = hash_body(&condition)?;
        if let Some(existing) = self.resolve(&condition.condition_id, &content_hash)? {
            return Ok((TheoryInstallDisposition::Unchanged, existing));
        }
        let revision = AgentMaintainedConditionRevision {
            condition_id: condition.condition_id.clone(),
            content_hash,
            condition,
            installed_at_seq,
        };
        self.revisions
            .insert(
                revision_key(&revision.condition_id, &revision.content_hash)?,
                encode(&revision)?,
            )
            .map_err(to_storage_io)?;
        self.db.flush().map_err(to_storage_io)?;
        Ok((TheoryInstallDisposition::Installed, revision))
    }

    /// Resolve and integrity-check one historical revision.
    pub fn resolve(
        &self,
        condition_id: &str,
        content_hash: &str,
    ) -> Result<Option<AgentMaintainedConditionRevision>, StorageError> {
        let Some(raw) = self
            .revisions
            .get(revision_key(condition_id, content_hash)?)
            .map_err(to_storage_io)?
        else {
            return Ok(None);
        };
        let revision: AgentMaintainedConditionRevision = decode(&raw)?;
        revision.condition.validate()?;
        if revision.condition_id != condition_id
            || revision.condition.condition_id != condition_id
            || revision.content_hash != content_hash
            || hash_body(&revision.condition)? != revision.content_hash
        {
            return Err(StorageError::InvalidPath(
                "maintained condition revision content identity is corrupt".to_string(),
            ));
        }
        Ok(Some(revision))
    }
}

fn hash_body(condition: &AgentMaintainedCondition) -> Result<String, StorageError> {
    Ok(blake3::hash(&encode(condition)?).to_hex().to_string())
}

fn revision_key(id: &str, hash: &str) -> Result<Vec<u8>, StorageError> {
    encode(&(id, hash))
}

fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>, StorageError> {
    serde_json::to_vec(value).map_err(to_storage_data)
}

fn decode<T: for<'de> Deserialize<'de>>(bytes: &[u8]) -> Result<T, StorageError> {
    serde_json::from_slice(bytes).map_err(to_storage_data)
}

fn require_non_empty(label: &str, value: &str) -> Result<(), StorageError> {
    if value.trim().is_empty() {
        return Err(StorageError::InvalidPath(format!(
            "{label} must not be empty"
        )));
    }
    Ok(())
}

fn to_storage_io(error: sled::Error) -> StorageError {
    StorageError::IoError(io::Error::other(error.to_string()))
}

fn to_storage_data(error: serde_json::Error) -> StorageError {
    StorageError::IoError(io::Error::new(
        io::ErrorKind::InvalidData,
        error.to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use meld_lang::{Literal, Term};

    fn condition(threshold: f64) -> AgentMaintainedCondition {
        AgentMaintainedCondition {
            condition_id: "docs-correct".into(),
            dimension_id: "docs_freshness".into(),
            desired: Condition::Above(Term::Literal(Literal::Number(threshold))),
            goal_priority: GoalPriority {
                urgency: 50,
                cost_ceiling: None,
            },
            desired_summary: "confidence above threshold".into(),
        }
    }

    #[test]
    fn exact_revisions_are_append_only_and_bind_intact_bodies() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let store = AgentMaintainedConditionRegistryStore::new(db).unwrap();
        let (_, first) = store.install(condition(0.7), 4).unwrap();
        let (same, replay) = store.install(condition(0.7), 9).unwrap();
        let (_, second) = store.install(condition(0.8), 10).unwrap();
        assert_eq!(same, TheoryInstallDisposition::Unchanged);
        assert_eq!(replay.installed_at_seq, 4);
        assert_ne!(first.content_hash, second.content_hash);
        assert_eq!(first.binding().unwrap().condition, first.condition);
        assert_eq!(
            store.resolve("docs-correct", &first.content_hash).unwrap(),
            Some(first)
        );
    }

    #[test]
    fn desired_comparison_must_be_ground() {
        let mut invalid = condition(0.7);
        invalid.desired = Condition::Above(Term::Variable("?threshold".into()));
        assert!(invalid.validate().is_err());
    }
}
