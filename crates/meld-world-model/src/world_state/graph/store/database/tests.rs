use std::sync::{
    atomic::{AtomicU8, AtomicUsize, Ordering},
    mpsc, Arc,
};
use std::time::Duration;

use agdb::{
    DbError, DbErrorType, DbImpl, FileStorage, FileStorageMemoryMapped, QueryBuilder as Q,
    StorageData, StorageSlice, SyncMode,
};
use tempfile::TempDir;

use super::Database;
use crate::error::StorageError;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
enum Fault {
    None,
    Read,
    Write,
    Sync,
    Flush,
}

#[derive(Clone)]
struct FaultControl(Arc<AtomicU8>);

impl FaultControl {
    fn new() -> Self {
        Self(Arc::new(AtomicU8::new(Fault::None as u8)))
    }

    fn arm(&self, fault: Fault) {
        self.0.store(fault as u8, Ordering::SeqCst);
    }

    fn fail(&self, operation: Fault) -> Result<(), DbError> {
        if self
            .0
            .compare_exchange(
                operation as u8,
                Fault::None as u8,
                Ordering::SeqCst,
                Ordering::SeqCst,
            )
            .is_ok()
        {
            return Err(DbError::from(std::io::Error::other(format!(
                "injected {operation:?} failure"
            ))));
        }
        Ok(())
    }

    fn is_consumed(&self) -> bool {
        self.0.load(Ordering::SeqCst) == Fault::None as u8
    }
}

struct FaultStorage<S> {
    inner: S,
    control: FaultControl,
}

impl<S> FaultStorage<S> {
    fn wrap(inner: S) -> (Self, FaultControl) {
        let control = FaultControl::new();
        (
            Self {
                inner,
                control: control.clone(),
            },
            control,
        )
    }
}

impl<S: StorageData> StorageData for FaultStorage<S> {
    fn backup(&self, name: &str) -> Result<(), DbError> {
        self.inner.backup(name)
    }

    fn copy(&self, name: &str) -> Result<Self, DbError> {
        Ok(Self {
            inner: self.inner.copy(name)?,
            control: self.control.clone(),
        })
    }

    fn flush(&mut self) -> Result<(), DbError> {
        self.control.fail(Fault::Flush)?;
        self.inner.flush()
    }

    fn len(&self) -> u64 {
        self.inner.len()
    }

    fn name(&self) -> &str {
        self.inner.name()
    }

    fn new(name: &str) -> Result<Self, DbError> {
        Ok(Self {
            inner: S::new(name)?,
            control: FaultControl::new(),
        })
    }

    fn read(&self, pos: u64, value_len: u64) -> Result<StorageSlice<'_>, DbError> {
        self.control.fail(Fault::Read)?;
        self.inner.read(pos, value_len)
    }

    fn rename(&mut self, name: &str) -> Result<(), DbError> {
        self.inner.rename(name)
    }

    fn resize(&mut self, len: u64) -> Result<(), DbError> {
        self.inner.resize(len)
    }

    fn set_sync_mode(&mut self, mode: SyncMode) {
        self.inner.set_sync_mode(mode);
    }

    fn sync_mode(&self) -> SyncMode {
        self.inner.sync_mode()
    }

    fn sync(&mut self) -> Result<(), DbError> {
        self.control.fail(Fault::Sync)?;
        self.inner.sync()
    }

    fn write(&mut self, pos: u64, bytes: &[u8]) -> Result<(), DbError> {
        self.control.fail(Fault::Write)?;
        self.inner.write(pos, bytes)
    }
}

fn data(error: impl std::fmt::Display) -> StorageError {
    StorageError::InvalidPath(error.to_string())
}

fn path(temp: &TempDir) -> String {
    temp.path()
        .join("graph.agdb")
        .to_string_lossy()
        .into_owned()
}

fn state<S: StorageData>(db: &DbImpl<S>) -> Result<String, DbError> {
    Ok(format!(
        "{:?}",
        db.exec(Q::select().ids("a").query())?.elements[0].values
    ))
}

fn has_text<S: StorageData>(db: &DbImpl<S>, key: &str, value: &str) -> Result<bool, DbError> {
    let result = db.exec(Q::select().ids("a").query())?;
    Ok(result.elements[0].values.contains(&(key, value).into()))
}

