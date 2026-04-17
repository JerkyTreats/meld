use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use crate::branches::contracts::BranchMigrationLedgerEntry;
use crate::error::{ApiError, StorageError};

pub fn append(path: &Path, entry: &BranchMigrationLedgerEntry) -> Result<(), ApiError> {
    let parent = path.parent().ok_or_else(|| {
        ApiError::ConfigError(format!(
            "Migration ledger path missing parent: {}",
            path.display()
        ))
    })?;
    fs::create_dir_all(parent).map_err(StorageError::IoError)?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(StorageError::IoError)?;
    let serialized = serde_json::to_string(entry).map_err(|err| {
        ApiError::ConfigError(format!(
            "Failed to serialize migration ledger entry: {}",
            err
        ))
    })?;
    writeln!(file, "{}", serialized).map_err(StorageError::IoError)?;
    Ok(())
}
