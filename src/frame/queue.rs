//! Frame Generation Queue
//!
//! Batch queue system for automatically generating context frames using LLM providers.
//! Handles large-scale operations efficiently through batching, rate limiting, and concurrent processing.

use crate::api::ContextApi;
use crate::error::ApiError;
use crate::store::NodeRecord;
use crate::tooling::adapter::{AgentAdapter, ContextApiAdapter};
use crate::types::{FrameID, NodeID};
use hex;
use parking_lot::RwLock;
use std::collections::{BinaryHeap, HashMap};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, Notify, Semaphore};
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

/// Priority level for generation requests
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Low = 0,      // Existing files during initial scan
    Normal = 1,   // Default priority
    High = 2,     // New files in watch mode
    Urgent = 3,   // User-initiated requests
}

/// Generation request
#[derive(Debug, Clone)]
pub struct GenerationRequest {
    /// NodeID to generate frame for
    pub node_id: NodeID,
    /// Agent ID that will generate the frame
    pub agent_id: String,
    /// Frame type to generate
    pub frame_type: String,
    /// Priority level (higher = more important)
    pub priority: Priority,
    /// Number of retry attempts made
    pub retry_count: usize,
    /// Timestamp when request was created
    pub created_at: Instant,
}

impl PartialEq for GenerationRequest {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority
            && self.created_at == other.created_at
            && self.node_id == other.node_id
            && self.agent_id == other.agent_id
    }
}

impl Eq for GenerationRequest {}

impl Ord for GenerationRequest {
    /// Order by priority (higher first), then by creation time (older first for same priority)
    /// BinaryHeap is a max-heap, so higher priority should compare as Greater
    /// For same priority, older items (smaller timestamp) should be Greater (processed first)
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.priority.cmp(&other.priority) {
            std::cmp::Ordering::Equal => {
                // Older items (smaller timestamp) should be Greater (processed first)
                self.created_at.cmp(&other.created_at).reverse()
            }
            // Higher priority (larger enum value) should be Greater
            ordering => ordering,
        }
    }
}

impl PartialOrd for GenerationRequest {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Configuration for the generation queue
#[derive(Debug, Clone)]
pub struct GenerationConfig {
    /// Maximum concurrent generations per agent
    pub max_concurrent_per_agent: usize,
    /// Batch size for processing requests
    pub batch_size: usize,
    /// Maximum retry attempts per request
    pub max_retry_attempts: usize,
    /// Delay between retries (milliseconds)
    pub retry_delay_ms: u64,
    /// Rate limit: minimum delay between requests per agent (milliseconds)
    pub rate_limit_ms: Option<u64>,
    /// Maximum queue size (prevents memory exhaustion)
    pub max_queue_size: usize,
    /// Number of worker tasks per agent
    pub workers_per_agent: usize,
}

impl Default for GenerationConfig {
    fn default() -> Self {
        Self {
            max_concurrent_per_agent: 3,
            batch_size: 50,
            max_retry_attempts: 3,
            retry_delay_ms: 1000,
            rate_limit_ms: Some(100), // 100ms between requests per agent
            max_queue_size: 10000,
            workers_per_agent: 2,
        }
    }
}

/// Queue statistics
#[derive(Debug, Clone, Default)]
pub struct QueueStats {
    /// Number of pending requests
    pub pending: usize,
    /// Number of requests currently being processed
    pub processing: usize,
    /// Number of completed requests
    pub completed: usize,
    /// Number of failed requests
    pub failed: usize,
}

/// Per-agent rate limiter
struct AgentRateLimiter {
    semaphore: Arc<Semaphore>,
    last_request: Arc<RwLock<HashMap<String, Instant>>>,
    min_delay: Option<Duration>,
}

impl AgentRateLimiter {
    fn new(max_concurrent: usize, min_delay_ms: Option<u64>) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            last_request: Arc::new(RwLock::new(HashMap::new())),
            min_delay: min_delay_ms.map(Duration::from_millis),
        }
    }

    async fn acquire(&self, agent_id: &str) -> Result<tokio::sync::SemaphorePermit<'_>, ApiError> {
        // Acquire semaphore (concurrency limit)
        let permit = self.semaphore.acquire().await
            .map_err(|_| ApiError::ProviderRateLimit("Semaphore closed".to_string()))?;

        // Check rate limit delay
        if let Some(min_delay) = self.min_delay {
            let sleep_duration = {
                let last = self.last_request.read();
                if let Some(last_time) = last.get(agent_id) {
                    let elapsed = last_time.elapsed();
                    if elapsed < min_delay {
                        Some(min_delay - elapsed)
                    } else {
                        None
                    }
                } else {
                    None
                }
            };
            
            // Sleep if needed (after dropping the guard)
            if let Some(duration) = sleep_duration {
                sleep(duration).await;
            }
            
            // Update last request time
            {
                let mut last = self.last_request.write();
                last.insert(agent_id.to_string(), Instant::now());
            }
        }

        Ok(permit)
    }
}

