//! Single-owner durable task network authority.
//!
//! Owner: task network.
//! Inputs: configured network identities, bounded command requests, and
//! bounded state queries.
//! Outputs: responses acknowledged after persistence and flush, snapshots of
//! durably acknowledged state, and final shutdown receipts.
//! Does not own: planning, task execution, publication scheduling, or root
//! runtime activation.

use std::collections::BTreeMap;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender, TrySendError};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use thiserror::Error;

use crate::task_network::command;
use crate::task_network::journal::JournalRecord;
use crate::task_network::outcome::{Publication, PublicationLedgerBinding, PublicationState};
use crate::task_network::readiness::compute_ready_set;
use crate::task_network::state::{DependencyEdge, NetworkState, ReadySet, TaskNode};
use crate::task_network::store::{
    network_storage_key, AuthorityStoreError, SledTaskNetworkStore, TaskNetworkStoreError,
    TaskNetworkStoreFactory,
};

const MAX_TASK_MATERIALIZATION_IDS: usize = 1_024;
const MAX_TASK_MATERIALIZATION_ID_BYTES: usize = 1_024;
const MAX_TASK_MATERIALIZATION_INCOMING_EDGES: usize = 1_024;
const MAX_TASK_MATERIALIZATION_BYTES: usize = 16 * 1_048_576;
const MAX_TASK_COMMAND_OUTCOME_ID_BYTES: usize = 1_024;

/// Public lifecycle of one task network authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskNetworkAuthorityLifecycle {
    /// New command and query work may enter the bounded mailbox.
    Open,
    /// Admission is fenced while previously accepted work drains.
    Closing,
    /// Both ports are permanently closed and the worker has stopped.
    Closed,
    /// Persistence became indeterminate or the durable epoch became stale.
    Poisoned,
}

/// Identity-bearing lifecycle observation for one authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskNetworkAuthorityLifecycleSnapshot {
    /// Stable network identity.
    pub network_id: String,
    /// Durable lifecycle epoch owned by the authority.
    pub epoch: u64,
    /// Current lifecycle state.
    pub lifecycle: TaskNetworkAuthorityLifecycle,
}

/// Lightweight durably acknowledged mutation snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskNetworkAuthoritySnapshot {
    /// Stable network identity.
    pub network_id: String,
    /// Durable lifecycle epoch serving this snapshot.
    pub epoch: u64,
    /// Latest durably acknowledged task-network revision.
    pub revision: u64,
    /// State hash paired with the acknowledged revision.
    pub state_hash: String,
    /// Canonical event ledger established by the first published outcome.
    pub publication_ledger_binding: Option<PublicationLedgerBinding>,
}

/// Compact identity and revision view for one durable task network.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskNetworkHead {
    /// Stable network identity.
    pub network_id: String,
    /// Latest durably acknowledged revision.
    pub revision: u64,
    /// State hash acknowledged at the revision.
    pub state_hash: String,
}

/// Bounded task materialization view for explicitly requested task identities.
#[derive(Debug, Clone, PartialEq)]
pub struct TaskNetworkMaterialization {
    /// Compact durable head observed with the materialization view.
    pub head: TaskNetworkHead,
    /// Present task nodes keyed by requested task identity.
    pub tasks: BTreeMap<String, TaskNode>,
    /// Incoming edges keyed by requested target task identity.
    pub incoming_edges: BTreeMap<String, Vec<DependencyEdge>>,
}

/// Typed cause retained after a task network authority poisons itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskNetworkAuthorityPoisonKind {
    /// Another authority superseded this worker's durable epoch.
    StaleEpoch,
    /// Durable storage returned a potentially transient I/O failure.
    Storage,
    /// Durable authority state could not be decoded or verified.
    CorruptState,
    /// The authority worker panicked while shutting down.
    WorkerPanic,
}

impl TaskNetworkAuthorityPoisonKind {
    /// Return whether replacing the authority may recover without operator repair.
    pub fn retryable(self) -> bool {
        matches!(self, Self::StaleEpoch | Self::Storage)
    }
}

