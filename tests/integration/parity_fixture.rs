//! Parity fixtures for the product convergence proof.
//!
//! The convergence proof compares the flywheel route against the existing
//! docs writer workflow route over an equivalent fixture. This module owns
//! the shared preparation surface for that comparison:
//!
//! - a deterministic branching workspace builder with configurable depth
//!   and breadth plus a richer default shape,
//! - a deterministic provider served through the same `local_custom`
//!   provider surface real runs use (an OpenAI-compatible chat endpoint
//!   resolved from provider config, invoked via
//!   `provider_execute_chat`), whose completions are a pure function of
//!   the normalized request content,
//! - a workflow-route baseline runner that drives the registered
//!   `docs_writer_thread_v1` package to completion and captures the
//!   produced `README.md` path set and contents,
//! - expected-filesystem assertion helpers with readable diffs.
//!
//! Determinism note: node paths embedded in directory prompts are
//! workspace-root absolute, so the provider replaces the concrete
//! workspace root with a fixed marker before deriving content. Everything
//! downstream of that normalization is a pure function of the fixture
//! tree, which is what makes two independent runs byte-identical.

use crate::integration::{
    create_test_agent, create_test_provider, register_docs_writer_capabilities, with_xdg_env,
};
use meld::capability::{CapabilityCatalog, CapabilityExecutorRegistry};
use meld::cli::{Commands, RunContext};
use meld::provider::{ProviderExecutionBinding, ProviderRuntimeOverrides};
use meld::task::{
    execute_task_to_completion, prepare_registered_workflow_task_run, TaskExecutor,
    WorkflowPackageTriggerRequest,
};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::thread;
use tempfile::TempDir;

/// Agent identity expected by the registered docs writer workflow.
pub const PARITY_AGENT_ID: &str = "docs-writer";
/// Provider config name the baseline run binds through.
pub const PARITY_PROVIDER_NAME: &str = "parity-provider";
/// Registered workflow the baseline route executes.
pub const PARITY_WORKFLOW_ID: &str = "docs_writer_thread_v1";
/// Task package that lowers the workflow into capability instances.
pub const PARITY_PACKAGE_ID: &str = "docs_writer";
/// Frame type the docs writer route publishes under.
pub const PARITY_FRAME_TYPE: &str = "context-docs-writer";
/// Provider turns per actionable folder in `docs_writer_thread_v1`.
pub const PARITY_TURNS_PER_FOLDER: usize = 4;

/// Marker substituted for the concrete workspace root before the
/// deterministic provider derives content from a request.
const WORKSPACE_ROOT_MARKER: &str = "<workspace>";

/// Folder names the workspace scan walker excludes from the tree, making
/// them non-actionable for the docs writer route.
const NON_ACTIONABLE_FOLDER_NAMES: &[&str] = &[".git", "target", "node_modules", ".cargo"];

const SHUTDOWN_REQUEST_LINE: &str = "POST /__parity_shutdown";

/// A deterministic file inside the fixture workspace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParityFileSpec {
    pub name: String,
    pub content: String,
}

impl ParityFileSpec {
    pub fn new(name: &str, content: &str) -> Self {
        Self {
            name: name.to_string(),
            content: content.to_string(),
        }
    }
}

/// A deterministic folder inside the fixture workspace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParityFolderSpec {
    pub name: String,
    pub files: Vec<ParityFileSpec>,
    pub folders: Vec<ParityFolderSpec>,
}

impl ParityFolderSpec {
    pub fn new(name: &str, files: Vec<ParityFileSpec>, folders: Vec<ParityFolderSpec>) -> Self {
        Self {
            name: name.to_string(),
            files,
            folders,
        }
    }
}

/// Deterministic branching workspace specification. The whole shape lives
/// under one target folder so the workflow trigger can address it by path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParityWorkspaceSpec {
    pub target: ParityFolderSpec,
}

