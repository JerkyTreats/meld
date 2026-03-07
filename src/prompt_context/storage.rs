//! Filesystem CAS storage for prompt context artifacts.

use crate::error::{ApiError, StorageError};
use crate::prompt_context::contracts::{PromptContextArtifactKind, PromptContextArtifactRef};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use walkdir::WalkDir;

pub struct PromptContextArtifactStorage {
    root: PathBuf,
}

impl PromptContextArtifactStorage {
    pub fn new<P: AsRef<Path>>(root: P) -> Result<Self, StorageError> {
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(&root)?;
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn write_utf8(
        &self,
        kind: PromptContextArtifactKind,
        value: &str,
    ) -> Result<PromptContextArtifactRef, ApiError> {
        self.write_bytes(kind, value.as_bytes())
    }

    pub fn write_bytes(
        &self,
        kind: PromptContextArtifactKind,
        bytes: &[u8],
    ) -> Result<PromptContextArtifactRef, ApiError> {
        if bytes.len() > kind.max_bytes() {
            return Err(ApiError::PromptContextArtifactBudgetExceeded {
                kind: kind.as_str().to_string(),
                actual_bytes: bytes.len(),
                max_bytes: kind.max_bytes(),
            });
        }

        let digest = blake3::hash(bytes).to_hex().to_string();
        let path = self.artifact_path_for_digest(&digest);
        if !path.exists() {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(StorageError::from)?;
            }
            let tmp = path.with_extension("blob.tmp");
            fs::write(&tmp, bytes).map_err(StorageError::from)?;
            fs::rename(&tmp, &path).map_err(|err| {
                let _ = fs::remove_file(&tmp);
                StorageError::IoError(std::io::Error::new(
                    err.kind(),
                    format!("Failed to move temp artifact into place: {}", err),
                ))
            })?;
        }

        Ok(PromptContextArtifactRef {
            artifact_id: digest.clone(),
            digest,
            byte_len: bytes.len(),
            kind,
        })
    }

    pub fn read_verified(&self, artifact: &PromptContextArtifactRef) -> Result<Vec<u8>, ApiError> {
        let path = self.artifact_path_for_digest(&artifact.artifact_id);
        if !path.exists() {
            return Err(ApiError::PromptContextArtifactNotFound {
                artifact_id: artifact.artifact_id.clone(),
            });
        }

        let bytes = fs::read(&path).map_err(StorageError::from)?;
        if bytes.len() != artifact.byte_len {
            return Err(ApiError::PromptContextArtifactSizeMismatch {
                artifact_id: artifact.artifact_id.clone(),
                expected_bytes: artifact.byte_len,
                actual_bytes: bytes.len(),
            });
        }

        let actual_digest = blake3::hash(&bytes).to_hex().to_string();
        if actual_digest != artifact.digest {
            return Err(ApiError::PromptContextArtifactDigestMismatch {
                artifact_id: artifact.artifact_id.clone(),
                expected_digest: artifact.digest.clone(),
                actual_digest,
            });
        }

        Ok(bytes)
    }

    pub fn read_by_artifact_id_verified(&self, artifact_id: &str) -> Result<Vec<u8>, ApiError> {
        if artifact_id.len() != 64 || !artifact_id.chars().all(|ch| ch.is_ascii_hexdigit()) {
            return Err(ApiError::ConfigError(format!(
                "artifact_id '{}' must be a 64 character hex digest",
                artifact_id
            )));
        }

        let path = self.artifact_path_for_digest(artifact_id);
        if !path.exists() {
            return Err(ApiError::PromptContextArtifactNotFound {
                artifact_id: artifact_id.to_string(),
            });
        }

        let bytes = fs::read(&path).map_err(StorageError::from)?;
        let actual_digest = blake3::hash(&bytes).to_hex().to_string();
        if actual_digest != artifact_id {
            return Err(ApiError::PromptContextArtifactDigestMismatch {
                artifact_id: artifact_id.to_string(),
                expected_digest: artifact_id.to_string(),
                actual_digest,
            });
        }

        Ok(bytes)
    }

