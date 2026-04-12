//! Control-owned orchestration for level-by-level generation execution.

use crate::context::generation::plan::{
    FailurePolicy, GenerationErrorDetail, GenerationItem, GenerationPlan, GenerationResult,
    LevelSummary,
};
use crate::context::queue::{FrameGenerationQueue, Priority};
use crate::control::events::{
    generation_completed_envelope, generation_failed_envelope, generation_started_envelope,
    level_completed_envelope, level_started_envelope, node_completed_envelope,
    node_failed_envelope, node_started_envelope, GenerationCompletedEventData,
    GenerationFailedEventData, GenerationStartedEventData, LevelCompletedEventData,
    LevelStartedEventData, NodeCompletedEventData, NodeFailedEventData, NodeStartedEventData,
};
use crate::error::ApiError;
use crate::telemetry::ProgressRuntime;
use crate::types::FrameID;
use futures::stream::{FuturesUnordered, StreamExt};
use std::sync::Arc;
use std::time::Duration;

#[allow(async_fn_in_trait)]
pub trait QueueSubmitter: Send + Sync {
    async fn enqueue_and_wait_item(
        &self,
        item: &GenerationItem,
        priority: Priority,
        plan_id: &str,
        wait_timeout: Option<Duration>,
    ) -> Result<FrameID, ApiError>;
}

impl QueueSubmitter for FrameGenerationQueue {
    async fn enqueue_and_wait_item(
        &self,
        item: &GenerationItem,
        priority: Priority,
        plan_id: &str,
        wait_timeout: Option<Duration>,
    ) -> Result<FrameID, ApiError> {
        self.enqueue_and_wait_with_program(
            item.node_id,
            item.agent_id.clone(),
            item.provider.clone(),
            Some(item.frame_type.clone()),
            item.program.clone(),
            priority,
            wait_timeout,
            crate::context::queue::GenerationRequestOptions {
                force: item.force,
                plan_id: Some(plan_id.to_string()),
            },
        )
        .await
    }
}

/// Executes a generation plan by submitting items to a queue and collecting results.
pub struct GenerationExecutor {
    progress: Option<Arc<ProgressRuntime>>,
    wait_timeout: Option<Duration>,
}

impl GenerationExecutor {
    const DEFAULT_WAIT_TIMEOUT: Duration = Duration::from_secs(300);
    const DEFAULT_WORKFLOW_WAIT_TIMEOUT: Duration = Duration::from_secs(900);

    pub fn new(progress: Option<Arc<ProgressRuntime>>) -> Self {
        Self {
            progress,
            wait_timeout: Some(Self::DEFAULT_WAIT_TIMEOUT),
        }
    }

    pub fn with_wait_timeout(
        progress: Option<Arc<ProgressRuntime>>,
        wait_timeout: Option<Duration>,
    ) -> Self {
        Self {
            progress,
            wait_timeout,
        }
    }

