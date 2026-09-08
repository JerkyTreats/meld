//! Durable exact revisions of Strategy-owned theory packages.

use std::collections::BTreeSet;
use std::io;

use serde::{Deserialize, Serialize};
use sled::{Db, Tree};

use super::StrategyTheoryPackage;
use crate::belief::{TheoryInstallDisposition, TheoryRevisionRef};
use crate::error::StorageError;

const REGISTRY_ID: &str = "strategy_theory";
const TREE_REVISIONS: &str = "strategy_theory_registry_revisions";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// One installed complete Strategy theory revision.
pub struct StrategyTheoryRevision {
    /// Stable Strategy theory identity.
    pub theory_id: String,
    /// Canonical package content hash.
    pub content_hash: String,
    /// Intact Strategy theory package.
    pub package: StrategyTheoryPackage,
    /// Sequence observed on first installation.
    pub installed_at_seq: u64,
}

impl StrategyTheoryRevision {
    /// Return the exact world-model reference.
    pub fn revision_ref(&self) -> TheoryRevisionRef {
        TheoryRevisionRef {
            registry: REGISTRY_ID.to_string(),
            id: self.theory_id.clone(),
            content_hash: self.content_hash.clone(),
        }
    }
}

#[derive(Clone)]
/// Append-only store for exact Strategy theory revisions.
pub struct StrategyTheoryRegistryStore {
    db: Db,
    revisions: Tree,
}

impl StrategyTheoryRegistryStore {
    /// Open the Strategy-owned tree in the world-model database.
    pub fn new(db: Db) -> Result<Self, StorageError> {
        Ok(Self {
            revisions: db.open_tree(TREE_REVISIONS).map_err(to_storage_io)?,
            db,
        })
    }

    /// Install or reuse one exact Strategy package revision.
    pub fn install(
        &self,
        package: StrategyTheoryPackage,
        installed_at_seq: u64,
    ) -> Result<(TheoryInstallDisposition, StrategyTheoryRevision), StorageError> {
        validate_strategy_theory_package(&package)?;
        let theory_id = package.snapshot.theory_id.clone();
        let content_hash = hash_body(&package)?;
        if let Some(existing) = self.resolve(&theory_id, &content_hash)? {
            return Ok((TheoryInstallDisposition::Unchanged, existing));
        }
        let revision = StrategyTheoryRevision {
            theory_id: theory_id.clone(),
            content_hash,
            package,
            installed_at_seq,
        };
        self.revisions
            .insert(
                revision_key(&theory_id, &revision.content_hash)?,
                encode(&revision)?,
            )
            .map_err(to_storage_io)?;
        self.db.flush().map_err(to_storage_io)?;
        Ok((TheoryInstallDisposition::Installed, revision))
    }

    /// Resolve and integrity-check one historical Strategy revision.
    pub fn resolve(
        &self,
        theory_id: &str,
        content_hash: &str,
    ) -> Result<Option<StrategyTheoryRevision>, StorageError> {
        let Some(raw) = self
            .revisions
            .get(revision_key(theory_id, content_hash)?)
            .map_err(to_storage_io)?
        else {
            return Ok(None);
        };
        let revision: StrategyTheoryRevision = decode(&raw)?;
        validate_strategy_theory_package(&revision.package)?;
        if revision.theory_id != theory_id
            || revision.package.snapshot.theory_id != theory_id
            || revision.content_hash != content_hash
            || hash_body(&revision.package)? != revision.content_hash
        {
            return Err(StorageError::InvalidPath(
                "strategy theory revision content identity is corrupt".to_string(),
            ));
        }
        Ok(Some(revision))
    }
}