/// Frame generation queue
pub struct FrameGenerationQueue {
    /// Pending requests (priority queue using BinaryHeap)
    queue: Arc<Mutex<BinaryHeap<GenerationRequest>>>,
    /// Notifier to wake workers when new items are enqueued
    notify: Arc<Notify>,
    /// Active worker tasks
    workers: Arc<RwLock<Vec<tokio::task::JoinHandle<()>>>>,
    /// Configuration
    config: GenerationConfig,
    /// API for frame operations
    api: Arc<ContextApi>,
    /// Adapter for LLM generation
    adapter: Arc<ContextApiAdapter>,
    /// Rate limiters per agent
    rate_limiters: Arc<RwLock<HashMap<String, AgentRateLimiter>>>,
    /// Running state
    running: Arc<RwLock<bool>>,
    /// Statistics
    stats: Arc<RwLock<QueueStats>>,
}

impl FrameGenerationQueue {
    /// Create a new generation queue
    pub fn new(
        api: Arc<ContextApi>,
        adapter: Arc<ContextApiAdapter>,
        config: GenerationConfig,
    ) -> Self {
        Self {
            queue: Arc::new(Mutex::new(BinaryHeap::new())),
            notify: Arc::new(Notify::new()),
            workers: Arc::new(RwLock::new(Vec::new())),
            config,
            api,
            adapter,
            rate_limiters: Arc::new(RwLock::new(HashMap::new())),
            running: Arc::new(RwLock::new(false)),
            stats: Arc::new(RwLock::new(QueueStats::default())),
        }
    }

    /// Enqueue a generation request
    pub async fn enqueue(
        &self,
        node_id: NodeID,
        agent_id: String,
        frame_type: Option<String>,
        priority: Priority,
    ) -> Result<(), ApiError> {
        let mut queue = self.queue.lock().await;

        // Check queue size limit
        if queue.len() >= self.config.max_queue_size {
            warn!(
                queue_size = queue.len(),
                max_size = self.config.max_queue_size,
                "Generation queue is full, dropping request"
            );
            return Err(ApiError::ConfigError(
                "Generation queue is full".to_string(),
            ));
        }

        // Use provided frame_type or default to "context-{agent_id}"
        let frame_type = frame_type.unwrap_or_else(|| format!("context-{}", agent_id));

        let request = GenerationRequest {
            node_id,
            agent_id: agent_id.clone(),
            frame_type,
            priority,
            retry_count: 0,
            created_at: Instant::now(),
        };

        // Push to priority queue (BinaryHeap maintains max-heap property)
        queue.push(request);

        // Update stats
        {
            let mut stats = self.stats.write();
            stats.pending += 1;
        }

        // Notify workers that a new item is available
        self.notify.notify_one();

        debug!(
            node_id = %hex::encode(node_id),
            agent_id = %agent_id,
            priority = ?priority,
            queue_size = queue.len(),
            "Enqueued generation request"
        );

        Ok(())
    }

