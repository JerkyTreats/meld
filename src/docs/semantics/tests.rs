use super::*;
use crate::context::generation::contracts::GenerationOrchestrationRequest;
use crate::docs::capability::{
    draft_patch_set, inspect_scope, DocsCapabilityConfig, DocsPatchSet, ReadmePatch,
};
use crate::docs::claim_validation::{
    assess_captured_readme, evidence_partitions, validate_patch_set, DocsClaimPolicy,
    DocsClaimPolicyRegistryStore, ProviderDocsClaimJudge,
};
use crate::provider::executor::ProviderPreparation;
use crate::provider::{
    CompletionOptions, CompletionResponse, CompletionStream, ModelProviderClient, ProviderConfig,
    ProviderExecutionBinding, ProviderRuntimeOverrides, ProviderType, TokenUsage,
};
use std::collections::BTreeMap;
use std::sync::Mutex;

struct UnusedClient;
#[async_trait::async_trait]
impl ModelProviderClient for UnusedClient {
    async fn complete(
        &self,
        _: Vec<ChatMessage>,
        _: CompletionOptions,
    ) -> Result<CompletionResponse, ApiError> {
        panic!("test must use the recording execution port")
    }
    async fn stream(
        &self,
        _: Vec<ChatMessage>,
        _: CompletionOptions,
    ) -> Result<CompletionStream, ApiError> {
        panic!("streaming is not part of this proof")
    }
    fn provider_name(&self) -> &str {
        "recording"
    }
    fn model_name(&self) -> &str {
        "recording-model"
    }
    async fn list_models(&self) -> Result<Vec<String>, ApiError> {
        Ok(vec![])
    }
}

#[derive(Default)]
struct RecordingProvider {
    calls: Mutex<Vec<(GenerationOrchestrationRequest, Vec<ChatMessage>)>>,
    reject_first: bool,
    forged_quote: bool,
    reject_invented: bool,
}
impl meld_execution::ProviderValidationPort for RecordingProvider {
    type Error = ApiError;
    type GenerationRequest = GenerationOrchestrationRequest;
    type ProviderPreparation = ProviderPreparation;
    fn prepare_provider_for_request(
        &self,
        _: &GenerationOrchestrationRequest,
    ) -> Result<ProviderPreparation, ApiError> {
        Ok(ProviderPreparation {
            provider_config: ProviderConfig {
                provider_name: Some("recording".into()),
                provider_type: ProviderType::Ollama,
                model: "recording-model".into(),
                api_key: None,
                endpoint: None,
                default_options: CompletionOptions::default(),
            },
            provider_type: "recording".into(),
            client: Box::new(UnusedClient),
        })
    }
    fn validate_provider_binding(
        &self,
        _: &meld_execution::ProviderExecutionBinding,
    ) -> Result<(), ApiError> {
        Ok(())
    }
}
#[async_trait::async_trait]
impl meld_execution::ProviderExecutionPort for RecordingProvider {
    type Error = ApiError;
    type GenerationRequest = GenerationOrchestrationRequest;
    type ProviderPreparation = ProviderPreparation;
    type ChatMessage = ChatMessage;
    type CompletionResponse = CompletionResponse;
    async fn execute_completion(
        &self,
        request: &GenerationOrchestrationRequest,
        _: &ProviderPreparation,
        messages: Vec<ChatMessage>,
        _: Option<&meld_execution::ExecutionEventContext>,
    ) -> Result<CompletionResponse, ApiError> {
        assert_eq!(messages.len(), 2);
        let instruction = &messages[0].content;
        let input: serde_json::Value = serde_json::from_str(&messages[1].content).unwrap();
        let content = match request.frame_type.as_str() {
            "docs-source-claims" => {
                assert!(input.get("readme").is_none());
                assert!(input.get("readme_claims").is_none());
                let mut claims = vec![
                    serde_json::json!({"statement":"`run` exists.","confidence":1.0,"quotes":["pub fn run"]}),
                ];
                if instruction.contains("include stop") {
                    claims.push(serde_json::json!({"statement":"`stop` exists.","confidence":1.0,"quotes":["pub fn stop"]}));
                }
                serde_json::json!({"complete":true,"claims":claims,"no_claims_reason":null})
                    .to_string()
            }
            "docs-claim-validation" => {
                let supported = !(instruction.contains("reject all")
                    || self.reject_first && request.retry_count == 0);
                let assessments = input["claims"].as_array().unwrap().iter().map(|claim| serde_json::json!({
                    "claim_id":claim["claim_id"], "verdict":if supported && !(self.reject_invented && claim["statement"].as_str().unwrap().contains("invented")) {"supported"} else {"unsupported"}, "confidence":1.0,
                    "citations":if supported {vec![serde_json::json!({"scope":"direct","quote":if self.forged_quote {"foreign text"} else {"pub fn run"}})]} else {vec![]},
                    "rationale":"deterministic proposal under selected test instruction"
                })).collect::<Vec<_>>();
                serde_json::json!({"assessments":assessments}).to_string()
            }
            "docs-claim-correspondence" => {
                let claims = input["sources"].as_array().unwrap().iter().map(|source| serde_json::json!({
                    "source_claim_id":source["claim"]["claim_id"],
                    "readme_claim_ids":if instruction.contains("omit") {vec![]} else {input["readme_claims"].as_array().unwrap().iter().map(|claim| claim["claim_id"].clone()).collect::<Vec<_>>()},
                    "confidence":1.0,"rationale":"controlled correspondence"
                })).collect::<Vec<_>>();
                serde_json::json!({"complete":true,"claims":claims}).to_string()
            }
            "docs-readme" | "docs-readme-revision" => {
                "# Installed theory title\n\n`run` exists.\n".into()
            }
            other => panic!("unexpected provider operation {other}"),
        };
        self.calls.lock().unwrap().push((request.clone(), messages));
        Ok(CompletionResponse {
            content,
            model: "recording-model".into(),
            usage: TokenUsage {
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
            },
            finish_reason: None,
        })
    }
}

