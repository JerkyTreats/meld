//! Read-only Docs observations over exact captured source and README bytes.
//! Observed claims are assertions found in a README, not judgments of correctness.

use super::capability::{DirectoryEvidence, DocsEvidenceBundle};
use super::claim_validation::ReadmeClaim;
use super::scope::DocsScopePolicy;
use crate::error::ApiError;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};

const MAX_DIRECTORIES: usize = 64;
const MAX_FILE_BYTES: usize = 8 * 1024;
pub(crate) const MAX_DIRECTORY_EVIDENCE_BYTES: usize = 24 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocsScopeObservation {
    pub revision_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<DocsScopePolicy>,
    pub sources: Vec<ObservedSource>,
    pub readmes: Vec<ObservedReadme>,
    /// Entries outside the inspected textual scope, with their selection reason.
    pub exclusions: Vec<ObservationExclusion>,
    /// Captured sources whose complete text is absent from the evidence bundle.
    pub coverage_gaps: Vec<ObservationExclusion>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservedSource {
    pub path: String,
    pub content_hash: String,
    pub byte_length: usize,
    /// Complete captured text when available; historical hash-only observations remain explicit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservationExclusion {
    pub path: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservedReadme {
    pub path: String,
    pub state: ObservedReadmeState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "disposition", rename_all = "snake_case")]
pub enum ObservedReadmeState {
    Present {
        content: String,
        content_hash: String,
        claims: Vec<ReadmeClaim>,
    },
    Missing,
    Unavailable {
        reason: String,
    },
}

fn io_error(error: std::io::Error) -> ApiError {
    ApiError::StorageError(crate::error::StorageError::IoError(error))
}

pub fn inspect_scope_selected(
    root: &Path,
    scope: &DocsScopePolicy,
) -> Result<DocsEvidenceBundle, ApiError> {
    scope.validate()?;
    inspect_scope_with_selected_read(root, scope, |path| std::fs::read(path).map_err(io_error))
}

#[cfg(test)]
pub fn inspect_scope(root: &Path) -> Result<DocsEvidenceBundle, ApiError> {
    inspect_scope_selected(
        root,
        super::claim_observation::test_support::policy()
            .semantics()?
            .scope()?,
    )
}

#[cfg(test)]
fn inspect_scope_with_read(
    root: &Path,
    read: impl FnMut(&Path) -> Result<Vec<u8>, ApiError>,
) -> Result<DocsEvidenceBundle, ApiError> {
    inspect_scope_with_selected_read(
        root,
        super::claim_observation::test_support::policy()
            .semantics()?
            .scope()?,
        read,
    )
}

pub(crate) fn validate_selected_scope(
    policy: &super::claim_validation::DocsClaimPolicy,
    bundle: &DocsEvidenceBundle,
) -> Result<(), ApiError> {
    validate_observation(bundle)?;
    if bundle
        .observation
        .as_ref()
        .and_then(|capture| capture.scope.as_ref())
        != Some(policy.semantics()?.scope()?)
    {
        return Err(ApiError::ConfigError(
            "Docs capture belongs to another scope selection".into(),
        ));
    }
    Ok(())
}

/// Stored captures without scope use the original document naming contract.
/// This reader preserves their identities; new capture and judgment require selection.
pub(crate) fn document_path(bundle: &DocsEvidenceBundle, directory: &str) -> String {
    if let Some(scope) = bundle
        .observation
        .as_ref()
        .and_then(|capture| capture.scope.as_ref())
    {
        scope.document_path(directory)
    } else if directory == "." {
        "README.md".into()
    } else {
        format!("{directory}/README.md")
    }
}

/// Verify captured bytes and their complete native observation identity without rereading files.
pub fn validate_observation(bundle: &DocsEvidenceBundle) -> Result<(), ApiError> {
    let observed = bundle
        .observation
        .as_ref()
        .ok_or_else(|| ApiError::ConfigError("Docs evidence has no owner observation".into()))?;
    for source in &observed.sources {
        if let Some(text) = &source.text {
            if text.len() != source.byte_length
                || blake3::hash(text.as_bytes()).to_hex().as_str() != source.content_hash
            {
                return Err(ApiError::ConfigError(
                    "Docs source observation disagrees with its captured bytes".into(),
                ));
            }
        }
    }
    for readme in &observed.readmes {
        if let ObservedReadmeState::Present {
            content,
            content_hash,
            claims,
        } = &readme.state
        {
            if blake3::hash(content.as_bytes()).to_hex().as_str() != content_hash
                || *claims != super::claim_validation::extract_claims(&readme.path, content)
            {
                return Err(ApiError::ConfigError(
                    "Docs README observation disagrees with its captured bytes".into(),
                ));
            }
        }
    }
    let mut seed = serde_json::to_vec(&(
        &bundle.source_fingerprint,
        &bundle.directories,
        &observed.sources,
        &observed.readmes,
        &observed.exclusions,
        &observed.coverage_gaps,
    ))
    .map_err(|error| ApiError::ConfigError(error.to_string()))?;
    if let Some(scope) = &observed.scope {
        scope.validate()?;
        seed.extend(
            serde_json::to_vec(scope).map_err(|error| ApiError::ConfigError(error.to_string()))?,
        );
    }
    if observed.revision_id != format!("docs-observation::{}", blake3::hash(&seed).to_hex()) {
        return Err(ApiError::ConfigError(
            "Docs observation identity is invalid".into(),
        ));
    }
    Ok(())
}

fn inspect_scope_with_selected_read(
    root: &Path,
    scope: &DocsScopePolicy,
    mut read: impl FnMut(&Path) -> Result<Vec<u8>, ApiError>,
) -> Result<DocsEvidenceBundle, ApiError> {
    let root = root.canonicalize().map_err(|error| {
        ApiError::ConfigError(format!(
            "docs target '{}' is unavailable: {error}",
            root.display()
        ))
    })?;
    let mut direct_files = BTreeMap::<PathBuf, Vec<PathBuf>>::new();
    let mut children = BTreeMap::<PathBuf, BTreeSet<PathBuf>>::new();
    let mut source_hasher = blake3::Hasher::new();
    let mut captured = BTreeMap::<PathBuf, Vec<u8>>::new();
    let mut source_records = BTreeMap::new();
    let mut coverage_gaps = Vec::new();
    let mut excluded = Vec::new();
    let mut excluded_directories = Vec::new();

    for entry in WalkDir::new(&root)
        .follow_links(false)
        .sort_by_file_name()
        .into_iter()
        .filter_entry(|entry| {
            let included = include_entry(entry, scope);
            if !included {
                excluded_directories.push(ObservationExclusion {
                    path: relative_display(
                        entry.path().strip_prefix(&root).unwrap_or(entry.path()),
                    ),
                    reason: "directory_policy".into(),
                });
            }
            included
        })
    {
        let entry = entry.map_err(|error| ApiError::ConfigError(error.to_string()))?;
        let path = entry.path();
        if entry.file_type().is_dir() {
            if scope.excludes_document(path) {
                excluded.push(ObservationExclusion {
                    path: relative_display(path.strip_prefix(&root).unwrap_or(path)),
                    reason: "README_is_directory".into(),
                });
            }
            direct_files.entry(path.to_path_buf()).or_default();
            if let Some(parent) = path.parent().filter(|parent| *parent != path) {
                if path != root && parent.starts_with(&root) {
                    children
                        .entry(parent.to_path_buf())
                        .or_default()
                        .insert(path.to_path_buf());
                }
            }
            continue;
        }
        if !entry.file_type().is_file() {
            excluded.push(ObservationExclusion {
                path: relative_display(path.strip_prefix(&root).unwrap_or(path)),
                reason: "non_regular_file".into(),
            });
            continue;
        }
        let Some(parent) = path.parent() else {
            continue;
        };
        let bytes = read(path)?;
        if scope.excludes_document(path) {
            captured.insert(path.to_path_buf(), bytes);
            continue;
        }
        if bytes.contains(&0) {
            excluded.push(ObservationExclusion {
                path: relative_display(path.strip_prefix(&root).unwrap_or(path)),
                reason: "binary_source".into(),
            });
            continue;
        }
        let relative = path.strip_prefix(&root).unwrap_or(path);
        source_hasher.update(relative.to_string_lossy().as_bytes());
        source_hasher.update(&(bytes.len() as u64).to_le_bytes());
        source_hasher.update(blake3::hash(&bytes).as_bytes());
        let relative_path = relative_display(relative);
        source_records.insert(
            path.to_path_buf(),
            ObservedSource {
                path: relative_path.clone(),
                content_hash: blake3::hash(&bytes).to_hex().to_string(),
                byte_length: bytes.len(),
                text: if bytes.len() <= MAX_FILE_BYTES {
                    std::str::from_utf8(&bytes).ok().map(str::to_owned)
                } else {
                    None
                },
            },
        );
        if bytes.len() > MAX_FILE_BYTES || std::str::from_utf8(&bytes).is_err() {
            coverage_gaps.push(ObservationExclusion {
                path: relative_path,
                reason: "source_text_not_fully_represented".into(),
            });
        }
        captured.insert(
            path.to_path_buf(),
            bytes[..bytes.len().min(MAX_FILE_BYTES)].to_vec(),
        );
        direct_files
            .entry(parent.to_path_buf())
            .or_default()
            .push(path.to_path_buf());
    }

    let mut meaningful = BTreeSet::new();
    for directory in direct_files
        .iter()
        .filter(|(_, files)| !files.is_empty())
        .map(|(directory, _)| directory)
    {
        let mut cursor = Some(directory.as_path());
        while let Some(path) = cursor {
            if !path.starts_with(&root) {
                break;
            }
            meaningful.insert(path.to_path_buf());
            if path == root {
                break;
            }
            cursor = path.parent();
        }
    }
    if meaningful.len() > MAX_DIRECTORIES {
        return Err(ApiError::ConfigError(format!(
            "docs scope contains {} meaningful directories, exceeding the bound of {MAX_DIRECTORIES}",
            meaningful.len()
        )));
    }

    for directory in direct_files
        .keys()
        .filter(|directory| !meaningful.contains(*directory))
    {
        excluded.push(ObservationExclusion {
            path: relative_display(directory.strip_prefix(&root).unwrap_or(directory)),
            reason: "no_managed_source".into(),
        });
    }

    let mut directories = meaningful.iter().cloned().collect::<Vec<_>>();
    directories.sort_by(|left, right| {
        right
            .components()
            .count()
            .cmp(&left.components().count())
            .then_with(|| left.cmp(right))
    });
    let mut evidence = Vec::new();
    let mut sources = Vec::new();
    let mut readmes = Vec::new();
    for directory in directories {
        let mut files = direct_files.remove(&directory).unwrap_or_default();
        files.sort();
        let mut rendered = String::new();
        let mut names = Vec::new();
        for file in files {
            let relative = file.strip_prefix(&root).unwrap_or(&file);
            names.push(relative.to_string_lossy().to_string());
            let bytes = captured
                .get(&file)
                .expect("captured source enumerated once");
            let path = relative_display(relative);
            sources.push(
                source_records
                    .get(&file)
                    .expect("captured source identity")
                    .clone(),
            );
            let text = String::from_utf8_lossy(bytes);
            let section = format!("\n--- {} ---\n{}\n", relative.display(), text);
            if rendered.len() + section.len() > MAX_DIRECTORY_EVIDENCE_BYTES {
                coverage_gaps.push(ObservationExclusion {
                    path,
                    reason: "directory_evidence_budget".into(),
                });
                continue;
            }
            rendered.push_str(&section);
        }
        let readme_path = directory.join(&scope.document_name);
        let path = relative_display(readme_path.strip_prefix(&root).unwrap_or(&readme_path));
        let readme = match captured.get(&readme_path) {
            Some(bytes) => match std::str::from_utf8(bytes) {
                Ok(content) => ObservedReadmeState::Present {
                    content: content.into(),
                    content_hash: blake3::hash(bytes).to_hex().to_string(),
                    claims: super::claim_validation::extract_claims(&path, content),
                },
                Err(_) => ObservedReadmeState::Unavailable {
                    reason: "README is not UTF-8".into(),
                },
            },
            None if excluded.iter().any(|entry| entry.path == path) => {
                ObservedReadmeState::Unavailable {
                    reason: "README is not a regular file".into(),
                }
            }
            None => ObservedReadmeState::Missing,
        };
        readmes.push(ObservedReadme {
            path,
            state: readme,
        });
        let child_directories = children
            .get(&directory)
            .into_iter()
            .flatten()
            .filter(|child| meaningful.contains(*child))
            .filter_map(|child| child.strip_prefix(&root).ok())
            .map(relative_display)
            .collect::<Vec<_>>();
        evidence.push(DirectoryEvidence {
            path: relative_display(directory.strip_prefix(&root).unwrap_or(&directory)),
            direct_files: names,
            child_directories,
            evidence: rendered,
        });
    }
    let source_fingerprint = source_hasher.finalize().to_hex().to_string();
    excluded.extend(excluded_directories);
    excluded.sort_by(|a, b| a.path.cmp(&b.path));
    sources.sort_by(|a, b| a.path.cmp(&b.path));
    readmes.sort_by(|a, b| a.path.cmp(&b.path));
    let mut seed = serde_json::to_vec(&(
        &source_fingerprint,
        &evidence,
        &sources,
        &readmes,
        &excluded,
        &coverage_gaps,
    ))
    .map_err(|error| ApiError::ConfigError(error.to_string()))?;
    seed.extend(
        serde_json::to_vec(scope).map_err(|error| ApiError::ConfigError(error.to_string()))?,
    );
    Ok(DocsEvidenceBundle {
        source_fingerprint,
        directories: evidence,
        observation: Some(DocsScopeObservation {
            revision_id: format!("docs-observation::{}", blake3::hash(&seed).to_hex()),
            scope: Some(scope.clone()),
            sources,
            readmes,
            exclusions: excluded,
            coverage_gaps,
        }),
    })
}

fn include_entry(entry: &DirEntry, scope: &DocsScopePolicy) -> bool {
    entry.depth() == 0
        || !entry.file_type().is_dir()
        || scope.includes_directory(&entry.file_name().to_string_lossy())
}

fn relative_display(path: &Path) -> String {
    if path.as_os_str().is_empty() {
        ".".to_string()
    } else {
        path.to_string_lossy().replace('\\', "/")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observation_retains_current_readme_claims_without_repair_or_publication() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("lib.rs"), "pub fn run() {}\n").unwrap();
        let missing = inspect_scope(root.path()).unwrap();
        assert!(matches!(
            missing.observation.as_ref().unwrap().readmes[0].state,
            ObservedReadmeState::Missing
        ));
        let content = "# Tool\n\n`run` starts the tool.\n";
        std::fs::write(root.path().join("README.md"), content).unwrap();
        let observed = inspect_scope(root.path()).unwrap();
        assert_eq!(observed, inspect_scope(root.path()).unwrap());
        assert_eq!(observed.source_fingerprint, missing.source_fingerprint);
        let observation = observed.observation.as_ref().unwrap();
        assert_ne!(
            observation.revision_id,
            missing.observation.as_ref().unwrap().revision_id
        );
        let ObservedReadmeState::Present {
            content: actual,
            content_hash,
            claims,
        } = &observation.readmes[0].state
        else {
            panic!("README was not observed")
        };
        assert_eq!(actual, content);
        assert_eq!(
            content_hash,
            &blake3::hash(content.as_bytes()).to_hex().to_string()
        );
        assert_eq!(
            claims,
            &super::super::claim_validation::extract_claims("README.md", content)
        );
        assert_eq!(
            std::fs::read_to_string(root.path().join("README.md")).unwrap(),
            content
        );
        std::fs::remove_file(root.path().join("README.md")).unwrap();
        assert_eq!(inspect_scope(root.path()).unwrap(), missing);
    }

    #[test]
    fn source_hash_and_evidence_use_one_capture_despite_a_concurrent_change() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("lib.rs");
        std::fs::write(&path, "original source\n").unwrap();
        let mut reads = 0;
        let observed = inspect_scope_with_read(root.path(), |path| {
            reads += 1;
            let bytes = std::fs::read(path).map_err(io_error)?;
            std::fs::write(path, "successor source\n").map_err(io_error)?;
            Ok(bytes)
        })
        .unwrap();
        assert_eq!(reads, 1);
        assert!(observed.directories[0].evidence.contains("original source"));
        assert!(!observed.directories[0]
            .evidence
            .contains("successor source"));
        assert_eq!(
            observed.observation.as_ref().unwrap().sources[0].content_hash,
            blake3::hash(b"original source\n").to_hex().to_string()
        );
        assert_ne!(
            observed.source_fingerprint,
            inspect_scope(root.path()).unwrap().source_fingerprint
        );
    }

    #[test]
    fn bounded_evidence_keeps_full_inventory_and_names_every_coverage_gap() {
        let root = tempfile::tempdir().unwrap();
        for index in 0..5 {
            std::fs::write(
                root.path().join(format!("source-{index}.rs")),
                "x".repeat(MAX_FILE_BYTES + 10),
            )
            .unwrap();
        }
        let observed = inspect_scope(root.path()).unwrap();
        assert_eq!(observed.directories[0].direct_files.len(), 5);
        let observation = observed.observation.unwrap();
        assert_eq!(observation.sources.len(), 5);
        assert!(observation
            .sources
            .iter()
            .all(|source| source.byte_length == MAX_FILE_BYTES + 10));
        for source in &observation.sources {
            assert!(observation
                .coverage_gaps
                .iter()
                .any(|gap| gap.path == source.path
                    && gap.reason == "source_text_not_fully_represented"));
        }
        assert!(observation
            .coverage_gaps
            .iter()
            .any(|gap| gap.reason == "directory_evidence_budget"));
    }

    #[test]
    fn nonregular_readme_is_unavailable_and_never_reported_absent() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("lib.rs"), "source\n").unwrap();
        std::fs::create_dir(root.path().join("README.md")).unwrap();
        let observed = inspect_scope(root.path()).unwrap();
        assert!(matches!(
            observed.observation.unwrap().readmes[0].state,
            ObservedReadmeState::Unavailable { .. }
        ));
        let historical: DocsEvidenceBundle = serde_json::from_value(serde_json::json!({
            "source_fingerprint": "historical", "directories": []
        }))
        .unwrap();
        assert!(historical.observation.is_none());
    }
}
