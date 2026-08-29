use std::io;

use serde::{de::DeserializeOwned, Serialize};
use sled::{Db, Tree};

use crate::curation::{
    stable_identity, CurationAcceptanceRecord, CurationAdmissionDecision, CurationOperation,
    CurationPublicationKind, CurationPublicationReceipt, CurationResult, StandingCurationRule,
    StandingCurationRuleRevision,
};
use crate::error::StorageError;

const TREE_RULES: &str = "curation_rule_revisions";
const TREE_ACTIVE_RULES: &str = "curation_active_rules";
const TREE_OPERATIONS: &str = "curation_operations";
const TREE_OPERATION_BY_SELECTION: &str = "curation_operation_by_selection";
const TREE_ACCEPTANCES: &str = "curation_acceptances";
const TREE_ACCEPTANCE_BY_OPERATION: &str = "curation_acceptance_by_operation";
const TREE_RESULTS: &str = "curation_results";
const TREE_RESULT_BY_OPERATION: &str = "curation_result_by_operation";
const TREE_PUBLICATION_RECEIPTS: &str = "curation_publication_receipts";

/// Curation-owned durable state inside the shared world-model database.
#[derive(Clone)]
pub struct CurationStore {
    db: Db,
    rules: Tree,
    active_rules: Tree,
    operations: Tree,
    operation_by_selection: Tree,
    acceptances: Tree,
    acceptance_by_operation: Tree,
    results: Tree,
    result_by_operation: Tree,
    publication_receipts: Tree,
}

impl CurationStore {
    pub fn new(db: Db) -> Result<Self, StorageError> {
        Ok(Self {
            rules: db.open_tree(TREE_RULES).map_err(to_storage_io)?,
            active_rules: db.open_tree(TREE_ACTIVE_RULES).map_err(to_storage_io)?,
            operations: db.open_tree(TREE_OPERATIONS).map_err(to_storage_io)?,
            operation_by_selection: db
                .open_tree(TREE_OPERATION_BY_SELECTION)
                .map_err(to_storage_io)?,
            acceptances: db.open_tree(TREE_ACCEPTANCES).map_err(to_storage_io)?,
            acceptance_by_operation: db
                .open_tree(TREE_ACCEPTANCE_BY_OPERATION)
                .map_err(to_storage_io)?,
            results: db.open_tree(TREE_RESULTS).map_err(to_storage_io)?,
            result_by_operation: db
                .open_tree(TREE_RESULT_BY_OPERATION)
                .map_err(to_storage_io)?,
            publication_receipts: db
                .open_tree(TREE_PUBLICATION_RECEIPTS)
                .map_err(to_storage_io)?,
            db,
        })
    }

    /// Install one exact rule revision and select it for its Agent.
    pub fn install_rule(
        &self,
        rule: StandingCurationRule,
        installed_at_seq: u64,
    ) -> Result<StandingCurationRuleRevision, StorageError> {
        rule.validate()?;
        let content_hash = stable_identity("standing-curation-rule-v1", &rule)?;
        let candidate = StandingCurationRuleRevision {
            rule_id: rule.rule_id.clone(),
            content_hash,
            installed_at_seq,
            rule,
        };
        candidate.validate()?;
        let key = rule_revision_key(&candidate.rule_id, &candidate.content_hash);
        let revision = match self.rules.get(&key).map_err(to_storage_io)? {
            Some(raw) => {
                let existing: StandingCurationRuleRevision = decode(&raw)?;
                existing.validate()?;
                if existing.rule != candidate.rule {
                    return Err(StorageError::InvalidPath(
                        "standing Curation rule identity collision".to_string(),
                    ));
                }
                existing
            }
            None => {
                self.rules
                    .insert(&key, encode(&candidate)?)
                    .map_err(to_storage_io)?;
                candidate
            }
        };
        if let Some(current) = self.active_rule(&revision.rule.agent_id)? {
            if current.installed_at_seq > revision.installed_at_seq
                || current.installed_at_seq == revision.installed_at_seq
                    && current.content_hash != revision.content_hash
            {
                return Err(StorageError::InvalidPath(
                    "standing Curation active rule cannot regress or fork its install sequence"
                        .to_string(),
                ));
            }
        }
        self.active_rules
            .insert(
                revision.rule.agent_id.as_bytes(),
                revision_key_bytes(&revision),
            )
            .map_err(to_storage_io)?;
        self.db.flush().map_err(to_storage_io)?;
        Ok(revision)
    }

