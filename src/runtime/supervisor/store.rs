//! Durable storage for root supervisor lifecycle records.

use std::path::{Path, PathBuf};

use serde::{de::DeserializeOwned, Serialize};
use sled::transaction::{ConflictableTransactionError, TransactionError, Transactional};
use thiserror::Error;

use super::contracts::{
    canonical_runtime_id, RuntimeDesiredState, RuntimeHealthSnapshot, RuntimeHeartbeat, RuntimeId,
    RuntimeInstance, RuntimeLease, RuntimeLeaseOwner, RuntimeLeaseStatus, RuntimeRestartRecord,
    RuntimeShutdownState, SupervisorContractError, SupervisorLifecycleEvent,
};

const TREE_INSTANCES: &str = "runtime_instances";
const TREE_DESIRED: &str = "runtime_desired_state";
const TREE_LEASES: &str = "runtime_leases";
const TREE_ACTIVE_LEASES: &str = "runtime_active_leases";
const TREE_HEARTBEATS: &str = "runtime_heartbeats";
const TREE_HEALTH: &str = "runtime_health_snapshots";
const TREE_RESTARTS: &str = "runtime_restarts";
const TREE_SHUTDOWNS: &str = "runtime_shutdowns";
const TREE_LIFECYCLE_EVENTS: &str = "runtime_lifecycle_events";

/// Sled-backed root supervisor store.
///
/// This store is reserved for lifecycle records such as runtime instances,
/// leases, heartbeats, health snapshots, restart decisions, and shutdown
/// records. It must not contain domain progress cursors.
#[derive(Clone)]
pub struct SupervisorStore {
    path: PathBuf,
    db: sled::Db,
    instances: sled::Tree,
    desired: sled::Tree,
    leases: sled::Tree,
    active_leases: sled::Tree,
    heartbeats: sled::Tree,
    health: sled::Tree,
    restarts: sled::Tree,
    shutdowns: sled::Tree,
    lifecycle_events: sled::Tree,
}

/// Error returned while using supervisor lifecycle storage.
#[derive(Debug, Error)]
pub enum SupervisorStoreError {
    /// Filesystem setup failed before sled opened.
    #[error("supervisor store io error: {0}")]
    Io(String),
    /// Sled returned a storage error.
    #[error("supervisor store sled error: {0}")]
    Sled(String),
    /// Record encoding or decoding failed.
    #[error("supervisor store codec error: {0}")]
    Codec(String),
    /// Supervisor contract validation failed.
    #[error("supervisor contract error: {0}")]
    Contract(#[from] SupervisorContractError),
    /// Persisted lifecycle data violates supervisor invariants.
    #[error("invalid supervisor record: {0}")]
    InvalidRecord(String),
    /// Another unexpired active lease already owns this runtime id.
    #[error("runtime id '{runtime_id}' already has active lease '{active_lease_id}'")]
    DuplicateActiveLease {
        /// Runtime id with an active owner.
        runtime_id: String,
        /// Current active lease id.
        active_lease_id: String,
    },
    /// A supervisor instance id is already registered.
    #[error("supervisor instance id '{instance_id}' is already registered")]
    DuplicateInstanceId {
        /// Caller-supplied duplicate instance id.
        instance_id: String,
    },
    /// Caller is not the active lease owner for the runtime id.
    #[error(
        "stale lease owner for runtime id '{runtime_id}', lease '{lease_id}', instance '{instance_id}'"
    )]
    StaleLeaseOwner {
        /// Runtime id supplied by the caller.
        runtime_id: String,
        /// Lease id supplied by the caller.
        lease_id: String,
        /// Instance id supplied by the caller.
        instance_id: String,
    },
    /// Requested record was not found.
    #[error("missing supervisor {record_type} record '{key}'")]
    NotFound {
        /// Record family.
        record_type: &'static str,
        /// Lookup key.
        key: String,
    },
}