    /// Enqueue multiple requests (batch enqueue)
    pub async fn enqueue_batch(
        &self,
        requests: Vec<(NodeID, String, Option<String>, Priority)>,
    ) -> Result<(), ApiError> {
        let batch_size = requests.len();
        let mut queue = self.queue.lock().await;

        // Check if batch would exceed queue size
        if queue.len() + batch_size > self.config.max_queue_size {
            warn!(
                queue_size = queue.len(),
                batch_size = batch_size,
                max_size = self.config.max_queue_size,
                "Batch would exceed queue size limit"
            );
            return Err(ApiError::ConfigError(
                "Batch would exceed generation queue size limit".to_string(),
            ));
        }

        // Use provided frame_type or default to "context-{agent_id}"
        for (node_id, agent_id, frame_type, priority) in requests {
            let frame_type = frame_type.unwrap_or_else(|| format!("context-{}", agent_id));
            let request = GenerationRequest {
                node_id,
                agent_id: agent_id.clone(),
                frame_type,
                priority,
                retry_count: 0,
                created_at: Instant::now(),
            };
            queue.push(request);
        }

        // Update stats
        {
            let mut stats = self.stats.write();
            stats.pending += batch_size;
        }

        // Notify workers (multiple times for multiple items)
        let notify_count = batch_size.min(self.config.workers_per_agent);
        for _ in 0..notify_count {
            self.notify.notify_one();
        }

        drop(queue);

        debug!(
            batch_size = batch_size,
            "Enqueued batch of generation requests"
        );

        Ok(())
    }

    /// Start background workers
    pub fn start(&self) -> Result<(), ApiError> {
        let mut running = self.running.write();
        if *running {
            return Ok(()); // Already running
        }
        *running = true;
        drop(running);

        // Get unique agent IDs from queue to determine worker count
        // We'll start workers that will process requests for any agent
        let worker_count = self.config.workers_per_agent;

        let mut workers = self.workers.write();
        for i in 0..worker_count {
            let queue = Arc::clone(&self.queue);
            let notify = Arc::clone(&self.notify);
            let adapter = Arc::clone(&self.adapter);
            let api = Arc::clone(&self.api);
            let config = self.config.clone();
            let rate_limiters = Arc::clone(&self.rate_limiters);
            let running = Arc::clone(&self.running);
            let stats = Arc::clone(&self.stats);

            let handle = tokio::spawn(async move {
                Self::worker_loop(
                    i,
                    queue,
                    notify,
                    adapter,
                    api,
                    config,
                    rate_limiters,
                    running,
                    stats,
                ).await;
            });

            workers.push(handle);
        }

        info!(
            worker_count = workers.len(),
            "Started frame generation queue workers"
        );

        Ok(())
    }

    /// Stop background workers (graceful shutdown)
    pub async fn stop(&self) -> Result<(), ApiError> {
        let mut running = self.running.write();
        if !*running {
            return Ok(()); // Already stopped
        }
        *running = false;
        drop(running);

        // Wait for all workers to finish
        let workers = std::mem::take(&mut *self.workers.write());
        for handle in workers {
            let _ = handle.await;
        }

        info!("Stopped frame generation queue workers");
        Ok(())
    }

    /// Get queue statistics
    pub fn stats(&self) -> QueueStats {
        self.stats.read().clone()
    }

    /// Wait for queue to drain (all requests processed)
    pub async fn wait_for_completion(&self, timeout: Option<Duration>) -> Result<(), ApiError> {
        let start = Instant::now();
        loop {
            let queue = self.queue.lock().await;
            let stats = self.stats.read();

            if queue.is_empty() && stats.processing == 0 {
                return Ok(());
            }

            if let Some(timeout) = timeout {
                if start.elapsed() >= timeout {
                    return Err(ApiError::ConfigError(
                        "Timeout waiting for queue to drain".to_string(),
                    ));
                }
            }

            drop(queue);
            drop(stats);
            sleep(Duration::from_millis(100)).await;
        }
    }

