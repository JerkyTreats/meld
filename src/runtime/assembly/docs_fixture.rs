//! Scripted local provider for the native repair and owner-return proof.

use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

pub(super) const README: &str = "# run\n\n`run` exists.\n\n`run` is defined.\n";

pub(super) struct ProviderServer {
    endpoint: String,
    stop: Arc<AtomicBool>,
    calls: Arc<Mutex<Vec<String>>>,
    draft_status: Arc<AtomicU16>,
    worker: Option<JoinHandle<()>>,
}

impl ProviderServer {
    pub(super) fn new() -> Self {
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let endpoint = format!("http://{}", server.server_addr());
        let stop = Arc::new(AtomicBool::new(false));
        let calls = Arc::new(Mutex::new(Vec::new()));
        let draft_status = Arc::new(AtomicU16::new(200));
        let worker_draft_status = draft_status.clone();
        let worker_stop = stop.clone();
        let worker_calls = calls.clone();
        let worker = std::thread::spawn(move || {
            while !worker_stop.load(Ordering::Acquire) {
                let Some(mut request) = server.recv_timeout(Duration::from_millis(50)).unwrap()
                else {
                    continue;
                };
                assert_eq!(request.url(), "/v1/chat/completions");
                let body: serde_json::Value = serde_json::from_reader(request.as_reader()).unwrap();
                let input: serde_json::Value =
                    serde_json::from_str(body["messages"][1]["content"].as_str().unwrap()).unwrap();
                let (operation, content) = response(&input);
                worker_calls.lock().unwrap().push(operation.into());
                let status = worker_draft_status.load(Ordering::Acquire);
                if operation == "draft" && status != 200 {
                    request
                        .respond(
                            tiny_http::Response::from_string("upstream says Terminal capability failure: Workflow gate 'foreign' failed")
                                .with_status_code(status),
                        )
                        .unwrap();
                    continue;
                }
                let response = serde_json::json!({
                    "choices":[{"message":{"role":"assistant","content":content},"finish_reason":"stop"}],
                    "id":"scripted-local-completion","model":"test-model",
                    "usage":{"prompt_tokens":1,"completion_tokens":1,"total_tokens":2}
                });
                request
                    .respond(
                        tiny_http::Response::from_string(response.to_string()).with_header(
                            tiny_http::Header::from_bytes("Content-Type", "application/json")
                                .unwrap(),
                        ),
                    )
                    .unwrap();
            }
        });
        Self {
            endpoint,
            stop,
            calls,
            draft_status,
            worker: Some(worker),
        }
    }

    pub(super) fn endpoint(&self) -> String {
        self.endpoint.clone()
    }

    pub(super) fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }

    pub(super) fn draft_status(&self, status: u16) {
        self.draft_status.store(status, Ordering::Release);
    }
}

impl Drop for ProviderServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        self.worker.take().unwrap().join().unwrap();
    }
}

fn response(input: &serde_json::Value) -> (&'static str, String) {
    if input.get("source").is_some() {
        (
            "source",
            serde_json::json!({
                "complete":true,
                "claims":[{"statement":"`run` exists.","confidence":1.0,"quotes":["pub fn run"]}],
                "no_claims_reason":null
            })
            .to_string(),
        )
    } else if let Some(claims) = input["claims"].as_array() {
        (
            "assessment",
            serde_json::json!({"assessments":claims.iter().map(|claim| serde_json::json!({
            "claim_id":claim["claim_id"],"verdict":"supported","confidence":1.0,
            "citations":[{"scope":"direct","quote":"pub fn run"}],
            "rationale":"controlled source declaration"
        })).collect::<Vec<_>>()})
            .to_string(),
        )
    } else if let Some(sources) = input["sources"].as_array() {
        ("correspondence", serde_json::json!({"complete":true,"claims":sources.iter().map(|source| serde_json::json!({
            "source_claim_id":source["claim"]["claim_id"],
            "readme_claim_ids":input["readme_claims"].as_array().unwrap().iter().map(|claim| claim["claim_id"].clone()).collect::<Vec<_>>(),
            "confidence":1.0,"rationale":"controlled correspondence"
        })).collect::<Vec<_>>()}).to_string())
    } else {
        ("draft", README.into())
    }
}

