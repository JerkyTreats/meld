//! Deterministic HTTP provider used by native runtime and Docs product proofs.

use serde_json::Value;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::Path;
use std::thread;

const WORKSPACE_ROOT_MARKER: &str = "<workspace>";
const SHUTDOWN_REQUEST_LINE: &str = "POST /__parity_shutdown";

pub struct DeterministicDocsProvider {
    endpoint: String,
    address: SocketAddr,
    handle: thread::JoinHandle<usize>,
}

impl DeterministicDocsProvider {
    /// Spawns the provider for a run rooted at `workspace_root`. The root
    /// is only used to normalize absolute node paths out of prompts.
    pub fn spawn(workspace_root: &Path) -> Self {
        Self::spawn_with_mode(workspace_root, false)
    }

    /// Spawns a deterministic provider for the current routed docs PDS.
    ///
    /// The routed capability validates generated README claims through the
    /// claim-assessment protocol rather than the legacy workflow verifier
    /// prompt. This mode cites one exact source line so deterministic guards
    /// exercise the real acceptance path.
    pub fn spawn_for_routed_validation(workspace_root: &Path) -> Self {
        Self::spawn_with_mode(workspace_root, true)
    }

    fn spawn_with_mode(workspace_root: &Path, routed_validation: bool) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let endpoint = format!("http://{address}");
        let workspace_marker = workspace_root.display().to_string();

        let handle = thread::spawn(move || {
            let mut handled = 0usize;
            loop {
                let (mut stream, _) = listener.accept().unwrap();
                let request = read_http_request(&mut stream);
                let request_text = String::from_utf8_lossy(&request);
                if request_text.starts_with(SHUTDOWN_REQUEST_LINE) {
                    let _ =
                        stream.write_all(b"HTTP/1.1 204 No Content\r\ncontent-length: 0\r\n\r\n");
                    break;
                }
                let completion = if routed_validation {
                    routed_validation_completion(&request, &workspace_marker)
                } else {
                    deterministic_completion(&request, &workspace_marker)
                };
                let response_body = format!(
                    r#"{{"id":"parity","object":"chat.completion","created":0,"model":"test-model","choices":[{{"index":0,"message":{{"role":"assistant","content":{}}},"finish_reason":"stop"}}],"usage":{{"prompt_tokens":1,"completion_tokens":1,"total_tokens":2}}}}"#,
                    serde_json::to_string(&completion).unwrap()
                );
                let response = format!(
                    "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{}",
                    response_body.len(),
                    response_body
                );
                stream.write_all(response.as_bytes()).unwrap();
                handled += 1;
            }
            handled
        });

        Self {
            endpoint,
            address,
            handle,
        }
    }

    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Stops the provider and returns the number of chat requests served.
    pub fn shutdown(self) -> usize {
        let mut stream = TcpStream::connect(self.address).unwrap();
        stream
            .write_all(
                format!("{SHUTDOWN_REQUEST_LINE} HTTP/1.1\r\ncontent-length: 0\r\n\r\n").as_bytes(),
            )
            .unwrap();
        let _ = stream.read(&mut [0u8; 64]);
        self.handle.join().unwrap()
    }
}

fn routed_validation_completion(request: &[u8], workspace_marker: &str) -> String {
    let body_start = find_header_end(request).expect("provider request missing header block");
    let body: Value =
        serde_json::from_slice(&request[body_start..]).expect("provider request body is not JSON");
    let user_content = body
        .get("messages")
        .and_then(Value::as_array)
        .and_then(|messages| {
            messages
                .iter()
                .rev()
                .find(|message| message.get("role").and_then(Value::as_str) == Some("user"))
        })
        .and_then(|message| message.get("content").and_then(Value::as_str))
        .expect("provider request has no user message");

    if let Some(claim_json) = user_content
        .strip_prefix("Assess every claim in this JSON array:\n")
        .and_then(|remaining| {
            remaining
                .split_once("\n\nEvidence inventory:")
                .map(|pair| pair.0)
        })
    {
        let claims: Vec<Value> = serde_json::from_str(claim_json).unwrap();
        let assessments = claims
            .into_iter()
            .map(|claim| {
                let claim_id = claim["claim_id"].as_str().unwrap();
                let statement = claim["statement"].as_str().unwrap();
                serde_json::json!({
                    "claim_id": claim_id,
                    "verdict": "supported",
                    "confidence": 1.0,
                    "citations": [{"scope": "direct", "quote": statement}],
                    "rationale": "Exact direct source line"
                })
            })
            .collect::<Vec<_>>();
        return serde_json::json!({"assessments": assessments}).to_string();
    }

    let normalized = user_content.replace(workspace_marker, WORKSPACE_ROOT_MARKER);
    let digest = stable_hash_hex(normalized.as_bytes());
    let direct = normalized
        .split_once("Direct source evidence:\n")
        .and_then(|pair| {
            pair.1
                .split_once("\n\nDescendant README evidence:")
                .map(|pair| pair.0)
        })
        .unwrap_or("");
    let source_line = direct
        .lines()
        .map(str::trim)
        .find(|line| {
            !line.is_empty()
                && !line.starts_with("--- ")
                && !line.starts_with('#')
                && !line.starts_with("```")
        })
        .unwrap_or("Source files are present in this directory.");
    format!("# Fixture Module {digest}\n\n## Source evidence\n\n{source_line}\n")
}