/// Validate a complete Strategy package before storage or activation.
pub fn validate_strategy_theory_package(
    package: &StrategyTheoryPackage,
) -> Result<(), StorageError> {
    require_non_empty("strategy theory id", &package.snapshot.theory_id)?;
    require_non_empty(
        "strategy evaluation policy id",
        &package.evaluation_policy.policy_id,
    )?;
    if package.snapshot.settlement_rules.is_empty() || package.capabilities.is_empty() {
        return Err(StorageError::InvalidPath(
            "strategy theory requires settlement rules and capabilities".to_string(),
        ));
    }
    if package.search_bounds.max_expansions == 0 || package.search_bounds.max_depth == 0 {
        return Err(StorageError::InvalidPath(
            "strategy search bounds must be greater than zero".to_string(),
        ));
    }
    let mut contracts = BTreeSet::new();
    for capability in &package.capabilities {
        require_non_empty("strategy capability contract id", &capability.contract_id)?;
        require_non_empty(
            "strategy capability outcome contract id",
            &capability.outcome_contract_id,
        )?;
        if !contracts.insert(capability.contract_id.as_str()) {
            return Err(StorageError::InvalidPath(format!(
                "duplicate strategy capability contract '{}'",
                capability.contract_id
            )));
        }
    }
    let mut methods = BTreeSet::new();
    for method in &package.methods {
        require_non_empty("strategy method id", &method.method_id)?;
        if !methods.insert(method.method_id.as_str()) {
            return Err(StorageError::InvalidPath(format!(
                "duplicate strategy method '{}'",
                method.method_id
            )));
        }
    }
    let mut dimensions = BTreeSet::new();
    for dimension in &package.requested_dimensions {
        require_non_empty("strategy requested dimension", dimension)?;
        if !dimensions.insert(dimension.as_str()) {
            return Err(StorageError::InvalidPath(format!(
                "duplicate strategy requested dimension '{dimension}'"
            )));
        }
    }
    let mut requested_authority = BTreeSet::new();
    for action_id in &package.requested_authority {
        require_non_empty("strategy requested authority action", action_id)?;
        if !requested_authority.insert(action_id.as_str()) {
            return Err(StorageError::InvalidPath(format!(
                "duplicate strategy requested authority action '{action_id}'"
            )));
        }
    }
    for rule in &package.snapshot.settlement_rules {
        if rule
            .repeat_on_changed_owners
            .iter()
            .any(|owner| owner.trim().is_empty())
            || rule
                .repeat_on_changed_owners
                .iter()
                .collect::<BTreeSet<_>>()
                .len()
                != rule.repeat_on_changed_owners.len()
        {
            return Err(StorageError::InvalidPath(
                "Repeated-work owner selection must name distinct nonempty owners".into(),
            ));
        }
        let mut ordering = BTreeSet::new();
        for constraint in &rule.task_ordering {
            if constraint.before_contract_id == constraint.after_contract_id
                || !contracts.contains(constraint.before_contract_id.as_str())
                || !contracts.contains(constraint.after_contract_id.as_str())
                || !ordering.insert(constraint)
            {
                return Err(StorageError::InvalidPath(
                    "Task ordering requires distinct exact selected contracts without duplicates"
                        .into(),
                ));
            }
        }

        require_non_empty(
            "prospective evidence route id",
            &rule.evidence_route.route_id,
        )?;
        require_non_empty(
            "prospective evidence dimension",
            &rule.evidence_route.dimension_id,
        )?;
        require_non_empty(
            "prospective outcome contract",
            &rule.evidence_route.outcome_contract_id,
        )?;
        require_non_empty(
            "prospective evidence schema",
            &rule.evidence_route.evidence_schema_id,
        )?;
        if !package.capabilities.iter().any(|capability| {
            capability.outcome_contract_id == rule.evidence_route.outcome_contract_id
        }) {
            return Err(StorageError::InvalidPath(format!(
                "strategy evidence route '{}' has no capability outcome binding",
                rule.evidence_route.route_id
            )));
        }
    }
    Ok(())
}

fn require_non_empty(label: &str, value: &str) -> Result<(), StorageError> {
    if value.trim().is_empty() {
        return Err(StorageError::InvalidPath(format!(
            "{label} must not be empty"
        )));
    }
    Ok(())
}

fn hash_body<T: Serialize>(body: &T) -> Result<String, StorageError> {
    Ok(blake3::hash(&encode(body)?).to_hex().to_string())
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

    fn package() -> StrategyTheoryPackage {
        serde_json::from_str(include_str!(
            "../../../../theory/docs_freshness/strategy_theory.docs_freshness.json"
        ))
        .unwrap()
    }

    #[test]
    fn empty_construction_extensions_preserve_historical_package_identity() {
        let package = package();
        let body = serde_json::to_value(&package).unwrap();
        assert!(body.get("methods").is_none());
        let mut with_empty = body.clone();
        with_empty["methods"] = serde_json::json!([]);
        assert!(body["snapshot"]["settlement_rules"][0]
            .get("task_ordering")
            .is_none());
        with_empty["snapshot"]["settlement_rules"][0]["task_ordering"] = serde_json::json!([]);
        let decoded: StrategyTheoryPackage = serde_json::from_value(with_empty).unwrap();
        assert_eq!(hash_body(&decoded).unwrap(), hash_body(&package).unwrap());
    }

    #[test]
    fn exact_strategy_revisions_are_idempotent_and_historical() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let store = StrategyTheoryRegistryStore::new(db).unwrap();
        let (_, first) = store.install(package(), 3).unwrap();
        let (same, replay) = store.install(package(), 8).unwrap();
        let mut changed = package();
        changed.search_bounds.max_expansions += 1;
        let (_, second) = store.install(changed, 9).unwrap();

        assert_eq!(same, TheoryInstallDisposition::Unchanged);
        assert_eq!(replay.installed_at_seq, 3);
        assert_ne!(first.content_hash, second.content_hash);
        assert_eq!(
            store
                .resolve(&first.theory_id, &first.content_hash)
                .unwrap(),
            Some(first)
        );
    }
}
