//! Filesystem CAS storage for prompt context artifacts.

use crate::error::{ApiError, StorageError};
use crate::prompt_context::contracts::{PromptContextArtifactKind, PromptContextArtifactRef};
use std::fs;
use std::path::{Path, PathBuf};

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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prompt_context::contracts::PromptContextArtifactKind;
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
}