/// Locates the end of an HTTP header block in a raw request buffer.
fn find_header_end(buffer: &[u8]) -> Option<usize> {
    buffer
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|index| index + 4)
}

/// Reads one HTTP request (headers plus content-length body) from a stream.
fn read_http_request(stream: &mut TcpStream) -> Vec<u8> {
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 4096];
    let mut header_end = None;

    loop {
        let read = stream.read(&mut chunk).unwrap();
        if read == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..read]);
        if header_end.is_none() {
            header_end = find_header_end(&buffer);
        }
        if let Some(end) = header_end {
            let headers = String::from_utf8_lossy(&buffer[..end]);
            let content_length = headers
                .lines()
                .find_map(|line| {
                    let lower = line.to_ascii_lowercase();
                    lower
                        .strip_prefix("content-length:")
                        .and_then(|value| value.trim().parse::<usize>().ok())
                })
                .unwrap_or(0);
            if buffer.len() >= end + content_length {
                break;
            }
        }
    }

    buffer
}

/// Derives the deterministic completion for one chat request. The turn is
/// classified by its prompt text (the same discriminator the canned
/// docs-writer server uses); the payload is a pure function of the
/// normalized final user message, so identical inputs always produce
/// identical outputs and distinct folder contexts produce distinct ones.
fn deterministic_completion(request: &[u8], workspace_marker: &str) -> String {
    let body_start = find_header_end(request).expect("provider request missing header block");
    let body: Value =
        serde_json::from_slice(&request[body_start..]).expect("provider request body is not JSON");
    let user_content = body
        .get("messages")
        .and_then(Value::as_array)
        .and_then(|messages| {
            messages
                .iter()
                .rev()
                .find(|message| message.get("role").and_then(Value::as_str) == Some("user"))
        })
        .and_then(|message| message.get("content").and_then(Value::as_str))
        .expect("provider request has no user message")
        .to_string();

    let normalized = user_content.replace(workspace_marker, WORKSPACE_ROOT_MARKER);
    let digest = stable_hash_hex(normalized.as_bytes());

    if normalized.contains("Build evidence for README generation") {
        format!(
            r#"{{"claims":[{{"claim_id":"claim-{digest}","statement":"Deterministic evidence {digest}.","evidence_path":"fixture://{digest}","evidence_symbol":"symbol_{digest}","evidence_quote":"quote {digest}"}}]}}"#
        )
    } else if normalized.contains("Validate each claim against the provided evidence") {
        format!(
            r#"{{"verified_claims":[{{"claim_id":"claim-{digest}","statement":"Deterministic verified claim {digest}.","evidence_path":"fixture://{digest}","evidence_symbol":"symbol_{digest}","evidence_quote":"quote {digest}"}}],"rejected_claims":[],"reasons":[]}}"#
        )
    } else if normalized.contains("Build a structured README draft") {
        format!(
            r#"{{"title":"Fixture Module {digest}","purpose":"Deterministic purpose {digest}.","usage":"Deterministic usage {digest}."}}"#
        )
    } else {
        format!(
            "# Fixture Module {digest}\n\n## Purpose\n\nDeterministic purpose {digest}.\n\n## Usage\n\nDeterministic usage {digest}.\n"
        )
    }
}

/// FNV-1a over the normalized request content. Stability across runs and
/// platforms is all that matters here; this is not a security hash.
fn stable_hash_hex(bytes: &[u8]) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}
