use std::collections::BTreeSet;

use serde::Serialize;

use super::contracts::*;

#[derive(Serialize)]
struct Identity<'a> {
    subject: &'a DependencySecuritySubjectV1,
    workspace_revision: &'a str,
    manifest_content_hash: &'a str,
    lockfile_content_hash: &'a str,
    components: &'a [ResolvedDependencyComponentV1],
    completeness: &'a InventoryCompleteness,
    observed_at: u64,
}

impl DependencyInventorySnapshotV1 {
    pub fn canonical(
        subject: DependencySecuritySubjectV1,
        workspace_revision: String,
        manifest_content_hash: String,
        lockfile_content_hash: String,
        mut components: Vec<ResolvedDependencyComponentV1>,
        completeness: InventoryCompleteness,
        observed_at: u64,
    ) -> Result<Self, String> {
        if !subject.inventory_scope.include_transitive
            || workspace_revision.trim().is_empty()
            || manifest_content_hash.trim().is_empty()
            || lockfile_content_hash.trim().is_empty()
        {
            return Err("inventory scope or source identity is incomplete".into());
        }
        for component in &mut components {
            component.dependency_paths.sort();
        }
        components.sort_by(|a, b| {
            (
                &a.package_name,
                &a.resolved_version,
                &a.source_identity,
                &a.component_id,
            )
                .cmp(&(
                    &b.package_name,
                    &b.resolved_version,
                    &b.source_identity,
                    &b.component_id,
                ))
        });
        let mut ids = BTreeSet::new();
        if components.iter().any(|c| {
            c.component_id.trim().is_empty()
                || c.package_name.trim().is_empty()
                || c.resolved_version.trim().is_empty()
                || c.source_identity.trim().is_empty()
                || !ids.insert(c.component_id.clone())
        }) {
            return Err("inventory components must be complete and unique".into());
        }
        let identity = Identity {
            subject: &subject,
            workspace_revision: &workspace_revision,
            manifest_content_hash: &manifest_content_hash,
            lockfile_content_hash: &lockfile_content_hash,
            components: &components,
            completeness: &completeness,
            observed_at,
        };
        let snapshot_id = content_hash(&identity)?;
        Ok(Self {
            snapshot_id,
            subject,
            workspace_revision,
            manifest_content_hash,
            lockfile_content_hash,
            components,
            completeness,
            observed_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use meld_events::DomainObjectRef;
    fn subject() -> DependencySecuritySubjectV1 {
        DependencySecuritySubjectV1 {
            subject: DomainObjectRef::new("workspace_fs", "node", "repo").unwrap(),
            ecosystem: PackageEcosystem::Cargo,
            inventory_scope: InventoryScopeV1 {
                manifest_ref: DomainObjectRef::new("workspace_fs", "file", "Cargo.toml").unwrap(),
                lockfile_ref: DomainObjectRef::new("workspace_fs", "file", "Cargo.lock").unwrap(),
                include_transitive: true,
            },
        }
    }
    fn component(id: &str) -> ResolvedDependencyComponentV1 {
        ResolvedDependencyComponentV1 {
            component_id: id.into(),
            ecosystem: PackageEcosystem::Cargo,
            package_name: id.into(),
            resolved_version: "1.0.0".into(),
            source_identity: "registry".into(),
            dependency_paths: vec![vec!["root".into(), id.into()]],
        }
    }
    #[test]
    fn inventory_identity_is_order_independent() {
        let a = DependencyInventorySnapshotV1::canonical(
            subject(),
            "rev".into(),
            "m".into(),
            "l".into(),
            vec![component("a"), component("b")],
            InventoryCompleteness::CompleteTransitive,
            10,
        )
        .unwrap();
        let b = DependencyInventorySnapshotV1::canonical(
            subject(),
            "rev".into(),
            "m".into(),
            "l".into(),
            vec![component("b"), component("a")],
            InventoryCompleteness::CompleteTransitive,
            10,
        )
        .unwrap();
        assert_eq!(a.snapshot_id, b.snapshot_id);
    }
}