/// Error returned by task network authority operations.
#[derive(Debug, Error)]
pub enum TaskNetworkAuthorityError {
    /// The bounded mailbox has no remaining admission capacity.
    #[error("task network authority mailbox is full for '{network_id}' at capacity {capacity}")]
    Full {
        /// Network whose mailbox is saturated.
        network_id: String,
        /// Configured mailbox capacity.
        capacity: usize,
    },
    /// Shutdown fenced new work before this request entered the mailbox.
    #[error("task network authority is closing for '{network_id}'")]
    Closing {
        /// Network whose authority is closing.
        network_id: String,
    },
    /// The authority worker and both ports are closed.
    #[error("task network authority is closed for '{network_id}'")]
    Closed {
        /// Network whose authority is closed.
        network_id: String,
    },
    /// The worker lost ownership of the durable lifecycle epoch.
    #[error(
        "task network authority epoch is stale for '{network_id}': expected {expected}, actual {actual}"
    )]
    StaleEpoch {
        /// Network whose epoch changed.
        network_id: String,
        /// Epoch owned by the worker.
        expected: u64,
        /// Current durable epoch.
        actual: u64,
    },
    /// The live authority is poisoned and cannot safely continue.
    #[error("task network authority is poisoned for '{network_id}': {reason}")]
    Poisoned {
        /// Network whose authority is poisoned.
        network_id: String,
        /// Typed cause used by supervisors to distinguish replacement from repair.
        kind: TaskNetworkAuthorityPoisonKind,
        /// Diagnostic for the first poisoning failure.
        reason: String,
    },
    /// Query and command capabilities came from different authority instances.
    #[error(
        "task network query authority '{query_network_id}' at epoch {query_epoch} does not match command authority '{command_network_id}' at epoch {command_epoch}"
    )]
    MismatchedPorts {
        /// Network identity carried by the query capability.
        query_network_id: String,
        /// Epoch carried by the query capability.
        query_epoch: u64,
        /// Network identity carried by the command capability.
        command_network_id: String,
        /// Epoch carried by the command capability.
        command_epoch: u64,
    },
    /// Authority configuration is invalid.
    #[error("invalid task network authority configuration: {0}")]
    InvalidConfiguration(String),
    /// A configured network identity was repeated.
    #[error("task network authority configured network '{0}' more than once")]
    DuplicateNetwork(String),
    /// The durable store could not be opened or initialized.
    #[error(transparent)]
    Store(#[from] TaskNetworkStoreError),
    /// The authority worker thread could not be created.
    #[error("failed to spawn task network authority worker: {0}")]
    WorkerSpawn(String),
}

/// Cloneable bounded command capability for one task network.
#[derive(Clone)]
pub struct TaskNetworkCommandPort {
    sender: SyncSender<Work>,
    shared: Arc<SharedLifecycle>,
}

impl TaskNetworkCommandPort {
    /// Try to admit one command and wait for its durable response.
    ///
    /// A full mailbox returns immediately. An admitted command returns only
    /// after its semantic state and command identity have crossed a successful
    /// sled flush.
    pub fn try_submit(
        &self,
        request: command::Request,
    ) -> Result<command::Response, TaskNetworkAuthorityError> {
        let (ack_sender, ack_receiver) = sync_channel(1);
        self.shared.try_admit(
            &self.sender,
            Work::Command {
                request: Box::new(request),
                ack: ack_sender,
            },
        )?;
        ack_receiver
            .recv()
            .map_err(|_| self.shared.closed_error())?
    }

    /// Return the identity and current lifecycle of the owning authority.
    pub fn lifecycle(&self) -> TaskNetworkAuthorityLifecycleSnapshot {
        self.shared.snapshot()
    }
}

/// Cloneable bounded query capability for one task network.
#[derive(Clone)]
pub struct TaskNetworkQueryPort {
    sender: SyncSender<Work>,
    shared: Arc<SharedLifecycle>,
}

impl TaskNetworkQueryPort {
    /// Return the exact durable outcome receipt for one command id.
    pub fn command_outcome(
        &self,
        command_id: impl Into<String>,
    ) -> Result<Option<command::OutcomeReceipt>, TaskNetworkAuthorityError> {
        let command_id = command_id.into();
        if command_id.trim().is_empty() || command_id.len() > MAX_TASK_COMMAND_OUTCOME_ID_BYTES {
            return Err(TaskNetworkAuthorityError::InvalidConfiguration(format!(
                "command outcome identity must be non-empty and at most {MAX_TASK_COMMAND_OUTCOME_ID_BYTES} bytes"
            )));
        }
        let (ack_sender, ack_receiver) = sync_channel(1);
        self.shared.try_admit(
            &self.sender,
            Work::CommandOutcome {
                command_id,
                ack: ack_sender,
            },
        )?;
        ack_receiver
            .recv()
            .map_err(|_| self.shared.closed_error())?
    }

    /// Return compact durable head metadata without cloning network products.
    pub fn head(&self) -> Result<TaskNetworkHead, TaskNetworkAuthorityError> {
        let (ack_sender, ack_receiver) = sync_channel(1);
        self.shared
            .try_admit(&self.sender, Work::Head { ack: ack_sender })?;
        ack_receiver
            .recv()
            .map_err(|_| self.shared.closed_error())?
    }

