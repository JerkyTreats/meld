use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use meld::runtime::lifecycle::{NativeObservationOwnerFactory, ParticipantLifecycleContextV1};
use meld::runtime::owners::registration::RegisteredOwner;
use meld::runtime::owners::runtime::{OwnerRuntimeGrants, PreparedOwnerRuntime};
use meld::runtime::owners::*;
use meld::theory::*;
use meld_events::{EventAuthority, EventAuthorityOpenOptions};

fn api(root: &Path) -> meld::api::ContextApi {
    meld::api::ContextApi::new(
        Arc::new(meld::store::SledNodeRecordStore::new(root.join("nodes")).unwrap()),
        Arc::new(meld::context::frame::FrameStorage::new(root.join("frames")).unwrap()),
        meld::heads::HeadIndex::new(),
        Arc::new(
            meld::prompt_context::PromptContextArtifactStorage::new(root.join("prompts")).unwrap(),
        ),
        Arc::new(parking_lot::RwLock::new(meld::agent::AgentRegistry::new())),
        Arc::new(parking_lot::RwLock::new(
            meld::provider::ProviderRegistry::new(),
        )),
        Arc::new(meld::concurrency::NodeLockManager::new()),
    )
}

#[test]
fn docs_package_installs_observes_invokes_and_reopens_through_native_ports() {
    let state = tempfile::tempdir().unwrap();
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(
        workspace.path().join("lib.rs"),
        "pub const PORT: u16 = 8000;\n",
    )
    .unwrap();
    let binary = Path::new(env!("CARGO_BIN_EXE_meld-docs-owner"));
    let selected = OwnerExecutableV1 {
        path: binary.into(),
        content_hash: blake3::hash(&std::fs::read(binary).unwrap())
            .to_hex()
            .to_string(),
    };
    let implementations = state.path().join("implementations");
    let limits = OwnerConnectionLimitsV1 {
        request_timeout_ms: 10_000,
        max_message_bytes: 16 * 1024 * 1024,
    };
    let owner = RegisteredOwner::open(
        "docs",
        &selected,
        &implementations,
        &state.path().join("revisions"),
        limits.clone(),
    )
    .unwrap();
    let policy = std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../theory/docs_freshness/claim_policy.docs-claims-strict-v1.json"),
    )
    .unwrap();
    let manifest = PdsPackageManifestV1 {
        schema_version: 1,
        package_id: "external-docs-policy".into(),
        package_version: "1".into(),
        description: None,
        imports: vec![],
        components: vec![PdsComponentEntry {
            component_id: "policy".into(),
            owner_component_id: "docs-claims-strict-v1".into(),
            route: TheoryRouteId::new("docs", "claim-policy", 1),
            component_schema_version: 1,
            content: ComponentContentRef::Embedded {
                canonical_bytes: policy,
            },
            requires: vec![],
        }],
    };
    let store =
        PdsPackageStore::new(sled::open(state.path().join("core-theory")).unwrap()).unwrap();
    let installed = TheoryRouter::new(
        TheoryRouteCatalog::build(owner.route_handlers()).unwrap(),
        store.clone(),
    )
    .install(&manifest.materialize(state.path()).unwrap(), 1, false, None)
    .unwrap();
    let subject = meld_events::DomainObjectRef::new("workspace_fs", "node", "docs").unwrap();
    let resources = owner
        .prepare_bindings(
            subject.clone(),
            BTreeMap::from([
                ("workspace".into(), workspace.path().display().to_string()),
                ("subject".into(), "docs".into()),
                ("agent".into(), "docs-agent".into()),
            ]),
            installed.components.clone(),
        )
        .unwrap();
    let authority = Arc::new(
        EventAuthority::open(
            sled::open(state.path().join("events")).unwrap(),
            EventAuthorityOpenOptions::default(),
        )
        .unwrap(),
    );
    let policy: meld_lang::AuthorityPolicy = serde_json::from_slice(
        &std::fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../theory/docs_freshness/authority_policy.docs_workspace_local.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let policy_hash = policy.content_hash().unwrap();
    let native = OwnerObservationPreparationV1 {
        subject,
        scope: meld_world_model::world_state::graph::contracts::OwnerPublicationScope {
            scope_id: "docs".into(),
            branch_id: Some("main".into()),
            perspective_id: Some("default".into()),
            valid_at: None,
        },
        session_id: "docs-native-session".into(),
        authority: meld_lang::AuthorityPolicyBinding::new(policy, policy_hash).unwrap(),
        event_routes: vec![meld_docs_owner::docs::publication::graph_route()],
    };
    let preparation = OwnerRuntimePreparationV1 {
        assignment_id: "docs-assignment".into(),
        activation_id: "docs-activation".into(),
        participant_id: "docs.observation".into(),
        state_root: state.path().join("assignment-state"),
        bindings: resources.bindings.clone(),
        installed_revisions: installed.components.clone(),
        ledger_id: authority.ledger_identity(),
        observation: Some(native),
    };
    let open = || {
        let grants = OwnerRuntimeGrants {
            events: Arc::new(meld_events::events::remote::LocalEventAuthorityClient::new(
                &authority,
            )),
            observation_routes: resources.observation_publications.clone(),
            invocation_routes: resources
                .capability_grants
                .iter()
                .map(|grant| (grant.selection.clone(), grant.publications.clone()))
                .collect(),
            observation_session: "docs-native-session".into(),
            retained_observation_publications: Default::default(),
            invocation_continuations: Default::default(),
            provider_binding: None,
            observation_provider_frame_types: resources.observation_provider_frame_types.clone(),
            invocation_provider_frame_types: resources
                .capability_grants
                .iter()
                .map(|grant| (grant.selection.clone(), grant.provider_frame_types.clone()))
                .collect(),
            agent_id: "docs-agent".into(),
        };
        PreparedOwnerRuntime::prepare(
            OwnerConnection::start(&selected, &implementations, limits.clone()).unwrap(),
            owner.description().clone(),
            preparation.clone(),
            grants,
        )
        .unwrap()
    };
    let runtime = open();
    let mut observer = runtime.build();
    let context = ParticipantLifecycleContextV1 {
        generation_id: "generation-one".into(),
        incarnation_id: "incarnation-one".into(),
        realization_id: "realization-one".into(),
        participant_id: "docs.observation".into(),
        owner_domain: "docs".into(),
        kind: ParticipantKind::BoundedActor,
        readiness_contract_ref: "docs.readiness.v1".into(),
        wake_contract_ref: "docs.wake.v1".into(),
        safe_point_contract_ref: "docs.safe-point.v1".into(),
        stop_contract_ref: "docs.stop.v1".into(),
        lease_ref: "lease-one".into(),
    };
    observer.native_readiness(&context).unwrap();
    let observed = observer.tick(meld::runtime::contracts::WorkBudget { max_items: 1 });
    assert!(observed.fatal_errors.is_empty(), "{observed:?}");
    assert_eq!(observed.items_committed, 1, "{observed:?}");
    let offer = owner
        .description()
        .implementations
        .iter()
        .find(|offer| offer.contract_ref.selector.capability_type_id == "docs.inspect_scope")
        .unwrap();
    let invoker = runtime
        .invoker(&offer.contract_ref, &offer.implementation_ref)
        .unwrap();
    let contract = invoker.contract();
    let init = meld::capability::CapabilityRuntimeInit {
        capability_instance_id: "inspection".into(),
        capability_type_id: contract.capability_type_id.clone(),
        capability_version: contract.capability_version,
        scope_ref: "docs".into(),
        scope_kind: "workspace".into(),
        binding_values: vec![],
        input_contract: contract.input_contract,
        output_contract: contract.output_contract,
        effect_contract: contract.effect_contract,
        execution_contract: contract.execution_contract,
    };
    let payload = meld::capability::CapabilityInvocationPayload {
        invocation_id: "exact-docs-inspection".into(),
        capability_instance_id: "inspection".into(),
        supplied_inputs: vec![],
        upstream_lineage: None,
        execution_context: Default::default(),
    };
    let executor = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let result = executor
        .block_on(invoker.invoke(
            &api(&state.path().join("api")),
            &init,
            &payload,
            Some(&meld::execution::ExecutionEventContext {
                session_id: "task-session".into(),
                effect_authority: None,
            }),
        ))
        .unwrap();
    assert_eq!(result.emitted_artifacts.len(), 1);
    assert_eq!(
        result.emitted_artifacts[0].artifact_type_id,
        "docs_evidence_bundle"
    );
    // A successor can prepare while its predecessor still owns mutable state.
    let reopened = open();
    observer.native_safe_point(&context).unwrap();
    observer.native_stop(&context).unwrap();
    observer.native_release(&context).unwrap();
    drop(observer);
    drop(invoker);
    drop(runtime);
    let mut observer = reopened.build();
    let context = ParticipantLifecycleContextV1 {
        generation_id: "generation-two".into(),
        incarnation_id: "incarnation-two".into(),
        ..context
    };
    observer.native_readiness(&context).unwrap();
    let report = observer.tick(meld::runtime::contracts::WorkBudget { max_items: 1 });
    assert!(report.fatal_errors.is_empty(), "{report:?}");
    assert_eq!(report.items_committed, 0, "{report:?}");
    assert!(workspace
        .path()
        .read_dir()
        .unwrap()
        .all(|entry| entry.unwrap().file_name() == "lib.rs"));
    assert_eq!(
        PdsPackageResolver::new(
            TheoryRouteCatalog::build(owner.route_handlers()).unwrap(),
            store
        )
        .resolve(&installed.receipt_id)
        .unwrap()
        .receipt,
        installed
    );
}

#[test]
fn ordinary_initialization_prepares_external_docs() {
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
    let binary = Path::new(env!("CARGO_BIN_EXE_meld-docs-owner"));
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
        binding_ids: ["workspace", "provider", "agent", "subject"]
            .into_iter()
            .map(String::from)
            .collect(),
    };
    let mut declaration: meld::config::StewardshipDeclaration = serde_json::from_value(serde_json::json!({
        "expression": "docs_freshness", "target_root": workspace.path(),
        "subject": {"domain_id": "workspace_fs", "object_kind": "node", "object_id": "docs"},
        "agent_id": "external-docs-agent", "principal_id": "workspace-owner", "provider_id": "main",
        "theory": { "belief_family_id": "docs_freshness", "evidence_mapping_id": "docs_freshness_outcome_interpretation_v1", "curation_rule_id": "docs_freshness", "maintained_condition_id": "docs_freshness", "strategy_theory_id": "docs_freshness", "authority_policy_id": "docs_workspace_local" }
    })).unwrap();
    declaration.bindings.insert(
        "owner::docs".into(),
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
        Some(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../theory/docs_freshness")),
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
        .get("docs.observation")
        .is_some());
}
