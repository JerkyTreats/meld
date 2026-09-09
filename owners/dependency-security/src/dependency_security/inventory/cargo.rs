//! Native Cargo inventory over an exact, frozen resolved graph.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::{Path, PathBuf};
use std::process::Stdio;

use serde::{Deserialize, Serialize};
use tokio::io::AsyncReadExt;

use crate::dependency_security::contracts::*;

/// Operational bounds are explicit acquisition inputs, never clean evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CargoInventoryLimits {
    pub maximum_bytes: usize,
    pub maximum_paths: usize,
    pub timeout_seconds: u64,
}

impl Default for CargoInventoryLimits {
    fn default() -> Self {
        Self {
            maximum_bytes: 16 * 1024 * 1024,
            maximum_paths: 100_000,
            timeout_seconds: 30,
        }
    }
}

impl CargoInventoryLimits {
    pub fn validate(&self) -> Result<(), String> {
        if self.maximum_bytes == 0
            || self.maximum_paths == 0
            || self.timeout_seconds == 0
            || self.maximum_bytes == usize::MAX
        {
            return Err("Cargo inventory acquisition bounds must be positive and finite".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Metadata {
    version: u32,
    workspace_root: PathBuf,
    workspace_members: Vec<String>,
    packages: Vec<Package>,
    resolve: Option<Resolve>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Package {
    id: String,
    name: String,
    version: String,
    source: Option<String>,
    manifest_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Resolve {
    nodes: Vec<Node>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Node {
    id: String,
    dependencies: Vec<String>,
    features: Vec<String>,
}

/// Read resolved Cargo evidence without updating the lockfile or using the network.
/// Repeated metadata and manifest capture refuse a moving source basis.
pub async fn observe(
    workspace: &Path,
    executable: &Path,
    subject: DependencySecuritySubjectV1,
    observed_at: u64,
    limits: &CargoInventoryLimits,
) -> Result<DependencyInventorySnapshotV1, String> {
    limits.validate()?;
    if !workspace.is_absolute() || !executable.is_absolute() {
        return Err(
            "Cargo inventory requires exact absolute workspace and executable bindings".into(),
        );
    }
    let workspace = workspace
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let root_manifest = workspace.join("Cargo.toml");
    let lockfile = workspace.join("Cargo.lock");
    let before_manifest = read_bounded(&root_manifest, limits.maximum_bytes)?;
    let before_lock = read_bounded(&lockfile, limits.maximum_bytes)?;
    let first = metadata(&workspace, executable, limits).await?;
    if first.version != 1
        || first
            .workspace_root
            .canonicalize()
            .map_err(|error| error.to_string())?
            != workspace
    {
        return Err("Cargo metadata names a different workspace or format".into());
    }
    let before_members = manifests(&first, limits.maximum_bytes)?;
    let second = metadata(&workspace, executable, limits).await?;
    let after_members = manifests(&second, limits.maximum_bytes)?;
    if first != second
        || before_members != after_members
        || before_manifest != read_bounded(&root_manifest, limits.maximum_bytes)?
        || before_lock != read_bounded(&lockfile, limits.maximum_bytes)?
    {
        return Err("Cargo inventory source changed during acquisition".into());
    }
    let (components, completeness) = components(
        &first,
        &before_members,
        limits.maximum_paths,
        limits.maximum_bytes,
    )?;
    DependencyInventorySnapshotV1::canonical(
        subject,
        content_hash(&(&first, &before_members, &before_manifest, &before_lock))?,
        blake3::hash(&before_manifest).to_hex().to_string(),
        blake3::hash(&before_lock).to_hex().to_string(),
        components,
        completeness,
        observed_at,
    )
}

pub(crate) fn read_bounded(path: &Path, maximum_bytes: usize) -> Result<Vec<u8>, String> {
    use std::io::Read;
    let file = std::fs::File::open(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    if !file
        .metadata()
        .map_err(|error| error.to_string())?
        .is_file()
    {
        return Err("inventory source must be a regular file".into());
    }
    let mut bytes = Vec::new();
    file.take(maximum_bytes as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() > maximum_bytes {
        return Err("inventory source exceeds its declared byte bound".into());
    }
    Ok(bytes)
}

fn manifests(
    metadata: &Metadata,
    maximum_bytes: usize,
) -> Result<BTreeMap<PathBuf, String>, String> {
    let mut remaining = maximum_bytes;
    let mut manifests = BTreeMap::new();
    for package in &metadata.packages {
        let path = package
            .manifest_path
            .canonicalize()
            .map_err(|error| error.to_string())?;
        if manifests.contains_key(&path) {
            continue;
        }
        let bytes = read_bounded(&path, remaining)?;
        remaining -= bytes.len();
        manifests.insert(path, blake3::hash(&bytes).to_hex().to_string());
    }
    Ok(manifests)
}

async fn metadata(
    workspace: &Path,
    executable: &Path,
    limits: &CargoInventoryLimits,
) -> Result<Metadata, String> {
    let mut child = tokio::process::Command::new(executable)
        .args([
            "metadata",
            "--format-version",
            "1",
            "--frozen",
            "--all-features",
            "--manifest-path",
        ])
        .arg(workspace.join("Cargo.toml"))
        .current_dir(workspace)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|error| format!("Cargo inventory could not start: {error}"))?;
    let mut stdout = child
        .stdout
        .take()
        .ok_or("Cargo stdout is absent")?
        .take(limits.maximum_bytes as u64 + 1);
    let mut stderr = child
        .stderr
        .take()
        .ok_or("Cargo stderr is absent")?
        .take(limits.maximum_bytes as u64 + 1);
    let mut output = Vec::new();
    let mut errors = Vec::new();
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(limits.timeout_seconds),
        async {
            tokio::join!(
                child.wait(),
                stdout.read_to_end(&mut output),
                stderr.read_to_end(&mut errors)
            )
        },
    )
    .await
    .map_err(|_| "Cargo inventory exceeded its declared time bound".to_string())?;
    let status = result.0.map_err(|error| error.to_string())?;
    result.1.map_err(|error| error.to_string())?;
    result.2.map_err(|error| error.to_string())?;
    if output.len() > limits.maximum_bytes || errors.len() > limits.maximum_bytes {
        return Err("Cargo inventory exceeded its declared output bound".into());
    }
    if !status.success() {
        return Err(format!(
            "Cargo inventory was not resolved: {}",
            String::from_utf8_lossy(&errors)
        ));
    }
    serde_json::from_slice(&output)
        .map_err(|error| format!("invalid Cargo inventory metadata: {error}"))
}

fn components(
    metadata: &Metadata,
    manifests: &BTreeMap<PathBuf, String>,
    maximum_paths: usize,
    maximum_bytes: usize,
) -> Result<(Vec<ResolvedDependencyComponentV1>, InventoryCompleteness), String> {
    let resolve = metadata
        .resolve
        .as_ref()
        .ok_or("Cargo inventory has no resolved graph")?;
    let packages: BTreeMap<_, _> = metadata
        .packages
        .iter()
        .map(|package| (&package.id, package))
        .collect();
    let nodes: BTreeMap<_, _> = resolve.nodes.iter().map(|node| (&node.id, node)).collect();
    if packages.len() != metadata.packages.len()
        || nodes.len() != resolve.nodes.len()
        || metadata.workspace_members.is_empty()
    {
        return Err("Cargo inventory contains duplicate identities or no workspace members".into());
    }
    let mut paths: BTreeMap<String, BTreeSet<Vec<String>>> = BTreeMap::new();
    let mut pending: VecDeque<_> = metadata
        .workspace_members
        .iter()
        .map(|id| vec![id.clone()])
        .collect();
    let mut path_bytes = pending.iter().flatten().map(String::len).sum::<usize>();
    if path_bytes > maximum_bytes || pending.len() > maximum_paths {
        return Err("Cargo workspace members exceed declared inventory bounds".into());
    }
    let mut count = 0;
    let mut reasons = BTreeSet::new();
    while let Some(path) = pending.pop_front() {
        if count == maximum_paths {
            reasons.insert("resolved dependency paths exceed the declared bound".to_string());
            break;
        }
        count += 1;
        let id = path.last().ok_or("empty dependency path")?;
        let node = nodes.get(id).ok_or("resolved dependency node is absent")?;
        if !packages.contains_key(id) {
            return Err("resolved dependency package is absent".into());
        }
        if !paths.entry(id.clone()).or_default().insert(path.clone()) {
            continue;
        }
        for dependency in &node.dependencies {
            if path.contains(dependency) {
                reasons.insert("resolved dependency graph contains a cycle".to_string());
                continue;
            }
            let child_bytes = path
                .iter()
                .map(String::len)
                .sum::<usize>()
                .saturating_add(dependency.len());
            if count + pending.len() >= maximum_paths
                || child_bytes > maximum_bytes.saturating_sub(path_bytes)
            {
                reasons.insert("resolved dependency paths exceed the declared bound".to_string());
                continue;
            }
            path_bytes += child_bytes;
            let mut child = path.clone();
            child.push(dependency.clone());
            pending.push_back(child);
        }
    }
    let mut components = Vec::new();
    for (id, paths) in paths {
        let package = packages[&id];
        let source_identity = match &package.source {
            Some(source) => source.clone(),
            None => {
                let path = package
                    .manifest_path
                    .canonicalize()
                    .map_err(|error| error.to_string())?;
                let hash = manifests
                    .get(&path)
                    .ok_or("local package manifest identity missing")?;
                format!("cargo-path:{}:{hash}", path.display())
            }
        };
        components.push(ResolvedDependencyComponentV1 {
            component_id: content_hash(&(&id, &source_identity))?,
            ecosystem: PackageEcosystem::Cargo,
            package_name: package.name.clone(),
            resolved_version: package.version.clone(),
            source_identity,
            dependency_paths: paths.into_iter().collect(),
        });
    }
    let completeness = if reasons.is_empty() {
        InventoryCompleteness::CompleteTransitive
    } else {
        InventoryCompleteness::Incomplete {
            reasons: reasons.into_iter().collect(),
        }
    };
    Ok((components, completeness))
}

#[cfg(test)]
mod tests {
    use super::*;
    use meld_events::DomainObjectRef;

    fn workspace() -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(
            root.path().join("Cargo.toml"),
            "[workspace]\nmembers = [\"app\", \"dep\"]\nresolver = \"2\"\n",
        )
        .unwrap();
        for name in ["app", "dep"] {
            std::fs::create_dir_all(root.path().join(name).join("src")).unwrap();
            let dependency = if name == "app" {
                "[dependencies]\ndep = { path = \"../dep\" }\n"
            } else {
                ""
            };
            std::fs::write(root.path().join(name).join("Cargo.toml"), format!("[package]\nname = \"{name}\"\nversion = \"1.0.0\"\nedition = \"2021\"\n{dependency}")).unwrap();
            std::fs::write(
                root.path().join(name).join("src/lib.rs"),
                "pub fn value() {}\n",
            )
            .unwrap();
        }
        let output = std::process::Command::new(env!("CARGO"))
            .args(["generate-lockfile", "--offline"])
            .current_dir(root.path())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        root
    }

    fn subject() -> DependencySecuritySubjectV1 {
        DependencySecuritySubjectV1 {
            subject: DomainObjectRef::new(
                "dependency-security",
                "cargo_dependency_graph",
                "test-workspace",
            )
            .unwrap(),
            ecosystem: PackageEcosystem::Cargo,
            inventory_scope: InventoryScopeV1 {
                manifest_ref: DomainObjectRef::new("workspace_fs", "file", "Cargo.toml").unwrap(),
                lockfile_ref: DomainObjectRef::new("workspace_fs", "file", "Cargo.lock").unwrap(),
                include_transitive: true,
            },
        }
    }

    #[tokio::test]
    async fn observes_real_cargo_graph_and_refuses_unresolved_source_change() {
        let root = workspace();
        let lock = std::fs::read(root.path().join("Cargo.lock")).unwrap();
        let inventory = observe(
            root.path(),
            Path::new(env!("CARGO")),
            subject(),
            42,
            &CargoInventoryLimits::default(),
        )
        .await
        .unwrap();
        inventory.validate().unwrap();
        assert_eq!(
            inventory.completeness,
            InventoryCompleteness::CompleteTransitive
        );
        assert_eq!(inventory.components.len(), 2);
        let dep = inventory
            .components
            .iter()
            .find(|component| component.package_name == "dep")
            .unwrap();
        assert!(dep.dependency_paths.iter().any(|path| path.len() == 2));
        assert_eq!(
            inventory.lockfile_content_hash,
            blake3::hash(&lock).to_hex().to_string()
        );
        assert_eq!(
            inventory,
            observe(
                root.path(),
                Path::new(env!("CARGO")),
                subject(),
                42,
                &CargoInventoryLimits::default()
            )
            .await
            .unwrap()
        );
        assert_eq!(std::fs::read(root.path().join("Cargo.lock")).unwrap(), lock);
        assert!(!root.path().join("target").exists());
        let manifest = root.path().join("dep/Cargo.toml");
        std::fs::write(
            &manifest,
            std::fs::read_to_string(&manifest)
                .unwrap()
                .replace("1.0.0", "1.1.0"),
        )
        .unwrap();
        assert!(observe(
            root.path(),
            Path::new(env!("CARGO")),
            subject(),
            43,
            &CargoInventoryLimits::default()
        )
        .await
        .is_err());
        assert_eq!(std::fs::read(root.path().join("Cargo.lock")).unwrap(), lock);
    }

    #[tokio::test]
    async fn bounded_paths_are_incomplete_evidence() {
        let root = workspace();
        let limits = CargoInventoryLimits {
            maximum_paths: 2,
            ..CargoInventoryLimits::default()
        };
        let inventory = observe(
            root.path(),
            Path::new(env!("CARGO")),
            subject(),
            42,
            &limits,
        )
        .await
        .unwrap();
        assert!(matches!(
            inventory.completeness,
            InventoryCompleteness::Incomplete { .. }
        ));
        let limits = CargoInventoryLimits {
            maximum_bytes: 8,
            ..CargoInventoryLimits::default()
        };
        assert!(observe(
            root.path(),
            Path::new(env!("CARGO")),
            subject(),
            42,
            &limits
        )
        .await
        .is_err());
    }
}