    pub fn active_rule(
        &self,
        agent_id: &str,
    ) -> Result<Option<StandingCurationRuleRevision>, StorageError> {
        let Some(key) = self
            .active_rules
            .get(agent_id.as_bytes())
            .map_err(to_storage_io)?
        else {
            return Ok(None);
        };
        let revision: StandingCurationRuleRevision = self
            .rules
            .get(key)
            .map_err(to_storage_io)?
            .map(|raw| decode(&raw))
            .transpose()?
            .ok_or_else(|| {
                StorageError::InvalidPath(
                    "standing Curation active rule points at a missing revision".to_string(),
                )
            })?;
        revision.validate()?;
        if revision.rule.agent_id != agent_id {
            return Err(StorageError::InvalidPath(
                "standing Curation active rule points at another Agent".to_string(),
            ));
        }
        Ok(Some(revision))
    }

    pub fn put_operation(&self, operation: &CurationOperation) -> Result<(), StorageError> {
        operation.validate()?;
        if let Some(existing) = self.operation_for_selection(&operation.selection_id)? {
            if existing != *operation {
                return Err(StorageError::InvalidPath(
                    "one standing Curation selection cannot create divergent operations"
                        .to_string(),
                ));
            }
            return Ok(());
        }
        put_exact(&self.operations, &operation.operation_id, operation)?;
        self.operation_by_selection
            .insert(
                operation.selection_id.as_bytes(),
                operation.operation_id.as_bytes(),
            )
            .map_err(to_storage_io)?;
        Ok(())
    }

    pub fn operation_for_selection(
        &self,
        selection_id: &str,
    ) -> Result<Option<CurationOperation>, StorageError> {
        let operation_id = self
            .operation_by_selection
            .get(selection_id.as_bytes())
            .map_err(to_storage_io)?;
        let Some(operation_id) = operation_id else {
            for row in self.operations.iter() {
                let (_, raw) = row.map_err(to_storage_io)?;
                let operation: CurationOperation = decode(&raw)?;
                operation.validate()?;
                if operation.selection_id == selection_id {
                    self.operation_by_selection
                        .insert(selection_id.as_bytes(), operation.operation_id.as_bytes())
                        .map_err(to_storage_io)?;
                    return Ok(Some(operation));
                }
            }
            return Ok(None);
        };
        let operation_id = std::str::from_utf8(&operation_id).map_err(|error| {
            StorageError::InvalidPath(format!("invalid Curation operation index: {error}"))
        })?;
        let operation: Option<CurationOperation> = get_optional(&self.operations, operation_id)?;
        if let Some(operation) = &operation {
            operation.validate()?;
        }
        Ok(operation)
    }

    pub fn put_acceptance(
        &self,
        acceptance: &CurationAcceptanceRecord,
    ) -> Result<(), StorageError> {
        self.validate_acceptance(acceptance)?;
        if let Some(existing) = self.acceptance_for_operation(&acceptance.operation_id)? {
            if existing != *acceptance {
                return Err(StorageError::InvalidPath(
                    "one Curation operation cannot have divergent admission decisions".to_string(),
                ));
            }
            return Ok(());
        }
        put_exact(&self.acceptances, &acceptance.acceptance_id, acceptance)?;
        self.acceptance_by_operation
            .insert(
                acceptance.operation_id.as_bytes(),
                acceptance.acceptance_id.as_bytes(),
            )
            .map_err(to_storage_io)?;
        self.db.flush().map_err(to_storage_io)?;
        Ok(())
    }

    pub fn acceptance(
        &self,
        acceptance_id: &str,
    ) -> Result<Option<CurationAcceptanceRecord>, StorageError> {
        let acceptance: Option<CurationAcceptanceRecord> =
            get_optional(&self.acceptances, acceptance_id)?;
        if let Some(acceptance) = &acceptance {
            self.validate_acceptance(acceptance)?;
        }
        Ok(acceptance)
    }

    /// Persist one terminal result together with its complete publication intent.
    pub fn put_result(&self, result: &CurationResult) -> Result<CurationResult, StorageError> {
        let operation = self.operation(&result.operation_id)?.ok_or_else(|| {
            StorageError::InvalidPath("Curation result requires its durable operation".to_string())
        })?;
        result.validate(&operation)?;
        let acceptance = self
            .acceptance_for_operation(&result.operation_id)?
            .ok_or_else(|| {
                StorageError::InvalidPath(
                    "Curation result requires a durable admission decision".to_string(),
                )
            })?;
        let rule = self.rule_revision(
            &acceptance.rule_revision.id,
            &acceptance.rule_revision.content_hash,
        )?;
        if acceptance != CurationAcceptanceRecord::for_operation(&operation, &rule)? {
            return Err(StorageError::InvalidPath(
                "Curation result references an invalid admission decision".to_string(),
            ));
        }
        if acceptance.decision != CurationAdmissionDecision::Admitted {
            return Err(StorageError::InvalidPath(
                "rejected Curation operation cannot acquire a terminal result".to_string(),
            ));
        }
        if let Some(existing) = self.result_for_operation(&result.operation_id)? {
            if existing != *result {
                return Err(StorageError::InvalidPath(
                    "one admitted Curation operation cannot have divergent terminal results"
                        .to_string(),
                ));
            }
            return Ok(existing);
        }
        put_exact(&self.results, &result.result_id, result)?;
        self.result_by_operation
            .insert(result.operation_id.as_bytes(), result.result_id.as_bytes())
            .map_err(to_storage_io)?;
        self.db.flush().map_err(to_storage_io)?;
        Ok(result.clone())
    }