    pub fn count_older_than(&self, cutoff_unix_secs: u64) -> Result<u64, ApiError> {
        let mut count = 0u64;
        for entry in WalkDir::new(&self.root)
            .follow_links(false)
            .into_iter()
            .filter_map(Result::ok)
        {
            if !entry.file_type().is_file() || !is_blob_file(entry.path()) {
                continue;
            }
            let modified = fs::metadata(entry.path())
                .map_err(StorageError::from)?
                .modified()
                .map_err(StorageError::from)?;
            let modified_secs = modified
                .duration_since(UNIX_EPOCH)
                .map_err(|err| {
                    StorageError::IoError(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Invalid file modification timestamp: {}", err),
                    ))
                })?
                .as_secs();
            if modified_secs <= cutoff_unix_secs {
                count += 1;
            }
        }
        Ok(count)
    }

    pub fn purge_older_than(&self, cutoff_unix_secs: u64) -> Result<u64, ApiError> {
        let mut purged = 0u64;
        for entry in WalkDir::new(&self.root)
            .follow_links(false)
            .into_iter()
            .filter_map(Result::ok)
        {
            if !entry.file_type().is_file() || !is_blob_file(entry.path()) {
                continue;
            }
            let modified = fs::metadata(entry.path())
                .map_err(StorageError::from)?
                .modified()
                .map_err(StorageError::from)?;
            let modified_secs = modified
                .duration_since(UNIX_EPOCH)
                .map_err(|err| {
                    StorageError::IoError(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Invalid file modification timestamp: {}", err),
                    ))
                })?
                .as_secs();
            if modified_secs <= cutoff_unix_secs {
                fs::remove_file(entry.path()).map_err(StorageError::from)?;
                purged += 1;
            }
        }
        self.prune_empty_shards()?;
        Ok(purged)
    }

    fn artifact_path_for_digest(&self, digest: &str) -> PathBuf {
        if digest.len() < 4 {
            return self.root.join("invalid").join(format!("{}.blob", digest));
        }
        let prefix1 = &digest[0..2];
        let prefix2 = &digest[2..4];
        self.root
            .join(prefix1)
            .join(prefix2)
            .join(format!("{}.blob", digest))
    }

    fn prune_empty_shards(&self) -> Result<(), ApiError> {
        let mut dirs: Vec<PathBuf> = WalkDir::new(&self.root)
            .follow_links(false)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_dir())
            .map(|entry| entry.into_path())
            .collect();
        dirs.sort_by_key(|path| std::cmp::Reverse(path.components().count()));
        for dir in dirs {
            if dir == self.root {
                continue;
            }
            let mut entries = fs::read_dir(&dir).map_err(StorageError::from)?;
            if entries.next().is_none() {
                fs::remove_dir(&dir).map_err(StorageError::from)?;
            }
        }
        Ok(())
    }
}

fn is_blob_file(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .map(|value| value.eq_ignore_ascii_case("blob"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prompt_context::contracts::PromptContextArtifactKind;
    use std::time::{Duration, UNIX_EPOCH};
    use tempfile::TempDir;

    #[test]
    fn write_is_content_addressed_and_deduped() {
        let temp = TempDir::new().unwrap();
        let storage = PromptContextArtifactStorage::new(temp.path()).unwrap();

        let left = storage
            .write_utf8(PromptContextArtifactKind::RenderedPrompt, "same-bytes")
            .unwrap();
        let right = storage
            .write_utf8(PromptContextArtifactKind::RenderedPrompt, "same-bytes")
            .unwrap();

        assert_eq!(left.artifact_id, right.artifact_id);
        assert_eq!(left.digest, right.digest);
    }

    #[test]
    fn read_verified_rejects_digest_mismatch() {
        let temp = TempDir::new().unwrap();
        let storage = PromptContextArtifactStorage::new(temp.path()).unwrap();
        let mut artifact = storage
            .write_utf8(PromptContextArtifactKind::RenderedPrompt, "payload")
            .unwrap();
        artifact.digest = "0".repeat(64);

        let err = storage.read_verified(&artifact).unwrap_err();
        assert!(matches!(
            err,
            ApiError::PromptContextArtifactDigestMismatch { .. }
        ));
    }

    #[test]
    fn read_by_artifact_id_verified_reads_and_verifies() {
        let temp = TempDir::new().unwrap();
        let storage = PromptContextArtifactStorage::new(temp.path()).unwrap();
        let artifact = storage
            .write_utf8(PromptContextArtifactKind::RenderedPrompt, "payload")
            .unwrap();

        let read = storage
            .read_by_artifact_id_verified(&artifact.artifact_id)
            .unwrap();
        assert_eq!(read, b"payload");
    }

    #[test]
    fn compact_counts_and_purges_old_artifacts() {
        let temp = TempDir::new().unwrap();
        let storage = PromptContextArtifactStorage::new(temp.path()).unwrap();
        let old_artifact = storage
            .write_utf8(PromptContextArtifactKind::RenderedPrompt, "old")
            .unwrap();
        std::thread::sleep(Duration::from_millis(2100));
        let new_artifact = storage
            .write_utf8(PromptContextArtifactKind::RenderedPrompt, "new")
            .unwrap();

        let old_path = storage.artifact_path_for_digest(&old_artifact.artifact_id);
        let new_path = storage.artifact_path_for_digest(&new_artifact.artifact_id);

        let old_ts = old_path
            .metadata()
            .unwrap()
            .modified()
            .unwrap()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let new_ts = new_path
            .metadata()
            .unwrap()
            .modified()
            .unwrap()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert!(new_ts > old_ts);

        let count = storage.count_older_than(old_ts).unwrap();
        assert_eq!(count, 1);

        let purged = storage.purge_older_than(old_ts).unwrap();
        assert_eq!(purged, 1);
        assert!(!old_path.exists());
        assert!(new_path.exists());
    }
}
