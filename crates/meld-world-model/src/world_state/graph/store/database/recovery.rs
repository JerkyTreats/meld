//! Prevent engine teardown from committing a quarantined handle.

use agdb::{DbError, StorageData, StorageSlice, SyncMode};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

pub(in crate::world_state::graph::store) struct RecoveryStorage<S: StorageData> {
    inner: S,
    recovery_only: Arc<AtomicBool>,
}

impl<S: StorageData> RecoveryStorage<S> {
    pub(super) fn wrap(inner: S, recovery_only: Arc<AtomicBool>) -> Self {
        Self {
            inner,
            recovery_only,
        }
    }

    fn available(&self) -> Result<(), DbError> {
        if self.recovery_only.load(Ordering::Acquire) {
            return Err(DbError::from(std::io::Error::other(
                "Graph quarantine forbids engine access during recovery teardown",
            )));
        }
        Ok(())
    }
}

impl<S: StorageData> StorageData for RecoveryStorage<S> {
    fn new(name: &str) -> Result<Self, DbError> {
        Ok(Self::wrap(S::new(name)?, Arc::new(AtomicBool::new(false))))
    }

    fn backup(&self, name: &str) -> Result<(), DbError> {
        self.available()?;
        self.inner.backup(name)
    }

    fn copy(&self, name: &str) -> Result<Self, DbError> {
        self.available()?;
        Ok(Self::wrap(
            self.inner.copy(name)?,
            Arc::new(AtomicBool::new(false)),
        ))
    }

    fn flush(&mut self) -> Result<(), DbError> {
        self.available()?;
        self.inner.flush()
    }

    fn len(&self) -> u64 {
        self.inner.len()
    }
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn read(&self, pos: u64, value_len: u64) -> Result<StorageSlice<'_>, DbError> {
        self.available()?;
        self.inner.read(pos, value_len)
    }

    fn rename(&mut self, name: &str) -> Result<(), DbError> {
        self.available()?;
        self.inner.rename(name)
    }

    fn resize(&mut self, len: u64) -> Result<(), DbError> {
        self.available()?;
        self.inner.resize(len)
    }

    fn set_sync_mode(&mut self, mode: SyncMode) {
        self.inner.set_sync_mode(mode);
    }
    fn sync_mode(&self) -> SyncMode {
        self.inner.sync_mode()
    }

    fn sync(&mut self) -> Result<(), DbError> {
        self.available()?;
        self.inner.sync()
    }

    fn write(&mut self, pos: u64, value: &[u8]) -> Result<(), DbError> {
        self.available()?;
        self.inner.write(pos, value)
    }
}