    pub fn result_for_operation(
        &self,
        operation_id: &str,
    ) -> Result<Option<CurationResult>, StorageError> {
        let result_id = self
            .result_by_operation
            .get(operation_id.as_bytes())
            .map_err(to_storage_io)?;
        let Some(result_id) = result_id else {
            for row in self.results.iter() {
                let (_, raw) = row.map_err(to_storage_io)?;
                let result: CurationResult = decode(&raw)?;
                if result.operation_id == operation_id {
                    let operation = self.operation(operation_id)?.ok_or_else(|| {
                        StorageError::InvalidPath(
                            "Curation result references a missing operation".to_string(),
                        )
                    })?;
                    result.validate(&operation)?;
                    self.result_by_operation
                        .insert(operation_id.as_bytes(), result.result_id.as_bytes())
                        .map_err(to_storage_io)?;
                    return Ok(Some(result));
                }
            }
            return Ok(None);
        };
        let result_id = std::str::from_utf8(&result_id).map_err(|error| {
            StorageError::InvalidPath(format!("invalid Curation result index: {error}"))
        })?;
        let result: Option<CurationResult> = get_optional(&self.results, result_id)?;
        if let Some(result) = &result {
            let operation = self.operation(operation_id)?.ok_or_else(|| {
                StorageError::InvalidPath(
                    "Curation result references a missing operation".to_string(),
                )
            })?;
            result.validate(&operation)?;
        }
        Ok(result)
    }

    pub fn put_publication_receipt(
        &self,
        receipt: &CurationPublicationReceipt,
    ) -> Result<(), StorageError> {
        self.validate_publication_receipt(receipt)?;
        let key = publication_receipt_key(&receipt.result_id, receipt.kind);
        put_exact(&self.publication_receipts, &key, receipt)?;
        self.db.flush().map_err(to_storage_io)?;
        Ok(())
    }

    pub fn publication_receipt(
        &self,
        result_id: &str,
        kind: CurationPublicationKind,
    ) -> Result<Option<CurationPublicationReceipt>, StorageError> {
        let receipt: Option<CurationPublicationReceipt> = get_optional(
            &self.publication_receipts,
            &publication_receipt_key(result_id, kind),
        )?;
        if let Some(receipt) = &receipt {
            if receipt.result_id != result_id || receipt.kind != kind {
                return Err(StorageError::InvalidPath(
                    "Curation publication receipt index has divergent identity".to_string(),
                ));
            }
            self.validate_publication_receipt(receipt)?;
        }
        Ok(receipt)
    }

    pub fn flush(&self) -> Result<(), StorageError> {
        self.db.flush().map_err(to_storage_io)?;
        Ok(())
    }

    fn operation(&self, operation_id: &str) -> Result<Option<CurationOperation>, StorageError> {
        let operation: Option<CurationOperation> = get_optional(&self.operations, operation_id)?;
        if let Some(operation) = &operation {
            operation.validate()?;
        }
        Ok(operation)
    }

    fn acceptance_for_operation(
        &self,
        operation_id: &str,
    ) -> Result<Option<CurationAcceptanceRecord>, StorageError> {
        let acceptance_id = self
            .acceptance_by_operation
            .get(operation_id.as_bytes())
            .map_err(to_storage_io)?;
        let Some(acceptance_id) = acceptance_id else {
            for row in self.acceptances.iter() {
                let (_, raw) = row.map_err(to_storage_io)?;
                let acceptance: CurationAcceptanceRecord = decode(&raw)?;
                self.validate_acceptance(&acceptance)?;
                if acceptance.operation_id == operation_id {
                    self.acceptance_by_operation
                        .insert(operation_id.as_bytes(), acceptance.acceptance_id.as_bytes())
                        .map_err(to_storage_io)?;
                    return Ok(Some(acceptance));
                }
            }
            return Ok(None);
        };
        let acceptance_id = std::str::from_utf8(&acceptance_id).map_err(|error| {
            StorageError::InvalidPath(format!("invalid Curation acceptance index: {error}"))
        })?;
        let acceptance: Option<CurationAcceptanceRecord> =
            get_optional(&self.acceptances, acceptance_id)?;
        if let Some(acceptance) = &acceptance {
            self.validate_acceptance(acceptance)?;
        }
        Ok(acceptance)
    }

