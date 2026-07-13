//! Durable activation of validated belief family configuration.

use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};
use sled::transaction::{TransactionError, Transactional};
use sled::{Db, Tree};

use crate::activation::{
    BeliefActivationReceipt, WorldModelActivationIdentity, WorldModelActivationInput,
};
use crate::belief::BeliefConfigLoader;
use crate::error::StorageError;

const TREE_CONFIG_SNAPSHOTS: &str = "belief_config_snapshots";
const TREE_CONFIG_BY_FAMILY: &str = "belief_config_by_family";
const TREE_ACTIVATION_RECEIPTS: &str = "belief_activation_receipts";

/// Exact immutable configuration snapshot used by a cross-domain activation fence.
#[derive(Debug, Clone)]
pub(crate) struct BeliefConfigSnapshotFence {
    pub(crate) tree: Tree,
    pub(crate) hash: String,
    pub(crate) json: String,
}

impl BeliefConfigSnapshotFence {
    /// Open the belief-owned snapshot tree for one validated fence identity.
    pub(crate) fn open(db: &Db, hash: String, json: String) -> Result<Self, StorageError> {
        if hash.trim().is_empty() || json.trim().is_empty() {
            return Err(StorageError::InvalidPath(
                "belief config fence requires an identity and serialized snapshot".to_string(),
            ));
        }
        Ok(Self {
            tree: db
                .open_tree(TREE_CONFIG_SNAPSHOTS)
                .map_err(|error| StorageError::IoError(std::io::Error::other(error.to_string())))?,
            hash,
            json,
        })
    }

    /// Reopen and verify one exact durable belief-owned configuration snapshot.
    pub(crate) fn reopen(db: &Db, hash: String, json: String) -> Result<Self, StorageError> {
        let fence = Self::open(db, hash, json)?;
        if fence
            .tree
            .get(fence.hash.as_bytes())
            .map_err(|error| StorageError::IoError(std::io::Error::other(error.to_string())))?
            .as_deref()
            != Some(fence.json.as_bytes())
        {
            return Err(StorageError::Backpressure(
                "durable belief config snapshot changed or is missing".to_string(),
            ));
        }
        Ok(fence)
    }
}

/// Failure returned while binding one belief family activation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BeliefActivationError {
    /// A stable family or activation identity already names divergent content.
    Conflict {
        /// Semantic field or record identity that diverged.
        field: String,
        /// Hash selected by activation input.
        configured_value_hash: String,
        /// Hash already durable in the belief store.
        durable_value_hash: String,
    },
    /// Validation, encoding, or storage failed.
    Storage {
        /// Bounded failure detail.
        message: String,
    },
}

impl fmt::Display for BeliefActivationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Conflict { field, .. } => {
                write!(formatter, "belief activation conflict at '{field}'")
            }
            Self::Storage { message } => write!(formatter, "belief activation failed: {message}"),
        }
    }
}

impl Error for BeliefActivationError {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BeliefFamilyBinding {
    family_id: String,
    config_snapshot_hash: String,
}

/// Belief-owned facade for durable family configuration activation.
#[derive(Clone)]
pub(crate) struct BeliefActivation {
    db: Db,
    config_snapshots: Tree,
    config_by_family: Tree,
    activation_receipts: Tree,
}

impl BeliefActivation {
    /// Open belief activation trees on the shared world-model database.
    pub(crate) fn new(db: Db) -> Result<Self, BeliefActivationError> {
        Ok(Self {
            config_snapshots: db.open_tree(TREE_CONFIG_SNAPSHOTS).map_err(storage_error)?,
            config_by_family: db.open_tree(TREE_CONFIG_BY_FAMILY).map_err(storage_error)?,
            activation_receipts: db
                .open_tree(TREE_ACTIVATION_RECEIPTS)
                .map_err(storage_error)?,
            db,
        })
    }