    /// Return lightweight mutation identity without cloning the full graph.
    pub fn snapshot(&self) -> Result<TaskNetworkAuthoritySnapshot, TaskNetworkAuthorityError> {
        let (ack_sender, ack_receiver) = sync_channel(1);
        self.shared
            .try_admit(&self.sender, Work::Snapshot { ack: ack_sender })?;
        ack_receiver
            .recv()
            .map_err(|_| self.shared.closed_error())?
    }

    /// Return the latest durably acknowledged reduced state.
    pub fn state(&self) -> Result<NetworkState, TaskNetworkAuthorityError> {
        let (ack_sender, ack_receiver) = sync_channel(1);
        self.shared
            .try_admit(&self.sender, Work::State { ack: ack_sender })?;
        ack_receiver
            .recv()
            .map_err(|_| self.shared.closed_error())?
    }

    /// Return durably acknowledged journal records in revision order.
    pub fn journal(&self) -> Result<Vec<JournalRecord>, TaskNetworkAuthorityError> {
        let (ack_sender, ack_receiver) = sync_channel(1);
        self.shared
            .try_admit(&self.sender, Work::Journal { ack: ack_sender })?;
        ack_receiver
            .recv()
            .map_err(|_| self.shared.closed_error())?
    }

    /// Return task nodes and incoming edges only for requested task identities.
    pub fn materialization(
        &self,
        task_instance_ids: Vec<String>,
    ) -> Result<TaskNetworkMaterialization, TaskNetworkAuthorityError> {
        validate_materialization_request(&task_instance_ids)?;
        let (ack_sender, ack_receiver) = sync_channel(1);
        self.shared.try_admit(
            &self.sender,
            Work::Materialization {
                task_instance_ids,
                ack: ack_sender,
            },
        )?;
        ack_receiver
            .recv()
            .map_err(|_| self.shared.closed_error())?
    }

    /// Compute the ready set from the latest durably acknowledged state.
    pub fn ready_set(&self) -> Result<ReadySet, TaskNetworkAuthorityError> {
        let (ack_sender, ack_receiver) = sync_channel(1);
        self.shared
            .try_admit(&self.sender, Work::ReadySet { ack: ack_sender })?;
        ack_receiver
            .recv()
            .map_err(|_| self.shared.closed_error())?
    }

    /// Return one durable publication outbox record by identity.
    pub fn publication(
        &self,
        publication_id: impl Into<String>,
    ) -> Result<Option<Publication>, TaskNetworkAuthorityError> {
        let (ack_sender, ack_receiver) = sync_channel(1);
        self.shared.try_admit(
            &self.sender,
            Work::Publication {
                publication_id: publication_id.into(),
                ack: ack_sender,
            },
        )?;
        ack_receiver
            .recv()
            .map_err(|_| self.shared.closed_error())?
    }

    /// Return at most the requested number of retryable publication records.
    ///
    /// Records are selected in deterministic publication identity order.
    pub fn retryable_publications(
        &self,
        limit: usize,
    ) -> Result<Vec<Publication>, TaskNetworkAuthorityError> {
        let (ack_sender, ack_receiver) = sync_channel(1);
        self.shared.try_admit(
            &self.sender,
            Work::RetryablePublications {
                limit,
                ack: ack_sender,
            },
        )?;
        ack_receiver
            .recv()
            .map_err(|_| self.shared.closed_error())?
    }

    /// Return the identity and current lifecycle of the owning authority.
    pub fn lifecycle(&self) -> TaskNetworkAuthorityLifecycleSnapshot {
        self.shared.snapshot()
    }
}

/// Query and command capabilities proven to share one live authority instance.
#[derive(Clone)]
pub struct TaskNetworkAuthorityPorts {
    query: TaskNetworkQueryPort,
    commands: TaskNetworkCommandPort,
}

impl TaskNetworkAuthorityPorts {
    /// Pair independently obtained capabilities only when they share exact authority identity.
    pub fn try_pair(
        query: TaskNetworkQueryPort,
        commands: TaskNetworkCommandPort,
    ) -> Result<Self, TaskNetworkAuthorityError> {
        if !Arc::ptr_eq(&query.shared, &commands.shared) {
            let query_identity = query.lifecycle();
            let command_identity = commands.lifecycle();
            return Err(TaskNetworkAuthorityError::MismatchedPorts {
                query_network_id: query_identity.network_id,
                query_epoch: query_identity.epoch,
                command_network_id: command_identity.network_id,
                command_epoch: command_identity.epoch,
            });
        }
        Ok(Self { query, commands })
    }

    /// Return the query capability owned by the paired authority.
    pub fn query(&self) -> &TaskNetworkQueryPort {
        &self.query
    }

    /// Return the command capability owned by the paired authority.
    pub fn commands(&self) -> &TaskNetworkCommandPort {
        &self.commands
    }