impl ParityWorkspaceSpec {
    /// Default proof shape: three folder levels, sibling fan-out of three
    /// actionable folders under the target, leaf-only folders (files
    /// without subfolders), a folder mixing files and subfolders, an empty
    /// folder, and one non-actionable folder (`target`, excluded by the
    /// scan walker's built-in ignore list).
    pub fn default_shape() -> Self {
        let target = ParityFolderSpec::new(
            "tree",
            vec![ParityFileSpec::new(
                "manifest.txt",
                "parity fixture manifest\n",
            )],
            vec![
                ParityFolderSpec::new(
                    "alpha",
                    vec![],
                    vec![
                        ParityFolderSpec::new(
                            "core",
                            vec![ParityFileSpec::new(
                                "engine.rs",
                                "pub fn ignite() -> &'static str { \"alpha core\" }\n",
                            )],
                            vec![],
                        ),
                        ParityFolderSpec::new(
                            "util",
                            vec![ParityFileSpec::new(
                                "strings.rs",
                                "pub fn shout(s: &str) -> String { s.to_uppercase() }\n",
                            )],
                            vec![],
                        ),
                    ],
                ),
                ParityFolderSpec::new(
                    "beta",
                    vec![ParityFileSpec::new(
                        "notes.txt",
                        "beta module operational notes\n",
                    )],
                    vec![ParityFolderSpec::new(
                        "adapters",
                        vec![
                            ParityFileSpec::new(
                                "http_adapter.rs",
                                "pub fn fetch(url: &str) -> String { url.to_string() }\n",
                            ),
                            ParityFileSpec::new(
                                "queue_adapter.rs",
                                "pub fn enqueue(item: &str) -> usize { item.len() }\n",
                            ),
                        ],
                        vec![],
                    )],
                ),
                ParityFolderSpec::new(
                    "gamma",
                    vec![ParityFileSpec::new(
                        "overview.txt",
                        "gamma subsystem overview\n",
                    )],
                    vec![ParityFolderSpec::new("hollow", vec![], vec![])],
                ),
                ParityFolderSpec::new(
                    "target",
                    vec![ParityFileSpec::new("cache.bin", "ignored build cache\n")],
                    vec![],
                ),
            ],
        );
        Self { target }
    }

    /// Uniform branching shape: `depth` folder levels below the target,
    /// `breadth` sibling folders per level, one deterministic file per
    /// folder whose content is derived from the folder's logical label.
    pub fn branching(depth: usize, breadth: usize) -> Self {
        Self {
            target: branching_folder("root".to_string(), 0, depth, breadth),
        }
    }

    /// Relative trigger path for the workflow route.
    pub fn target_path(&self) -> PathBuf {
        PathBuf::from(&self.target.name)
    }

    /// Writes the specified tree under `workspace_root`, creating it.
    pub fn write_to(&self, workspace_root: &Path) {
        fs::create_dir_all(workspace_root).unwrap();
        write_folder(workspace_root, &self.target);
    }

    /// Relative paths of every folder the docs writer route treats as
    /// actionable: each non-ignored directory reachable from the target.
    pub fn actionable_directories(&self) -> BTreeSet<String> {
        let mut directories = BTreeSet::new();
        collect_actionable(&self.target, "", &mut directories);
        directories
    }

    /// Relative `README.md` paths the workflow route is expected to
    /// publish over this shape.
    pub fn expected_readme_paths(&self) -> BTreeSet<String> {
        self.actionable_directories()
            .into_iter()
            .map(|directory| format!("{directory}/README.md"))
            .collect()
    }
}

fn branching_folder(label: String, level: usize, depth: usize, breadth: usize) -> ParityFolderSpec {
    let mut folders = Vec::new();
    if level < depth {
        for index in 0..breadth {
            folders.push(branching_folder(
                format!("{label}_{index}"),
                level + 1,
                depth,
                breadth,
            ));
        }
    }
    ParityFolderSpec {
        name: if level == 0 {
            "tree".to_string()
        } else {
            format!("branch_{label}")
        },
        files: vec![ParityFileSpec::new(
            &format!("module_{label}.rs"),
            &format!("pub fn feature_{label}() -> &'static str {{ \"{label}\" }}\n"),
        )],
        folders,
    }
}

