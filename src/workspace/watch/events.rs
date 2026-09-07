//! Watch events, batching, and configuration.

use crate::context::queue::GenerationConfig;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use crate::telemetry::ProgressRuntime;

/// Watch mode configuration
#[derive(Clone)]
pub struct WatchConfig {
    /// Workspace root directory
    pub workspace_root: PathBuf,
    /// Debounce window in milliseconds
    pub debounce_ms: u64,
    /// Batch window in milliseconds
    pub batch_window_ms: u64,
    /// Maximum events per batch
    pub max_batch_size: usize,
    /// Ignore patterns (glob patterns)
    pub ignore_patterns: Vec<String>,
    /// Maximum event queue size
    pub max_queue_size: usize,
    /// Enable automatic contextframe creation for agents
    pub auto_create_frames: bool,
    /// Batch size for contextframe creation
    pub frame_batch_size: usize,
    /// Enable automatic LLM-based frame generation
    pub auto_generate_frames: bool,
    /// Generation queue configuration
    pub generation_config: Option<GenerationConfig>,
    /// Optional active observability session
    pub session_id: Option<String>,
    /// Optional progress runtime for event emission
    pub progress: Option<Arc<ProgressRuntime>>,
}

impl Default for WatchConfig {
    fn default() -> Self {
        Self {
            workspace_root: PathBuf::from("."),
            debounce_ms: 100,
            batch_window_ms: 50,
            max_batch_size: 100,
            ignore_patterns: vec![
                "**/.git/**".to_string(),
                "**/.meld/**".to_string(),
                "**/target/**".to_string(),
                "**/node_modules/**".to_string(),
                "**/.DS_Store".to_string(),
                "**/*.swp".to_string(),
                "**/*.tmp".to_string(),
            ],
            max_queue_size: 10000,
            auto_create_frames: true,
            frame_batch_size: 50,
            auto_generate_frames: false,
            generation_config: None,
            session_id: None,
            progress: None,
        }
    }
}

/// Filesystem change event
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ChangeEvent {
    Created(PathBuf),
    Modified(PathBuf),
    Removed(PathBuf),
    Renamed { from: PathBuf, to: PathBuf },
}

/// Event batcher for grouping and debouncing events
pub(crate) struct EventBatcher {
    config: WatchConfig,
    pending_events: HashMap<PathBuf, ChangeEvent>,
    last_event_time: HashMap<PathBuf, Instant>,
}

impl EventBatcher {
    pub(crate) fn new(config: WatchConfig) -> Self {
        Self {
            config,
            pending_events: HashMap::new(),
            last_event_time: HashMap::new(),
        }
    }

    pub(crate) fn add_event(&mut self, event: ChangeEvent) -> bool {
        let path = match &event {
            ChangeEvent::Created(p) | ChangeEvent::Modified(p) | ChangeEvent::Removed(p) => {
                p.clone()
            }
            ChangeEvent::Renamed { to, .. } => to.clone(),
        };

        if self.should_ignore(&path) {
            return false;
        }

        let now = Instant::now();
        let debounce_window = std::time::Duration::from_millis(self.config.debounce_ms);

        if let Some(last_time) = self.last_event_time.get(&path) {
            if now.duration_since(*last_time) < debounce_window {
                self.pending_events.insert(path.clone(), event);
                return false;
            }
        }

        self.pending_events.insert(path.clone(), event);
        self.last_event_time.insert(path, now);

        self.pending_events.len() >= self.config.max_batch_size
    }

    pub(crate) fn take_batch(&mut self) -> Vec<ChangeEvent> {
        let events: Vec<_> = self.pending_events.values().cloned().collect();
        self.pending_events.clear();
        self.last_event_time.clear();
        events
    }

    fn should_ignore(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        for pattern in &self.config.ignore_patterns {
            if self.matches_pattern(&path_str, pattern) {
                return true;
            }
        }
        false
    }

    fn matches_pattern(&self, path: &str, pattern: &str) -> bool {
        let path_normalized = path.replace('\\', "/");
        let pattern_normalized = pattern.replace('\\', "/");

        if pattern_normalized.contains("**") {
            let parts: Vec<&str> = pattern_normalized.split("**").collect();
            if parts.len() == 2 {
                let prefix = parts[0];
                let suffix = parts[1];
                if prefix.is_empty() {
                    return path_normalized.contains(suffix);
                } else if suffix.is_empty() {
                    return path_normalized.starts_with(prefix);
                } else {
                    return path_normalized.starts_with(prefix) && path_normalized.contains(suffix);
                }
            }
        }

        if pattern_normalized.contains('*') {
            let parts: Vec<&str> = pattern_normalized.split('*').collect();
            if parts.len() == 2 {
                return path_normalized.starts_with(parts[0]) && path_normalized.contains(parts[1]);
            }
        }

        path_normalized == pattern_normalized || path_normalized.contains(&pattern_normalized)
    }
}
