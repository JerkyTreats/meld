//! Fail-closed access to one physical publication database.

use agdb::{DbImpl, StorageData};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

mod recovery;
use parking_lot::{MappedRwLockReadGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};
pub(super) use recovery::RecoveryStorage;

use crate::error::StorageError;

struct State<S: StorageData> {
    db: DbImpl<RecoveryStorage<S>>,
    quarantined: bool,
}

/// Every holder shares the failure state as well as the database lock.
/// Recovery requires releasing this handle and opening the database anew.
pub(super) struct Database<S: StorageData> {
    state: RwLock<State<S>>,
    recovery_only: Arc<AtomicBool>,
}

impl<S: StorageData> Database<S> {
    pub(super) fn with_storage(storage: S) -> Result<Self, StorageError> {
        let recovery_only = Arc::new(AtomicBool::new(false));
        let db = DbImpl::with_data(RecoveryStorage::wrap(storage, recovery_only.clone()))
            .map_err(super::publications::data)?;
        Ok(Self {
            state: RwLock::new(State {
                db,
                quarantined: false,
            }),
            recovery_only,
        })
    }

    pub(super) fn read(
        &self,
    ) -> Result<MappedRwLockReadGuard<'_, DbImpl<RecoveryStorage<S>>>, StorageError> {
        let state = self.state.read();
        if state.quarantined {
            return Err(unavailable());
        }
        Ok(RwLockReadGuard::map(state, |state| &state.db))
    }

    pub(super) fn mutate<T>(
        &self,
        operation: impl FnOnce(&mut DbImpl<RecoveryStorage<S>>) -> Result<T, StorageError>,
    ) -> Result<T, StorageError> {
        self.write()?.mutate(operation)
    }

    pub(super) fn write(&self) -> Result<WriteAccess<'_, S>, StorageError> {
        let state = self.state.write();
        if state.quarantined {
            return Err(unavailable());
        }
        Ok(WriteAccess { state })
    }
}

impl<S: StorageData> Drop for Database<S> {
    fn drop(&mut self) {
        if self.state.get_mut().quarantined {
            // agdb optimizes in DbImpl::drop. For an uncertain handle, forbid
            // that finalization; the underlying file backend still drops and
            // applies its retained WAL using its own recovery implementation.
            self.recovery_only.store(true, Ordering::Release);
        }
    }
}

/// Preflight checks can read under exclusive access without poisoning the
/// database on an ordinary domain rejection. Mutable access is only via mutate.
pub(super) struct WriteAccess<'a, S: StorageData> {
    state: RwLockWriteGuard<'a, State<S>>,
}

impl<S: StorageData> WriteAccess<'_, S> {
    pub(super) fn read(&self) -> Result<&DbImpl<RecoveryStorage<S>>, StorageError> {
        if self.state.quarantined {
            return Err(unavailable());
        }
        Ok(&self.state.db)
    }

    pub(super) fn mutate<T>(
        &mut self,
        operation: impl FnOnce(&mut DbImpl<RecoveryStorage<S>>) -> Result<T, StorageError>,
    ) -> Result<T, StorageError> {
        if self.state.quarantined {
            return Err(unavailable());
        }
        // Set before entering agdb: returned errors and unwinding both leave the
        // shared handle closed. Do not commit or sync to try to clean up an error.
        self.state.quarantined = true;
        match operation(&mut self.state.db) {
            Ok(value) => {
                self.state.quarantined = false;
                Ok(value)
            }
            Err(error) => Err(StorageError::InvalidPath(format!(
                "Graph database quarantined after failed operation: {error}; release all holders and reopen for recovery"
            ))),
        }
    }
}

fn unavailable() -> StorageError {
    StorageError::InvalidPath(
        "Graph database quarantined; release all holders and reopen for recovery".into(),
    )
}

#[cfg(test)]
mod tests;