fn has_number<S: StorageData>(db: &DbImpl<S>, key: &str, value: i64) -> Result<bool, DbError> {
    let result = db.exec(Q::select().ids("a").query())?;
    Ok(result.elements[0].values.contains(&(key, value).into()))
}

fn seeded_database<S: StorageData>(
    filename: &str,
    mode: SyncMode,
) -> (Database<FaultStorage<S>>, FaultControl) {
    let (storage, control) = FaultStorage::wrap(S::new(filename).unwrap());
    let database = Database::with_storage(storage).unwrap();
    database
        .mutate(|db| {
            db.set_sync_mode(mode);
            Ok(())
        })
        .unwrap();
    database
        .mutate(|db| {
            db.transaction_mut(|tx| {
                tx.exec_mut(
                    Q::insert()
                        .nodes()
                        .aliases("a")
                        .values_uniform(vec![("k", "original").into(), ("later", 0_i64).into()])
                        .query(),
                )?;
                Ok::<_, DbError>(())
            })
            .map_err(data)
        })
        .unwrap();
    (database, control)
}

fn assert_rollback_failure_quarantines<S: StorageData>(fault: Fault, mode: SyncMode) {
    let temp = TempDir::new().unwrap();
    let filename = path(&temp);
    let (database, control) = seeded_database::<S>(&filename, mode);
    let database = Arc::new(database);
    let clone = Arc::clone(&database);

    let result = database.mutate(|db| {
        db.transaction_mut(|tx| -> Result<(), DbError> {
            tx.exec_mut(
                Q::insert()
                    .values_uniform(vec![("k", "changed").into()])
                    .ids("a")
                    .query(),
            )?;
            control.arm(fault);
            Err(DbError::db(
                DbErrorType::NotAllowed,
                "intentional closure error",
            ))
        })
        .map_err(data)
    });
    let error = result.unwrap_err().to_string();
    assert!(error.contains("injected"), "{error}");
    assert!(control.is_consumed());

    let closures = Arc::new(AtomicUsize::new(0));
    let observed = closures.clone();
    assert!(clone
        .mutate(|_| {
            observed.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })
        .is_err());
    assert_eq!(closures.load(Ordering::SeqCst), 0);
    assert!(database.read().is_err());

    drop(clone);
    drop(database);
    let reopened = DbImpl::<S>::new(&filename).unwrap();
    let reopened_state = state(&reopened).unwrap();
    assert!(
        has_text(&reopened, "k", "original").unwrap(),
        "{reopened_state}"
    );
    assert!(
        !has_text(&reopened, "k", "changed").unwrap(),
        "{reopened_state}"
    );

    drop(reopened);
    let recovered = Database::with_storage(S::new(&filename).unwrap()).unwrap();
    recovered
        .mutate(|db| {
            db.set_sync_mode(mode);
            Ok(())
        })
        .unwrap();
    recovered
        .mutate(|db| {
            db.transaction_mut(|tx| {
                tx.exec_mut(
                    Q::insert()
                        .values_uniform(vec![("later", 1_i64).into()])
                        .ids("a")
                        .query(),
                )?;
                Ok::<_, DbError>(())
            })
            .map_err(data)
        })
        .unwrap();
    drop(recovered);

    let reopened_again = DbImpl::<S>::new(&filename).unwrap();
    assert!(has_text(&reopened_again, "k", "original").unwrap());
    assert!(has_number(&reopened_again, "later", 1).unwrap());
}

#[test]
fn file_storage_rollback_failures_quarantine_all_clones() {
    for mode in [SyncMode::None, SyncMode::Commit] {
        for fault in [Fault::Read, Fault::Write] {
            assert_rollback_failure_quarantines::<FileStorage>(fault, mode);
        }
    }
}

#[test]
fn memory_mapped_rollback_failures_quarantine_all_clones() {
    for mode in [SyncMode::None, SyncMode::Commit] {
        for fault in [Fault::Read, Fault::Write] {
            assert_rollback_failure_quarantines::<FileStorageMemoryMapped>(fault, mode);
        }
    }
}

