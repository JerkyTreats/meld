//! Durable task artifact repository key encoding.

use crate::task::artifact_repo::error::TaskArtifactRepoError;

pub(super) fn artifact_key(repo_id: &str, artifact_id: &str) -> Vec<u8> {
    let mut key = repo_prefix(repo_id);
    key.extend_from_slice(artifact_id.as_bytes());
    key
}

pub(super) fn artifact_id_from_key(
    repo_id: &str,
    key: &[u8],
) -> Result<String, TaskArtifactRepoError> {
    let tail = repo_key_tail(repo_id, key)?;
    String::from_utf8(tail.to_vec()).map_err(|error| {
        TaskArtifactRepoError::Decode(format!("artifact key UTF-8 failed: {error}"))
    })
}

pub(super) fn link_key(repo_id: &str, sequence: u64) -> Vec<u8> {
    let mut key = repo_prefix(repo_id);
    key.extend_from_slice(&sequence.to_be_bytes());
    key
}

pub(super) fn link_sequence_from_key(
    repo_id: &str,
    key: &[u8],
) -> Result<u64, TaskArtifactRepoError> {
    let tail = repo_key_tail(repo_id, key)?;
    let bytes: [u8; 8] = tail
        .try_into()
        .map_err(|_| TaskArtifactRepoError::Decode("link sequence key length mismatch".into()))?;
    Ok(u64::from_be_bytes(bytes))
}

pub(super) fn repo_meta_key(repo_id: &str) -> Vec<u8> {
    repo_prefix(repo_id)
}

pub(super) fn repo_prefix(repo_id: &str) -> Vec<u8> {
    let repo_bytes = repo_id.as_bytes();
    let mut key = Vec::with_capacity(4 + repo_bytes.len());
    key.extend_from_slice(&(repo_bytes.len() as u32).to_be_bytes());
    key.extend_from_slice(repo_bytes);
    key
}

fn repo_key_tail<'a>(repo_id: &str, key: &'a [u8]) -> Result<&'a [u8], TaskArtifactRepoError> {
    if key.len() < 4 {
        return Err(TaskArtifactRepoError::Decode(
            "repo key length prefix missing".into(),
        ));
    }
    let length_bytes: [u8; 4] = key[0..4]
        .try_into()
        .map_err(|_| TaskArtifactRepoError::Decode("repo key length prefix invalid".into()))?;
    let repo_len = u32::from_be_bytes(length_bytes) as usize;
    let repo_start = 4;
    let repo_end = repo_start + repo_len;
    if key.len() < repo_end {
        return Err(TaskArtifactRepoError::Decode(
            "repo key length prefix exceeds key length".into(),
        ));
    }
    let key_repo = std::str::from_utf8(&key[repo_start..repo_end]).map_err(|error| {
        TaskArtifactRepoError::Decode(format!("repo key UTF-8 failed: {error}"))
    })?;
    if key_repo != repo_id {
        return Err(TaskArtifactRepoError::Decode(
            "repo key id does not match requested repo".into(),
        ));
    }
    Ok(&key[repo_end..])
}