    fn validate_acceptance(
        &self,
        acceptance: &CurationAcceptanceRecord,
    ) -> Result<(), StorageError> {
        let operation = self.operation(&acceptance.operation_id)?.ok_or_else(|| {
            StorageError::InvalidPath(
                "Curation acceptance requires its durable operation".to_string(),
            )
        })?;
        let rule = self.rule_revision(
            &acceptance.rule_revision.id,
            &acceptance.rule_revision.content_hash,
        )?;
        if CurationAcceptanceRecord::for_operation(&operation, &rule)? != *acceptance {
            return Err(StorageError::InvalidPath(
                "Curation acceptance does not match its exact operation and rule".to_string(),
            ));
        }
        Ok(())
    }

    fn validate_publication_receipt(
        &self,
        receipt: &CurationPublicationReceipt,
    ) -> Result<(), StorageError> {
        let result: CurationResult =
            get_optional(&self.results, &receipt.result_id)?.ok_or_else(|| {
                StorageError::InvalidPath(
                    "Curation publication receipt requires its durable result".to_string(),
                )
            })?;
        let operation = self.operation(&result.operation_id)?.ok_or_else(|| {
            StorageError::InvalidPath(
                "Curation publication receipt references a missing operation".to_string(),
            )
        })?;
        result.validate(&operation)?;
        let expected_record_id = match receipt.kind {
            CurationPublicationKind::Semantic => result
                .semantic_publication
                .as_ref()
                .ok_or_else(|| {
                    StorageError::InvalidPath(
                        "semantic receipt requires semantic publication intent".to_string(),
                    )
                })?
                .event_record_id(),
            CurationPublicationKind::Terminal => result.event_record_id(),
        };
        if receipt.event_record_id != expected_record_id
            || receipt.event_position.ledger_id != result.source_event_position.ledger_id
            || receipt.event_position.after_seq <= result.source_event_position.after_seq
        {
            return Err(StorageError::InvalidPath(
                "Curation publication receipt does not match its Event intent".to_string(),
            ));
        }
        Ok(())
    }

    fn rule_revision(
        &self,
        rule_id: &str,
        content_hash: &str,
    ) -> Result<StandingCurationRuleRevision, StorageError> {
        let key = rule_revision_key(rule_id, content_hash);
        let revision: StandingCurationRuleRevision = self
            .rules
            .get(key)
            .map_err(to_storage_io)?
            .map(|raw| decode(&raw))
            .transpose()?
            .ok_or_else(|| {
                StorageError::InvalidPath(
                    "Curation operation references a missing rule revision".to_string(),
                )
            })?;
        revision.validate()?;
        Ok(revision)
    }
}

fn rule_revision_key(rule_id: &str, content_hash: &str) -> Vec<u8> {
    format!("{rule_id}::{content_hash}").into_bytes()
}

fn revision_key_bytes(revision: &StandingCurationRuleRevision) -> Vec<u8> {
    rule_revision_key(&revision.rule_id, &revision.content_hash)
}

fn publication_receipt_key(result_id: &str, kind: CurationPublicationKind) -> String {
    format!("{result_id}::{kind:?}")
}

fn put_exact<T: Serialize + DeserializeOwned + PartialEq>(
    tree: &Tree,
    key: &str,
    value: &T,
) -> Result<(), StorageError> {
    if let Some(raw) = tree.get(key.as_bytes()).map_err(to_storage_io)? {
        let existing: T = decode(&raw)?;
        if existing != *value {
            return Err(StorageError::InvalidPath(format!(
                "Curation durable identity '{key}' has divergent content"
            )));
        }
        return Ok(());
    }
    tree.insert(key.as_bytes(), encode(value)?)
        .map_err(to_storage_io)?;
    Ok(())
}

fn get_optional<T: DeserializeOwned>(tree: &Tree, key: &str) -> Result<Option<T>, StorageError> {
    tree.get(key.as_bytes())
        .map_err(to_storage_io)?
        .map(|raw| decode(&raw))
        .transpose()
}

fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>, StorageError> {
    serde_json::to_vec(value).map_err(to_storage_data)
}

fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, StorageError> {
    serde_json::from_slice(bytes).map_err(to_storage_data)
}

fn to_storage_io(error: sled::Error) -> StorageError {
    StorageError::IoError(io::Error::other(error.to_string()))
}

fn to_storage_data(error: serde_json::Error) -> StorageError {
    StorageError::IoError(io::Error::new(io::ErrorKind::InvalidData, error))
}
