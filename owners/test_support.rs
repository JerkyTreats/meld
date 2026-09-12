//! Executable owner selection for package and core integration fixtures.

use crate::config::{PhysicalBinding, PhysicalBindingRef, SelectedStewardshipPackage};
use crate::runtime::{owners::*, storage::OpenProductStores};
use std::collections::BTreeMap;

pub fn selection(owner: &str, binary: &str, required: &[&str]) -> (String, PhysicalBindingRef) {
    let executable = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join(binary);
    let bytes = std::fs::read(&executable).unwrap_or_else(|error| {
        panic!(
            "build workspace binaries before owner integration tests: {}: {error}",
            executable.display()
        )
    });
    let selection = catalog::OwnerSelectionV1 {
        executable: OwnerExecutableV1 {
            path: executable,
            content_hash: blake3::hash(&bytes).to_hex().to_string(),
        },
        limits: OwnerConnectionLimitsV1 {
            request_timeout_ms: 30_000,
            max_message_bytes: 16 * 1024 * 1024,
        },
        binding_ids: required.iter().map(|id| id.to_string()).collect(),
    };
    (
        format!("owner::{owner}"),
        PhysicalBindingRef::ConfigRef(serde_json::to_string(&selection).unwrap()),
    )
}

pub fn configure(stores: &mut OpenProductStores, root: &std::path::Path, owners: &[(&str, &str)]) {
    let bindings: BTreeMap<_, _> = owners
        .iter()
        .map(|(owner, binary)| selection(owner, binary, &[]))
        .collect();
    let binding = PhysicalBinding {
        agent_positions: Default::default(),
        bindings,
        workspace_root: Some(root.join("workspace")),
        subject: meld_events::DomainObjectRef::new("workspace_fs", "node", "fixture").unwrap(),
        agent_id: "fixture".into(),
        provider_id: None,
        storage_root: root.into(),
        package: SelectedStewardshipPackage {
            expression: "fixture".into(),
            principal_id: "fixture".into(),
            belief_family_id: "fixture".into(),
            evidence_mapping_id: "fixture".into(),
            curation_rule_id: "fixture".into(),
            maintained_condition_id: "fixture".into(),
            strategy_theory_id: "fixture".into(),
            authority_policy_id: "fixture".into(),
            claim_policy_id: String::new(),
        },
    };
    let authority = meld_events::EventAuthority::open(
        sled::Config::new().temporary(true).open().unwrap(),
        meld_events::EventAuthorityOpenOptions::default(),
    )
    .unwrap();
    stores.owners = catalog::OwnerCatalog::open(&binding, &authority).unwrap();
}
