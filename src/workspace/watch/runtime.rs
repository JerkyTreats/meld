//! Watch daemon and runtime logic.

use super::events::{ChangeEvent, EventBatcher, WatchConfig};
use crate::agent::AgentIdentity;
use crate::api::ContextApi;
use crate::context::head::backfill_legacy_heads_into_ledger;
use crate::context::queue::{FrameGenerationQueue, QueueEventContext};
use crate::error::ApiError;
use crate::heads::HeadIndex;
use crate::ignore;
use crate::store::{NodeRecord, NodeRecordStore};
use crate::tree::builder::TreeBuilder;
use crate::tree::path::canonicalize_path;
use crate::tree::walker::WalkerConfig;
use crate::types::NodeID;
use crate::workspace::commands::emit_workspace_snapshot_facts;
use crate::workspace::scan::stored_workspace_root_hash;
use notify::{Event, EventKind, RecursiveMode, Watcher};
use parking_lot::RwLock;
use serde_json::json;
use std::collections::{BTreeSet, HashSet};
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};

struct TreeUpdateOutcome {
    tree: crate::tree::builder::Tree,
    previous_root_hash: Option<String>,
    observed_nodes: Vec<NodeID>,
}

/// Watch mode daemon
pub struct WatchDaemon {
    api: Arc<ContextApi>,
    config: WatchConfig,
    running: Arc<RwLock<bool>>,
    generation_queue: Option<Arc<FrameGenerationQueue>>,
}

impl WatchDaemon {
    /// Create a new watch daemon
    pub fn new(api: Arc<ContextApi>, config: WatchConfig) -> Result<Self, ApiError> {
        let head_index_path = HeadIndex::persistence_path(&config.workspace_root);
        {
            let mut head_index = api.head_index().write();
            if let Ok(loaded) = HeadIndex::load_from_disk(&head_index_path) {
                *head_index = loaded;
                info!(
                    "Loaded head index from disk: {} entries",
                    head_index.heads.len()
                );
            } else {
                info!("Starting with empty head index");
            }
        }
        if let Some(progress) = &config.progress {
            let session_id = config
                .session_id
                .as_deref()
                .unwrap_or("context_head_backfill");
            let head_index = api.head_index().read();
            if let Err(err) = backfill_legacy_heads_into_ledger(
                progress,
                &head_index,
                api.frame_storage(),
                session_id,
            ) {
                warn!(error = %err, "failed to backfill legacy heads into ledger");
            }
        }

        let generation_queue = if config.auto_generate_frames {
            let queue_event_context = match (&config.session_id, &config.progress) {
                (Some(session_id), Some(progress)) => Some(QueueEventContext {
                    session_id: session_id.clone(),
                    progress: Arc::clone(progress),
                }),
                _ => None,
            };
            let queue = Arc::new(FrameGenerationQueue::with_event_context(
                Arc::clone(&api),
                config.generation_config.clone().unwrap_or_default(),
                queue_event_context,
            ));
            queue.start()?;
            info!("Frame generation queue started");
            Some(queue)
        } else {
            None
        };

        Ok(Self {
            api,
            config,
            running: Arc::new(RwLock::new(false)),
            generation_queue,
        })
    }

