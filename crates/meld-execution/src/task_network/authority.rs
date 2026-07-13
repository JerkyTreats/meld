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
use crate::task_network::state::{NetworkState, ReadySet};
use crate::task_network::store::{
    network_storage_key, AuthorityStoreError, SledTaskNetworkStore, TaskNetworkStoreError,
    TaskNetworkStoreFactory,
};

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
        /// Diagnostic for the first poisoning failure.
        reason: String,
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
        let thread_key = network_storage_key(&network_id)?;
        let store = factory.open_network(network_id)?;
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
    State {
        ack: SyncSender<Result<NetworkState, TaskNetworkAuthorityError>>,
    },
    Snapshot {
        ack: SyncSender<Result<TaskNetworkAuthoritySnapshot, TaskNetworkAuthorityError>>,
    },
    Journal {
        ack: SyncSender<Result<Vec<JournalRecord>, TaskNetworkAuthorityError>>,
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
    Poisoned(String),
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
            SharedState::Poisoned(reason) => return Err(self.poisoned_error(reason.clone())),
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

    fn poison(&self, reason: String) {
        let mut state = self.lock_state();
        if !matches!(*state, SharedState::Closed) {
            *state = SharedState::Poisoned(reason);
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

    fn poisoned_error(&self, reason: String) -> TaskNetworkAuthorityError {
        TaskNetworkAuthorityError::Poisoned {
            network_id: self.network_id.clone(),
            reason,
        }
    }
}

fn run_worker(
    mut store: SledTaskNetworkStore,
    epoch: u64,
    receiver: Receiver<Work>,
    shared: Arc<SharedLifecycle>,
) {
    let mut poison_reason: Option<String> = None;
    while let Ok(work) = receiver.recv() {
        match work {
            Work::Command { request, ack } => {
                let result = if let Some(reason) = &poison_reason {
                    Err(shared.poisoned_error(reason.clone()))
                } else {
                    match store.submit_at_authority_epoch(epoch, *request) {
                        Ok(response) => Ok(response),
                        Err(AuthorityStoreError::StaleEpoch { expected, actual }) => {
                            let reason =
                                format!("durable epoch changed from {expected} to {actual}");
                            poison_reason = Some(reason.clone());
                            shared.poison(reason);
                            Err(TaskNetworkAuthorityError::StaleEpoch {
                                network_id: shared.network_id.clone(),
                                expected,
                                actual,
                            })
                        }
                        Err(AuthorityStoreError::Store(error)) => {
                            let reason = error.to_string();
                            poison_reason = Some(reason.clone());
                            shared.poison(reason.clone());
                            Err(shared.poisoned_error(reason))
                        }
                    }
                };
                let _ = ack.send(result);
            }
            Work::State { ack } => {
                let result = validate_query_epoch(&store, epoch, &shared, &mut poison_reason)
                    .map(|()| store.state().clone());
                let _ = ack.send(result);
            }
            Work::Snapshot { ack } => {
                let result =
                    validate_query_epoch(&store, epoch, &shared, &mut poison_reason).map(|()| {
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
            Work::Journal { ack } => {
                let result = validate_query_epoch(&store, epoch, &shared, &mut poison_reason)
                    .map(|()| store.journal().to_vec());
                let _ = ack.send(result);
            }
            Work::ReadySet { ack } => {
                let result = validate_query_epoch(&store, epoch, &shared, &mut poison_reason)
                    .map(|()| compute_ready_set(store.state()));
                let _ = ack.send(result);
            }
            Work::Publication {
                publication_id,
                ack,
            } => {
                let result = validate_query_epoch(&store, epoch, &shared, &mut poison_reason)
                    .map(|()| store.state().publications.get(&publication_id).cloned());
                let _ = ack.send(result);
            }
            Work::RetryablePublications { limit, ack } => {
                let result =
                    validate_query_epoch(&store, epoch, &shared, &mut poison_reason).map(|()| {
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
                let was_poisoned = poison_reason.is_some();
                let result = match flush_result {
                    Ok(()) => Ok(WorkerShutdown {
                        state: store.state().clone(),
                        journal_records: store.journal().len(),
                        was_poisoned,
                    }),
                    Err(error) => Err(shared.poisoned_error(error.to_string())),
                };
                let _ = ack.send(result);
                return;
            }
        }
    }
    let _ = store.flush();
    shared.finish_shutdown();
}

fn validate_query_epoch(
    store: &SledTaskNetworkStore,
    epoch: u64,
    shared: &SharedLifecycle,
    poison_reason: &mut Option<String>,
) -> Result<(), TaskNetworkAuthorityError> {
    if let Some(reason) = poison_reason {
        return Err(shared.poisoned_error(reason.clone()));
    }
    match store.validate_authority_epoch(epoch) {
        Ok(()) => Ok(()),
        Err(AuthorityStoreError::StaleEpoch { expected, actual }) => {
            let reason = format!("durable epoch changed from {expected} to {actual}");
            *poison_reason = Some(reason.clone());
            shared.poison(reason);
            Err(TaskNetworkAuthorityError::StaleEpoch {
                network_id: shared.network_id.clone(),
                expected,
                actual,
            })
        }
        Err(AuthorityStoreError::Store(error)) => {
            let reason = error.to_string();
            *poison_reason = Some(reason.clone());
            shared.poison(reason.clone());
            Err(shared.poisoned_error(reason))
        }
    }
}

#[cfg(test)]
mod tests;
