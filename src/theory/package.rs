use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::contracts::{RoutedComponentSource, TheoryRouteId};
use super::error::{error, TheoryRouterError};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PdsPackageManifestV1 {
    pub schema_version: u32,
    pub package_id: String,
    pub package_version: String,
    pub description: Option<String>,
    #[serde(default)]
    pub imports: Vec<ExactPackageImport>,
    pub components: Vec<PdsComponentEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExactPackageImport {
    pub package_id: String,
    pub receipt_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PdsComponentEntry {
    pub component_id: String,
    pub owner_component_id: String,
    pub route: TheoryRouteId,
    pub component_schema_version: u32,
    pub content: ComponentContentRef,
    #[serde(default)]
    pub requires: Vec<ComponentRequirement>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ComponentContentRef {
    Embedded {
        canonical_bytes: Vec<u8>,
    },
    RelativeFile {
        path: String,
        content_hash: String,
    },
    PublishedExact {
        publisher: String,
        id: String,
        content_identity: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentRequirement {
    pub package: PackageRequirementTarget,
    pub component_id: String,
    pub required_route: Option<TheoryRouteId>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PackageRequirementTarget {
    SelfPackage,
    ExactImport {
        package_id: String,
        receipt_id: String,
    },
}

#[derive(Debug, Clone)]
pub struct MaterializedPdsPackage {
    pub manifest: PdsPackageManifestV1,
    pub package_content_hash: String,
    pub components: Vec<RoutedComponentSource>,
}

pub trait PublishedComponentResolver {
    fn resolve(
        &self,
        publisher: &str,
        id: &str,
        content_identity: &str,
    ) -> Result<Vec<u8>, TheoryRouterError>;
}

#[derive(Serialize)]
struct PackageIdentity<'a> {
    schema_version: u32,
    package_id: &'a str,
    package_version: &'a str,
    imports: &'a [ExactPackageImport],
    components: Vec<ComponentIdentity<'a>>,
}

#[derive(Serialize)]
struct ComponentIdentity<'a> {
    component_id: &'a str,
    owner_component_id: &'a str,
    route: &'a TheoryRouteId,
    component_schema_version: u32,
    source_identity: String,
    requirements: &'a [ComponentRequirement],
}

impl PdsPackageManifestV1 {
    /// Materialize bounded source bytes and derive the semantic package hash.
    pub fn materialize(
        &self,
        source_root: &Path,
    ) -> Result<MaterializedPdsPackage, TheoryRouterError> {
        self.materialize_with_published(source_root, None)
    }

    pub fn materialize_with_published(
        &self,
        source_root: &Path,
        published: Option<&dyn PublishedComponentResolver>,
    ) -> Result<MaterializedPdsPackage, TheoryRouterError> {
        if self.schema_version != 1 {
            return Err(error(
                "package_source_invalid",
                "unsupported manifest schema",
            ));
        }
        validate_name(&self.package_id)?;
        validate_name(&self.package_version)?;
        let mut manifest = self.clone();
        manifest.imports.sort_by(|left, right| {
            left.package_id
                .cmp(&right.package_id)
                .then(left.receipt_id.cmp(&right.receipt_id))
        });
        reject_duplicates(
            manifest.imports.iter().map(|item| item.package_id.as_str()),
            "import",
        )?;
        for import in &manifest.imports {
            validate_name(&import.package_id)?;
            validate_name(&import.receipt_id)?;
        }
        manifest
            .components
            .sort_by(|left, right| left.component_id.cmp(&right.component_id));
        reject_duplicates(
            manifest
                .components
                .iter()
                .map(|item| item.component_id.as_str()),
            "component",
        )?;

        let mut components = Vec::new();
        for entry in &mut manifest.components {
            validate_name(&entry.component_id)?;
            validate_name(&entry.owner_component_id)?;
            validate_route(&entry.route)?;
            if entry.component_schema_version == 0 {
                return Err(error(
                    "package_source_invalid",
                    "component schema version is zero",
                ));
            }
            entry.requires.sort();
            let canonical_bytes = materialize_content(source_root, &entry.content, published)?;
            let source_content_hash = blake3::hash(&canonical_bytes).to_hex().to_string();
            if let ComponentContentRef::RelativeFile { content_hash, .. } = &entry.content {
                if content_hash != &source_content_hash {
                    return Err(error(
                        "package_identity_mismatch",
                        format!("component '{}' source hash mismatch", entry.component_id),
                    ));
                }
            }
            components.push(RoutedComponentSource {
                component_id: entry.component_id.clone(),
                owner_component_id: entry.owner_component_id.clone(),
                route: entry.route.clone(),
                component_schema_version: entry.component_schema_version,
                source_content_hash,
                canonical_bytes: Arc::from(canonical_bytes),
            });
        }
        validate_self_requirements(&manifest)?;
        reject_requirement_cycles(&manifest)?;
        let package_content_hash = package_hash(&manifest, &components)?;
        Ok(MaterializedPdsPackage {
            manifest,
            package_content_hash,
            components,
        })
    }
}

fn materialize_content(
    root: &Path,
    content: &ComponentContentRef,
    published: Option<&dyn PublishedComponentResolver>,
) -> Result<Vec<u8>, TheoryRouterError> {
    match content {
        ComponentContentRef::Embedded { canonical_bytes } => Ok(canonical_bytes.clone()),
        ComponentContentRef::PublishedExact {
            publisher,
            id,
            content_identity,
        } => {
            validate_name(publisher)?;
            validate_name(id)?;
            validate_name(content_identity)?;
            let Some(resolver) = published else {
                return Err(error(
                    "package_source_invalid",
                    "published exact component has no compiled publisher resolver",
                ));
            };
            resolver.resolve(publisher, id, content_identity)
        }
        ComponentContentRef::RelativeFile { path, .. } => {
            let relative = Path::new(path);
            if relative.is_absolute()
                || relative.components().any(|part| {
                    matches!(
                        part,
                        Component::ParentDir | Component::RootDir | Component::Prefix(_)
                    )
                })
            {
                return Err(error(
                    "package_source_invalid",
                    "component path escapes source root",
                ));
            }
            let canonical_root = root
                .canonicalize()
                .map_err(|failure| error("package_source_invalid", failure.to_string()))?;
            let target = canonical_root.join(relative);
            let canonical_target = target
                .canonicalize()
                .map_err(|failure| error("package_source_invalid", failure.to_string()))?;
            if !canonical_target.starts_with(&canonical_root) {
                return Err(error(
                    "package_source_invalid",
                    "component symlink escapes source root",
                ));
            }
            let bytes = std::fs::read(&canonical_target)
                .map_err(|failure| error("package_source_invalid", failure.to_string()))?;
            if bytes.len() > 8 * 1024 * 1024 {
                return Err(error(
                    "package_source_invalid",
                    "component source exceeds size limit",
                ));
            }
            Ok(bytes)
        }
    }
}

fn package_hash(
    manifest: &PdsPackageManifestV1,
    components: &[RoutedComponentSource],
) -> Result<String, TheoryRouterError> {
    let sources: BTreeMap<&str, &str> = components
        .iter()
        .map(|item| {
            (
                item.component_id.as_str(),
                item.source_content_hash.as_str(),
            )
        })
        .collect();
    let identity = PackageIdentity {
        schema_version: manifest.schema_version,
        package_id: &manifest.package_id,
        package_version: &manifest.package_version,
        imports: &manifest.imports,
        components: manifest
            .components
            .iter()
            .map(|item| ComponentIdentity {
                component_id: &item.component_id,
                owner_component_id: &item.owner_component_id,
                route: &item.route,
                component_schema_version: item.component_schema_version,
                source_identity: match &item.content {
                    ComponentContentRef::PublishedExact {
                        publisher,
                        id,
                        content_identity,
                    } => format!("published::{publisher}::{id}::{content_identity}"),
                    _ => format!("source::{}", sources[&item.component_id.as_str()]),
                },
                requirements: &item.requires,
            })
            .collect(),
    };
    let bytes = serde_json::to_vec(&identity)
        .map_err(|failure| error("package_source_invalid", failure.to_string()))?;
    Ok(blake3::hash(&bytes).to_hex().to_string())
}

fn validate_self_requirements(manifest: &PdsPackageManifestV1) -> Result<(), TheoryRouterError> {
    let components: BTreeMap<&str, &TheoryRouteId> = manifest
        .components
        .iter()
        .map(|item| (item.component_id.as_str(), &item.route))
        .collect();
    for entry in &manifest.components {
        for requirement in &entry.requires {
            if matches!(requirement.package, PackageRequirementTarget::SelfPackage) {
                let Some(route) = components.get(requirement.component_id.as_str()) else {
                    return Err(error(
                        "component_requirement_missing",
                        format!(
                            "component '{}' requires missing component '{}'",
                            entry.component_id, requirement.component_id
                        ),
                    ));
                };
                if requirement
                    .required_route
                    .as_ref()
                    .is_some_and(|required| required != *route)
                {
                    return Err(error(
                        "component_requirement_missing",
                        "required route does not match component",
                    ));
                }
            }
        }
    }
    Ok(())
}

fn reject_requirement_cycles(manifest: &PdsPackageManifestV1) -> Result<(), TheoryRouterError> {
    let edges: BTreeMap<&str, Vec<&str>> = manifest
        .components
        .iter()
        .map(|entry| {
            (
                entry.component_id.as_str(),
                entry
                    .requires
                    .iter()
                    .filter(|item| matches!(item.package, PackageRequirementTarget::SelfPackage))
                    .map(|item| item.component_id.as_str())
                    .collect(),
            )
        })
        .collect();
    fn visit<'a>(
        node: &'a str,
        edges: &BTreeMap<&'a str, Vec<&'a str>>,
        visiting: &mut BTreeSet<&'a str>,
        visited: &mut BTreeSet<&'a str>,
    ) -> bool {
        if visiting.contains(node) {
            return true;
        }
        if !visited.insert(node) {
            return false;
        }
        visiting.insert(node);
        let cycle = edges.get(node).is_some_and(|next| {
            next.iter()
                .any(|child| visit(child, edges, visiting, visited))
        });
        visiting.remove(node);
        cycle
    }
    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    if edges
        .keys()
        .any(|node| visit(node, &edges, &mut visiting, &mut visited))
    {
        return Err(error(
            "component_requirement_cycle",
            "component requirement cycle",
        ));
    }
    Ok(())
}

fn validate_route(route: &TheoryRouteId) -> Result<(), TheoryRouterError> {
    validate_name(&route.owner_domain)?;
    validate_name(&route.component_kind)?;
    if route.route_version == 0 {
        return Err(error("package_source_invalid", "route version is zero"));
    }
    Ok(())
}

pub(crate) fn validate_name(value: &str) -> Result<(), TheoryRouterError> {
    if value.is_empty()
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(error(
            "package_source_invalid",
            format!("invalid normalized identity '{value}'"),
        ));
    }
    Ok(())
}

fn reject_duplicates<'a>(
    values: impl Iterator<Item = &'a str>,
    kind: &str,
) -> Result<(), TheoryRouterError> {
    let mut seen = BTreeSet::new();
    for value in values {
        if !seen.insert(value) {
            return Err(error(
                "package_source_invalid",
                format!("duplicate {kind} identity '{value}'"),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn route() -> TheoryRouteId {
        TheoryRouteId::new("fake", "body", 1)
    }

    fn manifest(order: bool) -> PdsPackageManifestV1 {
        let mut components = vec![
            PdsComponentEntry {
                component_id: "a".to_string(),
                owner_component_id: "a".to_string(),
                route: route(),
                component_schema_version: 1,
                content: ComponentContentRef::Embedded {
                    canonical_bytes: b"a".to_vec(),
                },
                requires: vec![],
            },
            PdsComponentEntry {
                component_id: "b".to_string(),
                owner_component_id: "b".to_string(),
                route: route(),
                component_schema_version: 1,
                content: ComponentContentRef::Embedded {
                    canonical_bytes: b"b".to_vec(),
                },
                requires: vec![],
            },
        ];
        if order {
            components.reverse();
        }
        PdsPackageManifestV1 {
            schema_version: 1,
            package_id: "test.package".to_string(),
            package_version: "1.0.0".to_string(),
            description: Some("ignored".to_string()),
            imports: vec![],
            components,
        }
    }

    #[test]
    fn package_identity_ignores_source_order() {
        let first = manifest(false).materialize(Path::new(".")).unwrap();
        let mut second_manifest = manifest(true);
        second_manifest.description = Some("also ignored".to_string());
        let second = second_manifest.materialize(Path::new(".")).unwrap();
        assert_eq!(first.package_content_hash, second.package_content_hash);
    }

    #[test]
    fn source_escape_and_hash_mismatch_fail_before_dispatch() {
        let root = tempfile::tempdir().unwrap();
        let mut package = manifest(false);
        package.components[0].content = ComponentContentRef::RelativeFile {
            path: "../escape".to_string(),
            content_hash: "missing".to_string(),
        };
        assert_eq!(
            package
                .materialize(root.path())
                .unwrap_err()
                .diagnostic_code,
            "package_source_invalid"
        );
        std::fs::write(root.path().join("body"), b"actual").unwrap();
        package.components[0].content = ComponentContentRef::RelativeFile {
            path: "body".to_string(),
            content_hash: "wrong".to_string(),
        };
        assert_eq!(
            package
                .materialize(root.path())
                .unwrap_err()
                .diagnostic_code,
            "package_identity_mismatch"
        );
    }
}