    /// Return the identity and current lifecycle shared by both capabilities.
    pub fn lifecycle(&self) -> TaskNetworkAuthorityLifecycleSnapshot {
        self.query.lifecycle()
    }
}

/// Final proof that one authority fenced admission, drained, flushed, and
/// joined its worker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskNetworkAuthorityShutdownReceipt {
    /// Stable network identity.
    pub network_id: String,
    /// Durable lifecycle epoch held until shutdown.
    pub epoch: u64,
    /// Final durably acknowledged task network revision.
    pub final_revision: u64,
    /// Final durably acknowledged task network state hash.
    pub final_state_hash: String,
    /// Number of final durably acknowledged journal records.
    pub journal_records: usize,
    /// Whether the authority had entered a poisoned state before closing.
    pub was_poisoned: bool,
}

/// Owner of one durable task network store and its serialized worker.
pub struct TaskNetworkAuthority {
    sender: SyncSender<Work>,
    shared: Arc<SharedLifecycle>,
    join: Option<JoinHandle<()>>,
    shutdown_receipt: Option<TaskNetworkAuthorityShutdownReceipt>,
}

impl TaskNetworkAuthority {
    /// Open one configured network exactly once and start its bounded worker.
    pub fn open(
        factory: &TaskNetworkStoreFactory,
        network_id: impl Into<String>,
        mailbox_capacity: usize,
    ) -> Result<Self, TaskNetworkAuthorityError> {
        if mailbox_capacity == 0 {
            return Err(TaskNetworkAuthorityError::InvalidConfiguration(
                "mailbox capacity must be greater than zero".to_string(),
            ));
        }
        let network_id = network_id.into();
        let thread_key = network_storage_key(&network_id).map_err(authority_open_error)?;
        let store = factory
            .open_network(network_id)
            .map_err(authority_open_error)?;
        Self::spawn_store(store, thread_key, mailbox_capacity)
    }

    fn spawn_store(
        mut store: SledTaskNetworkStore,
        thread_key: String,
        mailbox_capacity: usize,
    ) -> Result<Self, TaskNetworkAuthorityError> {
        let network_id = store.state().network_id.clone();
        let epoch = store.acquire_authority_epoch()?;
        let shared = Arc::new(SharedLifecycle::new(
            network_id.clone(),
            epoch,
            mailbox_capacity,
        ));
        let worker_shared = Arc::clone(&shared);
        let (sender, receiver) = sync_channel(mailbox_capacity);
        let join = std::thread::Builder::new()
            .name(format!("meld-task-network-{thread_key}"))
            .spawn(move || run_worker(store, epoch, receiver, worker_shared))
            .map_err(|error| TaskNetworkAuthorityError::WorkerSpawn(error.to_string()))?;
        Ok(Self {
            sender,
            shared,
            join: Some(join),
            shutdown_receipt: None,
        })
    }

    /// Return a cloneable command capability.
    pub fn command_port(&self) -> TaskNetworkCommandPort {
        TaskNetworkCommandPort {
            sender: self.sender.clone(),
            shared: Arc::clone(&self.shared),
        }
    }

    /// Return a cloneable query capability.
    pub fn query_port(&self) -> TaskNetworkQueryPort {
        TaskNetworkQueryPort {
            sender: self.sender.clone(),
            shared: Arc::clone(&self.shared),
        }
    }

    /// Return paired query and command capabilities from this exact authority instance.
    pub fn ports(&self) -> TaskNetworkAuthorityPorts {
        TaskNetworkAuthorityPorts {
            query: self.query_port(),
            commands: self.command_port(),
        }
    }

    /// Return the identity and current lifecycle of this authority.
    pub fn lifecycle(&self) -> TaskNetworkAuthorityLifecycleSnapshot {
        self.shared.snapshot()
    }

    /// Fence admission, drain accepted work, flush, close both ports, and join.
    pub fn shutdown(
        &mut self,
    ) -> Result<TaskNetworkAuthorityShutdownReceipt, TaskNetworkAuthorityError> {
        if let Some(receipt) = &self.shutdown_receipt {
            return Ok(receipt.clone());
        }
        let was_poisoned = self.shared.begin_shutdown()?;
        let (ack_sender, ack_receiver) = sync_channel(1);
        self.sender
            .send(Work::Shutdown { ack: ack_sender })
            .map_err(|_| self.shared.closed_error())?;
        let outcome = ack_receiver
            .recv()
            .map_err(|_| self.shared.closed_error())?;
        if let Some(join) = self.join.take() {
            if join.join().is_err() {
                self.shared.finish_shutdown();
                return Err(TaskNetworkAuthorityError::Poisoned {
                    network_id: self.shared.network_id.clone(),
                    kind: TaskNetworkAuthorityPoisonKind::WorkerPanic,
                    reason: "authority worker panicked during shutdown".to_string(),
                });
            }
        }
        self.shared.finish_shutdown();
        let final_state = outcome?;
        let receipt = TaskNetworkAuthorityShutdownReceipt {
            network_id: self.shared.network_id.clone(),
            epoch: self.shared.epoch,
            final_revision: final_state.state.revision,
            final_state_hash: final_state.state.state_hash,
            journal_records: final_state.journal_records,
            was_poisoned: was_poisoned || final_state.was_poisoned,
        };
        self.shutdown_receipt = Some(receipt.clone());
        Ok(receipt)
    }
}