    /// Bind one validated family config and return its durable activation receipt.
    pub(crate) fn activate(
        &self,
        input: &WorldModelActivationInput,
        identity: &WorldModelActivationIdentity,
    ) -> Result<BeliefActivationReceipt, BeliefActivationError> {
        let snapshot =
            BeliefConfigLoader::snapshot(input.belief_family.clone()).map_err(|error| {
                BeliefActivationError::Storage {
                    message: error.to_string(),
                }
            })?;
        if snapshot.hash != identity.belief_config_hash {
            return Err(BeliefActivationError::Conflict {
                field: "belief_config_hash".to_string(),
                configured_value_hash: identity.belief_config_hash.clone(),
                durable_value_hash: snapshot.hash,
            });
        }
        let config_json = serde_json::to_vec(&snapshot.config).map_err(encode_error)?;
        let receipt = BeliefActivationReceipt {
            family_id: snapshot.config.family_id.clone(),
            config_snapshot_hash: identity.belief_config_hash.clone(),
            activation_hash: identity.activation_hash.clone(),
            activation_id: identity.activation_id.clone(),
        };
        let binding = BeliefFamilyBinding {
            family_id: receipt.family_id.clone(),
            config_snapshot_hash: receipt.config_snapshot_hash.clone(),
        };
        let binding_bytes = serde_json::to_vec(&binding).map_err(encode_error)?;
        let receipt_bytes = serde_json::to_vec(&receipt).map_err(encode_error)?;
        let receipt_key = receipt_key(&receipt.activation_id, &receipt.family_id);

        (
            &self.config_snapshots,
            &self.config_by_family,
            &self.activation_receipts,
        )
            .transaction(|(snapshots, families, receipts)| {
                if let Some(raw) = families.get(binding.family_id.as_bytes())? {
                    let durable: BeliefFamilyBinding = serde_json::from_slice(&raw)
                        .map_err(|error| abort_storage(error.to_string()))?;
                    if durable != binding {
                        return Err(abort_conflict(
                            "belief_family.config_snapshot_hash",
                            binding.config_snapshot_hash.as_bytes(),
                            durable.config_snapshot_hash.as_bytes(),
                        ));
                    }
                }
                if let Some(raw) = snapshots.get(binding.config_snapshot_hash.as_bytes())? {
                    if raw.as_ref() != config_json.as_slice() {
                        return Err(abort_conflict(
                            "belief_config_snapshot.content",
                            &config_json,
                            raw.as_ref(),
                        ));
                    }
                }
                if let Some(raw) = receipts.get(receipt_key.as_bytes())? {
                    let durable: BeliefActivationReceipt = serde_json::from_slice(&raw)
                        .map_err(|error| abort_storage(error.to_string()))?;
                    if durable != receipt {
                        return Err(abort_conflict(
                            "belief_activation_receipt",
                            &receipt_bytes,
                            raw.as_ref(),
                        ));
                    }
                }

                snapshots.insert(
                    binding.config_snapshot_hash.as_bytes(),
                    config_json.as_slice(),
                )?;
                families.insert(binding.family_id.as_bytes(), binding_bytes.as_slice())?;
                receipts.insert(receipt_key.as_bytes(), receipt_bytes.as_slice())?;
                Ok(())
            })
            .map_err(map_transaction_error)?;
        self.flush()?;
        Ok(receipt)
    }

    /// Reconfirm every immutable belief activation product without writing.
    pub(crate) fn confirm(
        &self,
        input: &WorldModelActivationInput,
        identity: &WorldModelActivationIdentity,
    ) -> Result<BeliefActivationReceipt, BeliefActivationError> {
        let snapshot =
            BeliefConfigLoader::snapshot(input.belief_family.clone()).map_err(|error| {
                BeliefActivationError::Storage {
                    message: error.to_string(),
                }
            })?;
        if snapshot.hash != identity.belief_config_hash {
            return Err(conflict(
                "belief_config_hash",
                identity.belief_config_hash.as_bytes(),
                snapshot.hash.as_bytes(),
            ));
        }
        let expected_binding = BeliefFamilyBinding {
            family_id: snapshot.config.family_id.clone(),
            config_snapshot_hash: snapshot.hash.clone(),
        };
        let durable_binding: BeliefFamilyBinding = required_record(
            &self.config_by_family,
            &snapshot.config.family_id,
            "belief family binding",
        )?;
        if durable_binding != expected_binding {
            return Err(conflict_serialized(
                "belief_family.config_snapshot_hash",
                &expected_binding,
                &durable_binding,
            ));
        }
        let expected_snapshot = serde_json::to_vec(&snapshot.config).map_err(encode_error)?;
        let durable_snapshot = self
            .config_snapshots
            .get(snapshot.hash.as_bytes())
            .map_err(storage_error)?
            .ok_or_else(|| BeliefActivationError::Storage {
                message: "belief config snapshot is missing".to_string(),
            })?;
        if durable_snapshot.as_ref() != expected_snapshot.as_slice() {
            return Err(conflict(
                "belief_config_snapshot.content",
                &expected_snapshot,
                durable_snapshot.as_ref(),
            ));
        }
        let expected_receipt = BeliefActivationReceipt {
            family_id: snapshot.config.family_id.clone(),
            config_snapshot_hash: snapshot.hash,
            activation_hash: identity.activation_hash.clone(),
            activation_id: identity.activation_id.clone(),
        };
        let durable_receipt: BeliefActivationReceipt = required_record(
            &self.activation_receipts,
            &receipt_key(&identity.activation_id, &snapshot.config.family_id),
            "belief activation receipt",
        )?;
        if durable_receipt != expected_receipt {
            return Err(conflict_serialized(
                "belief_activation_receipt",
                &expected_receipt,
                &durable_receipt,
            ));
        }
        Ok(durable_receipt)
    }

