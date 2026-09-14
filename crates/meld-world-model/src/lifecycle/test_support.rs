use std::fs::{OpenOptions, TryLockError};
use std::path::Path;
use std::time::{Duration, Instant};

const SLED_CLOSE_READINESS_TIMEOUT: Duration = Duration::from_secs(1);
const SLED_CLOSE_READINESS_DELAY: Duration = Duration::from_millis(1);

/// Wait for sled 0.34.7 asynchronous writers to release the physical database
/// lock, then perform one fresh open. The probe never takes over a live holder.
pub(crate) fn reopen_sled_after_close(path: &Path) -> sled::Db {
    reopen_sled_after_close_with_timeout(path, SLED_CLOSE_READINESS_TIMEOUT)
        .unwrap_or_else(|error| panic!("{error}"))
}

fn reopen_sled_after_close_with_timeout(
    path: &Path,
    timeout: Duration,
) -> Result<sled::Db, String> {
    let lock_path = path.join("db");
    let deadline = Instant::now() + timeout;
    loop {
        let probe = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&lock_path)
            .map_err(|error| {
                format!(
                    "failed to open sled lock probe at {}: {error}",
                    lock_path.display()
                )
            })?;
        match probe.try_lock() {
            Ok(()) => {
                probe
                    .unlock()
                    .map_err(|error| format!("failed to release sled lock probe: {error}"))?;
                drop(probe);
                return sled::open(path).map_err(|error| error.to_string());
            }
            Err(TryLockError::WouldBlock) if Instant::now() < deadline => {
                std::thread::sleep(SLED_CLOSE_READINESS_DELAY);
            }
            Err(TryLockError::WouldBlock) => {
                return Err(format!(
                    "sled physical close did not release {} within {} ms",
                    lock_path.display(),
                    timeout.as_millis()
                ));
            }
            Err(error) => return Err(format!("sled lock probe failed: {error}")),
        }
    }
}

#[test]
fn physical_reopen_waits_for_release_without_taking_over_a_live_holder() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("world-model.sled");
    let holder = sled::open(&path).unwrap();
    let error = reopen_sled_after_close_with_timeout(&path, Duration::from_millis(10)).unwrap_err();
    assert!(error.contains("did not release"));

    let release = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(5));
        drop(holder);
    });
    let reopened = reopen_sled_after_close_with_timeout(&path, Duration::from_secs(1)).unwrap();
    release.join().unwrap();
    drop(reopened);
}