fn authority_open_error(error: TaskNetworkStoreError) -> TaskNetworkAuthorityError {
    match error {
        TaskNetworkStoreError::InvalidConfiguration(message) => {
            TaskNetworkAuthorityError::InvalidConfiguration(message)
        }
        error => TaskNetworkAuthorityError::Store(error),
    }
}

impl Drop for TaskNetworkAuthority {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

/// Owner set that opens one authority for every unique configured network.
pub struct TaskNetworkAuthorities {
    authorities: BTreeMap<String, TaskNetworkAuthority>,
}

impl TaskNetworkAuthorities {
    /// Open each configured network exactly once.
    pub fn open(
        factory: &TaskNetworkStoreFactory,
        network_ids: impl IntoIterator<Item = String>,
        mailbox_capacity: usize,
    ) -> Result<Self, TaskNetworkAuthorityError> {
        if mailbox_capacity == 0 {
            return Err(TaskNetworkAuthorityError::InvalidConfiguration(
                "mailbox capacity must be greater than zero".to_string(),
            ));
        }
        let mut configured = BTreeMap::new();
        for network_id in network_ids {
            if configured.insert(network_id.clone(), ()).is_some() {
                return Err(TaskNetworkAuthorityError::DuplicateNetwork(network_id));
            }
        }
        let mut authorities = BTreeMap::new();
        for network_id in configured.into_keys() {
            let authority =
                TaskNetworkAuthority::open(factory, network_id.clone(), mailbox_capacity)?;
            authorities.insert(network_id, authority);
        }
        Ok(Self { authorities })
    }

    /// Return the command capability for one configured network.
    pub fn command_port(&self, network_id: &str) -> Option<TaskNetworkCommandPort> {
        self.authorities
            .get(network_id)
            .map(TaskNetworkAuthority::command_port)
    }

    /// Return the query capability for one configured network.
    pub fn query_port(&self, network_id: &str) -> Option<TaskNetworkQueryPort> {
        self.authorities
            .get(network_id)
            .map(TaskNetworkAuthority::query_port)
    }

    /// Return paired capabilities for one configured network authority.
    pub fn ports(&self, network_id: &str) -> Option<TaskNetworkAuthorityPorts> {
        self.authorities
            .get(network_id)
            .map(TaskNetworkAuthority::ports)
    }

    /// Return the number of uniquely configured authorities.
    pub fn len(&self) -> usize {
        self.authorities.len()
    }

    /// Return whether no network authorities are configured.
    pub fn is_empty(&self) -> bool {
        self.authorities.is_empty()
    }

    /// Shut down every configured authority and return final receipts.
    pub fn shutdown_all(
        &mut self,
    ) -> Result<Vec<TaskNetworkAuthorityShutdownReceipt>, TaskNetworkAuthorityError> {
        self.authorities
            .values_mut()
            .map(TaskNetworkAuthority::shutdown)
            .collect()
    }
}

enum Work {
    Command {
        request: Box<command::Request>,
        ack: SyncSender<Result<command::Response, TaskNetworkAuthorityError>>,
    },
    CommandOutcome {
        command_id: String,
        ack: SyncSender<Result<Option<command::OutcomeReceipt>, TaskNetworkAuthorityError>>,
    },
    State {
        ack: SyncSender<Result<NetworkState, TaskNetworkAuthorityError>>,
    },
    Snapshot {
        ack: SyncSender<Result<TaskNetworkAuthoritySnapshot, TaskNetworkAuthorityError>>,
    },
    Head {
        ack: SyncSender<Result<TaskNetworkHead, TaskNetworkAuthorityError>>,
    },
    Journal {
        ack: SyncSender<Result<Vec<JournalRecord>, TaskNetworkAuthorityError>>,
    },
    Materialization {
        task_instance_ids: Vec<String>,
        ack: SyncSender<Result<TaskNetworkMaterialization, TaskNetworkAuthorityError>>,
    },
    ReadySet {
        ack: SyncSender<Result<ReadySet, TaskNetworkAuthorityError>>,
    },
    Publication {
        publication_id: String,
        ack: SyncSender<Result<Option<Publication>, TaskNetworkAuthorityError>>,
    },
    RetryablePublications {
        limit: usize,
        ack: SyncSender<Result<Vec<Publication>, TaskNetworkAuthorityError>>,
    },
    Shutdown {
        ack: SyncSender<Result<WorkerShutdown, TaskNetworkAuthorityError>>,
    },
}

struct WorkerShutdown {
    state: NetworkState,
    journal_records: usize,
    was_poisoned: bool,
}

struct SharedLifecycle {
    network_id: String,
    epoch: u64,
    mailbox_capacity: usize,
    state: Mutex<SharedState>,
}

enum SharedState {
    Open,
    Closing,
    Closed,
    Poisoned(AuthorityPoison),
}

#[derive(Clone)]
struct AuthorityPoison {
    kind: TaskNetworkAuthorityPoisonKind,
    reason: String,
}

impl AuthorityPoison {
    fn stale_epoch(expected: u64, actual: u64) -> Self {
        Self {
            kind: TaskNetworkAuthorityPoisonKind::StaleEpoch,
            reason: format!("durable epoch changed from {expected} to {actual}"),
        }
    }

