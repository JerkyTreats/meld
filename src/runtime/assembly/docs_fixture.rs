//! Scripted local provider for the native repair and owner-return proof.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

pub(super) const README: &str = "# run\n\n`run` exists.\n\n`run` is defined.\n";

pub(super) struct ProviderServer {
    endpoint: String,
    stop: Arc<AtomicBool>,
    calls: Arc<Mutex<Vec<String>>>,
    worker: Option<JoinHandle<()>>,
}

impl ProviderServer {
    pub(super) fn new() -> Self {
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let endpoint = format!("http://{}", server.server_addr());
        let stop = Arc::new(AtomicBool::new(false));
        let calls = Arc::new(Mutex::new(Vec::new()));
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
            worker: Some(worker),
        }
    }

    pub(super) fn endpoint(&self) -> String {
        self.endpoint.clone()
    }

    pub(super) fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
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