    /// Worker loop for processing requests
    async fn worker_loop(
        worker_id: usize,
        queue: Arc<Mutex<BinaryHeap<GenerationRequest>>>,
        notify: Arc<Notify>,
        adapter: Arc<ContextApiAdapter>,
        api: Arc<ContextApi>,
        config: GenerationConfig,
        rate_limiters: Arc<RwLock<HashMap<String, AgentRateLimiter>>>,
        running: Arc<RwLock<bool>>,
        stats: Arc<RwLock<QueueStats>>,
    ) {
        debug!(worker_id, "Worker started");

        while *running.read() {
            // Get next request from queue (highest priority first)
            let request = {
                let mut queue_guard = queue.lock().await;
                queue_guard.pop()
            };

            let Some(mut request) = request else {
                // No requests, wait for notification or timeout
                // Use a timeout to periodically check if we should stop
                let notify_future = notify.notified();
                let timeout_future = sleep(Duration::from_millis(100));
                tokio::select! {
                    _ = notify_future => {
                        // New item available, continue loop
                        continue;
                    }
                    _ = timeout_future => {
                        // Timeout, check if we should continue
                        continue;
                    }
                }
            };

            // Update stats
            {
                let mut stats = stats.write();
                stats.pending = stats.pending.saturating_sub(1);
                stats.processing += 1;
            }

            // Get or create rate limiter for this agent
            // We need to clone the Arc references, not the limiter itself
            let (semaphore, last_request, min_delay) = {
                let mut limiters = rate_limiters.write();
                let limiter = limiters
                    .entry(request.agent_id.clone())
                    .or_insert_with(|| {
                        AgentRateLimiter::new(
                            config.max_concurrent_per_agent,
                            config.rate_limit_ms,
                        )
                    });
                (Arc::clone(&limiter.semaphore), Arc::clone(&limiter.last_request), limiter.min_delay)
            };
            
            // Create a temporary rate limiter for this request
            let rate_limiter = AgentRateLimiter {
                semaphore,
                last_request,
                min_delay,
            };

            // Acquire rate limiter permit
            let _permit = match rate_limiter.acquire(&request.agent_id).await {
                Ok(permit) => permit,
                Err(e) => {
                    error!(
                        worker_id,
                        agent_id = %request.agent_id,
                        error = %e,
                        "Failed to acquire rate limiter permit"
                    );
                    // Re-queue request (maintains priority order automatically)
                    let mut queue_guard = queue.lock().await;
                    queue_guard.push(request);
                    {
                        let mut stats = stats.write();
                        stats.processing = stats.processing.saturating_sub(1);
                        stats.pending += 1;
                    }
                    continue;
                }
            };

            // Process request
            let result = Self::process_request(
                &request,
                &adapter,
                &api,
                &config,
            ).await;

            // Update stats (drop guard before await)
            let should_retry = {
                let mut stats_guard = stats.write();
                stats_guard.processing = stats_guard.processing.saturating_sub(1);
                match &result {
                    Ok(_) => {
                        stats_guard.completed += 1;
                        false
                    }
                    Err(_) => {
                        // Check if we should retry
                        let retry = request.retry_count < config.max_retry_attempts
                            && Self::is_retryable_error(result.as_ref().unwrap_err());
                        if retry {
                            // Will update stats after re-queuing
                        } else {
                            stats_guard.failed += 1;
                            error!(
                                worker_id,
                                node_id = %hex::encode(request.node_id),
                                agent_id = %request.agent_id,
                                retry_count = request.retry_count,
                                "Generation request failed permanently"
                            );
                        }
                        retry
                    }
                }
            };
            
            // Re-queue if needed (after dropping stats guard)
            if should_retry {
                request.retry_count += 1;
                // Add retry delay before re-queuing
                sleep(Duration::from_millis(config.retry_delay_ms)).await;
                
                let mut queue_guard = queue.lock().await;
                queue_guard.push(request);
                drop(queue_guard);
                
                // Notify workers that a retry is available
                notify.notify_one();
                
                // Update stats after re-queuing
                let mut stats_guard = stats.write();
                stats_guard.pending += 1;
            }
        }

        debug!(worker_id, "Worker stopped");
    }