fn config(root: &std::path::Path) -> DocsCapabilityConfig {
    DocsCapabilityConfig {
        target_root: root.into(),
        subject_id: "subject".into(),
        agent_id: "agent".into(),
        provider: ProviderExecutionBinding::new("recording", ProviderRuntimeOverrides::default())
            .unwrap(),
    }
}
fn policy() -> DocsClaimPolicy {
    let mut policy = crate::docs::claim_observation::test_support::policy();
    let semantics = policy.semantic_theory.as_mut().unwrap();
    semantics.source_extraction = "Extract run only.".into();
    semantics.readme_judgment = "Accept the test assertions.".into();
    semantics.correspondence = "Represent the test assertions.".into();
    semantics.drafting = "Use the installed test title.".into();
    semantics.revision = "Revise using the installed test title.".into();
    semantics.claim_guards.clear();
    policy
}

#[test]
fn keyed_response_contracts_bind_exact_input_identities() {
    let format = serde_json::json!({"x-meld-map-from-field":{
        "array":"/claims", "field":"/id", "value_schema":{
            "type":"object", "properties":{"matches":{"type":"array","items":{
                "type":"string","x-meld-enum-from-field":{"array":"/targets","field":"/id"}
            }}}, "required":["matches"]
        }
    }});
    let input = serde_json::json!({"claims":[{"id":"b"},{"id":"a"}],"targets":[{"id":"target"}]});
    let bound = bind_evidence_choices(&format, &input).unwrap();
    assert_eq!(bound["required"], serde_json::json!(["b", "a"]));
    assert_eq!(bound["properties"].as_object().unwrap().len(), 2);
    assert_eq!(
        bound["properties"]["a"]["properties"]["matches"]["items"]["enum"],
        serde_json::json!(["target"])
    );
    let duplicate =
        serde_json::json!({"claims":[{"id":"a"},{"id":"a"}],"targets":[{"id":"target"}]});
    assert!(bind_evidence_choices(&format, &duplicate).is_err());
}

#[test]
fn evidence_choice_contracts_preserve_exact_text_without_inventing_citations() {
    let format = serde_json::json!({"type":"string","maxLength":5,
        "x-meld-enum-from-lines":["/source", "/empty"]});
    let input = serde_json::json!({"source":"    αβγδεζη\nshort\nshort", "empty":""});
    let bound = bind_evidence_choices(&format, &input).unwrap();
    let choices = bound["enum"].as_array().unwrap();
    assert_eq!(choices.len(), 3);
    for choice in choices {
        let quote = choice.as_str().unwrap();
        assert!(input["source"].as_str().unwrap().contains(quote));
        assert!(quote.chars().count() <= 5);
    }
    assert!(bound.get("x-meld-enum-from-lines").is_none());
    assert!(bind_evidence_choices(&format, &serde_json::json!({"source":""})).is_err());
    assert!(bind_evidence_choices(&format, &serde_json::json!({"source":"","empty":""})).is_err());
    let missing_selection = serde_json::json!({"type":"string"});
    assert_eq!(
        bind_evidence_choices(&missing_selection, &input).unwrap(),
        missing_selection
    );
    let conditional = serde_json::json!({"anyOf":[
        {"x-meld-if-text":"/source", "type":"object", "properties":{
            "scope":{"const":"source"},
            "quote":{"x-meld-enum-from-lines":["/source"]}
        }},
        {"x-meld-if-text":"/empty", "type":"object", "properties":{
            "scope":{"const":"empty"},
            "quote":{"x-meld-enum-from-lines":["/empty"]}
        }}
    ]});
    let bound = bind_evidence_choices(&conditional, &input).unwrap();
    assert_eq!(bound["anyOf"].as_array().unwrap().len(), 1);
    assert_eq!(bound["anyOf"][0]["properties"]["scope"]["const"], "source");
    let boolean_enum = serde_json::json!({"enum":[true,false]});
    assert_eq!(
        bind_evidence_choices(&boolean_enum, &input).unwrap(),
        boolean_enum
    );
}