    /// Start the watch daemon
    pub fn start(&self) -> Result<(), ApiError> {
        *self.running.write() = true;

        info!("Building initial tree");
        self.emit_event_best_effort(
            "watch_started",
            json!({
                "workspace": self.config.workspace_root.to_string_lossy().to_string()
            }),
        );
        self.build_initial_tree()?;
        info!("Initial tree built successfully");

        let (tx, rx) = mpsc::channel();
        let mut watcher = notify::recommended_watcher(move |res| {
            if let Err(e) = tx.send(res) {
                error!("Error sending watch event: {}", e);
            }
        })
        .map_err(|e| {
            ApiError::StorageError(crate::error::StorageError::IoError(std::io::Error::other(
                format!("Failed to create watcher: {}", e),
            )))
        })?;

        watcher
            .watch(&self.config.workspace_root, RecursiveMode::Recursive)
            .map_err(|e| {
                ApiError::StorageError(crate::error::StorageError::IoError(std::io::Error::other(
                    format!("Failed to watch directory: {}", e),
                )))
            })?;

        info!(workspace = ?self.config.workspace_root, "Watching workspace");

        let mut batcher = EventBatcher::new(self.config.clone());
        let batch_window = Duration::from_millis(self.config.batch_window_ms);

        let mut last_batch_time = Instant::now();
        let mut pending_events = Vec::new();

        loop {
            if !*self.running.read() {
                break;
            }

            let timeout = batch_window.saturating_sub(last_batch_time.elapsed());
            match rx.recv_timeout(timeout) {
                Ok(Ok(event)) => {
                    if let Some(change_event) = self.convert_event(event) {
                        if batcher.add_event(change_event.clone()) {
                            pending_events.extend(batcher.take_batch());
                        } else {
                            pending_events.push(change_event);
                        }
                    }
                }
                Ok(Err(e)) => {
                    warn!("Watch error: {}", e);
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    if !pending_events.is_empty() && last_batch_time.elapsed() >= batch_window {
                        self.process_events(std::mem::take(&mut pending_events))?;
                        last_batch_time = Instant::now();
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    error!("Watcher channel disconnected");
                    break;
                }
            }

            if !pending_events.is_empty() && last_batch_time.elapsed() >= batch_window {
                self.process_events(std::mem::take(&mut pending_events))?;
                last_batch_time = Instant::now();
            }
        }

        Ok(())
    }

    /// Stop the watch daemon
    pub async fn stop(&self) -> Result<(), ApiError> {
        *self.running.write() = false;

        if let Some(queue) = &self.generation_queue {
            queue.stop().await?;
        }

        Ok(())
    }

    fn build_initial_tree(&self) -> Result<(), ApiError> {
        let walker_config = WalkerConfig {
            follow_symlinks: false,
            ignore_patterns: self.config.ignore_patterns.clone(),
            max_depth: None,
        };
        let builder =
            TreeBuilder::new(self.config.workspace_root.clone()).with_walker_config(walker_config);
        let tree = builder.build().map_err(ApiError::from)?;
        let previous_root_hash = stored_workspace_root_hash(
            self.api.node_store().as_ref(),
            &self.config.workspace_root,
            &tree.root_id,
        )?;

        NodeRecord::populate_store_from_tree(
            self.api.node_store().as_ref() as &dyn NodeRecordStore,
            &tree,
        )
        .map_err(ApiError::from)?;

        if let (Some(session_id), Some(progress)) = (&self.config.session_id, &self.config.progress)
        {
            let all_node_ids = tree.nodes.keys().copied().collect::<Vec<_>>();
            emit_workspace_snapshot_facts(
                progress,
                session_id,
                &self.config.workspace_root,
                &tree,
                previous_root_hash.as_deref(),
                &all_node_ids,
            )?;
        }

        let _ = ignore::maybe_sync_gitignore_after_tree(
            &self.config.workspace_root,
            tree.find_gitignore_node_id().as_ref(),
        );

        if self.config.auto_create_frames {
            info!("Creating missing contextframes for all nodes");
            let all_node_ids: Vec<NodeID> = tree.nodes.keys().copied().collect();
            self.ensure_agent_frames_batched(&all_node_ids)?;
            info!("Contextframe creation completed");
        }

        Ok(())
    }

    fn convert_event(&self, event: Event) -> Option<ChangeEvent> {
        match event.kind {
            EventKind::Create(_) => event.paths.first().map(|p| ChangeEvent::Created(p.clone())),
            EventKind::Modify(notify::event::ModifyKind::Name(_)) => {
                if event.paths.len() >= 2 {
                    Some(ChangeEvent::Renamed {
                        from: event.paths[0].clone(),
                        to: event.paths[1].clone(),
                    })
                } else if event.paths.len() == 1 {
                    event
                        .paths
                        .first()
                        .map(|p| ChangeEvent::Modified(p.clone()))
                } else {
                    None
                }
            }
            EventKind::Modify(_) => event
                .paths
                .first()
                .map(|p| ChangeEvent::Modified(p.clone())),
            EventKind::Remove(_) => event.paths.first().map(|p| ChangeEvent::Removed(p.clone())),
            _ => None,
        }
    }

    fn process_events(&self, events: Vec<ChangeEvent>) -> Result<(), ApiError> {
        if events.is_empty() {
            return Ok(());
        }

        info!(event_count = events.len(), "Processing change events");
        for event in &events {
            let (kind, path) = match event {
                ChangeEvent::Created(p) => ("created", p.to_string_lossy().to_string()),
                ChangeEvent::Modified(p) => ("modified", p.to_string_lossy().to_string()),
                ChangeEvent::Removed(p) => ("removed", p.to_string_lossy().to_string()),
                ChangeEvent::Renamed { to, .. } => ("renamed", to.to_string_lossy().to_string()),
            };
            self.emit_event_best_effort("file_changed", json!({ "kind": kind, "path": path }));
        }

        let mut affected_paths = HashSet::new();
        for event in &events {
            match event {
                ChangeEvent::Created(p) | ChangeEvent::Modified(p) | ChangeEvent::Removed(p) => {
                    affected_paths.insert(p.clone());
                }
                ChangeEvent::Renamed { from, to } => {
                    affected_paths.insert(from.clone());
                    affected_paths.insert(to.clone());
                }
            }
        }

        let update = self.update_tree_for_paths(&affected_paths)?;

        if let (Some(session_id), Some(progress)) = (&self.config.session_id, &self.config.progress)
        {
            emit_workspace_snapshot_facts(
                progress,
                session_id,
                &self.config.workspace_root,
                &update.tree,
                update.previous_root_hash.as_deref(),
                &update.observed_nodes,
            )?;
        }

        if self.config.auto_create_frames {
            self.ensure_agent_frames_batched(&update.observed_nodes)?;
        }

        info!(
            event_count = events.len(),
            affected_nodes = update.observed_nodes.len(),
            "Processed change events"
        );
        self.emit_event_best_effort(
            "batch_processed",
            json!({ "event_count": events.len(), "affected_nodes": update.observed_nodes.len() }),
        );

        Ok(())
    }

    fn update_tree_for_paths(
        &self,
        paths: &HashSet<PathBuf>,
    ) -> Result<TreeUpdateOutcome, ApiError> {
        let walker_config = WalkerConfig {
            follow_symlinks: false,
            ignore_patterns: self.config.ignore_patterns.clone(),
            max_depth: None,
        };
        let builder =
            TreeBuilder::new(self.config.workspace_root.clone()).with_walker_config(walker_config);
        let tree = builder.build().map_err(ApiError::from)?;
        let previous_root_hash = stored_workspace_root_hash(
            self.api.node_store().as_ref(),
            &self.config.workspace_root,
            &tree.root_id,
        )?;

        let _ = ignore::maybe_sync_gitignore_after_tree(
            &self.config.workspace_root,
            tree.find_gitignore_node_id().as_ref(),
        );

        let affected_nodes = collect_observed_nodes(&tree, paths);

        NodeRecord::populate_store_from_tree(
            self.api.node_store().as_ref() as &dyn NodeRecordStore,
            &tree,
        )
        .map_err(ApiError::from)?;

        Ok(TreeUpdateOutcome {
            tree,
            previous_root_hash,
            observed_nodes: affected_nodes,
        })
    }

    /// Ensure contextframes exist for all agents for the given nodes (batched)
    pub(crate) fn ensure_agent_frames_batched(&self, node_ids: &[NodeID]) -> Result<(), ApiError> {
        if node_ids.is_empty() {
            return Ok(());
        }

        let agents: Vec<AgentIdentity> = {
            let registry = self.api.agent_registry().read();
            registry.list_all().into_iter().cloned().collect()
        };

        if agents.is_empty() {
            warn!(
                "No agents registered, skipping contextframe creation. Please configure agents in your config file."
            );
            return Ok(());
        }

        info!(
            agent_count = agents.len(),
            agents = ?agents.iter().map(|agent| &agent.agent_id).collect::<Vec<_>>(),
            "Found {} agent(s) for contextframe creation",
            agents.len()
        );

        let batch_size = self.config.frame_batch_size;
        let mut created_count = 0;
        let mut skipped_count = 0;

        for chunk in node_ids.chunks(batch_size) {
            for node_id in chunk {
                for agent in &agents {
                    if let Some(workflow_id) = agent.workflow_binding() {
                        self.emit_event_best_effort(
                            "workflow_watch_rejected",
                            json!({
                                "node_id": hex::encode(node_id),
                                "agent_id": agent.agent_id,
                                "workflow_id": workflow_id,
                                "error": crate::workflow::retired_execution_error().to_string()
                            }),
                        );
                        warn!(agent_id = %agent.agent_id, workflow_id, "Legacy Workflow execution is retired; configure a native reconciliation product");
                        continue;
                    }

                    match self.api.ensure_agent_frame(
                        *node_id,
                        agent.agent_id.clone(),
                        None,
                        self.generation_queue.as_ref().map(Arc::clone),
                    ) {
                        Ok(Some(frame_id)) => {
                            created_count += 1;
                            debug!(
                                node_id = %hex::encode(node_id),
                                agent_id = %agent.agent_id,
                                frame_id = %hex::encode(frame_id),
                                "Created contextframe"
                            );
                        }
                        Ok(None) => {
                            skipped_count += 1;
                            debug!(
                                node_id = %hex::encode(node_id),
                                agent_id = %agent.agent_id,
                                "Skipped contextframe (already exists or agent cannot write)"
                            );
                        }
                        Err(e) => {
                            warn!(
                                node_id = %hex::encode(node_id),
                                agent_id = %agent.agent_id,
                                error = %e,
                                "Failed to create contextframe"
                            );
                        }
                    }
                }
            }
        }

        info!(
            node_count = node_ids.len(),
            agent_count = agents.len(),
            created = created_count,
            skipped = skipped_count,
            "Ensured agent contextframes"
        );

        Ok(())
    }

    fn emit_event_best_effort(&self, event_type: &str, data: serde_json::Value) {
        if let (Some(session_id), Some(progress)) = (&self.config.session_id, &self.config.progress)
        {
            progress.emit_event_best_effort(session_id, event_type, data);
        }
    }
}

fn collect_observed_nodes(
    tree: &crate::tree::builder::Tree,
    paths: &HashSet<PathBuf>,
) -> Vec<NodeID> {
    if paths.is_empty() {
        return Vec::new();
    }

    let canonical_paths: Vec<PathBuf> = paths
        .iter()
        .map(|path| canonicalize_path(path).unwrap_or_else(|_| path.clone()))
        .collect();
    let mut observed = BTreeSet::new();
    observed.insert(tree.root_id);

    for (node_id, node) in &tree.nodes {
        let Ok(record) = NodeRecord::from_merkle_node(*node_id, node, tree) else {
            continue;
        };
        let canonical_record_path =
            canonicalize_path(&record.path).unwrap_or_else(|_| record.path.clone());
        if canonical_paths
            .iter()
            .any(|path| canonical_record_path == *path || path.starts_with(&canonical_record_path))
        {
            observed.insert(*node_id);
        }
    }

    observed.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::{AgentIdentity, AgentRegistry, AgentRole};
    use crate::config::{MerkleConfig, ProviderConfig, ProviderType, WorkflowConfig};
    use crate::context::frame::storage::FrameStorage;
    use crate::events::{EventAuthority, EventAuthorityOpenOptions};
    use crate::heads::HeadIndex;
    use crate::prompt_context::PromptContextArtifactStorage;
    use crate::provider::ProviderRegistry;
    use crate::session::{SessionRuntime, SessionStore};
    use crate::store::persistence::SledNodeRecordStore;
    use crate::store::{NodeRecord, NodeType};
    use crate::telemetry::ProgressRuntime;
    use crate::workflow::WorkflowRegistry;
    use crate::workspace::events::WorkspaceNodeObservedEventData;
    use meld_events::events::test_support::{EventStore, EventStoreTestSupport as _}; // boundary-allow: event-test
    use meld_world_model::world_state::graph::contracts::{
        OwnerPublicationOperation, OWNER_PUBLICATION_EVENT_TYPE,
    };
    use std::path::Path;
    use tempfile::TempDir;

    fn open_test_progress(db: sled::Db) -> (Arc<ProgressRuntime>, Arc<EventStore>) {
        let authority =
            EventAuthority::open(db.clone(), EventAuthorityOpenOptions::default()).unwrap();
        let event_store = EventStore::shared(db.clone()).unwrap(); // boundary-allow: event-test
        let session_store = SessionStore::shared(db).unwrap();
        let sessions = Arc::new(SessionRuntime::new(session_store));
        let progress = Arc::new(ProgressRuntime::from_capabilities(
            authority.append_capability(),
            sessions,
        ));
        (progress, event_store)
    }

    fn create_test_api(workspace_root: &Path) -> ContextApi {
        let store_path = workspace_root.join("store");
        let frame_storage_path = workspace_root.join("frames");
        let artifact_storage_path = workspace_root.join("artifacts");
        std::fs::create_dir_all(&frame_storage_path).unwrap();
        std::fs::create_dir_all(&artifact_storage_path).unwrap();

        let node_store = Arc::new(SledNodeRecordStore::new(&store_path).unwrap());
        let frame_storage = Arc::new(FrameStorage::new(&frame_storage_path).unwrap());
        let prompt_context_storage =
            Arc::new(PromptContextArtifactStorage::new(&artifact_storage_path).unwrap());
        let head_index = Arc::new(parking_lot::RwLock::new(HeadIndex::new()));
        let agent_registry = Arc::new(parking_lot::RwLock::new(AgentRegistry::new()));
        let provider_registry = Arc::new(parking_lot::RwLock::new(ProviderRegistry::new()));
        let lock_manager = Arc::new(crate::concurrency::NodeLockManager::new());

        ContextApi::with_workspace_root(
            node_store,
            frame_storage,
            head_index,
            prompt_context_storage,
            agent_registry,
            provider_registry,
            lock_manager,
            workspace_root.to_path_buf(),
        )
    }

    fn put_test_file_node(api: &ContextApi, workspace_root: &Path, node_id: NodeID) {
        let file_path = workspace_root.join("doc.txt");
        std::fs::write(&file_path, "hello").unwrap();
        api.node_store()
            .put(&NodeRecord {
                node_id,
                path: file_path,
                node_type: NodeType::File {
                    size: 5,
                    content_hash: [1u8; 32],
                },
                children: Vec::new(),
                parent: None,
                frame_set_root: None,
                metadata: Default::default(),
                tombstoned_at: None,
            })
            .unwrap();
    }

    fn create_watch_test_runtime(
        temp: &TempDir,
        workspace_root: PathBuf,
    ) -> (WatchDaemon, Arc<ProgressRuntime>, Arc<EventStore>, String) {
        let api = Arc::new(create_test_api(&workspace_root));
        let progress_db = sled::open(temp.path().join("progress")).unwrap();
        let (progress, event_store) = open_test_progress(progress_db);
        let session_id = progress
            .start_command_session("workspace.watch".to_string())
            .unwrap();
        let config = WatchConfig {
            workspace_root,
            session_id: Some(session_id.clone()),
            progress: Some(Arc::clone(&progress)),
            auto_create_frames: false,
            ..WatchConfig::default()
        };
        let daemon = WatchDaemon::new(api, config).unwrap();
        (daemon, progress, event_store, session_id)
    }

    #[test]
    fn watch_startup_reconstructs_and_idempotently_retries_the_exact_revision() {
        let temp = TempDir::new().unwrap();
        let workspace_root = temp.path().join("workspace");
        std::fs::create_dir_all(&workspace_root).unwrap();
        std::fs::write(workspace_root.join("doc.txt"), "hello").unwrap();
        let (daemon, progress, event_store, session_id) =
            create_watch_test_runtime(&temp, workspace_root);

        daemon.build_initial_tree().unwrap();
        progress.barrier().unwrap();
        daemon.build_initial_tree().unwrap();
        progress.barrier().unwrap();

        let owner_events = event_store
            .read_events_after(&session_id, 0)
            .unwrap()
            .into_iter()
            .filter(|event| event.event_type == OWNER_PUBLICATION_EVENT_TYPE)
            .collect::<Vec<_>>();
        assert_eq!(owner_events.len(), 1);
        let operation: OwnerPublicationOperation =
            serde_json::from_value(owner_events[0].data.clone()).unwrap();
        operation.validate().unwrap();
        assert!(operation.batch.objects.iter().any(|publication| publication
            .qualifications
            .get("path")
            .is_some_and(|path| path.ends_with("doc.txt"))));
    }

    #[test]
    fn ensure_agent_frames_keeps_legacy_path_for_unbound_agents() {
        let temp = TempDir::new().unwrap();
        let workspace_root = temp.path().join("workspace");
        std::fs::create_dir_all(&workspace_root).unwrap();
        let api = Arc::new(create_test_api(&workspace_root));
        let node_id = crate::types::Hash::from([7u8; 32]);
        put_test_file_node(api.as_ref(), &workspace_root, node_id);

        {
            let mut agents = api.agent_registry().write();
            agents.register(AgentIdentity::new(
                "writer-unbound".to_string(),
                AgentRole::Writer,
            ));
        }

        let config = WatchConfig {
            workspace_root,
            ..WatchConfig::default()
        };
        let daemon = WatchDaemon::new(api.clone(), config).unwrap();
        daemon.ensure_agent_frames_batched(&[node_id]).unwrap();

        let has_frame = api
            .has_agent_frame(&node_id, "writer-unbound")
            .expect("frame check should succeed");
        assert!(has_frame);
    }

    #[test]
    fn watch_rejects_retired_workflow_with_or_without_a_provider() {
        let temp = TempDir::new().unwrap();
        let workspace_root = temp.path().join("workspace");
        std::fs::create_dir_all(&workspace_root).unwrap();
        let api = Arc::new(create_test_api(&workspace_root));
        let node_id = crate::types::Hash::from([8u8; 32]);
        put_test_file_node(api.as_ref(), &workspace_root, node_id);

        {
            let mut agents = api.agent_registry().write();
            let mut bound = AgentIdentity::new("writer-bound".to_string(), AgentRole::Writer);
            bound.workflow_id = Some("docs_writer_thread_v1".to_string());
            agents.register(bound);
        }

        let config = WatchConfig {
            workspace_root,
            ..WatchConfig::default()
        };
        let daemon = WatchDaemon::new(api.clone(), config).unwrap();
        daemon.ensure_agent_frames_batched(&[node_id]).unwrap();

        let has_frame = api
            .has_agent_frame(&node_id, "writer-bound")
            .expect("frame check should succeed");
        assert!(!has_frame);

        {
            let mut providers = api.provider_registry().write();
            let provider_config = ProviderConfig {
                provider_name: Some("only-provider".to_string()),
                provider_type: ProviderType::LocalCustom,
                model: "test-model".to_string(),
                api_key: None,
                endpoint: Some("http://127.0.0.1:9".to_string()),
                default_options: crate::provider::CompletionOptions::default(),
            };
            providers
                .load_from_config(&MerkleConfig {
                    providers: std::collections::HashMap::from([(
                        "only-provider".to_string(),
                        provider_config,
                    )]),
                    ..Default::default()
                })
                .unwrap();
        }

        daemon.ensure_agent_frames_batched(&[node_id]).unwrap();
        assert!(!api.has_agent_frame(&node_id, "writer-bound").unwrap());
    }

    #[test]
    fn ensure_agent_frames_rejects_retired_workflow_despite_completed_thread() {
        let temp = TempDir::new().unwrap();
        let workspace_root = temp.path().join("workspace");
        std::fs::create_dir_all(&workspace_root).unwrap();
        let api = Arc::new(create_test_api(&workspace_root));
        let node_id = crate::types::Hash::from([9u8; 32]);
        put_test_file_node(api.as_ref(), &workspace_root, node_id);

        {
            let mut agents = api.agent_registry().write();
            let mut bound = AgentIdentity::new("writer-bound".to_string(), AgentRole::Writer);
            bound.workflow_id = Some("watch_skip_workflow".to_string());
            agents.register(bound);
        }

        {
            let mut providers = api.provider_registry().write();
            let provider_config = ProviderConfig {
                provider_name: Some("only-provider".to_string()),
                provider_type: ProviderType::LocalCustom,
                model: "test-model".to_string(),
                api_key: None,
                endpoint: Some("http://127.0.0.1:9".to_string()),
                default_options: crate::provider::CompletionOptions::default(),
            };
            providers
                .load_from_config(&MerkleConfig {
                    providers: std::collections::HashMap::from([(
                        "only-provider".to_string(),
                        provider_config,
                    )]),
                    ..Default::default()
                })
                .unwrap();
        }

        let workflow_dir = temp.path().join("workflows");
        let prompt_dir = workflow_dir.join("prompts");
        std::fs::create_dir_all(&prompt_dir).unwrap();
        std::fs::write(prompt_dir.join("watch_skip.md"), "Summarize the target.").unwrap();
        std::fs::write(
            workflow_dir.join("watch_skip_workflow.yaml"),
            r#"workflow_id: watch_skip_workflow
version: 1
title: Watch Skip Workflow
description: Exercises watch workflow dispatch without provider execution
thread_policy:
  start_conditions: {}
  dedupe_key_fields: []
  max_turn_retries: 1
turns:
  - turn_id: summarize
    seq: 1
    title: Summarize
    prompt_ref: prompts/watch_skip.md
    input_refs:
      - target_context
    output_type: summary
    gate_id: pass_gate
    retry_limit: 1
    timeout_ms: 60000
gates:
  - gate_id: pass_gate
    gate_type: no_semantic_drift
    required_fields: []
    rules: null
    fail_on_violation: false
artifact_policy:
  store_output: true
  store_prompt_render: true
  store_context_payload: true
  max_output_bytes: 1024
failure_policy:
  mode: fail_fast
  resume_from_failed_turn: true
  stop_on_gate_fail: false
"#,
        )
        .unwrap();
        let registry = WorkflowRegistry::load(&WorkflowConfig {
            user_profile_dir: Some(workflow_dir),
        })
        .unwrap();
        let registered_profile = registry.get("watch_skip_workflow").unwrap().clone();
        let thread_id = meld_execution::workflow::workflow_thread_id(
            &registered_profile.profile,
            node_id,
            "context-writer-bound",
        );
        let legacy_root = crate::config::xdg::workspace_data_dir(&workspace_root)
            .unwrap()
            .join("workflow/threads");
        std::fs::create_dir_all(&legacy_root).unwrap();
        let legacy_record = legacy_root.join(format!("{thread_id}.json"));
        let historical = serde_json::to_vec(&json!({
            "thread_id": thread_id, "status": "Completed", "final_frame_id": null
        }))
        .unwrap();
        std::fs::write(&legacy_record, &historical).unwrap();

        let progress_db = sled::open(temp.path().join("progress-watch-workflow")).unwrap();
        let (progress, event_store) = open_test_progress(progress_db);
        let session_id = progress
            .start_command_session("workspace.watch".to_string())
            .unwrap();
        let config = WatchConfig {
            workspace_root,
            session_id: Some(session_id.clone()),
            progress: Some(Arc::clone(&progress)),
            ..WatchConfig::default()
        };
        let daemon = WatchDaemon::new(api, config).unwrap();

        daemon.ensure_agent_frames_batched(&[node_id]).unwrap();
        progress.barrier().unwrap();

        let events = event_store.read_events_after(&session_id, 0).unwrap();
        let result = events
            .iter()
            .find(|event| event.event_type == "workflow_watch_rejected")
            .expect("retirement rejection should be emitted");
        assert_eq!(result.data["workflow_id"], "watch_skip_workflow");
        assert!(result.data["error"].as_str().unwrap().contains("retired"));
        assert_eq!(std::fs::read(legacy_record).unwrap(), historical);
    }

    #[test]
    fn retired_workflow_addresses_cannot_enter_an_unstarted_queue() {
        use crate::context::generation::{TargetExecutionProgram, TargetExecutionProgramKind};
        use crate::context::queue::{
            FrameGenerationQueue, GenerationConfig, GenerationRequestOptions, Priority,
        };
        use crate::provider::{ProviderExecutionBinding, ProviderRuntimeOverrides};
        use std::sync::Arc;
        let temp_dir = TempDir::new().unwrap();
        {
            let workspace_root = temp_dir.path().join("workspace");
            std::fs::create_dir_all(&workspace_root).unwrap();
            let api = Arc::new(create_test_api(&workspace_root));
            let queue = FrameGenerationQueue::new(api, GenerationConfig::default());
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                for program in [
                    TargetExecutionProgram::workflow("retained-profile"),
                    TargetExecutionProgram {
                        kind: TargetExecutionProgramKind::SingleShot,
                        workflow_id: Some("retained-profile".into()),
                    },
                ] {
                    let error = queue
                        .enqueue_and_wait_with_program(
                            crate::types::Hash::from([7; 32]),
                            "absent-agent".into(),
                            ProviderExecutionBinding::new(
                                "absent-provider",
                                ProviderRuntimeOverrides::default(),
                            )
                            .unwrap(),
                            None,
                            program,
                            Priority::Urgent,
                            Some(std::time::Duration::from_millis(10)),
                            GenerationRequestOptions::default(),
                        )
                        .await
                        .unwrap_err();
                    assert!(error.to_string().contains("retired"), "{error}");
                }
                queue
                    .wait_for_completion(Some(std::time::Duration::ZERO))
                    .await
                    .unwrap();
            });
        }
    }

    #[test]
    fn watch_batch_emits_canonical_workspace_snapshot_facts() {
        let temp = TempDir::new().unwrap();
        let workspace_root = temp.path().join("workspace");
        std::fs::create_dir_all(&workspace_root).unwrap();
        let target = workspace_root.join("doc.txt");
        std::fs::write(&target, "hello").unwrap();

        let (daemon, progress, event_store, session_id) =
            create_watch_test_runtime(&temp, workspace_root.clone());
        daemon
            .process_events(vec![ChangeEvent::Modified(target)])
            .unwrap();
        progress.barrier().unwrap();

        let emitted = event_store.read_events_after(&session_id, 0).unwrap();
        assert!(emitted
            .iter()
            .any(|event| event.event_type == "file_changed"));
        assert!(emitted
            .iter()
            .any(|event| event.event_type == "batch_processed"));
        assert!(emitted
            .iter()
            .any(|event| event.event_type == "workspace_fs.source_attached"));
        assert!(emitted
            .iter()
            .any(|event| event.event_type == "workspace_fs.snapshot_materialized"));
        assert!(emitted
            .iter()
            .any(|event| event.event_type == "workspace_fs.snapshot_selected"));
        assert!(emitted
            .iter()
            .any(|event| event.event_type == "workspace_fs.node_observed"));
    }

    #[test]
    fn watch_batch_reuses_source_identity() {
        let temp = TempDir::new().unwrap();
        let workspace_root = temp.path().join("workspace");
        std::fs::create_dir_all(&workspace_root).unwrap();
        let target = workspace_root.join("doc.txt");
        std::fs::write(&target, "hello").unwrap();

        let (daemon, progress, event_store, session_id) =
            create_watch_test_runtime(&temp, workspace_root.clone());
        daemon
            .process_events(vec![ChangeEvent::Modified(target.clone())])
            .unwrap();
        progress.barrier().unwrap();

        std::fs::write(&target, "hello again").unwrap();
        daemon
            .process_events(vec![ChangeEvent::Modified(target)])
            .unwrap();
        progress.barrier().unwrap();

        let events = event_store.read_events_after(&session_id, 0).unwrap();
        let mut source_ids = std::collections::BTreeSet::new();
        for event in events
            .iter()
            .filter(|event| event.domain_id == "workspace_fs" && !event.objects.is_empty())
        {
            for object in &event.objects {
                if object.object_kind == "source" {
                    source_ids.insert(object.object_id.clone());
                }
            }
        }
        assert_eq!(source_ids.len(), 1);
    }

    #[test]
    fn watch_batch_only_selects_snapshot_when_root_hash_changes() {
        let temp = TempDir::new().unwrap();
        let workspace_root = temp.path().join("workspace");
        std::fs::create_dir_all(&workspace_root).unwrap();
        let target = workspace_root.join("doc.txt");
        std::fs::write(&target, "hello").unwrap();

        let (daemon, progress, event_store, session_id) =
            create_watch_test_runtime(&temp, workspace_root.clone());
        daemon
            .process_events(vec![ChangeEvent::Modified(target.clone())])
            .unwrap();
        progress.barrier().unwrap();
        let first_selected = event_store
            .read_events_after(&session_id, 0)
            .unwrap()
            .into_iter()
            .filter(|event| event.event_type == "workspace_fs.snapshot_selected")
            .count();

        daemon
            .process_events(vec![ChangeEvent::Modified(target.clone())])
            .unwrap();
        progress.barrier().unwrap();
        let same_root_selected = event_store
            .read_events_after(&session_id, 0)
            .unwrap()
            .into_iter()
            .filter(|event| event.event_type == "workspace_fs.snapshot_selected")
            .count();
        assert_eq!(same_root_selected, first_selected);

        std::fs::write(&target, "changed").unwrap();
        daemon
            .process_events(vec![ChangeEvent::Modified(target)])
            .unwrap();
        progress.barrier().unwrap();
        let changed_root_selected = event_store
            .read_events_after(&session_id, 0)
            .unwrap()
            .into_iter()
            .filter(|event| event.event_type == "workspace_fs.snapshot_selected")
            .count();
        assert!(changed_root_selected > same_root_selected);
    }

    #[test]
    fn watch_batch_emits_observed_nodes_for_affected_scope_only() {
        let temp = TempDir::new().unwrap();
        let workspace_root = temp.path().join("workspace");
        let docs = workspace_root.join("docs");
        let notes = workspace_root.join("notes");
        std::fs::create_dir_all(&docs).unwrap();
        std::fs::create_dir_all(&notes).unwrap();
        let changed = docs.join("doc.txt");
        let untouched = notes.join("other.txt");
        std::fs::write(&changed, "hello").unwrap();
        std::fs::write(&untouched, "unchanged").unwrap();

        let (daemon, progress, event_store, session_id) =
            create_watch_test_runtime(&temp, workspace_root.clone());
        daemon
            .process_events(vec![ChangeEvent::Modified(changed.clone())])
            .unwrap();
        progress.barrier().unwrap();

        let observed_paths: Vec<String> = event_store
            .read_events_after(&session_id, 0)
            .unwrap()
            .into_iter()
            .filter(|event| event.event_type == "workspace_fs.node_observed")
            .map(|event| {
                serde_json::from_value::<WorkspaceNodeObservedEventData>(event.data.clone())
                    .unwrap()
            })
            .map(|data| data.path)
            .collect();

        assert!(observed_paths.iter().any(|path| path.ends_with("doc.txt")));
        assert!(observed_paths.iter().any(|path| path.ends_with("docs")));
        assert!(!observed_paths
            .iter()
            .any(|path| path.ends_with("other.txt")));
    }
}
