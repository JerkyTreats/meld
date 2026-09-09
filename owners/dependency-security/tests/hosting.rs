use meld::runtime::owners::*;
use meld_events::{EventAuthority, EventAuthorityOpenOptions};
use std::{path::Path, sync::Arc};

#[test]
fn ordinary_initialization_prepares_external_security() {
    use meld::config::{PhysicalBinding, PhysicalBindingRef};
    use meld::runtime::assembly::{
        ProductRuntimeAssembly, ProductRuntimeConfig, StewardshipComposition,
    };
    use meld::runtime::owners::catalog::OwnerSelectionV1;
    let state = tempfile::tempdir().unwrap();
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(
        workspace.path().join("lib.rs"),
        "pub const PORT: u16 = 8000;\n",
    )
    .unwrap();
    let mut config = meld::config::MerkleConfig::default();
    config.system.storage.product_root = Some(state.path().join("runtime"));
    config.providers.insert(
        "main".into(),
        meld::config::ProviderConfig {
            provider_name: Some("main".into()),
            provider_type: meld::provider::ProviderType::Ollama,
            model: "test-model".into(),
            api_key: None,
            endpoint: None,
            default_options: Default::default(),
        },
    );
    let binary = Path::new(env!("CARGO_BIN_EXE_meld-dependency-security-owner"));
    let selection = OwnerSelectionV1 {
        executable: OwnerExecutableV1 {
            path: binary.into(),
            content_hash: blake3::hash(&std::fs::read(binary).unwrap())
                .to_hex()
                .to_string(),
        },
        limits: OwnerConnectionLimitsV1 {
            request_timeout_ms: 10_000,
            max_message_bytes: 16 * 1024 * 1024,
        },
        binding_ids: [
            "workspace",
            "dependency-security.cargo",
            "dependency-security.advisories",
        ]
        .into_iter()
        .map(String::from)
        .collect(),
    };
    let mut declaration: meld::config::StewardshipDeclaration = serde_json::from_value(serde_json::json!({
        "expression": "dependency_security_fixture", "target_root": workspace.path(),
        "subject": {"domain_id": "workspace_fs", "object_kind": "node", "object_id": "dependency-graph"},
        "agent_id": "external-docs-agent", "principal_id": "workspace-owner", "provider_id": "main",
        "theory": { "belief_family_id": "dependency_security_posture", "evidence_mapping_id": "dependency_security_outcome_mapping_v1", "curation_rule_id": "dependency_security_posture", "maintained_condition_id": "dependency_security_posture", "strategy_theory_id": "dependency_security_fixture", "authority_policy_id": "dependency_security_fixture_read_only" }
    })).unwrap();
    declaration.provider_id = None;
    let advisories = state.path().join("advisories.json");
    std::fs::write(&advisories, "{}").unwrap();
    declaration.bindings.insert(
        "dependency-security.cargo".into(),
        PhysicalBindingRef::ExecutableRef(env!("CARGO").into()),
    );
    declaration.bindings.insert(
        "dependency-security.advisories".into(),
        PhysicalBindingRef::EndpointRef(advisories.display().to_string()),
    );
    declaration.bindings.insert(
        "owner::dependency-security".into(),
        PhysicalBindingRef::ConfigRef(serde_json::to_string(&selection).unwrap()),
    );
    config
        .stewardship
        .declarations
        .insert("docs".into(), declaration);
    let binding = PhysicalBinding::resolve(&config).unwrap();
    let authority = Arc::new(
        EventAuthority::open(
            sled::open(state.path().join("events")).unwrap(),
            EventAuthorityOpenOptions::default(),
        )
        .unwrap(),
    );
    let open = || {
        ProductRuntimeAssembly::load_composed(
            ProductRuntimeConfig::for_product_root(binding.storage_root.clone()),
            authority.clone(),
            Some(StewardshipComposition {
                binding: binding.clone(),
            }),
        )
        .unwrap()
    };
    let assembly = open();
    let report = meld::init::world::tooling::run_world_init(
        &assembly,
        &config,
        workspace.path(),
        &[],
        Some(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../theory/dependency_security")),
        "external-init",
    )
    .unwrap();
    assert!(!format!("{report:?}").is_empty());
    drop(assembly);
    let reopened = open();
    assert!(
        reopened.capability_runtime().is_some(),
        "{:?}",
        reopened.diagnostics()
    );
    assert!(reopened
        .handle_factories()
        .get("dependency_security.observation")
        .is_some());
}