    /// Process a single generation request
    async fn process_request(
        request: &GenerationRequest,
        adapter: &ContextApiAdapter,
        api: &ContextApi,
        _config: &GenerationConfig,
    ) -> Result<FrameID, ApiError> {
        debug!(
            node_id = %hex::encode(request.node_id),
            agent_id = %request.agent_id,
            attempt = request.retry_count + 1,
            "Processing generation request"
        );

        // Get agent
        let agent = api.get_agent(&request.agent_id)?;

        // Get node record
        let node_record = api
            .node_store()
            .get(&request.node_id)
            .map_err(ApiError::from)?
            .ok_or_else(|| ApiError::NodeNotFound(request.node_id))?;

        // Validate agent has required prompts
        let missing_prompts = Self::validate_agent_prompts(&agent, &node_record);
        if !missing_prompts.is_empty() {
            error!(
                agent_id = %request.agent_id,
                node_id = %hex::encode(request.node_id),
                missing = ?missing_prompts,
                "Agent has provider configured but missing required prompts. Skipping generation."
            );
            return Err(ApiError::ConfigError(format!(
                "Agent '{}' missing required prompts: {}",
                request.agent_id,
                missing_prompts.join(", ")
            )));
        }

        // Generate prompts (system_prompt is validated but adapter gets it from agent metadata)
        let (_system_prompt, user_prompt) = Self::generate_prompts(&agent, &node_record)?;

        // Generate frame using adapter
        let start = Instant::now();
        let frame_id = adapter
            .generate_frame(
                request.node_id,
                user_prompt,
                request.frame_type.clone(),
                request.agent_id.clone(),
            )
            .await?;

        let duration = start.elapsed();
        info!(
            node_id = %hex::encode(request.node_id),
            agent_id = %request.agent_id,
            frame_id = %hex::encode(frame_id),
            duration_ms = duration.as_millis(),
            "Frame generation completed"
        );

        Ok(frame_id)
    }

    /// Validate that agent has all required prompts
    pub fn validate_agent_prompts(agent: &crate::agent::AgentIdentity, node_record: &NodeRecord) -> Vec<String> {
        let mut missing = Vec::new();

        if !agent.metadata.contains_key("system_prompt") {
            missing.push("system_prompt".to_string());
        }

        match node_record.node_type {
            crate::store::NodeType::File { .. } => {
                if !agent.metadata.contains_key("user_prompt_file") {
                    missing.push("user_prompt_file".to_string());
                }
            }
            crate::store::NodeType::Directory => {
                if !agent.metadata.contains_key("user_prompt_directory") {
                    missing.push("user_prompt_directory".to_string());
                }
            }
        }

        missing
    }

    /// Generate prompts from agent metadata
    fn generate_prompts(
        agent: &crate::agent::AgentIdentity,
        node_record: &NodeRecord,
    ) -> Result<(String, String), ApiError> {
        // Get system prompt
        let system_prompt = agent
            .metadata
            .get("system_prompt")
            .ok_or_else(|| {
                ApiError::ConfigError(format!(
                    "Agent '{}' missing system_prompt",
                    agent.agent_id
                ))
            })?
            .clone();

        // Get user prompt template based on node type
        let user_prompt_template = match node_record.node_type {
            crate::store::NodeType::File { .. } => {
                agent.metadata.get("user_prompt_file").ok_or_else(|| {
                    ApiError::ConfigError(format!(
                        "Agent '{}' missing user_prompt_file",
                        agent.agent_id
                    ))
                })?
            }
            crate::store::NodeType::Directory => {
                agent.metadata.get("user_prompt_directory").ok_or_else(|| {
                    ApiError::ConfigError(format!(
                        "Agent '{}' missing user_prompt_directory",
                        agent.agent_id
                    ))
                })?
            }
        };

        // Replace placeholders in template
        let mut user_prompt = user_prompt_template
            .replace("{path}", &node_record.path.display().to_string())
            .replace("{node_type}", match node_record.node_type {
                crate::store::NodeType::File { .. } => "File",
                crate::store::NodeType::Directory => "Directory",
            });

        // For file nodes, add file size if available
        if let crate::store::NodeType::File { size, .. } = node_record.node_type {
            user_prompt = user_prompt.replace("{file_size}", &size.to_string());
        }

        Ok((system_prompt, user_prompt))
    }

    /// Check if an error is retryable
    fn is_retryable_error(error: &ApiError) -> bool {
        match error {
            ApiError::ConfigError(_) => false, // Don't retry config errors
            ApiError::ProviderNotConfigured(_) => false,
            ApiError::ProviderRateLimit(_) => true,
            ApiError::ProviderRequestFailed(_) => true,
            ApiError::ProviderError(_) => true,
            _ => true, // Retry other errors by default
        }
    }
}