fn write_folder(parent: &Path, folder: &ParityFolderSpec) {
    let folder_path = parent.join(&folder.name);
    fs::create_dir_all(&folder_path).unwrap();
    for file in &folder.files {
        fs::write(folder_path.join(&file.name), &file.content).unwrap();
    }
    for child in &folder.folders {
        write_folder(&folder_path, child);
    }
}

fn collect_actionable(folder: &ParityFolderSpec, prefix: &str, out: &mut BTreeSet<String>) {
    if NON_ACTIONABLE_FOLDER_NAMES.contains(&folder.name.as_str()) {
        return;
    }
    let path = if prefix.is_empty() {
        folder.name.clone()
    } else {
        format!("{prefix}/{}", folder.name)
    };
    out.insert(path.clone());
    for child in &folder.folders {
        collect_actionable(child, &path, out);
    }
}

/// Snapshot of every regular file under a root, keyed by relative path.
/// Used to prove the workspace builder itself is byte-deterministic.
pub fn capture_file_snapshot(root: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut snapshot = BTreeMap::new();
    collect_files(root, root, &mut |relative, bytes| {
        snapshot.insert(relative, bytes);
    });
    snapshot
}

fn collect_files(root: &Path, current: &Path, visit: &mut impl FnMut(String, Vec<u8>)) {
    let mut entries: Vec<_> = fs::read_dir(current)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            collect_files(root, &path, visit);
        } else {
            let relative = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            visit(relative, fs::read(&path).unwrap());
        }
    }
}

/// Captured README publication baseline: the set of relative `README.md`
/// paths under a workspace root and their exact contents.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ReadmeBaseline {
    entries: BTreeMap<String, String>,
}

impl ReadmeBaseline {
    /// Captures every `README.md` under `workspace_root`.
    pub fn capture(workspace_root: &Path) -> Self {
        let mut entries = BTreeMap::new();
        collect_files(workspace_root, workspace_root, &mut |relative, bytes| {
            if relative == "README.md" || relative.ends_with("/README.md") {
                entries.insert(relative, String::from_utf8(bytes).unwrap());
            }
        });
        Self { entries }
    }

    /// Builds a baseline from explicit entries, for assertion-helper tests
    /// and hand-built expectations.
    pub fn from_entries<I, P, C>(entries: I) -> Self
    where
        I: IntoIterator<Item = (P, C)>,
        P: Into<String>,
        C: Into<String>,
    {
        Self {
            entries: entries
                .into_iter()
                .map(|(path, content)| (path.into(), content.into()))
                .collect(),
        }
    }

    pub fn readme_paths(&self) -> BTreeSet<String> {
        self.entries.keys().cloned().collect()
    }