    fn store(error: &TaskNetworkStoreError) -> Self {
        let kind = match error {
            TaskNetworkStoreError::Storage(_) => TaskNetworkAuthorityPoisonKind::Storage,
            TaskNetworkStoreError::InvalidConfiguration(_)
            | TaskNetworkStoreError::CorruptStorage(_)
            | TaskNetworkStoreError::Decode(_) => TaskNetworkAuthorityPoisonKind::CorruptState,
        };
        Self {
            kind,
            reason: error.to_string(),
        }
    }
}

impl SharedLifecycle {
    fn new(network_id: String, epoch: u64, mailbox_capacity: usize) -> Self {
        Self {
            network_id,
            epoch,
            mailbox_capacity,
            state: Mutex::new(SharedState::Open),
        }
    }

    fn try_admit(
        &self,
        sender: &SyncSender<Work>,
        work: Work,
    ) -> Result<(), TaskNetworkAuthorityError> {
        let mut state = self.lock_state();
        match &*state {
            SharedState::Open => {}
            SharedState::Closing => return Err(self.closing_error()),
            SharedState::Closed => return Err(self.closed_error()),
            SharedState::Poisoned(poison) => return Err(self.poisoned_error(poison.clone())),
        }
        match sender.try_send(work) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(_)) => Err(TaskNetworkAuthorityError::Full {
                network_id: self.network_id.clone(),
                capacity: self.mailbox_capacity,
            }),
            Err(TrySendError::Disconnected(_)) => {
                *state = SharedState::Closed;
                Err(self.closed_error())
            }
        }
    }

    fn begin_shutdown(&self) -> Result<bool, TaskNetworkAuthorityError> {
        let mut state = self.lock_state();
        match &*state {
            SharedState::Open => {
                *state = SharedState::Closing;
                Ok(false)
            }
            SharedState::Poisoned(_) => Ok(true),
            SharedState::Closing => Err(self.closing_error()),
            SharedState::Closed => Err(self.closed_error()),
        }
    }

    fn poison(&self, poison: AuthorityPoison) {
        let mut state = self.lock_state();
        if !matches!(*state, SharedState::Closed) {
            *state = SharedState::Poisoned(poison);
        }
    }

    fn finish_shutdown(&self) {
        *self.lock_state() = SharedState::Closed;
    }

    fn snapshot(&self) -> TaskNetworkAuthorityLifecycleSnapshot {
        let lifecycle = match &*self.lock_state() {
            SharedState::Open => TaskNetworkAuthorityLifecycle::Open,
            SharedState::Closing => TaskNetworkAuthorityLifecycle::Closing,
            SharedState::Closed => TaskNetworkAuthorityLifecycle::Closed,
            SharedState::Poisoned(_) => TaskNetworkAuthorityLifecycle::Poisoned,
        };
        TaskNetworkAuthorityLifecycleSnapshot {
            network_id: self.network_id.clone(),
            epoch: self.epoch,
            lifecycle,
        }
    }

    fn lock_state(&self) -> std::sync::MutexGuard<'_, SharedState> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn closing_error(&self) -> TaskNetworkAuthorityError {
        TaskNetworkAuthorityError::Closing {
            network_id: self.network_id.clone(),
        }
    }

    fn closed_error(&self) -> TaskNetworkAuthorityError {
        TaskNetworkAuthorityError::Closed {
            network_id: self.network_id.clone(),
        }
    }

    fn poisoned_error(&self, poison: AuthorityPoison) -> TaskNetworkAuthorityError {
        TaskNetworkAuthorityError::Poisoned {
            network_id: self.network_id.clone(),
            kind: poison.kind,
            reason: poison.reason,
        }
    }
}