/// Read published owner evidence without opening the child's mutable store.
#[derive(Clone)]
pub(super) struct ObservationReader {
    pub scope: meld_world_model::world_state::graph::contracts::OwnerPublicationScope,
    events: meld_events::EventReplayCapability,
}
impl ObservationReader {
    pub fn for_assembly(assembly: &super::ProductRuntimeAssembly) -> Arc<Self> {
        Arc::new(Self {
            scope: meld_world_model::world_state::graph::contracts::OwnerPublicationScope {
                scope_id: assembly
                    .physical_binding
                    .as_ref()
                    .unwrap()
                    .assignment_scope_id(),
                branch_id: Some("main".into()),
                perspective_id: Some("default".into()),
                valid_at: None,
            },
            events: assembly.event_authority().replay_capability(),
        })
    }
    pub fn revision(
        &self,
        id: &str,
    ) -> Result<Option<meld_docs_owner::docs::publication::DocsObservationRevision>, String> {
        Ok(self
            .revisions()?
            .into_iter()
            .find(|revision| revision.revision_id == id))
    }
    pub fn current_revision(
        &self,
    ) -> Result<Option<meld_docs_owner::docs::publication::DocsObservationRevision>, String> {
        Ok(self.revisions()?.pop())
    }
    pub fn descends_from(&self, revision: &str, ancestor: &str) -> Result<bool, String> {
        let revisions = self.revisions()?;
        let mut cursor = revision;
        loop {
            if cursor == ancestor {
                return Ok(true);
            }
            let Some(prior) = revisions
                .iter()
                .find(|item| item.revision_id == cursor)
                .and_then(|item| item.predecessor.as_deref())
            else {
                return Ok(false);
            };
            cursor = prior;
        }
    }
    fn revisions(
        &self,
    ) -> Result<Vec<meld_docs_owner::docs::publication::DocsObservationRevision>, String> {
        let mut cursor = meld_events::LedgerCursor {
            ledger_id: self.events.ledger_identity(),
            after_seq: 0,
        };
        let mut revisions = Vec::new();
        loop {
            let page = self
                .events
                .replay(meld_events::ReplayRequest {
                    cursor,
                    limit: 1024,
                })
                .map_err(|e| e.to_string())?;
            for record in &page.records {
                if record.event_type != "docs.observation"
                    || record.stream_id != self.scope.scope_id
                {
                    continue;
                }
                let operation: meld_world_model::world_state::graph::contracts::OwnerPublicationOperation = serde_json::from_value(record.data.clone()).map_err(|e| e.to_string())?;
                for object in operation.batch.objects {
                    if object.object_ref.object_kind == "scope_observation" {
                        revisions.push(
                            serde_json::from_str(
                                object
                                    .qualifications
                                    .get("observation")
                                    .ok_or("observation absent")?,
                            )
                            .map_err(|e| e.to_string())?,
                        );
                    }
                }
            }
            if page.records.is_empty() || page.next_cursor.after_seq >= page.coverage.tip_seq {
                break;
            }
            cursor = page.next_cursor;
        }
        Ok(revisions)
    }
}