impl SupervisorStore {
    /// Open supervisor lifecycle storage at the supplied path.
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, SupervisorStoreError> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(to_io)?;
        }
        let db = sled::open(&path).map_err(to_sled)?;
        let instances = db.open_tree(TREE_INSTANCES).map_err(to_sled)?;
        let desired = db.open_tree(TREE_DESIRED).map_err(to_sled)?;
        let leases = db.open_tree(TREE_LEASES).map_err(to_sled)?;
        let active_leases = db.open_tree(TREE_ACTIVE_LEASES).map_err(to_sled)?;
        let heartbeats = db.open_tree(TREE_HEARTBEATS).map_err(to_sled)?;
        let health = db.open_tree(TREE_HEALTH).map_err(to_sled)?;
        let restarts = db.open_tree(TREE_RESTARTS).map_err(to_sled)?;
        let shutdowns = db.open_tree(TREE_SHUTDOWNS).map_err(to_sled)?;
        let lifecycle_events = db.open_tree(TREE_LIFECYCLE_EVENTS).map_err(to_sled)?;
        let store = Self {
            path,
            db,
            instances,
            desired,
            leases,
            active_leases,
            heartbeats,
            health,
            restarts,
            shutdowns,
            lifecycle_events,
        };
        store.migrate_requirement_era_runtime_ids()?;
        Ok(store)
    }

    /// Open supervisor lifecycle storage only when the store path already exists.
    pub fn open_existing(path: impl Into<PathBuf>) -> Result<Option<Self>, SupervisorStoreError> {
        let path = path.into();
        if !path.exists() {
            return Ok(None);
        }
        Self::open(path).map(Some)
    }

    /// Return the filesystem path used by this supervisor store.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Flush supervisor lifecycle records separately from product stores.
    pub fn flush(&self) -> Result<(), SupervisorStoreError> {
        self.db.flush().map_err(to_sled)?;
        Ok(())
    }

    fn migrate_requirement_era_runtime_ids(&self) -> Result<(), SupervisorStoreError> {
        let mut changed = false;
        changed |= migrate_record_tree::<RuntimeDesiredState, _>(&self.desired, |record| {
            record.runtime_id.as_str().as_bytes().to_vec()
        })?;
        changed |= migrate_record_tree::<RuntimeLease, _>(&self.leases, |record| {
            lease_key(&record.runtime_id, &record.lease_id)
        })?;
        changed |= migrate_active_lease_index(&self.active_leases)?;
        changed |= migrate_record_tree::<RuntimeHeartbeat, _>(&self.heartbeats, |record| {
            record.runtime_id.as_str().as_bytes().to_vec()
        })?;
        changed |= migrate_record_tree::<RuntimeHealthSnapshot, _>(&self.health, |record| {
            record.runtime_id.as_str().as_bytes().to_vec()
        })?;
        changed |= migrate_record_tree::<RuntimeRestartRecord, _>(&self.restarts, |record| {
            restart_key(&record.runtime_id, &record.restart_id)
        })?;
        changed |= normalize_record_values::<SupervisorLifecycleEvent>(&self.lifecycle_events)?;
        if changed {
            self.db.flush().map_err(to_sled)?;
        }
        Ok(())
    }

    /// Persist a supervisor instance record.
    pub fn put_runtime_instance(
        &self,
        record: &RuntimeInstance,
    ) -> Result<(), SupervisorStoreError> {
        require_non_empty("instance_id", &record.instance_id)?;
        self.instances
            .insert(record.instance_id.as_bytes(), encode(record)?)
            .map_err(to_sled)?;
        Ok(())
    }

    /// Register one new supervisor instance without overwriting an existing id.
    pub fn register_runtime_instance(
        &self,
        record: &RuntimeInstance,
    ) -> Result<(), SupervisorStoreError> {
        require_non_empty("instance_id", &record.instance_id)?;
        let key = record.instance_id.as_bytes().to_vec();
        let encoded = encode(record)?;
        self.instances
            .transaction(|instances| {
                if instances.get(key.clone())?.is_some() {
                    return Err(ConflictableTransactionError::Abort(
                        SupervisorStoreError::DuplicateInstanceId {
                            instance_id: record.instance_id.clone(),
                        },
                    ));
                }
                instances.insert(key.clone(), encoded.clone())?;
                Ok(())
            })
            .map_err(to_transaction)
    }

    /// Read a supervisor instance record.
    pub fn get_runtime_instance(
        &self,
        instance_id: &str,
    ) -> Result<Option<RuntimeInstance>, SupervisorStoreError> {
        read_optional(&self.instances, instance_id.as_bytes())
    }

    /// List supervisor instances ordered by start time and instance id.
    pub fn list_runtime_instances(&self) -> Result<Vec<RuntimeInstance>, SupervisorStoreError> {
        let mut records: Vec<RuntimeInstance> = read_all(&self.instances)?;
        records.sort_by(|left, right| {
            left.started_at_ms
                .cmp(&right.started_at_ms)
                .then_with(|| left.instance_id.cmp(&right.instance_id))
        });
        Ok(records)
    }

    /// Return the latest supervisor instance by start time and instance id.
    pub fn latest_runtime_instance(&self) -> Result<Option<RuntimeInstance>, SupervisorStoreError> {
        Ok(self.list_runtime_instances()?.into_iter().last())
    }

    /// Persist desired state for one runtime id.
    pub fn put_desired_runtime_state(
        &self,
        record: &RuntimeDesiredState,
    ) -> Result<(), SupervisorStoreError> {
        self.desired
            .insert(record.runtime_id.as_str().as_bytes(), encode(record)?)
            .map_err(to_sled)?;
        Ok(())
    }

    /// Read desired state for one runtime id.
    pub fn get_desired_runtime_state(
        &self,
        runtime_id: &RuntimeId,
    ) -> Result<Option<RuntimeDesiredState>, SupervisorStoreError> {
        read_optional(&self.desired, runtime_id.as_str().as_bytes())
    }

    /// List desired runtime state ordered by runtime id.
    pub fn list_desired_runtime_state(
        &self,
    ) -> Result<Vec<RuntimeDesiredState>, SupervisorStoreError> {
        let mut records: Vec<RuntimeDesiredState> = read_all(&self.desired)?;
        records.sort_by(|left, right| left.runtime_id.cmp(&right.runtime_id));
        Ok(records)
    }

    /// Persist a historical lease record without changing active ownership.
    pub fn put_runtime_lease(&self, record: &RuntimeLease) -> Result<(), SupervisorStoreError> {
        require_non_empty("lease_id", &record.lease_id)?;
        require_non_empty("instance_id", &record.instance_id)?;
        self.leases
            .insert(
                lease_key(&record.runtime_id, &record.lease_id),
                encode(record)?,
            )
            .map_err(to_sled)?;
        Ok(())
    }

    /// Read one lease record.
    pub fn get_runtime_lease(
        &self,
        runtime_id: &RuntimeId,
        lease_id: &str,
    ) -> Result<Option<RuntimeLease>, SupervisorStoreError> {
        read_optional(&self.leases, &lease_key(runtime_id, lease_id))
    }

    /// List historical leases ordered by acquisition time and stable identity.
    pub fn list_runtime_leases(&self) -> Result<Vec<RuntimeLease>, SupervisorStoreError> {
        let mut records: Vec<RuntimeLease> = read_all(&self.leases)?;
        records.sort_by(|left, right| {
            left.acquired_at_ms
                .cmp(&right.acquired_at_ms)
                .then_with(|| left.runtime_id.cmp(&right.runtime_id))
                .then_with(|| left.lease_id.cmp(&right.lease_id))
        });
        Ok(records)
    }

    /// Read the current active lease for one runtime id.
    pub fn get_active_runtime_lease(
        &self,
        runtime_id: &RuntimeId,
    ) -> Result<Option<RuntimeLease>, SupervisorStoreError> {
        let Some(active_key) = self
            .active_leases
            .get(runtime_id.as_str().as_bytes())
            .map_err(to_sled)?
        else {
            return Ok(None);
        };
        let raw = self
            .leases
            .get(active_key.as_ref())
            .map_err(to_sled)?
            .ok_or_else(|| {
                SupervisorStoreError::InvalidRecord(format!(
                    "active lease index for '{}' points at missing lease '{}'",
                    runtime_id,
                    key_debug(active_key.as_ref())
                ))
            })?;
        decode(&raw).map(Some)
    }

    /// Atomically acquire an active runtime lease.
    pub fn acquire_runtime_lease(
        &self,
        runtime_id: RuntimeId,
        lease_id: impl Into<String>,
        instance_id: impl Into<String>,
        acquired_at_ms: u64,
        lease_duration_ms: u64,
    ) -> Result<RuntimeLease, SupervisorStoreError> {
        let lease_id = lease_id.into();
        let instance_id = instance_id.into();
        require_non_empty("lease_id", &lease_id)?;
        require_non_empty("instance_id", &instance_id)?;
        let expires_at_ms = checked_expiry(acquired_at_ms, lease_duration_ms)?;
        let record = RuntimeLease {
            runtime_id: runtime_id.clone(),
            lease_id: lease_id.clone(),
            instance_id,
            acquired_at_ms,
            renewed_at_ms: None,
            released_at_ms: None,
            expires_at_ms,
            status: RuntimeLeaseStatus::Active,
        };
        let active_key = runtime_id.as_str().as_bytes().to_vec();
        let new_lease_key = lease_key(&runtime_id, &lease_id);
        let encoded_record = encode(&record)?;

        (&self.leases, &self.active_leases)
            .transaction(|(leases, active_leases)| {
                if let Some(active_value) = active_leases.get(active_key.clone())? {
                    let mut active_lease =
                        read_lease_in_transaction(leases, active_value.as_ref())?;
                    if active_lease.is_active_at(acquired_at_ms) {
                        return Err(ConflictableTransactionError::Abort(
                            SupervisorStoreError::DuplicateActiveLease {
                                runtime_id: runtime_id.to_string(),
                                active_lease_id: active_lease.lease_id,
                            },
                        ));
                    }
                    if active_lease.has_active_status() {
                        active_lease.status = RuntimeLeaseStatus::Expired;
                        leases.insert(active_value.as_ref(), encode_transaction(&active_lease)?)?;
                    }
                    active_leases.remove(active_key.clone())?;
                }

                if leases.get(new_lease_key.clone())?.is_some() {
                    return Err(ConflictableTransactionError::Abort(
                        SupervisorStoreError::InvalidRecord(format!(
                            "lease '{}' already exists for runtime '{}'",
                            lease_id, runtime_id
                        )),
                    ));
                }
                leases.insert(new_lease_key.clone(), encoded_record.clone())?;
                active_leases.insert(active_key.clone(), new_lease_key.clone())?;
                Ok(record.clone())
            })
            .map_err(to_transaction)
    }

    /// Renew the active lease for its current owner.
    pub fn renew_runtime_lease(
        &self,
        owner: &RuntimeLeaseOwner,
        renewed_at_ms: u64,
        lease_duration_ms: u64,
    ) -> Result<RuntimeLease, SupervisorStoreError> {
        require_non_empty("lease_id", &owner.lease_id)?;
        require_non_empty("instance_id", &owner.instance_id)?;
        let expires_at_ms = checked_expiry(renewed_at_ms, lease_duration_ms)?;
        let active_key = owner.runtime_id.as_str().as_bytes().to_vec();
        let expected_lease_key = lease_key(&owner.runtime_id, &owner.lease_id);

        (&self.leases, &self.active_leases)
            .transaction(|(leases, active_leases)| {
                require_active_owner_in_transaction(
                    active_leases,
                    &active_key,
                    &expected_lease_key,
                    owner,
                )?;
                let mut lease = read_lease_in_transaction(leases, &expected_lease_key)?;
                if !lease.is_owned_by(owner) || !lease.is_active_at(renewed_at_ms) {
                    return Err(ConflictableTransactionError::Abort(stale_owner(owner)));
                }
                let last_owner_observed_at_ms = lease.renewed_at_ms.unwrap_or(lease.acquired_at_ms);
                if renewed_at_ms < last_owner_observed_at_ms {
                    return Err(ConflictableTransactionError::Abort(
                        SupervisorStoreError::InvalidRecord(format!(
                            "lease renewal for '{}' moved backwards from {} to {}",
                            owner.runtime_id, last_owner_observed_at_ms, renewed_at_ms
                        )),
                    ));
                }
                lease.status = RuntimeLeaseStatus::Active;
                lease.renewed_at_ms = Some(renewed_at_ms);
                lease.expires_at_ms = expires_at_ms;
                leases.insert(expected_lease_key.clone(), encode_transaction(&lease)?)?;
                Ok(lease)
            })
            .map_err(to_transaction)
    }

    /// Write the latest heartbeat for the active lease owner.
    pub fn write_runtime_heartbeat(
        &self,
        heartbeat: &RuntimeHeartbeat,
    ) -> Result<(), SupervisorStoreError> {
        require_non_empty("lease_id", &heartbeat.lease_id)?;
        require_non_empty("instance_id", &heartbeat.instance_id)?;
        let owner = RuntimeLeaseOwner {
            runtime_id: heartbeat.runtime_id.clone(),
            lease_id: heartbeat.lease_id.clone(),
            instance_id: heartbeat.instance_id.clone(),
        };
        let active_key = heartbeat.runtime_id.as_str().as_bytes().to_vec();
        let expected_lease_key = lease_key(&heartbeat.runtime_id, &heartbeat.lease_id);
        let heartbeat_key = heartbeat.runtime_id.as_str().as_bytes().to_vec();
        let encoded_heartbeat = encode(heartbeat)?;

        (&self.leases, &self.active_leases, &self.heartbeats)
            .transaction(|(leases, active_leases, heartbeats)| {
                require_active_owner_in_transaction(
                    active_leases,
                    &active_key,
                    &expected_lease_key,
                    &owner,
                )?;
                let lease = read_lease_in_transaction(leases, &expected_lease_key)?;
                if !lease.is_owned_by(&owner) || !lease.is_active_at(heartbeat.observed_at_ms) {
                    return Err(ConflictableTransactionError::Abort(stale_owner(&owner)));
                }
                let last_owner_observed_at_ms = lease.renewed_at_ms.unwrap_or(lease.acquired_at_ms);
                if heartbeat.observed_at_ms < last_owner_observed_at_ms {
                    return Err(ConflictableTransactionError::Abort(
                        SupervisorStoreError::InvalidRecord(format!(
                            "heartbeat for '{}' moved backwards from lease time {} to {}",
                            owner.runtime_id, last_owner_observed_at_ms, heartbeat.observed_at_ms
                        )),
                    ));
                }
                if let Some(raw) = heartbeats.get(heartbeat_key.clone())? {
                    let current: RuntimeHeartbeat = decode_transaction(&raw)?;
                    if heartbeat.observed_at_ms < current.observed_at_ms {
                        return Err(ConflictableTransactionError::Abort(
                            SupervisorStoreError::InvalidRecord(format!(
                                "heartbeat for '{}' moved backwards from {} to {}",
                                owner.runtime_id, current.observed_at_ms, heartbeat.observed_at_ms
                            )),
                        ));
                    }
                }
                heartbeats.insert(heartbeat_key.clone(), encoded_heartbeat.clone())?;
                Ok(())
            })
            .map_err(to_transaction)
    }

    /// Read the latest heartbeat for one runtime id.
    pub fn get_runtime_heartbeat(
        &self,
        runtime_id: &RuntimeId,
    ) -> Result<Option<RuntimeHeartbeat>, SupervisorStoreError> {
        read_optional(&self.heartbeats, runtime_id.as_str().as_bytes())
    }

    /// Release the active lease for its owner.
    ///
    /// Releasing an already released lease by the same owner is idempotent.
    pub fn release_runtime_lease(
        &self,
        owner: &RuntimeLeaseOwner,
        released_at_ms: u64,
    ) -> Result<RuntimeLease, SupervisorStoreError> {
        require_non_empty("lease_id", &owner.lease_id)?;
        require_non_empty("instance_id", &owner.instance_id)?;
        let active_key = owner.runtime_id.as_str().as_bytes().to_vec();
        let expected_lease_key = lease_key(&owner.runtime_id, &owner.lease_id);

        (&self.leases, &self.active_leases)
            .transaction(
                |(leases, active_leases)| match active_leases.get(active_key.clone())? {
                    Some(active_value) => {
                        if active_value.as_ref() != expected_lease_key.as_slice() {
                            return Err(ConflictableTransactionError::Abort(stale_owner(owner)));
                        }
                        let mut lease = read_lease_in_transaction(leases, &expected_lease_key)?;
                        if !lease.is_owned_by(owner) {
                            return Err(ConflictableTransactionError::Abort(stale_owner(owner)));
                        }
                        lease.status = RuntimeLeaseStatus::Released;
                        lease.released_at_ms = lease.released_at_ms.or(Some(released_at_ms));
                        leases.insert(expected_lease_key.clone(), encode_transaction(&lease)?)?;
                        active_leases.remove(active_key.clone())?;
                        Ok(lease)
                    }
                    None => {
                        let lease = read_lease_in_transaction(leases, &expected_lease_key)?;
                        if lease.is_owned_by(owner) && lease.status == RuntimeLeaseStatus::Released
                        {
                            Ok(lease)
                        } else {
                            Err(ConflictableTransactionError::Abort(stale_owner(owner)))
                        }
                    }
                },
            )
            .map_err(to_transaction)
    }

    /// Mark expired active leases as expired and clear active ownership.
    pub fn recover_expired_leases(
        &self,
        now_ms: u64,
    ) -> Result<Vec<RuntimeLease>, SupervisorStoreError> {
        let active_entries = self
            .active_leases
            .iter()
            .map(|entry| {
                entry
                    .map(|(runtime_key, lease_key)| (runtime_key.to_vec(), lease_key.to_vec()))
                    .map_err(to_sled)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut recovered = Vec::new();

        for (runtime_key, active_lease_key) in active_entries {
            let expired = (&self.leases, &self.active_leases)
                .transaction(|(leases, active_leases)| {
                    let Some(current_active_key) = active_leases.get(runtime_key.clone())? else {
                        return Ok(None);
                    };
                    if current_active_key.as_ref() != active_lease_key.as_slice() {
                        return Ok(None);
                    }

                    let mut lease = read_lease_in_transaction(leases, &active_lease_key)?;
                    if lease.has_active_status() && lease.expires_at_ms <= now_ms {
                        lease.status = RuntimeLeaseStatus::Expired;
                        leases.insert(active_lease_key.clone(), encode_transaction(&lease)?)?;
                        active_leases.remove(runtime_key.clone())?;
                        Ok(Some(lease))
                    } else {
                        Ok(None)
                    }
                })
                .map_err(to_transaction)?;
            if let Some(lease) = expired {
                recovered.push(lease);
            }
        }

        Ok(recovered)
    }

    /// Persist one health snapshot.
    pub fn put_health_snapshot(
        &self,
        record: &RuntimeHealthSnapshot,
    ) -> Result<(), SupervisorStoreError> {
        self.health
            .insert(record.runtime_id.as_str().as_bytes(), encode(record)?)
            .map_err(to_sled)?;
        Ok(())
    }

    /// Read one health snapshot.
    pub fn get_health_snapshot(
        &self,
        runtime_id: &RuntimeId,
    ) -> Result<Option<RuntimeHealthSnapshot>, SupervisorStoreError> {
        read_optional(&self.health, runtime_id.as_str().as_bytes())
    }

    /// Persist one restart audit record.
    pub fn put_restart_record(
        &self,
        record: &RuntimeRestartRecord,
    ) -> Result<(), SupervisorStoreError> {
        require_non_empty("restart_id", &record.restart_id)?;
        self.restarts
            .insert(
                restart_key(&record.runtime_id, &record.restart_id),
                encode(record)?,
            )
            .map_err(to_sled)?;
        Ok(())
    }

    /// Read one restart audit record.
    pub fn get_restart_record(
        &self,
        runtime_id: &RuntimeId,
        restart_id: &str,
    ) -> Result<Option<RuntimeRestartRecord>, SupervisorStoreError> {
        read_optional(&self.restarts, &restart_key(runtime_id, restart_id))
    }

    /// Persist one graceful shutdown state record.
    pub fn put_shutdown_state(
        &self,
        record: &RuntimeShutdownState,
    ) -> Result<(), SupervisorStoreError> {
        require_non_empty("shutdown_id", &record.shutdown_id)?;
        require_non_empty("instance_id", &record.instance_id)?;
        self.shutdowns
            .insert(record.shutdown_id.as_bytes(), encode(record)?)
            .map_err(to_sled)?;
        Ok(())
    }

    /// Read one graceful shutdown state record.
    pub fn get_shutdown_state(
        &self,
        shutdown_id: &str,
    ) -> Result<Option<RuntimeShutdownState>, SupervisorStoreError> {
        read_optional(&self.shutdowns, shutdown_id.as_bytes())
    }

    /// Persist one supervisor lifecycle event.
    pub fn put_lifecycle_event(
        &self,
        record: &SupervisorLifecycleEvent,
    ) -> Result<(), SupervisorStoreError> {
        require_non_empty("event_id", &record.event_id)?;
        require_non_empty("instance_id", &record.instance_id)?;
        self.lifecycle_events
            .insert(record.event_id.as_bytes(), encode(record)?)
            .map_err(to_sled)?;
        Ok(())
    }

    /// Read one supervisor lifecycle event.
    pub fn get_lifecycle_event(
        &self,
        event_id: &str,
    ) -> Result<Option<SupervisorLifecycleEvent>, SupervisorStoreError> {
        read_optional(&self.lifecycle_events, event_id.as_bytes())
    }

    /// Read the latest lifecycle event for one runtime id.
    pub fn latest_lifecycle_event_for_runtime(
        &self,
        runtime_id: &RuntimeId,
    ) -> Result<Option<SupervisorLifecycleEvent>, SupervisorStoreError> {
        let mut latest = None;
        for entry in self.lifecycle_events.iter() {
            let (_key, raw) = entry.map_err(to_sled)?;
            let event: SupervisorLifecycleEvent = decode(&raw)?;
            if event.runtime_id.as_ref() == Some(runtime_id) {
                latest = match latest {
                    Some(current) if latest_lifecycle_event_wins(&current, &event) == current => {
                        Some(current)
                    }
                    _ => Some(event),
                };
            }
        }
        Ok(latest)
    }
}

fn to_io(error: impl ToString) -> SupervisorStoreError {
    SupervisorStoreError::Io(error.to_string())
}

fn to_sled(error: impl ToString) -> SupervisorStoreError {
    SupervisorStoreError::Sled(error.to_string())
}

fn to_codec(error: impl ToString) -> SupervisorStoreError {
    SupervisorStoreError::Codec(error.to_string())
}

fn to_transaction(error: TransactionError<SupervisorStoreError>) -> SupervisorStoreError {
    match error {
        TransactionError::Abort(error) => error,
        TransactionError::Storage(error) => to_sled(error),
    }
}

fn encode<T: Serialize>(record: &T) -> Result<Vec<u8>, SupervisorStoreError> {
    bincode::serialize(record).map_err(to_codec)
}

fn decode<T: DeserializeOwned>(raw: &[u8]) -> Result<T, SupervisorStoreError> {
    bincode::deserialize(raw).map_err(to_codec)
}

fn encode_transaction<T: Serialize>(
    record: &T,
) -> Result<Vec<u8>, ConflictableTransactionError<SupervisorStoreError>> {
    encode(record).map_err(ConflictableTransactionError::Abort)
}

fn decode_transaction<T: DeserializeOwned>(
    raw: &[u8],
) -> Result<T, ConflictableTransactionError<SupervisorStoreError>> {
    decode(raw).map_err(ConflictableTransactionError::Abort)
}

fn read_optional<T: DeserializeOwned>(
    tree: &sled::Tree,
    key: &[u8],
) -> Result<Option<T>, SupervisorStoreError> {
    tree.get(key)
        .map_err(to_sled)?
        .map(|raw| decode(&raw))
        .transpose()
}

fn read_all<T: DeserializeOwned>(tree: &sled::Tree) -> Result<Vec<T>, SupervisorStoreError> {
    tree.iter()
        .map(|entry| {
            let (_key, raw) = entry.map_err(to_sled)?;
            decode(&raw)
        })
        .collect()
}

fn migrate_record_tree<T, F>(
    tree: &sled::Tree,
    canonical_key: F,
) -> Result<bool, SupervisorStoreError>
where
    T: DeserializeOwned + Serialize,
    F: Fn(&T) -> Vec<u8>,
{
    let entries = tree
        .iter()
        .map(|entry| {
            entry
                .map(|(key, value)| (key.to_vec(), value.to_vec()))
                .map_err(to_sled)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut changed = false;
    for (source_key, raw) in entries {
        let record: T = decode(&raw)?;
        let target_key = canonical_key(&record);
        let normalized = encode(&record)?;
        if let Some(target_raw) = tree.get(&target_key).map_err(to_sled)? {
            let target_record: T = decode(&target_raw)?;
            let normalized_target = encode(&target_record)?;
            if normalized_target != normalized {
                return Err(runtime_id_migration_collision(&source_key, &target_key));
            }
        }
        if source_key != target_key || raw != normalized {
            tree.insert(&target_key, normalized).map_err(to_sled)?;
            if source_key != target_key {
                tree.remove(&source_key).map_err(to_sled)?;
            }
            changed = true;
        }
    }
    Ok(changed)
}

fn migrate_active_lease_index(tree: &sled::Tree) -> Result<bool, SupervisorStoreError> {
    let entries = tree
        .iter()
        .map(|entry| {
            entry
                .map(|(key, value)| (key.to_vec(), value.to_vec()))
                .map_err(to_sled)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut changed = false;
    for (source_key, source_value) in entries {
        let target_key = canonicalize_runtime_key(&source_key)?;
        let target_value = canonicalize_joined_runtime_key(&source_value)?;
        if let Some(existing) = tree.get(&target_key).map_err(to_sled)? {
            let normalized_existing = canonicalize_joined_runtime_key(&existing)?;
            if normalized_existing != target_value {
                return Err(runtime_id_migration_collision(&source_key, &target_key));
            }
        }
        if source_key != target_key || source_value != target_value {
            tree.insert(&target_key, target_value).map_err(to_sled)?;
            if source_key != target_key {
                tree.remove(&source_key).map_err(to_sled)?;
            }
            changed = true;
        }
    }
    Ok(changed)
}

fn normalize_record_values<T>(tree: &sled::Tree) -> Result<bool, SupervisorStoreError>
where
    T: DeserializeOwned + Serialize,
{
    let entries = tree
        .iter()
        .map(|entry| {
            entry
                .map(|(key, value)| (key.to_vec(), value.to_vec()))
                .map_err(to_sled)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut changed = false;
    for (key, raw) in entries {
        let record: T = decode(&raw)?;
        let normalized = encode(&record)?;
        if raw != normalized {
            tree.insert(key, normalized).map_err(to_sled)?;
            changed = true;
        }
    }
    Ok(changed)
}

fn canonicalize_runtime_key(key: &[u8]) -> Result<Vec<u8>, SupervisorStoreError> {
    let value = std::str::from_utf8(key).map_err(|_| {
        SupervisorStoreError::InvalidRecord(format!(
            "runtime id key '{}' is not UTF-8",
            key_debug(key)
        ))
    })?;
    Ok(canonical_runtime_id(value).as_bytes().to_vec())
}

fn canonicalize_joined_runtime_key(key: &[u8]) -> Result<Vec<u8>, SupervisorStoreError> {
    let Some(separator) = key.iter().position(|byte| *byte == 0) else {
        return Err(SupervisorStoreError::InvalidRecord(format!(
            "runtime composite key '{}' has no separator",
            key_debug(key)
        )));
    };
    let runtime_id = std::str::from_utf8(&key[..separator]).map_err(|_| {
        SupervisorStoreError::InvalidRecord(format!(
            "runtime composite key '{}' has a non-UTF-8 runtime id",
            key_debug(key)
        ))
    })?;
    let mut normalized = canonical_runtime_id(runtime_id).as_bytes().to_vec();
    normalized.extend_from_slice(&key[separator..]);
    Ok(normalized)
}

fn runtime_id_migration_collision(source: &[u8], target: &[u8]) -> SupervisorStoreError {
    SupervisorStoreError::InvalidRecord(format!(
        "runtime id migration from '{}' conflicts with canonical key '{}'",
        key_debug(source),
        key_debug(target)
    ))
}

fn read_lease_in_transaction(
    leases: &sled::transaction::TransactionalTree,
    key: &[u8],
) -> Result<RuntimeLease, ConflictableTransactionError<SupervisorStoreError>> {
    let raw = leases.get(key)?.ok_or_else(|| {
        ConflictableTransactionError::Abort(SupervisorStoreError::InvalidRecord(format!(
            "active lease index points at missing lease '{}'",
            key_debug(key)
        )))
    })?;
    decode_transaction(&raw)
}

fn require_active_owner_in_transaction(
    active_leases: &sled::transaction::TransactionalTree,
    active_key: &[u8],
    expected_lease_key: &[u8],
    owner: &RuntimeLeaseOwner,
) -> Result<(), ConflictableTransactionError<SupervisorStoreError>> {
    let Some(active_value) = active_leases.get(active_key)? else {
        return Err(ConflictableTransactionError::Abort(stale_owner(owner)));
    };
    if active_value.as_ref() == expected_lease_key {
        Ok(())
    } else {
        Err(ConflictableTransactionError::Abort(stale_owner(owner)))
    }
}

fn require_non_empty(field: &'static str, value: &str) -> Result<(), SupervisorStoreError> {
    if value.trim().is_empty() {
        Err(SupervisorStoreError::InvalidRecord(format!(
            "{field} must be non-empty"
        )))
    } else {
        Ok(())
    }
}

fn checked_expiry(start_ms: u64, duration_ms: u64) -> Result<u64, SupervisorStoreError> {
    start_ms.checked_add(duration_ms).ok_or_else(|| {
        SupervisorStoreError::InvalidRecord("lease expiry overflowed u64".to_string())
    })
}

fn lease_key(runtime_id: &RuntimeId, lease_id: &str) -> Vec<u8> {
    joined_key(runtime_id.as_str(), lease_id)
}

fn restart_key(runtime_id: &RuntimeId, restart_id: &str) -> Vec<u8> {
    joined_key(runtime_id.as_str(), restart_id)
}

fn joined_key(left: &str, right: &str) -> Vec<u8> {
    let mut key = Vec::with_capacity(left.len() + 1 + right.len());
    key.extend_from_slice(left.as_bytes());
    key.push(0);
    key.extend_from_slice(right.as_bytes());
    key
}

fn key_debug(key: &[u8]) -> String {
    String::from_utf8_lossy(key).replace('\0', "\\0")
}

fn stale_owner(owner: &RuntimeLeaseOwner) -> SupervisorStoreError {
    SupervisorStoreError::StaleLeaseOwner {
        runtime_id: owner.runtime_id.to_string(),
        lease_id: owner.lease_id.clone(),
        instance_id: owner.instance_id.clone(),
    }
}

fn latest_lifecycle_event_wins(
    left: &SupervisorLifecycleEvent,
    right: &SupervisorLifecycleEvent,
) -> SupervisorLifecycleEvent {
    if left.occurred_at_ms > right.occurred_at_ms
        || (left.occurred_at_ms == right.occurred_at_ms && left.event_id >= right.event_id)
    {
        left.clone()
    } else {
        right.clone()
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::super::contracts::*;
    use super::*;

    const LEGACY_GRAPH_RUNTIME_ID: &str = "world_model.graph.replay";
    const CANONICAL_GRAPH_RUNTIME_ID: &str = "world_model.graph_replay";

    fn legacy_encoded<T: Serialize>(record: &T) -> Vec<u8> {
        let mut encoded = encode(record).unwrap();
        let position = encoded
            .windows(CANONICAL_GRAPH_RUNTIME_ID.len())
            .position(|window| window == CANONICAL_GRAPH_RUNTIME_ID.as_bytes())
            .expect("encoded record contains the canonical runtime id");
        encoded[position..position + LEGACY_GRAPH_RUNTIME_ID.len()]
            .copy_from_slice(LEGACY_GRAPH_RUNTIME_ID.as_bytes());
        encoded
    }

    #[test]
    fn open_migrates_requirement_era_ids_across_lifecycle_families() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("supervisor.sled");
        let db = sled::open(&path).unwrap();
        let runtime_id = runtime_id(CANONICAL_GRAPH_RUNTIME_ID);
        let desired = RuntimeDesiredState {
            runtime_id: runtime_id.clone(),
            enabled: true,
            restart_policy: RestartPolicy::OnHeartbeatExpiry,
        };
        let lease = RuntimeLease {
            runtime_id: runtime_id.clone(),
            lease_id: "lease-a".to_string(),
            instance_id: "instance-a".to_string(),
            acquired_at_ms: 10,
            renewed_at_ms: None,
            released_at_ms: None,
            expires_at_ms: 100,
            status: RuntimeLeaseStatus::Active,
        };
        let heartbeat = RuntimeHeartbeat {
            runtime_id: runtime_id.clone(),
            lease_id: "lease-a".to_string(),
            instance_id: "instance-a".to_string(),
            observed_at_ms: 20,
            health: healthy(),
            diagnostic: None,
        };
        let health = RuntimeHealthSnapshot {
            runtime_id: runtime_id.clone(),
            lease_id: Some("lease-a".to_string()),
            observed_at_ms: 20,
            status: RuntimeHealthStatus::Healthy,
            last_heartbeat_at_ms: Some(20),
            retryable_error_count: 0,
            fatal_error_count: 0,
            budget_exhausted: false,
            restart_count: 0,
            last_restart_cause: None,
        };
        let restart = RuntimeRestartRecord {
            restart_id: "restart-a".to_string(),
            runtime_id: runtime_id.clone(),
            instance_id: "instance-a".to_string(),
            previous_lease_id: Some("lease-old".to_string()),
            cause: RestartCause::HeartbeatExpired,
            attempt: 1,
            requested_at_ms: 30,
            backoff_ms: 10,
        };
        let event = SupervisorLifecycleEvent {
            event_id: "event-a".to_string(),
            instance_id: "instance-a".to_string(),
            runtime_id: Some(runtime_id.clone()),
            lease_id: Some("lease-a".to_string()),
            occurred_at_ms: 20,
            event_type: SupervisorLifecycleEventType::HeartbeatAccepted,
            message: None,
        };

        db.open_tree(TREE_DESIRED)
            .unwrap()
            .insert(LEGACY_GRAPH_RUNTIME_ID, legacy_encoded(&desired))
            .unwrap();
        db.open_tree(TREE_LEASES)
            .unwrap()
            .insert(
                joined_key(LEGACY_GRAPH_RUNTIME_ID, "lease-a"),
                legacy_encoded(&lease),
            )
            .unwrap();
        db.open_tree(TREE_ACTIVE_LEASES)
            .unwrap()
            .insert(
                LEGACY_GRAPH_RUNTIME_ID,
                joined_key(LEGACY_GRAPH_RUNTIME_ID, "lease-a"),
            )
            .unwrap();
        db.open_tree(TREE_HEARTBEATS)
            .unwrap()
            .insert(LEGACY_GRAPH_RUNTIME_ID, legacy_encoded(&heartbeat))
            .unwrap();
        db.open_tree(TREE_HEALTH)
            .unwrap()
            .insert(LEGACY_GRAPH_RUNTIME_ID, legacy_encoded(&health))
            .unwrap();
        db.open_tree(TREE_RESTARTS)
            .unwrap()
            .insert(
                joined_key(LEGACY_GRAPH_RUNTIME_ID, "restart-a"),
                legacy_encoded(&restart),
            )
            .unwrap();
        db.open_tree(TREE_LIFECYCLE_EVENTS)
            .unwrap()
            .insert("event-a", legacy_encoded(&event))
            .unwrap();
        db.flush().unwrap();
        drop(db);

        let store = SupervisorStore::open(&path).unwrap();

        assert_eq!(
            store.get_desired_runtime_state(&runtime_id).unwrap(),
            Some(desired)
        );
        assert_eq!(
            store.get_runtime_lease(&runtime_id, "lease-a").unwrap(),
            Some(lease.clone())
        );
        assert_eq!(
            store.get_active_runtime_lease(&runtime_id).unwrap(),
            Some(lease)
        );
        assert_eq!(
            store.get_runtime_heartbeat(&runtime_id).unwrap(),
            Some(heartbeat)
        );
        assert_eq!(
            store.get_health_snapshot(&runtime_id).unwrap(),
            Some(health)
        );
        assert_eq!(
            store.get_restart_record(&runtime_id, "restart-a").unwrap(),
            Some(restart)
        );
        assert_eq!(store.get_lifecycle_event("event-a").unwrap(), Some(event));
        for tree in [
            &store.desired,
            &store.leases,
            &store.active_leases,
            &store.heartbeats,
            &store.health,
            &store.restarts,
        ] {
            assert!(tree.iter().all(|entry| !entry
                .unwrap()
                .0
                .starts_with(LEGACY_GRAPH_RUNTIME_ID.as_bytes())));
        }
    }

    #[test]
    fn open_rejects_divergent_alias_and_canonical_records() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("supervisor.sled");
        let db = sled::open(&path).unwrap();
        let tree = db.open_tree(TREE_DESIRED).unwrap();
        let canonical = RuntimeDesiredState {
            runtime_id: runtime_id(CANONICAL_GRAPH_RUNTIME_ID),
            enabled: true,
            restart_policy: RestartPolicy::OnHeartbeatExpiry,
        };
        let mut legacy = canonical.clone();
        legacy.enabled = false;
        tree.insert(CANONICAL_GRAPH_RUNTIME_ID, encode(&canonical).unwrap())
            .unwrap();
        tree.insert(LEGACY_GRAPH_RUNTIME_ID, legacy_encoded(&legacy))
            .unwrap();
        db.flush().unwrap();
        drop(tree);
        drop(db);

        let error = match SupervisorStore::open(&path) {
            Ok(_) => panic!("divergent runtime aliases must fail migration"),
            Err(error) => error,
        };

        assert!(matches!(error, SupervisorStoreError::InvalidRecord(_)));
        assert!(error.to_string().contains("conflicts with canonical key"));
    }

    #[test]
    fn supervisor_records_round_trip_through_sled_trees() {
        let (temp, store) = open_store();
        let runtime_id = runtime_id("events.ledger");
        let instance = RuntimeInstance {
            instance_id: "instance-a".to_string(),
            product_root: temp.path().to_path_buf(),
            started_at_ms: 10,
            stopped_at_ms: None,
            status: RuntimeInstanceStatus::Running,
        };
        let desired = RuntimeDesiredState {
            runtime_id: runtime_id.clone(),
            enabled: true,
            restart_policy: RestartPolicy::OnHeartbeatExpiry,
        };

        store.put_runtime_instance(&instance).unwrap();
        store.put_desired_runtime_state(&desired).unwrap();
        let lease = store
            .acquire_runtime_lease(runtime_id.clone(), "lease-a", "instance-a", 20, 100)
            .unwrap();
        let heartbeat = RuntimeHeartbeat {
            runtime_id: runtime_id.clone(),
            lease_id: "lease-a".to_string(),
            instance_id: "instance-a".to_string(),
            observed_at_ms: 30,
            health: healthy(),
            diagnostic: Some(RuntimeDiagnosticSummary {
                actor_id: "events.ledger".to_string(),
                retryable_issue_count: 1,
                fatal_issue_count: 0,
                budget_exhausted: false,
                last_error_code: Some("retryable_io".to_string()),
            }),
        };
        let snapshot = RuntimeHealthSnapshot {
            runtime_id: runtime_id.clone(),
            lease_id: Some("lease-a".to_string()),
            observed_at_ms: 35,
            status: RuntimeHealthStatus::Healthy,
            last_heartbeat_at_ms: Some(30),
            retryable_error_count: 1,
            fatal_error_count: 0,
            budget_exhausted: false,
            restart_count: 2,
            last_restart_cause: Some(RestartCause::HeartbeatExpired),
        };
        let restart = RuntimeRestartRecord {
            restart_id: "restart-a".to_string(),
            runtime_id: runtime_id.clone(),
            instance_id: "instance-a".to_string(),
            previous_lease_id: Some("lease-old".to_string()),
            cause: RestartCause::HeartbeatExpired,
            attempt: 2,
            requested_at_ms: 40,
            backoff_ms: 500,
        };
        let shutdown = RuntimeShutdownState {
            shutdown_id: "shutdown-a".to_string(),
            instance_id: "instance-a".to_string(),
            requested_at_ms: 50,
            completed_at_ms: Some(60),
            status: RuntimeShutdownStatus::Completed,
        };
        let event = SupervisorLifecycleEvent {
            event_id: "event-a".to_string(),
            instance_id: "instance-a".to_string(),
            runtime_id: Some(runtime_id.clone()),
            lease_id: Some("lease-a".to_string()),
            occurred_at_ms: 55,
            event_type: SupervisorLifecycleEventType::HeartbeatAccepted,
            message: Some("heartbeat accepted".to_string()),
        };

        store.write_runtime_heartbeat(&heartbeat).unwrap();
        store.put_health_snapshot(&snapshot).unwrap();
        store.put_restart_record(&restart).unwrap();
        store.put_shutdown_state(&shutdown).unwrap();
        store.put_lifecycle_event(&event).unwrap();
        store.flush().unwrap();

        assert_eq!(
            store.get_runtime_instance("instance-a").unwrap(),
            Some(instance)
        );
        assert_eq!(
            store.get_desired_runtime_state(&runtime_id).unwrap(),
            Some(desired)
        );
        assert_eq!(
            store.get_runtime_lease(&runtime_id, "lease-a").unwrap(),
            Some(lease)
        );
        assert_eq!(
            store.get_runtime_heartbeat(&runtime_id).unwrap(),
            Some(heartbeat)
        );
        assert_eq!(
            store.get_health_snapshot(&runtime_id).unwrap(),
            Some(snapshot)
        );
        assert_eq!(
            store.get_restart_record(&runtime_id, "restart-a").unwrap(),
            Some(restart)
        );
        assert_eq!(
            store.get_shutdown_state("shutdown-a").unwrap(),
            Some(shutdown)
        );
        assert_eq!(store.get_lifecycle_event("event-a").unwrap(), Some(event));
    }

    #[test]
    fn open_existing_returns_none_for_missing_store() {
        let temp = tempfile::tempdir().unwrap();
        let missing = temp.path().join("missing-supervisor.sled");

        let store = SupervisorStore::open_existing(&missing).unwrap();

        assert!(store.is_none());
        assert!(!missing.exists());
    }

    #[test]
    fn latest_runtime_instance_uses_started_time_and_instance_id_order() {
        let (temp, store) = open_store();
        for instance in [
            RuntimeInstance {
                instance_id: "instance-b".to_string(),
                product_root: temp.path().to_path_buf(),
                started_at_ms: 20,
                stopped_at_ms: None,
                status: RuntimeInstanceStatus::Running,
            },
            RuntimeInstance {
                instance_id: "instance-a".to_string(),
                product_root: temp.path().to_path_buf(),
                started_at_ms: 30,
                stopped_at_ms: None,
                status: RuntimeInstanceStatus::Running,
            },
            RuntimeInstance {
                instance_id: "instance-c".to_string(),
                product_root: temp.path().to_path_buf(),
                started_at_ms: 30,
                stopped_at_ms: None,
                status: RuntimeInstanceStatus::Stopped,
            },
        ] {
            store.put_runtime_instance(&instance).unwrap();
        }

        let listed = store.list_runtime_instances().unwrap();
        let latest = store.latest_runtime_instance().unwrap().unwrap();

        assert_eq!(
            listed
                .iter()
                .map(|instance| instance.instance_id.as_str())
                .collect::<Vec<_>>(),
            vec!["instance-b", "instance-a", "instance-c"]
        );
        assert_eq!(latest.instance_id, "instance-c");
    }

    #[test]
    fn list_desired_runtime_state_sorts_by_runtime_id() {
        let (_temp, store) = open_store();
        for runtime_id in [
            "world_model.graph_replay",
            "event.append",
            "execution.planning",
        ] {
            store
                .put_desired_runtime_state(&RuntimeDesiredState {
                    runtime_id: self::runtime_id(runtime_id),
                    enabled: true,
                    restart_policy: RestartPolicy::OnHeartbeatExpiry,
                })
                .unwrap();
        }

        let listed = store.list_desired_runtime_state().unwrap();

        assert_eq!(
            listed
                .iter()
                .map(|state| state.runtime_id.as_str())
                .collect::<Vec<_>>(),
            vec![
                "event.append",
                "execution.planning",
                "world_model.graph_replay"
            ]
        );
    }

    #[test]
    fn latest_lifecycle_event_for_runtime_uses_time_and_event_id_order() {
        let (_temp, store) = open_store();
        let runtime_id = runtime_id("events.ledger");
        let older = SupervisorLifecycleEvent {
            event_id: "event-a".to_string(),
            instance_id: "instance-a".to_string(),
            runtime_id: Some(runtime_id.clone()),
            lease_id: None,
            occurred_at_ms: 20,
            event_type: SupervisorLifecycleEventType::RuntimeStartRequested,
            message: None,
        };
        let same_time_later_id = SupervisorLifecycleEvent {
            event_id: "event-b".to_string(),
            occurred_at_ms: 30,
            event_type: SupervisorLifecycleEventType::HeartbeatAccepted,
            ..older.clone()
        };
        let same_time_earlier_id = SupervisorLifecycleEvent {
            event_id: "event-0".to_string(),
            occurred_at_ms: 30,
            event_type: SupervisorLifecycleEventType::LeaseAcquired,
            ..older.clone()
        };
        store.put_lifecycle_event(&older).unwrap();
        store.put_lifecycle_event(&same_time_later_id).unwrap();
        store.put_lifecycle_event(&same_time_earlier_id).unwrap();

        let latest = store
            .latest_lifecycle_event_for_runtime(&runtime_id)
            .unwrap()
            .unwrap();

        assert_eq!(latest.event_id, "event-b");
        assert_eq!(
            latest.event_type,
            SupervisorLifecycleEventType::HeartbeatAccepted
        );
    }

    #[test]
    fn duplicate_active_lease_is_rejected() {
        let (_temp, store) = open_store();
        let runtime_id = runtime_id("events.ledger");

        store
            .acquire_runtime_lease(runtime_id.clone(), "lease-a", "instance-a", 100, 100)
            .unwrap();
        let error = store
            .acquire_runtime_lease(runtime_id, "lease-b", "instance-b", 110, 100)
            .unwrap_err();

        assert!(matches!(
            error,
            SupervisorStoreError::DuplicateActiveLease { .. }
        ));
    }

    #[test]
    fn stale_owner_cannot_renew_active_lease() {
        let (_temp, store) = open_store();
        let runtime_id = runtime_id("events.ledger");
        store
            .acquire_runtime_lease(runtime_id.clone(), "lease-a", "instance-a", 100, 100)
            .unwrap();
        let stale_owner = RuntimeLeaseOwner {
            runtime_id,
            lease_id: "lease-b".to_string(),
            instance_id: "instance-a".to_string(),
        };

        let error = store
            .renew_runtime_lease(&stale_owner, 120, 100)
            .unwrap_err();

        assert!(matches!(
            error,
            SupervisorStoreError::StaleLeaseOwner { .. }
        ));
    }

    #[test]
    fn lease_renewal_cannot_move_backwards() {
        let (_temp, store) = open_store();
        let runtime_id = runtime_id("events.ledger");
        let lease = store
            .acquire_runtime_lease(runtime_id.clone(), "lease-a", "instance-a", 100, 100)
            .unwrap();
        let owner = lease.owner();

        store.renew_runtime_lease(&owner, 130, 100).unwrap();
        let error = store.renew_runtime_lease(&owner, 120, 100).unwrap_err();

        assert!(matches!(error, SupervisorStoreError::InvalidRecord(_)));
        assert_eq!(
            store
                .get_runtime_lease(&runtime_id, "lease-a")
                .unwrap()
                .unwrap()
                .renewed_at_ms,
            Some(130)
        );
    }

    #[test]
    fn heartbeat_must_match_active_lease_owner() {
        let (_temp, store) = open_store();
        let runtime_id = runtime_id("events.ledger");
        store
            .acquire_runtime_lease(runtime_id.clone(), "lease-a", "instance-a", 100, 100)
            .unwrap();
        let heartbeat = RuntimeHeartbeat {
            runtime_id: runtime_id.clone(),
            lease_id: "lease-a".to_string(),
            instance_id: "instance-a".to_string(),
            observed_at_ms: 120,
            health: healthy(),
            diagnostic: None,
        };
        store.write_runtime_heartbeat(&heartbeat).unwrap();

        let stale_heartbeat = RuntimeHeartbeat {
            lease_id: "lease-b".to_string(),
            ..heartbeat
        };
        let error = store.write_runtime_heartbeat(&stale_heartbeat).unwrap_err();

        assert!(matches!(
            error,
            SupervisorStoreError::StaleLeaseOwner { .. }
        ));
        assert_eq!(
            store
                .get_runtime_heartbeat(&runtime_id)
                .unwrap()
                .unwrap()
                .lease_id,
            "lease-a"
        );
    }

    #[test]
    fn heartbeat_cannot_move_backwards() {
        let (_temp, store) = open_store();
        let runtime_id = runtime_id("events.ledger");
        store
            .acquire_runtime_lease(runtime_id.clone(), "lease-a", "instance-a", 100, 100)
            .unwrap();
        let heartbeat = RuntimeHeartbeat {
            runtime_id: runtime_id.clone(),
            lease_id: "lease-a".to_string(),
            instance_id: "instance-a".to_string(),
            observed_at_ms: 130,
            health: healthy(),
            diagnostic: None,
        };
        store.write_runtime_heartbeat(&heartbeat).unwrap();

        let older_heartbeat = RuntimeHeartbeat {
            observed_at_ms: 120,
            ..heartbeat
        };
        let error = store.write_runtime_heartbeat(&older_heartbeat).unwrap_err();

        assert!(matches!(error, SupervisorStoreError::InvalidRecord(_)));
        assert_eq!(
            store
                .get_runtime_heartbeat(&runtime_id)
                .unwrap()
                .unwrap()
                .observed_at_ms,
            130
        );
    }

    #[test]
    fn release_is_idempotent_for_same_owner() {
        let (_temp, store) = open_store();
        let runtime_id = runtime_id("events.ledger");
        let lease = store
            .acquire_runtime_lease(runtime_id.clone(), "lease-a", "instance-a", 100, 100)
            .unwrap();
        let owner = lease.owner();

        let first = store.release_runtime_lease(&owner, 130).unwrap();
        let second = store.release_runtime_lease(&owner, 150).unwrap();

        assert_eq!(first.status, RuntimeLeaseStatus::Released);
        assert_eq!(first, second);
        assert_eq!(first.released_at_ms, Some(130));
        assert!(store
            .get_active_runtime_lease(&runtime_id)
            .unwrap()
            .is_none());
        store
            .acquire_runtime_lease(runtime_id, "lease-b", "instance-b", 160, 100)
            .unwrap();
    }

    #[test]
    fn expired_lease_recovery_preserves_history_and_allows_new_owner() {
        let (_temp, store) = open_store();
        let runtime_id = runtime_id("events.ledger");
        let lease = store
            .acquire_runtime_lease(runtime_id.clone(), "lease-a", "instance-a", 100, 10)
            .unwrap();
        let owner = lease.owner();

        assert!(store.recover_expired_leases(109).unwrap().is_empty());
        assert!(matches!(
            store
                .acquire_runtime_lease(runtime_id.clone(), "lease-b", "instance-b", 109, 100)
                .unwrap_err(),
            SupervisorStoreError::DuplicateActiveLease { .. }
        ));
        let recovered = store.recover_expired_leases(111).unwrap();
        assert_eq!(recovered.len(), 1);
        assert_eq!(recovered[0].status, RuntimeLeaseStatus::Expired);
        assert!(store
            .get_active_runtime_lease(&runtime_id)
            .unwrap()
            .is_none());
        assert_eq!(
            store
                .get_runtime_lease(&runtime_id, "lease-a")
                .unwrap()
                .unwrap()
                .status,
            RuntimeLeaseStatus::Expired
        );
        store
            .acquire_runtime_lease(runtime_id, "lease-b", "instance-b", 111, 100)
            .unwrap();
        assert!(matches!(
            store.renew_runtime_lease(&owner, 112, 100).unwrap_err(),
            SupervisorStoreError::StaleLeaseOwner { .. }
        ));
    }

    fn open_store() -> (TempDir, SupervisorStore) {
        let temp = tempfile::tempdir().unwrap();
        let store = SupervisorStore::open(temp.path().join("supervisor.sled")).unwrap();
        (temp, store)
    }

    fn runtime_id(value: &str) -> RuntimeId {
        RuntimeId::new(value).unwrap()
    }

    fn healthy() -> RuntimeHealth {
        RuntimeHealth {
            status: RuntimeHealthStatus::Healthy,
            retryable_error_count: 0,
            fatal_error_count: 0,
            budget_exhausted: false,
            last_successful_tick_at_ms: Some(10),
            last_error_code: None,
        }
    }
}
