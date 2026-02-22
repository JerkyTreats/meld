//! Watch daemon and runtime logic.

use super::events::{ChangeEvent, EventBatcher, WatchConfig};
use crate::api::ContextApi;
use crate::context::queue::{FrameGenerationQueue, QueueEventContext};
use crate::error::ApiError;
use crate::heads::HeadIndex;
use crate::ignore;
use crate::store::{NodeRecord, NodeRecordStore};
use crate::tree::builder::TreeBuilder;
use crate::tree::path::canonicalize_path;
use crate::tree::walker::WalkerConfig;
use crate::types::NodeID;
use notify::{Event, EventKind, RecursiveMode, Watcher};
use parking_lot::RwLock;
use serde_json::json;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};

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
            ApiError::StorageError(crate::error::StorageError::IoError(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to create watcher: {}", e),
            )))
        })?;

        watcher
            .watch(&self.config.workspace_root, RecursiveMode::Recursive)
            .map_err(|e| {
                ApiError::StorageError(crate::error::StorageError::IoError(std::io::Error::new(
                    std::io::ErrorKind::Other,
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
                        self.process_events(pending_events.drain(..).collect())?;
                        last_batch_time = Instant::now();
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    error!("Watcher channel disconnected");
                    break;
                }
            }

            if !pending_events.is_empty() && last_batch_time.elapsed() >= batch_window {
                self.process_events(pending_events.drain(..).collect())?;
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

        NodeRecord::populate_store_from_tree(
            self.api.node_store().as_ref() as &dyn NodeRecordStore,
            &tree,
        )
        .map_err(ApiError::from)?;

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

        let affected_nodes = self.update_tree_for_paths(&affected_paths)?;

        if self.config.auto_create_frames {
            self.ensure_agent_frames_batched(&affected_nodes)?;
        }

        info!(
            event_count = events.len(),
            affected_nodes = affected_nodes.len(),
            "Processed change events"
        );
        self.emit_event_best_effort(
            "batch_processed",
            json!({ "event_count": events.len(), "affected_nodes": affected_nodes.len() }),
        );

        Ok(())
    }

    fn update_tree_for_paths(&self, paths: &HashSet<PathBuf>) -> Result<Vec<NodeID>, ApiError> {
        let walker_config = WalkerConfig {
            follow_symlinks: false,
            ignore_patterns: self.config.ignore_patterns.clone(),
            max_depth: None,
        };
        let builder =
            TreeBuilder::new(self.config.workspace_root.clone()).with_walker_config(walker_config);
        let tree = builder.build().map_err(ApiError::from)?;

        let _ = ignore::maybe_sync_gitignore_after_tree(
            &self.config.workspace_root,
            tree.find_gitignore_node_id().as_ref(),
        );

        let mut affected_nodes = Vec::new();

        for (node_id, _node) in &tree.nodes {
            let node_record = self.api.node_store().get(node_id).map_err(ApiError::from)?;
            if let Some(record) = node_record {
                let canonical_path = canonicalize_path(&record.path).unwrap_or(record.path.clone());
                if paths.iter().any(|p| {
                    canonicalize_path(p)
                        .map(|cp| cp == canonical_path)
                        .unwrap_or(false)
                }) {
                    affected_nodes.push(*node_id);
                }
            }
        }

        NodeRecord::populate_store_from_tree(
            self.api.node_store().as_ref() as &dyn NodeRecordStore,
            &tree,
        )
        .map_err(ApiError::from)?;

        let mut all_affected = affected_nodes.clone();
        for node_id in &affected_nodes {
            self.collect_ancestors(*node_id, &mut all_affected)?;
        }

        Ok(all_affected)
    }

    fn collect_ancestors(
        &self,
        node_id: NodeID,
        collected: &mut Vec<NodeID>,
    ) -> Result<(), ApiError> {
        let node_record = self
            .api
            .node_store()
            .get(&node_id)
            .map_err(ApiError::from)?;
        if let Some(record) = node_record {
            if let Some(parent_id) = record.parent {
                if !collected.contains(&parent_id) {
                    collected.push(parent_id);
                    self.collect_ancestors(parent_id, collected)?;
                }
            }
        }
        Ok(())
    }

    /// Ensure contextframes exist for all agents for the given nodes (batched)
    pub(crate) fn ensure_agent_frames_batched(&self, node_ids: &[NodeID]) -> Result<(), ApiError> {
        if node_ids.is_empty() {
            return Ok(());
        }

        let agents: Vec<String> = {
            let registry = self.api.agent_registry().read();
            registry
                .list_all()
                .iter()
                .map(|a| a.agent_id.clone())
                .collect()
        };

        if agents.is_empty() {
            warn!("No agents registered, skipping contextframe creation. Please configure agents in your config file.");
            return Ok(());
        }

        info!(
            agent_count = agents.len(),
            agents = ?agents,
            "Found {} agent(s) for contextframe creation",
            agents.len()
        );

        let batch_size = self.config.frame_batch_size;
        let mut created_count = 0;
        let mut skipped_count = 0;

        for chunk in node_ids.chunks(batch_size) {
            for node_id in chunk {
                for agent_id in &agents {
                    match self.api.ensure_agent_frame(
                        *node_id,
                        agent_id.clone(),
                        None,
                        self.generation_queue.as_ref().map(Arc::clone),
                    ) {
                        Ok(Some(frame_id)) => {
                            created_count += 1;
                            debug!(
                                node_id = %hex::encode(node_id),
                                agent_id = %agent_id,
                                frame_id = %hex::encode(frame_id),
                                "Created contextframe"
                            );
                        }
                        Ok(None) => {
                            skipped_count += 1;
                            debug!(
                                node_id = %hex::encode(node_id),
                                agent_id = %agent_id,
                                "Skipped contextframe (already exists or agent cannot write)"
                            );
                        }
                        Err(e) => {
                            warn!(
                                node_id = %hex::encode(node_id),
                                agent_id = %agent_id,
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