#[test]
fn installed_response_formats_travel_with_the_exact_judgment_request() {
    let root = tempfile::tempdir().unwrap();
    let mut policy = policy();
    let config = config(root.path());
    let operation = DocsJudgmentOperation::SourceExtraction;
    let format = serde_json::json!({"type":"json_schema","json_schema":{
        "name":"source_claims", "strict":true, "schema":{"type":"object"}
    }});
    let semantics = policy.semantic_theory.as_mut().unwrap();
    semantics.response_formats.insert(operation, format.clone());
    let first = semantics
        .generation(&config, "policy", operation, serde_json::json!({}), 0, 0)
        .unwrap();
    assert_eq!(
        first.request.provider.runtime_overrides.extra_body_fields["response_format"],
        format
    );
    semantics.response_formats.clear();
    let second = semantics
        .generation(&config, "policy", operation, serde_json::json!({}), 0, 0)
        .unwrap();
    assert!(!second
        .request
        .provider
        .runtime_overrides
        .extra_body_fields
        .contains_key("response_format"));
    assert_ne!(first.request.request_id, second.request.request_id);
    semantics.response_formats.insert(operation, format);
    let mut conflicting = config;
    conflicting
        .provider
        .runtime_overrides
        .extra_body_fields
        .insert(
            "response_format".into(),
            serde_json::json!({"type":"json_object"}),
        );
    assert!(semantics
        .generation(
            &conflicting,
            "policy",
            operation,
            serde_json::json!({}),
            0,
            0
        )
        .is_err());
}

#[tokio::test]
async fn installed_instructions_reach_real_provider_paths_and_change_judgments() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(
        root.path().join("lib.rs"),
        "pub fn run() {}\npub fn stop() {}\n",
    )
    .unwrap();
    std::fs::write(root.path().join("README.md"), "`run` exists.\n").unwrap();
    let bundle = inspect_scope(root.path()).unwrap();
    let store =
        DocsClaimPolicyRegistryStore::new(sled::Config::new().temporary(true).open().unwrap())
            .unwrap();
    let (_, first) = store.install(policy(), 1).unwrap();
    let mut changed = first.policy.clone();
    let semantics = changed.semantic_theory.as_mut().unwrap();
    semantics.source_extraction = "Extract run and include stop.".into();
    semantics.readme_judgment = "reject all test assertions".into();
    semantics.correspondence = "omit every test assertion".into();
    let (_, second) = store.install(changed, 2).unwrap();
    let api = RecordingProvider::default();
    let config = config(root.path());
    let judge = ProviderDocsClaimJudge {
        api: &api,
        config: &config,
        event_context: None,
    };
    let mut products = Vec::new();
    for revision in [&first, &second] {
        let sources = crate::docs::source_claims::advance_source_claims(
            &judge,
            &revision.policy,
            &bundle,
            None,
            usize::MAX,
        )
        .await
        .unwrap();
        let claims = crate::docs::claim_observation::assess_observed_claims(
            &judge,
            &revision.policy,
            &bundle,
        )
        .await
        .unwrap();
        let correspondence = crate::docs::correspondence::advance_correspondence(
            &judge,
            &revision.policy,
            &bundle,
            &sources,
            None,
            usize::MAX,
        )
        .await
        .unwrap();
        products.push((sources, claims, correspondence));
    }
    assert_eq!(products[0].0.files[0].claims.len(), 1);
    assert_eq!(products[1].0.files[0].claims.len(), 2);
    assert_ne!(products[0].0.report_id, products[1].0.report_id);
    assert!(!products[0]
        .0
        .matches_input(&second.policy, &bundle)
        .unwrap());
    use crate::docs::claim_observation::ObservedClaimDisposition;
    assert!(
        matches!(&products[0].1.readmes[0].disposition, ObservedClaimDisposition::Assessed { report } if report.accepted)
    );
    assert!(
        matches!(&products[1].1.readmes[0].disposition, ObservedClaimDisposition::Assessed { report } if !report.accepted)
    );
    assert!(!products[0].2.readmes[0].claims[0]
        .readme_claim_ids
        .is_empty());
    assert!(products[1].2.readmes[0]
        .claims
        .iter()
        .all(|claim| claim.readme_claim_ids.is_empty()));
    let initial_judgment_request = {
        let calls = api.calls.lock().unwrap();
        assert_eq!(calls.len(), 6);
        for (index, revision) in [&first, &second].iter().enumerate() {
            let semantics = revision.policy.semantics().unwrap();
            for (offset, instruction) in [
                &semantics.source_extraction,
                &semantics.readme_judgment,
                &semantics.correspondence,
            ]
            .iter()
            .enumerate()
            {
                assert_eq!(&calls[index * 3 + offset].1[0].content, *instruction);
            }
        }
        for offset in 0..3 {
            assert_ne!(calls[offset].0.request_id, calls[3 + offset].0.request_id);
        }
        calls[1].0.node_id
    };
    std::fs::write(
        root.path().join("lib.rs"),
        "pub fn run() {}\npub fn stop() {}\n// changed source evidence\n",
    )
    .unwrap();
    let changed_capture = inspect_scope(root.path()).unwrap();
    crate::docs::claim_observation::assess_observed_claims(&judge, &first.policy, &changed_capture)
        .await
        .unwrap();
    assert_ne!(
        api.calls.lock().unwrap().last().unwrap().0.node_id,
        initial_judgment_request
    );
}