    pub fn content(&self, path: &str) -> Option<&str> {
        self.entries.get(path).map(String::as_str)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Readable differences from this baseline (expected) to `actual`.
    /// Empty means path sets and contents match exactly.
    pub fn diff(&self, actual: &Self) -> Vec<String> {
        let mut differences = Vec::new();
        for (path, expected_content) in &self.entries {
            match actual.entries.get(path) {
                None => differences.push(format!("missing README '{path}'")),
                Some(actual_content) if actual_content != expected_content => {
                    differences.push(format!(
                        "content mismatch at '{path}':\n--- expected ---\n{expected_content}\n--- actual ---\n{actual_content}"
                    ));
                }
                Some(_) => {}
            }
        }
        for path in actual.entries.keys() {
            if !self.entries.contains_key(path) {
                differences.push(format!("unexpected README '{path}'"));
            }
        }
        differences
    }
}

/// Asserts `actual` matches the `expected` baseline, panicking with the
/// readable path-set and content differences on mismatch.
pub fn assert_baselines_match(expected: &ReadmeBaseline, actual: &ReadmeBaseline) {
    let differences = expected.diff(actual);
    assert!(
        differences.is_empty(),
        "README baseline mismatch ({} difference(s)):\n{}",
        differences.len(),
        differences.join("\n")
    );
}

/// Asserts the filesystem under `workspace_root` matches a captured
/// baseline: README path-set equality and byte content equality.
pub fn assert_workspace_matches_baseline(workspace_root: &Path, expected: &ReadmeBaseline) {
    assert_baselines_match(expected, &ReadmeBaseline::capture(workspace_root));
}

/// Deterministic docs-writer provider behind the same `local_custom`
/// provider surface real runs resolve: an OpenAI-compatible
/// chat-completions endpoint. Every completion is a pure function of the
/// request's final user message after workspace-root normalization, so
/// equivalent fixtures yield byte-identical turn outputs across runs.
pub struct DeterministicDocsProvider {
    endpoint: String,
    address: SocketAddr,
    handle: thread::JoinHandle<usize>,
}

impl DeterministicDocsProvider {
    /// Spawns the provider for a run rooted at `workspace_root`. The root
    /// is only used to normalize absolute node paths out of prompts.
    pub fn spawn(workspace_root: &Path) -> Self {
        Self::spawn_with_evidence_sabotage(workspace_root, 0)
    }

    /// Spawns the provider with the first `sabotage_first` evidence-gather
    /// completions replaced by prose that satisfies no gate — the
    /// transient-bad-output shape the execute retry budget exists for.
    /// Pass a number larger than any plausible attempt budget to model a
    /// permanently failing turn.
    pub fn spawn_with_evidence_sabotage(workspace_root: &Path, sabotage_first: usize) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let endpoint = format!("http://{address}");
        let workspace_marker = workspace_root.display().to_string();