fn run_worker(
    mut store: SledTaskNetworkStore,
    epoch: u64,
    receiver: Receiver<Work>,
    shared: Arc<SharedLifecycle>,
) {
    let mut poison: Option<AuthorityPoison> = None;
    while let Ok(work) = receiver.recv() {
        match work {
            Work::Command { request, ack } => {
                let result = if let Some(poison) = &poison {
                    Err(shared.poisoned_error(poison.clone()))
                } else {
                    match store.submit_at_authority_epoch(epoch, *request) {
                        Ok(response) => Ok(response),
                        Err(AuthorityStoreError::StaleEpoch { expected, actual }) => {
                            let stale_poison = AuthorityPoison::stale_epoch(expected, actual);
                            poison = Some(stale_poison.clone());
                            shared.poison(stale_poison);
                            Err(TaskNetworkAuthorityError::StaleEpoch {
                                network_id: shared.network_id.clone(),
                                expected,
                                actual,
                            })
                        }
                        Err(AuthorityStoreError::Store(error)) => {
                            let store_poison = AuthorityPoison::store(&error);
                            poison = Some(store_poison.clone());
                            shared.poison(store_poison.clone());
                            Err(shared.poisoned_error(store_poison))
                        }
                    }
                };
                let _ = ack.send(result);
            }
            Work::CommandOutcome { command_id, ack } => {
                let result =
                    validate_query_epoch(&store, epoch, &shared, &mut poison).and_then(|()| {
                        match store.command_outcome_receipt(&command_id) {
                            Ok(receipt) => Ok(receipt),
                            Err(error) => {
                                let store_poison = AuthorityPoison::store(&error);
                                poison = Some(store_poison.clone());
                                shared.poison(store_poison.clone());
                                Err(shared.poisoned_error(store_poison))
                            }
                        }
                    });
                let _ = ack.send(result);
            }
            Work::State { ack } => {
                let result = validate_query_epoch(&store, epoch, &shared, &mut poison)
                    .map(|()| store.state().clone());
                let _ = ack.send(result);
            }
            Work::Snapshot { ack } => {
                let result = validate_query_epoch(&store, epoch, &shared, &mut poison).map(|()| {
                    TaskNetworkAuthoritySnapshot {
                        network_id: store.state().network_id.clone(),
                        epoch,
                        revision: store.state().revision,
                        state_hash: store.state().state_hash.clone(),
                        publication_ledger_binding: store.publication_ledger_binding().cloned(),
                    }
                });
                let _ = ack.send(result);
            }
            Work::Head { ack } => {
                let result = validate_query_epoch(&store, epoch, &shared, &mut poison)
                    .map(|()| task_network_head(store.state()));
                let _ = ack.send(result);
            }
            Work::Journal { ack } => {
                let result = validate_query_epoch(&store, epoch, &shared, &mut poison)
                    .map(|()| store.journal().to_vec());
                let _ = ack.send(result);
            }
            Work::Materialization {
                task_instance_ids,
                ack,
            } => {
                let result = validate_query_epoch(&store, epoch, &shared, &mut poison)
                    .and_then(|()| task_network_materialization(&store, task_instance_ids));
                let _ = ack.send(result);
            }
            Work::ReadySet { ack } => {
                let result = validate_query_epoch(&store, epoch, &shared, &mut poison)
                    .map(|()| compute_ready_set(store.state()));
                let _ = ack.send(result);
            }
            Work::Publication {
                publication_id,
                ack,
            } => {
                let result = validate_query_epoch(&store, epoch, &shared, &mut poison)
                    .map(|()| store.state().publications.get(&publication_id).cloned());
                let _ = ack.send(result);
            }
            Work::RetryablePublications { limit, ack } => {
                let result = validate_query_epoch(&store, epoch, &shared, &mut poison).map(|()| {
                    store
                        .state()
                        .publications
                        .values()
                        .filter(|publication| {
                            matches!(
                                publication.state,
                                PublicationState::Pending
                                    | PublicationState::Failed { .. }
                                    | PublicationState::Published { receipt: None, .. }
                            )
                        })
                        .take(limit)
                        .cloned()
                        .collect()
                });
                let _ = ack.send(result);
            }
            Work::Shutdown { ack } => {
                let flush_result = store.flush();
                let was_poisoned = poison.is_some();
                let result = match flush_result {
                    Ok(()) => Ok(WorkerShutdown {
                        state: store.state().clone(),
                        journal_records: store.journal().len(),
                        was_poisoned,
                    }),
                    Err(error) => Err(shared.poisoned_error(AuthorityPoison::store(&error))),
                };
                let _ = ack.send(result);
                return;
            }
        }
    }
    let _ = store.flush();
    shared.finish_shutdown();
}

