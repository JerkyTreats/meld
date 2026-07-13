use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use super::contracts::{
    ActivationLoadError, ActivationSource, DocsFreshnessActivationDocument,
    ValidatedDocsFreshnessActivation, MAX_ACTIVATION_SOURCE_BYTES,
};
use super::normalization::normalize_and_validate;

pub(crate) fn load(
    workspace_root: &Path,
    activation_path: &Path,
) -> Result<ValidatedDocsFreshnessActivation, ActivationLoadError> {
    let canonical_workspace_root = dunce::canonicalize(workspace_root)
        .map_err(|error| ActivationLoadError::Workspace(error.to_string()))?;
    if !canonical_workspace_root.is_dir() {
        return Err(ActivationLoadError::Workspace(
            "canonical workspace root must be a directory".to_string(),
        ));
    }
    let source_candidate = resolve_source_path(&canonical_workspace_root, activation_path);
    let canonical_source_path = dunce::canonicalize(&source_candidate)
        .map_err(|error| ActivationLoadError::Source(error.to_string()))?;
    if canonical_source_path
        .extension()
        .and_then(|value| value.to_str())
        != Some("toml")
    {
        return Err(ActivationLoadError::Source(
            "source must use the .toml extension".to_string(),
        ));
    }

    let file = File::open(&canonical_source_path)
        .map_err(|error| ActivationLoadError::Source(error.to_string()))?;
    let metadata = file
        .metadata()
        .map_err(|error| ActivationLoadError::Source(error.to_string()))?;
    if !metadata.is_file() {
        return Err(ActivationLoadError::Source(
            "opened source must be a regular file".to_string(),
        ));
    }
    let mut bytes = Vec::with_capacity(MAX_ACTIVATION_SOURCE_BYTES.min(metadata.len() as usize));
    file.take((MAX_ACTIVATION_SOURCE_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|error| ActivationLoadError::Source(error.to_string()))?;
    if bytes.len() > MAX_ACTIVATION_SOURCE_BYTES {
        return Err(ActivationLoadError::SourceTooLarge);
    }
    let source_text = std::str::from_utf8(&bytes)
        .map_err(|error| ActivationLoadError::Parse(error.to_string()))?;
    let document: DocsFreshnessActivationDocument = toml::from_str(source_text)
        .map_err(|error| ActivationLoadError::Parse(error.to_string()))?;
    let normalized = normalize_and_validate(document, &canonical_workspace_root)?;
    Ok(ValidatedDocsFreshnessActivation {
        source: ActivationSource {
            canonical_path: canonical_source_path,
            byte_count: bytes.len(),
        },
        canonical_workspace_root,
        resolved_target: normalized.resolved_target,
        document: normalized.document,
        activation_hash: normalized.activation_hash,
        runtime_inputs: normalized.runtime_inputs,
    })
}

fn resolve_source_path(workspace_root: &Path, activation_path: &Path) -> PathBuf {
    if activation_path.is_absolute() {
        activation_path.to_path_buf()
    } else {
        workspace_root.join(activation_path)
    }
}