    /// Flush all belief activation writes.
    pub(crate) fn flush(&self) -> Result<(), BeliefActivationError> {
        self.db.flush().map_err(storage_error)?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
enum ActivationAbort {
    Conflict {
        field: String,
        configured_value_hash: String,
        durable_value_hash: String,
    },
    Storage(String),
}

fn receipt_key(activation_id: &str, family_id: &str) -> String {
    let identity = serde_json::to_vec(&(activation_id, family_id))
        .expect("belief activation receipt identity serialization is infallible");
    hash(&identity)
}

fn abort_conflict(
    field: &str,
    configured: &[u8],
    durable: &[u8],
) -> sled::transaction::ConflictableTransactionError<ActivationAbort> {
    sled::transaction::ConflictableTransactionError::Abort(ActivationAbort::Conflict {
        field: field.to_string(),
        configured_value_hash: hash(configured),
        durable_value_hash: hash(durable),
    })
}

fn abort_storage(
    message: String,
) -> sled::transaction::ConflictableTransactionError<ActivationAbort> {
    sled::transaction::ConflictableTransactionError::Abort(ActivationAbort::Storage(message))
}

fn map_transaction_error(error: TransactionError<ActivationAbort>) -> BeliefActivationError {
    match error {
        TransactionError::Abort(ActivationAbort::Conflict {
            field,
            configured_value_hash,
            durable_value_hash,
        }) => BeliefActivationError::Conflict {
            field,
            configured_value_hash,
            durable_value_hash,
        },
        TransactionError::Abort(ActivationAbort::Storage(message)) => {
            BeliefActivationError::Storage { message }
        }
        TransactionError::Storage(error) => storage_error(error),
    }
}

fn hash(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}

fn required_record<T: serde::de::DeserializeOwned>(
    tree: &Tree,
    key: &str,
    record_name: &str,
) -> Result<T, BeliefActivationError> {
    let raw = tree
        .get(key.as_bytes())
        .map_err(storage_error)?
        .ok_or_else(|| BeliefActivationError::Storage {
            message: format!("{record_name} is missing"),
        })?;
    serde_json::from_slice(&raw).map_err(|error| BeliefActivationError::Storage {
        message: format!("cannot decode {record_name}: {error}"),
    })
}

fn conflict(field: &str, configured: &[u8], durable: &[u8]) -> BeliefActivationError {
    BeliefActivationError::Conflict {
        field: field.to_string(),
        configured_value_hash: hash(configured),
        durable_value_hash: hash(durable),
    }
}

fn conflict_serialized<T: Serialize>(
    field: &str,
    configured: &T,
    durable: &T,
) -> BeliefActivationError {
    let configured =
        serde_json::to_vec(configured).unwrap_or_else(|error| error.to_string().into_bytes());
    let durable =
        serde_json::to_vec(durable).unwrap_or_else(|error| error.to_string().into_bytes());
    conflict(field, &configured, &durable)
}

fn encode_error(error: serde_json::Error) -> BeliefActivationError {
    BeliefActivationError::Storage {
        message: error.to_string(),
    }
}

fn storage_error(error: sled::Error) -> BeliefActivationError {
    BeliefActivationError::Storage {
        message: error.to_string(),
    }
}