        let handle = thread::spawn(move || {
            let mut handled = 0usize;
            let mut sabotaged = 0usize;
            loop {
                let (mut stream, _) = listener.accept().unwrap();
                let request = read_http_request(&mut stream);
                let request_text = String::from_utf8_lossy(&request);
                if request_text.starts_with(SHUTDOWN_REQUEST_LINE) {
                    let _ =
                        stream.write_all(b"HTTP/1.1 204 No Content\r\ncontent-length: 0\r\n\r\n");
                    break;
                }
                let is_evidence_request =
                    request_text.contains("Build evidence for README generation");
                let completion = if is_evidence_request && sabotaged < sabotage_first {
                    sabotaged += 1;
                    "I could not locate any citable claims in the provided context.".to_string()
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

/// Outcome of one workflow-route baseline run over a fixture workspace.
pub struct WorkflowRouteBaselineRun {
    /// README paths and contents published into the fixture workspace.
    pub baseline: ReadmeBaseline,
    /// Chat requests served by the deterministic provider.
    pub provider_requests: usize,
    /// Capability instances completed by the task run.
    pub completed_instances: usize,
    /// Capability instances in the fully expanded compiled task.
    pub capability_instances: usize,
}

/// Runs the existing registered docs writer workflow route to completion
/// over a fresh workspace built from `spec`, using the deterministic
/// provider through the real provider binding, and captures the README
/// publication baseline. Each call is fully isolated: fresh XDG homes,
/// fresh workspace, fresh stores.
pub fn run_workflow_route_baseline(spec: &ParityWorkspaceSpec) -> WorkflowRouteBaselineRun {
    try_run_workflow_route(spec, 0).expect("workflow route baseline run failed")
}

/// Fallible workflow-route run with the first `sabotage_first` evidence
/// completions replaced by gate-failing prose. The baseline wrapper runs
/// with zero sabotage; gate-retry tests pass small counts to prove the
/// execute retry budget heals transients, and large counts to prove
/// exhausted budgets fail terminally.
pub fn try_run_workflow_route(
    spec: &ParityWorkspaceSpec,
    sabotage_first: usize,
) -> Result<WorkflowRouteBaselineRun, String> {
    let temp_dir = TempDir::new().unwrap();
    with_xdg_env(&temp_dir, || {
        meld::init::initialize_workflows(false).unwrap();

        let workspace_root = temp_dir.path().join("workspace");
        spec.write_to(&workspace_root);

        create_test_agent(PARITY_AGENT_ID, Some(PARITY_WORKFLOW_ID));
        let provider = DeterministicDocsProvider::spawn_with_evidence_sabotage(
            &workspace_root,
            sabotage_first,
        );
        create_test_provider(PARITY_PROVIDER_NAME, provider.endpoint());

        let run_context = RunContext::new(workspace_root.clone(), None).unwrap();
        run_context
            .execute(&Commands::Scan { force: true })
            .unwrap();

        let registered_profile = run_context
            .workflow_registry()
            .read()
            .get(PARITY_WORKFLOW_ID)
            .unwrap()
            .clone();
        let mut catalog = CapabilityCatalog::new();
        let mut registry = CapabilityExecutorRegistry::new();
        register_docs_writer_capabilities(&mut catalog, &mut registry);

        let prepared = prepare_registered_workflow_task_run(
            run_context.api(),
            &workspace_root,
            &registered_profile,
            &WorkflowPackageTriggerRequest {
                package_id: PARITY_PACKAGE_ID.to_string(),
                workflow_id: PARITY_WORKFLOW_ID.to_string(),
                node_id: None,
                path: Some(spec.target_path()),
                agent_id: PARITY_AGENT_ID.to_string(),
                provider: ProviderExecutionBinding::new(
                    PARITY_PROVIDER_NAME,
                    ProviderRuntimeOverrides::default(),
                )
                .unwrap(),
                frame_type: PARITY_FRAME_TYPE.to_string(),
                belief_family_id: None,
                force: true,
                session_id: Some("session_parity_baseline".to_string()),
            },
            &catalog,
        )
        .unwrap();

        let mut executor = TaskExecutor::new(
            prepared.compiled_task.clone(),
            prepared.init_payload.clone(),
            "repo_parity_baseline",
        )
        .unwrap();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(execute_task_to_completion(
            run_context.api(),
            &mut executor,
            &catalog,
            &registry,
            None,
            None,
        ));

        let provider_requests = provider.shutdown();

        match result {
            Ok(summary) => Ok(WorkflowRouteBaselineRun {
                baseline: ReadmeBaseline::capture(&workspace_root),
                provider_requests,
                completed_instances: summary.completed_instances,
                capability_instances: executor.compiled_task().capability_instances.len(),
            }),
            Err(error) => Err(error.to_string()),
        }
    })
}

/// Outcome of the four-phase incremental staleness scenario.
pub struct IncrementalScenarioOutcome {
    /// Provider requests served by the initial forced full run.
    pub full_run_requests: usize,
    /// Provider requests served by an unforced run with published READMEs
    /// still present in the workspace.
    pub feedback_requests: usize,
    /// Error from the unforced run after published READMEs were removed.
    pub zero_work_error: String,
    /// Provider requests served by the unforced run after one source
    /// mutation, with published READMEs removed.
    pub mutation_requests: usize,
}

/// Runs four workflow-route phases over ONE workspace to separate the
/// staleness machinery from the published-artifact feedback blocker:
///
/// 1. forced full generation;
/// 2. unforced rerun with published READMEs in place — pins the blocker:
///    publication mutated the tree, every node re-identified, everything
///    regenerates;
/// 3. published READMEs deleted, rescan — node ids revert to their
///    source-scoped values, every head is found, and the run refuses to
///    fabricate work;
/// 4. one source file mutated — exactly the mutated ancestor chain
///    regenerates.
pub fn run_incremental_workflow_scenario(
    spec: &ParityWorkspaceSpec,
    mutate: impl FnOnce(&Path),
) -> IncrementalScenarioOutcome {
    let temp_dir = TempDir::new().unwrap();
    with_xdg_env(&temp_dir, || {
        meld::init::initialize_workflows(false).unwrap();

        let workspace_root = temp_dir.path().join("workspace");
        spec.write_to(&workspace_root);
        create_test_agent(PARITY_AGENT_ID, Some(PARITY_WORKFLOW_ID));

        let mut catalog = CapabilityCatalog::new();
        let mut registry = CapabilityExecutorRegistry::new();
        register_docs_writer_capabilities(&mut catalog, &mut registry);
        let rt = tokio::runtime::Runtime::new().unwrap();

        let phase = |force: bool, session_id: &str| -> (usize, Result<(), String>) {
            let provider = DeterministicDocsProvider::spawn(&workspace_root);
            create_test_provider(PARITY_PROVIDER_NAME, provider.endpoint());
            let run_context = RunContext::new(workspace_root.clone(), None).unwrap();
            run_context
                .execute(&Commands::Scan { force: true })
                .unwrap();
            let registered_profile = run_context
                .workflow_registry()
                .read()
                .get(PARITY_WORKFLOW_ID)
                .unwrap()
                .clone();
            let result = prepare_registered_workflow_task_run(
                run_context.api(),
                &workspace_root,
                &registered_profile,
                &WorkflowPackageTriggerRequest {
                    package_id: PARITY_PACKAGE_ID.to_string(),
                    workflow_id: PARITY_WORKFLOW_ID.to_string(),
                    node_id: None,
                    path: Some(spec.target_path()),
                    agent_id: PARITY_AGENT_ID.to_string(),
                    provider: ProviderExecutionBinding::new(
                        PARITY_PROVIDER_NAME,
                        ProviderRuntimeOverrides::default(),
                    )
                    .unwrap(),
                    frame_type: PARITY_FRAME_TYPE.to_string(),
                    belief_family_id: None,
                    force,
                    session_id: Some(session_id.to_string()),
                },
                &catalog,
            )
            .map_err(|error| error.to_string())
            .and_then(|prepared| {
                let mut executor = TaskExecutor::new(
                    prepared.compiled_task.clone(),
                    prepared.init_payload.clone(),
                    format!("repo_{session_id}"),
                )
                .map_err(|error| error.to_string())?;
                rt.block_on(execute_task_to_completion(
                    run_context.api(),
                    &mut executor,
                    &catalog,
                    &registry,
                    None,
                    None,
                ))
                .map(|_| ())
                .map_err(|error| error.to_string())
            });
            (provider.shutdown(), result)
        };

        let (full_run_requests, first) = phase(true, "session_incremental_full");
        first.expect("forced full run must complete");

        let (feedback_requests, second) = phase(false, "session_incremental_feedback");
        second.expect("unforced rerun with READMEs present must complete");

        remove_published_readmes(&workspace_root);
        let (fresh_requests, third) = phase(false, "session_incremental_fresh");
        let zero_work_error =
            third.expect_err("unforced run over a source-fresh workspace must refuse");
        assert_eq!(
            fresh_requests, 0,
            "a zero-work refusal must not consume provider requests"
        );

        mutate(&workspace_root);
        let (mutation_requests, fourth) = phase(false, "session_incremental_delta");
        fourth.expect("unforced run after mutation must complete");

        IncrementalScenarioOutcome {
            full_run_requests,
            feedback_requests,
            zero_work_error,
            mutation_requests,
        }
    })
}

/// Removes every published `README.md` under the workspace tree so node
/// identity reverts to its source-scoped value.
fn remove_published_readmes(workspace_root: &Path) {
    fn walk(dir: &Path) {
        for entry in fs::read_dir(dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path);
            } else if path.file_name().and_then(|name| name.to_str()) == Some("README.md") {
                fs::remove_file(&path).unwrap();
            }
        }
    }
    walk(workspace_root);
}