#[tokio::test]
async fn draft_and_revision_use_selected_instructions_without_rewriting_the_model_title() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("child")).unwrap();
    std::fs::write(root.path().join("child/lib.rs"), "pub fn run() {}\n").unwrap();
    std::fs::write(root.path().join("child/README.md"), "`run` exists.\n").unwrap();
    let config = config(root.path());
    let mut policy = policy();
    policy.maximum_revision_attempts = 2;
    let api = RecordingProvider::default();
    let drafts = draft_patch_set(
        &api,
        &config,
        &policy,
        &inspect_scope(root.path()).unwrap(),
        None,
    )
    .await
    .unwrap();
    assert!(drafts
        .patches
        .iter()
        .all(|patch| patch.content.starts_with("# Installed theory title")));
    assert!(api.calls.lock().unwrap().iter().any(|(_, messages)| {
        let input: serde_json::Value = serde_json::from_str(&messages[1].content).unwrap();
        input["directory"] == "child" && input["current_readme"] == "`run` exists.\n"
    }));
    assert!(api
        .calls
        .lock()
        .unwrap()
        .iter()
        .all(|(_, messages)| messages[0].content == policy.semantics().unwrap().drafting));
    std::fs::remove_dir_all(root.path().join("child")).unwrap();
    std::fs::write(root.path().join("lib.rs"), "pub fn run() {}\n").unwrap();
    let bundle = inspect_scope(root.path()).unwrap();
    let content = "# run\n\n`run` exists.\n";
    let patch = ReadmePatch {
        path: "README.md".into(),
        content: content.into(),
        content_hash: blake3::hash(content.as_bytes()).to_hex().to_string(),
    };
    let api = RecordingProvider {
        reject_first: true,
        ..Default::default()
    };
    let validated = validate_patch_set(
        &api,
        &config,
        &policy,
        &bundle,
        &DocsPatchSet {
            source_fingerprint: bundle.source_fingerprint.clone(),
            patches: vec![patch],
        },
        None,
    )
    .await
    .unwrap();
    assert_eq!(validated.reports[0].revision_attempts, 1);
    assert!(validated.patches[0]
        .content
        .starts_with("# Installed theory title"));
    let calls = api.calls.lock().unwrap();
    assert_eq!(calls[1].0.frame_type, "docs-readme-revision");
    assert_eq!(calls[1].1[0].content, policy.semantics().unwrap().revision);
}