fn task_network_head(state: &NetworkState) -> TaskNetworkHead {
    TaskNetworkHead {
        network_id: state.network_id.clone(),
        revision: state.revision,
        state_hash: state.state_hash.clone(),
    }
}

fn task_network_materialization(
    store: &SledTaskNetworkStore,
    task_instance_ids: Vec<String>,
) -> Result<TaskNetworkMaterialization, TaskNetworkAuthorityError> {
    let requested = task_instance_ids
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>();
    let mut tasks = BTreeMap::new();
    let mut incoming_edges = BTreeMap::new();
    let mut encoded_bytes = 0usize;
    for task_id in requested {
        let Some((task, edges)) = store.task_materialization(&task_id) else {
            incoming_edges.insert(task_id, Vec::new());
            continue;
        };
        if edges.len() > MAX_TASK_MATERIALIZATION_INCOMING_EDGES {
            return Err(TaskNetworkAuthorityError::InvalidConfiguration(format!(
                "task '{task_id}' has {} incoming edges, exceeding materialization limit {MAX_TASK_MATERIALIZATION_INCOMING_EDGES}",
                edges.len()
            )));
        }
        let task_bytes = serde_json::to_vec(task).map_err(|error| {
            TaskNetworkAuthorityError::InvalidConfiguration(format!(
                "task '{task_id}' materialization encoding failed: {error}"
            ))
        })?;
        let edge_bytes = serde_json::to_vec(edges).map_err(|error| {
            TaskNetworkAuthorityError::InvalidConfiguration(format!(
                "task '{task_id}' incoming edge encoding failed: {error}"
            ))
        })?;
        encoded_bytes = encoded_bytes
            .checked_add(task_id.len())
            .and_then(|bytes| bytes.checked_add(task_bytes.len()))
            .and_then(|bytes| bytes.checked_add(edge_bytes.len()))
            .ok_or_else(|| {
                TaskNetworkAuthorityError::InvalidConfiguration(
                    "task materialization encoded size overflowed".to_string(),
                )
            })?;
        if encoded_bytes > MAX_TASK_MATERIALIZATION_BYTES {
            return Err(TaskNetworkAuthorityError::InvalidConfiguration(format!(
                "task materialization exceeds {MAX_TASK_MATERIALIZATION_BYTES} encoded bytes"
            )));
        }
        tasks.insert(task_id.clone(), task.clone());
        incoming_edges.insert(task_id, edges.to_vec());
    }
    Ok(TaskNetworkMaterialization {
        head: task_network_head(store.state()),
        tasks,
        incoming_edges,
    })
}

fn validate_materialization_request(
    task_instance_ids: &[String],
) -> Result<(), TaskNetworkAuthorityError> {
    if task_instance_ids.len() > MAX_TASK_MATERIALIZATION_IDS {
        return Err(TaskNetworkAuthorityError::InvalidConfiguration(format!(
            "task materialization requested {} identities, exceeding limit {MAX_TASK_MATERIALIZATION_IDS}",
            task_instance_ids.len()
        )));
    }
    if let Some(task_id) = task_instance_ids.iter().find(|task_id| {
        task_id.trim().is_empty() || task_id.len() > MAX_TASK_MATERIALIZATION_ID_BYTES
    }) {
        return Err(TaskNetworkAuthorityError::InvalidConfiguration(format!(
            "task materialization identity has invalid byte length {}",
            task_id.len()
        )));
    }
    Ok(())
}

fn validate_query_epoch(
    store: &SledTaskNetworkStore,
    epoch: u64,
    shared: &SharedLifecycle,
    poison: &mut Option<AuthorityPoison>,
) -> Result<(), TaskNetworkAuthorityError> {
    if let Some(poison) = poison {
        return Err(shared.poisoned_error(poison.clone()));
    }
    match store.validate_authority_epoch(epoch) {
        Ok(()) => Ok(()),
        Err(AuthorityStoreError::StaleEpoch { expected, actual }) => {
            let stale_poison = AuthorityPoison::stale_epoch(expected, actual);
            *poison = Some(stale_poison.clone());
            shared.poison(stale_poison);
            Err(TaskNetworkAuthorityError::StaleEpoch {
                network_id: shared.network_id.clone(),
                expected,
                actual,
            })
        }
        Err(AuthorityStoreError::Store(error)) => {
            let store_poison = AuthorityPoison::store(&error);
            *poison = Some(store_poison.clone());
            shared.poison(store_poison.clone());
            Err(shared.poisoned_error(store_poison))
        }
    }
}

#[cfg(test)]
mod tests;