    pub async fn execute<Q: QueueSubmitter>(
        &self,
        queue: &Q,
        plan: GenerationPlan,
    ) -> Result<GenerationResult, ApiError> {
        plan.validate()?;
        let mut result = GenerationResult::new(plan.plan_id.clone());
        let session_id = plan.session_id.clone();

        self.emit_envelope(
            session_id.as_deref(),
            generation_started_envelope(
                session_id.as_deref().unwrap_or(&plan.plan_id),
                GenerationStartedEventData {
                    plan_id: plan.plan_id.clone(),
                    total_levels: plan.total_levels,
                    total_nodes: plan.total_nodes,
                    target_path: plan.target_path.clone(),
                    program_kind: plan
                        .levels
                        .first()
                        .and_then(|level| level.first())
                        .map(|item| item.program.kind_str().to_string())
                        .unwrap_or_else(|| "unknown".to_string()),
                    workflow_id: plan
                        .levels
                        .first()
                        .and_then(|level| level.first())
                        .and_then(|item| item.program.workflow_id().map(ToString::to_string)),
                },
            ),
        );

        for (level_index, level_items) in plan.levels.iter().enumerate() {
            let plan_id = plan.plan_id.clone();
            let queue_priority: Priority = plan.priority.into();
            self.emit_envelope(
                session_id.as_deref(),
                level_started_envelope(
                    session_id.as_deref().unwrap_or(&plan_id),
                    LevelStartedEventData {
                        plan_id: plan_id.clone(),
                        level_index,
                        total_count: level_items.len(),
                    },
                ),
            );

            let mut generated_count = 0usize;
            let mut failed_count = 0usize;
            let mut futures = FuturesUnordered::new();
            for item in level_items {
                let item_plan_id = plan.plan_id.clone();
                self.emit_envelope(
                    session_id.as_deref(),
                    node_started_envelope(
                        session_id.as_deref().unwrap_or(&item_plan_id),
                        NodeStartedEventData {
                            plan_id: item_plan_id.clone(),
                            level_index,
                            node_id: hex::encode(item.node_id),
                            path: item.path.clone(),
                            agent_id: item.agent_id.clone(),
                            provider_name: item.provider.provider_name.clone(),
                            frame_type: item.frame_type.clone(),
                            program_kind: item.program.kind_str().to_string(),
                            workflow_id: item.program.workflow_id().map(ToString::to_string),
                        },
                    ),
                );

                let submit_plan_id = plan.plan_id.clone();
                let wait_timeout = self.wait_timeout_for_item(item);
                futures.push(async move {
                    let res = queue
                        .enqueue_and_wait_item(item, queue_priority, &submit_plan_id, wait_timeout)
                        .await;
                    (item, res)
                });
            }

            let mut fail_immediately_hit = false;
            while let Some((item, outcome)) = futures.next().await {
                match outcome {
                    Ok(frame_id) => {
                        generated_count += 1;
                        result.successes.insert(item.node_id, frame_id);
                        self.emit_envelope(
                            session_id.as_deref(),
                            node_completed_envelope(
                                session_id.as_deref().unwrap_or(&plan.plan_id),
                                NodeCompletedEventData {
                                    plan_id: plan.plan_id.clone(),
                                    level_index,
                                    node_id: hex::encode(item.node_id),
                                    path: item.path.clone(),
                                    frame_id: hex::encode(frame_id),
                                    program_kind: item.program.kind_str().to_string(),
                                    workflow_id: item.program.workflow_id().map(ToString::to_string),
                                },
                            ),
                        );
                    }
                    Err(err) => {
                        failed_count += 1;
                        result.failures.insert(
                            item.node_id,
                            GenerationErrorDetail {
                                message: err.to_string(),
                            },
                        );
                        self.emit_envelope(
                            session_id.as_deref(),
                            node_failed_envelope(
                                session_id.as_deref().unwrap_or(&plan.plan_id),
                                NodeFailedEventData {
                                    plan_id: plan.plan_id.clone(),
                                    level_index,
                                    node_id: hex::encode(item.node_id),
                                    path: item.path.clone(),
                                    error: err.to_string(),
                                    program_kind: item.program.kind_str().to_string(),
                                    workflow_id: item.program.workflow_id().map(ToString::to_string),
                                },
                            ),
                        );
                        if matches!(plan.failure_policy, FailurePolicy::FailImmediately) {
                            fail_immediately_hit = true;
                            break;
                        }
                    }
                }
            }

            result.total_generated += generated_count;
            result.total_failed += failed_count;
            result.level_summaries.push(LevelSummary {
                level_index,
                generated_count,
                failed_count,
                total_count: level_items.len(),
            });

            self.emit_envelope(
                session_id.as_deref(),
                level_completed_envelope(
                    session_id.as_deref().unwrap_or(&plan.plan_id),
                    LevelCompletedEventData {
                        plan_id: plan.plan_id.clone(),
                        level_index,
                        generated_count,
                        failed_count,
                        total_count: level_items.len(),
                    },
                ),
            );

            if fail_immediately_hit {
                self.emit_envelope(
                    session_id.as_deref(),
                    generation_failed_envelope(
                        session_id.as_deref().unwrap_or(&plan.plan_id),
                        GenerationFailedEventData {
                            plan_id: plan.plan_id.clone(),
                            reason: "fail_immediately".to_string(),
                            failed_level_index: None,
                            total_generated: result.total_generated,
                            total_failed: result.total_failed,
                        },
                    ),
                );
                return Err(ApiError::GenerationFailed(format!(
                    "Generation failed immediately for plan {}",
                    plan.plan_id
                )));
            }

            if failed_count > 0 && matches!(plan.failure_policy, FailurePolicy::StopOnLevelFailure)
            {
                self.emit_envelope(
                    session_id.as_deref(),
                    generation_failed_envelope(
                        session_id.as_deref().unwrap_or(&plan.plan_id),
                        GenerationFailedEventData {
                            plan_id: plan.plan_id.clone(),
                            reason: "stop_on_level_failure".to_string(),
                            failed_level_index: Some(level_index),
                            total_generated: result.total_generated,
                            total_failed: result.total_failed,
                        },
                    ),
                );
                return Ok(result);
            }
        }

        self.emit_envelope(
            session_id.as_deref(),
            generation_completed_envelope(
                session_id.as_deref().unwrap_or(&result.plan_id),
                GenerationCompletedEventData {
                    plan_id: result.plan_id.clone(),
                    total_generated: result.total_generated,
                    total_failed: result.total_failed,
                },
            ),
        );

        Ok(result)
    }