/// Scripted inference crosses the same provider callback as production owners.
pub(super) struct JudgeProvider(
    pub Arc<dyn meld_docs_owner::docs::claim_validation::DocsClaimJudge>,
);
#[async_trait::async_trait]
impl crate::provider::ProviderCompletionPort for JudgeProvider {
    async fn complete_provider_request(
        &self,
        _request: &crate::context::generation::contracts::GenerationOrchestrationRequest,
        messages: Vec<crate::provider::ChatMessage>,
        _context: Option<&crate::execution::ExecutionEventContext>,
    ) -> Result<crate::provider::ProviderCompletion, crate::error::ApiError> {
        use meld_docs_owner::docs::{
            capability::*, claim_validation::*, correspondence::*, source_claims::*,
        };
        let input: serde_json::Value = serde_json::from_str(&messages[1].content).unwrap();
        let policy = meld_docs_owner::docs::claim_observation::test_support::policy();
        let failure = |e: meld_docs_owner::error::ApiError| {
            crate::error::ApiError::ProviderError(e.to_string())
        };
        let content = if input.get("source").is_some() {
            let source = serde_json::from_value(input["source"].clone()).unwrap();
            let proposed = self
                .0
                .extract_source(&DocsSourceClaimRequest {
                    source: &source,
                    policy: &policy,
                })
                .await
                .map_err(failure)?;
            serde_json::json!({"complete": proposed.complete, "no_claims_reason": proposed.no_claims_reason,
                "claims": proposed.claims.iter().map(|claim| serde_json::json!({"statement":claim.statement,"confidence":claim.confidence,"quotes":claim.quotes})).collect::<Vec<_>>()})
        } else if input.get("sources").is_some() {
            let owned: Vec<(String, ObservedSourceClaim)> = input["sources"]
                .as_array()
                .unwrap()
                .iter()
                .map(|value| {
                    (
                        value["path"].as_str().unwrap().into(),
                        serde_json::from_value(value["claim"].clone()).unwrap(),
                    )
                })
                .collect();
            let sources: Vec<_> = owned
                .iter()
                .map(|(path, claim)| CorrespondenceSource { path, claim })
                .collect();
            let claims: Vec<ReadmeClaim> =
                serde_json::from_value(input["readme_claims"].clone()).unwrap();
            serde_json::to_value(
                self.0
                    .correspond(&DocsCorrespondenceRequest {
                        readme_path: input["readme_path"].as_str().unwrap(),
                        sources: &sources,
                        readme_claims: &claims,
                        policy: &policy,
                    })
                    .await
                    .map_err(failure)?,
            )
            .unwrap()
        } else {
            let directory = DirectoryEvidence {
                path: input["directory"].as_str().unwrap().into(),
                direct_files: vec![],
                child_directories: vec![],
                evidence: String::new(),
            };
            let patch = ReadmePatch {
                path: input["readme_path"].as_str().unwrap().into(),
                content: input["readme_context"].as_str().unwrap().into(),
                content_hash: input["readme_content_hash"].as_str().unwrap().into(),
            };
            let evidence = EvidencePartitions {
                inventory: input["inventory"].as_str().unwrap().into(),
                direct: input["direct_evidence"].as_str().unwrap().into(),
                descendant: input["descendant_evidence"].as_str().unwrap().into(),
            };
            let claims: Vec<ReadmeClaim> = serde_json::from_value(input["claims"].clone()).unwrap();
            let assessments = self
                .0
                .assess(&DocsClaimJudgmentRequest {
                    policy: &policy,
                    directory: &directory,
                    patch: &patch,
                    evidence: &evidence,
                    claims: &claims,
                    revision_attempt: 0,
                    batch_index: 0,
                })
                .await
                .map_err(failure)?;
            serde_json::json!({"assessments":assessments.iter().map(|a| serde_json::json!({"claim_id":a.claim_id,"verdict":a.verdict,"confidence":a.confidence,"citations":a.citations,"rationale":a.rationale})).collect::<Vec<_>>()})
        };
        Ok(crate::provider::ProviderCompletion {
            preparation: crate::provider::ProviderExecutionDescription {
                provider_type: "fixture".into(),
                requested_model: "fixture".into(),
                configuration_identity: "fixture-judge".into(),
            },
            response: crate::provider::CompletionResponse {
                content: content.to_string(),
                model: "fixture".into(),
                usage: crate::provider::TokenUsage {
                    prompt_tokens: 1,
                    completion_tokens: 1,
                    total_tokens: 2,
                },
                finish_reason: Some("stop".into()),
                execution_metadata: Default::default(),
            },
        })
    }
}

#[path = "../../../owners/test_support.rs"]
mod owner_fixture;

pub(crate) fn select_owners(
    bindings: &mut std::collections::BTreeMap<String, crate::config::PhysicalBindingRef>,
) {
    bindings.extend([
        owner_fixture::selection(
            "docs",
            "meld-docs-owner",
            &["workspace", "provider", "agent", "subject"],
        ),
        owner_fixture::selection(
            "dependency-security",
            "meld-dependency-security-owner",
            &[
                "workspace",
                "agent",
                "subject",
                "dependency-security.cargo",
                "dependency-security.advisories",
            ],
        ),
    ]);
}
pub(crate) fn configure_stores(
    stores: &mut crate::runtime::storage::OpenProductStores,
    root: &std::path::Path,
) {
    owner_fixture::configure(
        stores,
        root,
        &[
            ("docs", "meld-docs-owner"),
            ("dependency-security", "meld-dependency-security-owner"),
        ],
    );
}