#[tokio::test]
async fn selected_guards_change_semantic_admissibility_but_cannot_admit_forged_evidence() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("lib.rs"), "pub fn run() {}\n").unwrap();
    let bundle = inspect_scope(root.path()).unwrap();
    let directory = &bundle.directories[0];
    let evidence = evidence_partitions(directory, &BTreeMap::new());
    let content = "`invented` exists.\n";
    let patch = ReadmePatch {
        path: "README.md".into(),
        content: content.into(),
        content_hash: blake3::hash(content.as_bytes()).to_hex().to_string(),
    };
    let config = config(root.path());
    let api = RecordingProvider::default();
    let judge = ProviderDocsClaimJudge {
        api: &api,
        config: &config,
        event_context: None,
    };
    let policy = policy();
    assert!(
        assess_captured_readme(&judge, &policy, directory, &evidence, &patch)
            .await
            .unwrap()
            .accepted
    );
    let mut guarded = policy.clone();
    guarded.semantic_theory.as_mut().unwrap().claim_guards =
        vec![DocsClaimGuard::LiteralPresenceV1];
    assert!(
        !assess_captured_readme(&judge, &guarded, directory, &evidence, &patch)
            .await
            .unwrap()
            .accepted
    );
    assert_ne!(policy.content_identity(), guarded.content_identity());
    let bad_api = RecordingProvider {
        forged_quote: true,
        ..Default::default()
    };
    let bad_judge = ProviderDocsClaimJudge {
        api: &bad_api,
        config: &config,
        event_context: None,
    };
    assert!(
        assess_captured_readme(&bad_judge, &policy, directory, &evidence, &patch)
            .await
            .is_err()
    );
    let count = api.calls.lock().unwrap().len();
    let mut missing = policy.clone();
    missing.semantic_theory = None;
    assert!(
        assess_captured_readme(&judge, &missing, directory, &evidence, &patch)
            .await
            .is_err()
    );
    assert_eq!(api.calls.lock().unwrap().len(), count);
    let mut invalid = policy.clone();
    invalid
        .semantic_theory
        .as_mut()
        .unwrap()
        .source_extraction
        .clear();
    assert!(invalid.validate().is_err());
    let mut unknown = serde_json::to_value(&policy).unwrap();
    unknown["semantic_theory"]["claim_guards"] = serde_json::json!(["unknown_guard"]);
    assert!(serde_json::from_value::<DocsClaimPolicy>(unknown).is_err());
}

#[tokio::test]
async fn installed_repair_responses_choose_refusal_revision_or_pruning() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("lib.rs"), "pub fn run() {}\n").unwrap();
    let bundle = inspect_scope(root.path()).unwrap();
    let config = config(root.path());
    let content = "# run module\n\n`run` exists in this module.\n\nAn invented assertion.\n";
    let patch = ReadmePatch {
        path: "README.md".into(),
        content: content.into(),
        content_hash: blake3::hash(content.as_bytes()).to_hex().to_string(),
    };
    let patches = DocsPatchSet {
        source_fingerprint: bundle.source_fingerprint.clone(),
        patches: vec![patch],
    };
    let store =
        DocsClaimPolicyRegistryStore::new(sled::Config::new().temporary(true).open().unwrap())
            .unwrap();
    for actions in [
        vec![],
        vec![DocsRepairAction::ReviseV1],
        vec![DocsRepairAction::PruneRejectedClaimsV1],
        vec![
            DocsRepairAction::PruneRejectedClaimsV1,
            DocsRepairAction::ReviseV1,
        ],
    ] {
        let mut selected = policy();
        selected.semantic_theory.as_mut().unwrap().repair_actions = Some(actions.clone());
        let (_, revision) = store.install(selected, 1).unwrap();
        let api = RecordingProvider {
            reject_invented: true,
            ..Default::default()
        };
        let result =
            validate_patch_set(&api, &config, &revision.policy, &bundle, &patches, None).await;
        if actions.is_empty() {
            assert!(result
                .unwrap_err()
                .to_string()
                .contains("installed Docs repair responses"));
        } else {
            let accepted = result.unwrap();
            assert_eq!(accepted.reports[0].revision_attempts, 1);
            assert!(!accepted.patches[0].content.contains("invented"));
            assert_eq!(
                accepted.patches[0]
                    .content
                    .starts_with("# Installed theory title"),
                actions[0] == DocsRepairAction::ReviseV1
            );
        }
        let calls = api.calls.lock().unwrap();
        let expected_calls = if actions.first() == Some(&DocsRepairAction::ReviseV1) {
            3
        } else {
            1
        };
        assert_eq!(calls.len(), expected_calls);
        assert!(!root.path().join("README.md").exists());
    }
    let mut missing = policy();
    missing.semantic_theory.as_mut().unwrap().repair_actions = None;
    assert!(store.install(missing, 2).is_err());
    let mut over_budget = policy();
    over_budget.maximum_revision_attempts = 0;
    assert!(store.install(over_budget, 2).is_err());
    let mut unsupported = serde_json::to_value(policy()).unwrap();
    unsupported["semantic_theory"]["repair_actions"] =
        serde_json::json!(["unimplemented_response"]);
    assert!(serde_json::from_value::<DocsClaimPolicy>(unsupported).is_err());
}
