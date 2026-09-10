//! Canonical external runtime-state path validation shared by configured and XDG roots.

use crate::error::ApiError;
use std::path::{Component, Path, PathBuf};

pub(crate) fn validate_external_product_root(
    candidate: PathBuf,
    workspace: &Path,
    required_root: Option<&Path>,
    replacement: Option<PathBuf>,
) -> Result<PathBuf, ApiError> {
    let candidate = normalize_absolute(&candidate)?;
    let workspace = resolve_existing_ancestor(&normalize_absolute(workspace)?)?;
    let resolved = resolve_existing_ancestor(&candidate)?;
    if path_is_within(&candidate, &workspace) || path_is_within(&resolved, &workspace) {
        return Err(ApiError::ProductRootInsideWorkspace {
            old_path: candidate,
            new_path: replacement.unwrap_or(resolved),
        });
    }
    if required_root.is_some_and(|root| !path_is_within(&resolved, root)) {
        return Err(ApiError::ConfigError(format!(
            "Relative product runtime root resolves outside workspace XDG data root: {}",
            resolved.display()
        )));
    }
    Ok(resolved)
}

pub(crate) fn normalize_absolute(path: &Path) -> Result<PathBuf, ApiError> {
    if !path.is_absolute() {
        return Err(ApiError::ConfigError(format!(
            "Product runtime path must resolve to an absolute path: {}",
            path.display()
        )));
    }

    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(segment) => normalized.push(segment),
        }
    }
    Ok(normalized)
}

pub(crate) fn resolve_existing_ancestor(path: &Path) -> Result<PathBuf, ApiError> {
    for ancestor in path.ancestors() {
        if ancestor.exists() {
            let resolved = ancestor.canonicalize().map_err(|error| {
                ApiError::ConfigError(format!(
                    "Failed to resolve product runtime path {}: {error}",
                    path.display()
                ))
            })?;
            let suffix = path.strip_prefix(ancestor).map_err(|error| {
                ApiError::ConfigError(format!(
                    "Failed to normalize product runtime path {}: {error}",
                    path.display()
                ))
            })?;
            return Ok(if suffix.as_os_str().is_empty() {
                resolved
            } else {
                resolved.join(suffix)
            });
        }
    }

    Err(ApiError::ConfigError(format!(
        "Product runtime path has no resolvable ancestor: {}",
        path.display()
    )))
}

pub(crate) fn path_is_within(path: &Path, root: &Path) -> bool {
    path == root || path.starts_with(root)
}