#[test]
fn sync_failure_quarantines_database() {
    let temp = TempDir::new().unwrap();
    let filename = path(&temp);
    let (database, control) = seeded_database::<FileStorage>(&filename, SyncMode::Commit);
    control.arm(Fault::Sync);

    let error = database
        .mutate(|db| db.sync().map_err(data))
        .unwrap_err()
        .to_string();
    assert!(error.contains("injected Sync failure"), "{error}");
    assert!(control.is_consumed());
    assert!(database.read().is_err());
    assert!(database.mutate(|_| Ok(())).is_err());

    drop(database);
    let reopened = DbImpl::<FileStorage>::new(&filename).unwrap();
    assert!(has_text(&reopened, "k", "original").unwrap());
    assert!(has_number(&reopened, "later", 0).unwrap());
}

#[test]
fn commit_flush_failure_quarantines_and_recovers_prior_state() {
    let temp = TempDir::new().unwrap();
    let filename = path(&temp);
    let (database, control) = seeded_database::<FileStorage>(&filename, SyncMode::Commit);

    let error = database
        .mutate(|db| {
            db.transaction_mut(|tx| {
                tx.exec_mut(
                    Q::insert()
                        .values_uniform(vec![("later", 1_i64).into()])
                        .ids("a")
                        .query(),
                )?;
                control.arm(Fault::Flush);
                Ok::<_, DbError>(())
            })
            .map_err(data)
        })
        .unwrap_err()
        .to_string();
    assert!(error.contains("injected Flush failure"), "{error}");
    assert!(control.is_consumed());
    assert!(database.read().is_err());

    drop(database);
    let reopened = DbImpl::<FileStorage>::new(&filename).unwrap();
    assert!(has_text(&reopened, "k", "original").unwrap());
    assert!(has_number(&reopened, "later", 0).unwrap());
}

#[test]
fn panic_quarantines_database_and_is_resumed() {
    let temp = TempDir::new().unwrap();
    let filename = path(&temp);
    let (database, _) = seeded_database::<FileStorage>(&filename, SyncMode::None);

    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = database.mutate::<()>(|_| panic!("intentional mutation panic"));
    }));
    assert!(panic.is_err());
    assert!(database.read().is_err());

    let closures = AtomicUsize::new(0);
    assert!(database
        .mutate(|_| {
            closures.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })
        .is_err());
    assert_eq!(closures.load(Ordering::SeqCst), 0);
}

#[test]
fn failed_writer_rejects_its_own_reads_and_a_waiting_reader() {
    let temp = TempDir::new().unwrap();
    let filename = path(&temp);
    let (database, _) = seeded_database::<FileStorage>(&filename, SyncMode::None);
    let database = Arc::new(database);
    let mut writer = database.write().unwrap();
    let reader_database = Arc::clone(&database);
    let (started_tx, started_rx) = mpsc::channel();
    let (result_tx, result_rx) = mpsc::channel();

    let reader = std::thread::spawn(move || {
        started_tx.send(()).unwrap();
        result_tx.send(reader_database.read().is_err()).unwrap();
    });
    started_rx.recv().unwrap();
    assert!(result_rx.recv_timeout(Duration::from_millis(25)).is_err());

    assert!(writer
        .mutate(|_| Err::<(), _>(StorageError::InvalidPath("injected failure".into())))
        .is_err());
    assert!(writer.read().is_err());
    drop(writer);

    assert!(result_rx.recv_timeout(Duration::from_secs(1)).unwrap());
    reader.join().unwrap();
}

#[test]
fn successful_transaction_remains_available_and_survives_reopen() {
    let temp = TempDir::new().unwrap();
    let filename = path(&temp);
    let (database, _) = seeded_database::<FileStorageMemoryMapped>(&filename, SyncMode::Commit);

    database
        .mutate(|db| {
            db.transaction_mut(|tx| {
                tx.exec_mut(
                    Q::insert()
                        .values_uniform(vec![("later", 1_i64).into()])
                        .ids("a")
                        .query(),
                )?;
                Ok::<_, DbError>(())
            })
            .map_err(data)
        })
        .unwrap();
    assert!(has_number(&database.read().unwrap(), "later", 1).unwrap());

    drop(database);
    let reopened = DbImpl::<FileStorageMemoryMapped>::new(&filename).unwrap();
    let reopened_state = state(&reopened).unwrap();
    assert!(
        has_text(&reopened, "k", "original").unwrap(),
        "{reopened_state}"
    );
    assert!(
        has_number(&reopened, "later", 1).unwrap(),
        "{reopened_state}"
    );
}