    fn wait_timeout_for_item(&self, item: &GenerationItem) -> Option<Duration> {
        if self.wait_timeout.is_some() && item.program.workflow_id().is_some() {
            return Some(Self::DEFAULT_WORKFLOW_WAIT_TIMEOUT);
        }
        self.wait_timeout
    }

    fn emit_envelope(
        &self,
        session_id: Option<&str>,
        envelope: crate::telemetry::events::ProgressEnvelope,
    ) {
        if let (Some(progress), Some(session_id)) = (self.progress.as_ref(), session_id) {
            progress.emit_domain_event_best_effort(
                session_id,
                &envelope.domain_id,
                &envelope.stream_id,
                &envelope.event_type,
                envelope.content_hash.clone(),
                envelope.data.clone(),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::generation::plan::{GenerationNodeType, PlanPriority};
    use crate::types::Hash;
    use parking_lot::Mutex;
    use std::collections::HashMap;

    struct MockQueue {
        outcomes: Mutex<HashMap<String, Result<FrameID, ApiError>>>,
        received_timeouts: Mutex<Vec<Option<Duration>>>,
    }

    impl MockQueue {
        fn new(outcomes: HashMap<String, Result<FrameID, ApiError>>) -> Self {
            Self {
                outcomes: Mutex::new(outcomes),
                received_timeouts: Mutex::new(Vec::new()),
            }
        }
    }

    impl QueueSubmitter for MockQueue {
        async fn enqueue_and_wait_item(
            &self,
            item: &GenerationItem,
            _priority: Priority,
            _plan_id: &str,
            wait_timeout: Option<Duration>,
        ) -> Result<FrameID, ApiError> {
            self.received_timeouts.lock().push(wait_timeout);
            self.outcomes
                .lock()
                .remove(&hex::encode(item.node_id))
                .unwrap_or_else(|| Ok(Hash::from([9u8; 32])))
        }
    }

    fn item(id: u8) -> GenerationItem {
        GenerationItem {
            node_id: Hash::from([id; 32]),
            path: format!("/tmp/{id}.txt"),
            node_type: GenerationNodeType::File,
            agent_id: "writer".to_string(),
            provider: crate::provider::ProviderExecutionBinding::new(
                "provider",
                crate::provider::ProviderRuntimeOverrides::default(),
            )
            .unwrap(),
            frame_type: "context-writer".to_string(),
            force: false,
            program: crate::context::generation::TargetExecutionProgram::single_shot(),
        }
    }

    fn workflow_item(id: u8) -> GenerationItem {
        GenerationItem {
            node_id: Hash::from([id; 32]),
            path: format!("/tmp/{id}.txt"),
            node_type: GenerationNodeType::File,
            agent_id: "writer".to_string(),
            provider: crate::provider::ProviderExecutionBinding::new(
                "provider",
                crate::provider::ProviderRuntimeOverrides::default(),
            )
            .unwrap(),
            frame_type: "context-writer".to_string(),
            force: false,
            program: crate::context::generation::TargetExecutionProgram::workflow(
                "docs_writer_thread_v1",
            ),
        }
    }

    fn plan(policy: FailurePolicy) -> GenerationPlan {
        GenerationPlan {
            plan_id: "plan-1".to_string(),
            source: "test".to_string(),
            session_id: None,
            levels: vec![vec![item(1), item(2)], vec![item(3)]],
            priority: PlanPriority::Urgent,
            failure_policy: policy,
            target_path: "/tmp".to_string(),
            total_nodes: 3,
            total_levels: 2,
        }
    }

    #[tokio::test]
    async fn continue_policy_collects_all_failures() {
        let mut outcomes = HashMap::new();
        outcomes.insert(
            hex::encode(Hash::from([2u8; 32])),
            Err(ApiError::GenerationFailed("boom".to_string())),
        );
        let queue = MockQueue::new(outcomes);
        let executor = GenerationExecutor::new(None);
        let result = executor
            .execute(&queue, plan(FailurePolicy::Continue))
            .await
            .unwrap();
        assert_eq!(result.total_generated, 2);
        assert_eq!(result.total_failed, 1);
    }

    #[tokio::test]
    async fn stop_on_level_failure_returns_partial_result() {
        let mut outcomes = HashMap::new();
        outcomes.insert(
            hex::encode(Hash::from([1u8; 32])),
            Err(ApiError::GenerationFailed("boom".to_string())),
        );
        let queue = MockQueue::new(outcomes);
        let executor = GenerationExecutor::new(None);
        let result = executor
            .execute(&queue, plan(FailurePolicy::StopOnLevelFailure))
            .await
            .unwrap();
        assert_eq!(result.level_summaries.len(), 1);
        assert_eq!(result.total_failed, 1);
    }

    #[tokio::test]
    async fn executor_uses_default_wait_timeout() {
        let queue = MockQueue::new(HashMap::new());
        let executor = GenerationExecutor::new(None);
        let _ = executor
            .execute(&queue, plan(FailurePolicy::Continue))
            .await
            .unwrap();

        let timeouts = queue.received_timeouts.lock();
        assert!(!timeouts.is_empty());
        assert!(timeouts
            .iter()
            .all(|value| *value == Some(Duration::from_secs(300))));
    }

    #[tokio::test]
    async fn executor_allows_overriding_wait_timeout() {
        let queue = MockQueue::new(HashMap::new());
        let executor = GenerationExecutor::with_wait_timeout(None, Some(Duration::from_secs(2)));
        let _ = executor
            .execute(&queue, plan(FailurePolicy::Continue))
            .await
            .unwrap();

        let timeouts = queue.received_timeouts.lock();
        assert!(!timeouts.is_empty());
        assert!(timeouts
            .iter()
            .all(|value| *value == Some(Duration::from_secs(2))));
    }

    #[tokio::test]
    async fn executor_extends_default_wait_timeout_for_workflow_items() {
        let queue = MockQueue::new(HashMap::new());
        let executor = GenerationExecutor::new(None);
        let workflow_plan = GenerationPlan {
            plan_id: "plan-1".to_string(),
            source: "test".to_string(),
            session_id: None,
            levels: vec![vec![workflow_item(1)]],
            priority: PlanPriority::Urgent,
            failure_policy: FailurePolicy::Continue,
            target_path: "/tmp".to_string(),
            total_nodes: 1,
            total_levels: 1,
        };

        let _ = executor.execute(&queue, workflow_plan).await.unwrap();

        let timeouts = queue.received_timeouts.lock();
        assert_eq!(*timeouts, vec![Some(Duration::from_secs(900))]);
    }
}
