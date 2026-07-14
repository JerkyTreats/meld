//! Bounded in-process dispatch from task-network claims to executable routes.
//!
//! The task network owns readiness, claims, initialization, outcomes, and the
//! publication outbox. This adapter owns one bounded orchestration pass. Route
//! implementations own capability-specific workspace scan and docs writer
//! execution through the already-existing root execution APIs.

mod routes;

pub use routes::{DocsFreshnessDispatchConfig, DocsFreshnessDispatchRoutes};

use meld_execution::capability::{
    CapabilityExecutionContext, CapabilityInvocationPayload, CapabilityInvocationResult,
};
use meld_execution::error::ApiError;
use meld_execution::task::{ArtifactProducerRef, ArtifactRecord, TaskArtifactRepoFactory};
use meld_execution::task_network::authority::{
    TaskNetworkAuthorityError, TaskNetworkAuthorityPorts,
};
use meld_execution::task_network::command::{Command, Request as CommandRequest, Response};
use meld_execution::task_network::contracts::stable_id;
use meld_execution::task_network::dispatch::{
    build_executor_for_claim_with_artifact_repo, failed_outcome_from_executor,
    succeeded_outcome_from_executor, AttributedOutcome, Claim, Request as DispatchRequest,
};
use meld_execution::task_network::initialization::{
    materialize_task_initialization, MaterializedTaskInitialization,
};
use meld_execution::task_network::mutation::ReadPrecondition;
use meld_execution::task_network::state::TaskNode;
use serde::Serialize;

const DISPATCH_ACTOR_ID: &str = "execution.task_dispatch";
const WORKSPACE_SCAN_CAPABILITY_TYPE_ID: &str = "workspace_scan";
const DOCS_WRITER_CAPABILITY_TYPE_ID: &str = "task_package.docs_writer";

/// Capability-specific input selected only after the task claim is durable.
pub struct TaskDispatchRouteRequest<'a> {
    /// Planned task node whose lineage selected the concrete route.
    pub node: &'a TaskNode,
    /// Fenced claim authorizing this execution attempt.
    pub claim: &'a Claim,
    /// Fully materialized task initialization and source provenance.
    pub initialization: &'a MaterializedTaskInitialization,
    /// Outer task invocation used to bind returned artifacts to the planned task.
    pub invocation: &'a CapabilityInvocationPayload,
}

/// Concrete execution routes required by the first docs freshness flywheel.
///
/// Implementations may call asynchronous execution by hosting their own
/// bounded runtime. The dispatch actor stays synchronous so it can be called
/// directly by the existing supervisor tick boundary.
pub trait TaskDispatchRoutes {
    /// Execute one planned workspace scan and return its typed artifacts.
    fn dispatch_workspace_scan(
        &self,
        request: TaskDispatchRouteRequest<'_>,
    ) -> Result<CapabilityInvocationResult, ApiError>;

    /// Execute one planned docs writer package and return mapped docs artifacts.
    fn dispatch_docs_writer(
        &self,
        request: TaskDispatchRouteRequest<'_>,
    ) -> Result<CapabilityInvocationResult, ApiError>;
}

/// Input for one bounded dispatch pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskDispatchRequest {
    /// Worker identity retained in the fenced task claim.
    pub worker_id: String,
}

/// Terminal disposition of one bounded dispatch pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskDispatchDisposition {
    /// No task was ready at the observed network revision.
    Idle,
    /// One ready task executed successfully and recorded an outcome.
    Succeeded,
    /// One claimed task failed in its concrete route and recorded an outcome.
    Failed,
}

/// Report for one pass that claims at most one ready task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskDispatchReport {
    /// Stable runtime actor identifier.
    pub actor_id: String,
    /// Task network inspected by the pass.
    pub network_id: String,
    /// Revision used to choose the ready task.
    pub input_revision: u64,
    /// Revision after the optional task outcome command.
    pub output_revision: u64,
    /// Selected task identity when one task was ready.
    pub task_instance_id: Option<String>,
    /// Selected capability route when one task was ready.
    pub capability_type_id: Option<String>,
    /// True only after a fenced claim was observed in network state.
    pub claimed: bool,
    /// True only after an attributed outcome command was accepted or replayed.
    pub outcome_recorded: bool,
    /// Terminal bounded-pass disposition.
    pub disposition: TaskDispatchDisposition,
    /// Concrete route failure retained after a failed outcome is recorded.
    pub route_error: Option<String>,
}

/// Error returned before one bounded pass can reach a recorded task outcome.
#[derive(Debug, thiserror::Error)]
pub enum TaskDispatchError {
    /// Dispatch request content is invalid.
    #[error("task dispatch request is invalid: {0}")]
    InvalidRequest(String),
    /// The task network authority could not serve the pass.
    #[error(transparent)]
    Authority(#[from] TaskNetworkAuthorityError),
    /// A ready task selected a capability not owned by this first adapter.
    #[error("task dispatch does not support capability '{capability_type_id}' for task '{task_instance_id}'")]
    UnsupportedRoute {
        /// Ready task that selected the unsupported route.
        task_instance_id: String,
        /// Capability type found in planning lineage.
        capability_type_id: String,
    },
    /// The authority rejected a claim command.
    #[error("task dispatch claim was rejected: {0}")]
    ClaimRejected(String),
    /// Claimed network state did not contain the expected records.
    #[error("task dispatch claimed state is invalid: {0}")]
    ClaimedState(String),
    /// Initialization could not be materialized from accepted network state.
    #[error("task dispatch initialization failed: {0}")]
    Initialization(String),
    /// The task artifact repository could not be opened or flushed.
    #[error("task dispatch artifact repository failed: {0}")]
    ArtifactRepository(String),
    /// The outer task executor rejected dispatch state or route artifacts.
    #[error("task dispatch executor failed: {0}")]
    Executor(#[from] ApiError),
    /// The authority rejected an attributed outcome command.
    #[error("task dispatch outcome was rejected: {0}")]
    OutcomeRejected(String),
}

/// Small in-process actor that claims and executes no more than one ready task.
pub struct TaskDispatchActor<R> {
    ports: TaskNetworkAuthorityPorts,
    artifact_repos: TaskArtifactRepoFactory,
    routes: R,
}

impl<R> TaskDispatchActor<R>
where
    R: TaskDispatchRoutes,
{
    /// Bind the adapter to one exact task-network authority and artifact store.
    pub fn new(
        ports: TaskNetworkAuthorityPorts,
        artifact_repos: TaskArtifactRepoFactory,
        routes: R,
    ) -> Self {
        Self {
            ports,
            artifact_repos,
            routes,
        }
    }

    /// Return the stable runtime actor id.
    pub fn actor_id(&self) -> &'static str {
        DISPATCH_ACTOR_ID
    }

    /// Claim and dispatch at most one ready task.
    pub fn run_once(
        &self,
        request: TaskDispatchRequest,
    ) -> Result<TaskDispatchReport, TaskDispatchError> {
        if request.worker_id.trim().is_empty() {
            return Err(TaskDispatchError::InvalidRequest(
                "worker id must be non-empty".to_string(),
            ));
        }

        let ready = self.ports.query().ready_set()?;
        let Some(task_instance_id) = ready.task_instance_ids.first().cloned() else {
            return Ok(TaskDispatchReport {
                actor_id: DISPATCH_ACTOR_ID.to_string(),
                network_id: ready.network_id,
                input_revision: ready.revision,
                output_revision: ready.revision,
                task_instance_id: None,
                capability_type_id: None,
                claimed: false,
                outcome_recorded: false,
                disposition: TaskDispatchDisposition::Idle,
                route_error: None,
            });
        };

        let selected_state = self.ports.query().state()?;
        if selected_state.revision != ready.revision
            || selected_state.state_hash != ready.state_hash
        {
            return Err(TaskDispatchError::ClaimRejected(
                "ready selection changed before route validation".to_string(),
            ));
        }
        let selected_node = selected_state.tasks.get(&task_instance_id).ok_or_else(|| {
            TaskDispatchError::ClaimedState(format!(
                "ready task '{task_instance_id}' is missing from selected state"
            ))
        })?;
        let capability_type_id = selected_node.lineage.capability_type_id.clone();
        validate_supported_route(&task_instance_id, &capability_type_id)?;

        let claim_id = dispatch_identity(
            "task-dispatch-claim",
            &ready.network_id,
            &task_instance_id,
            &request.worker_id,
        );
        let claim_command_id = dispatch_identity(
            "task-dispatch-claim-command",
            &ready.network_id,
            &task_instance_id,
            &request.worker_id,
        );
        let claim_response = self.ports.commands().try_submit(CommandRequest {
            command_id: claim_command_id,
            network_id: ready.network_id.clone(),
            base_revision: ready.revision,
            base_state_hash: ready.state_hash.clone(),
            read_preconditions: vec![ReadPrecondition::RevisionIs(ready.revision)],
            command: Command::ClaimReadyTask(DispatchRequest {
                claim_id: claim_id.clone(),
                task_instance_id: task_instance_id.clone(),
                worker_id: request.worker_id.clone(),
                idempotency_key: dispatch_identity(
                    "task-dispatch-claim-once",
                    &ready.network_id,
                    &task_instance_id,
                    &request.worker_id,
                ),
            }),
        })?;
        require_command_accepted("claim", &claim_response)?;

        let claimed_state = self.ports.query().state()?;
        let node = claimed_state
            .tasks
            .get(&task_instance_id)
            .cloned()
            .ok_or_else(|| {
                TaskDispatchError::ClaimedState(format!(
                    "claimed task '{task_instance_id}' is missing"
                ))
            })?;
        let claim = claimed_state
            .claims
            .get(&claim_id)
            .cloned()
            .ok_or_else(|| {
                TaskDispatchError::ClaimedState(format!("accepted claim '{claim_id}' is missing"))
            })?;
        let initialization = materialize_task_initialization(&claimed_state, &task_instance_id)
            .map_err(|error| {
                TaskDispatchError::Initialization(
                    error
                        .diagnostics
                        .into_iter()
                        .map(|diagnostic| diagnostic.message)
                        .collect::<Vec<_>>()
                        .join("; "),
                )
            })?;

        let repo_id = artifact_repo_id(&ready.network_id, &task_instance_id);
        let artifact_repo = self
            .artifact_repos
            .open_repo(&repo_id)
            .map_err(|error| TaskDispatchError::ArtifactRepository(error.to_string()))?;
        let mut executor = build_executor_for_claim_with_artifact_repo(
            &node,
            &claim,
            initialization.payload.clone(),
            artifact_repo,
        )?;
        let mut invocations =
            executor.release_ready_invocations(CapabilityExecutionContext::default())?;
        if invocations.len() != 1 {
            return Err(TaskDispatchError::ClaimedState(format!(
                "planned task '{task_instance_id}' released {} outer invocations",
                invocations.len()
            )));
        }
        let invocation = invocations.remove(0);
        if invocation.capability_instance_id != node.lineage.step_id {
            return Err(TaskDispatchError::ClaimedState(format!(
                "planned task '{task_instance_id}' invocation does not match lineage step"
            )));
        }

        let route_result = match capability_type_id.as_str() {
            WORKSPACE_SCAN_CAPABILITY_TYPE_ID => {
                self.routes
                    .dispatch_workspace_scan(TaskDispatchRouteRequest {
                        node: &node,
                        claim: &claim,
                        initialization: &initialization,
                        invocation: &invocation,
                    })
            }
            DOCS_WRITER_CAPABILITY_TYPE_ID => {
                self.routes.dispatch_docs_writer(TaskDispatchRouteRequest {
                    node: &node,
                    claim: &claim,
                    initialization: &initialization,
                    invocation: &invocation,
                })
            }
            _ => unreachable!("route was validated before claim"),
        };

        let outcome_id = dispatch_identity(
            "task-dispatch-outcome",
            &ready.network_id,
            &task_instance_id,
            &claim.claim_id,
        );
        let route_result = route_result.and_then(|result| {
            validate_route_result(&capability_type_id, &result)?;
            Ok(result)
        });
        let route_error = match route_result {
            Ok(result) => {
                executor.record_success(&invocation.invocation_id, result.emitted_artifacts)?;
                None
            }
            Err(error) => {
                let error_message = error.to_string();
                executor.record_failure(
                    &invocation.invocation_id,
                    failure_artifact(&node, &invocation, &error_message),
                    error_message.clone(),
                )?;
                Some(error_message)
            }
        };
        executor
            .artifact_repo()
            .flush()
            .map_err(|error| TaskDispatchError::ArtifactRepository(error.to_string()))?;

        let outcome = match &route_error {
            Some(error) => {
                failed_outcome_from_executor(&outcome_id, &claim, &executor, error.clone())
            }
            None => succeeded_outcome_from_executor(&outcome_id, &claim, &executor),
        };
        let attributed = AttributedOutcome::for_task(outcome, &node)?;
        let outcome_command_id = dispatch_identity(
            "task-dispatch-outcome-command",
            &ready.network_id,
            &task_instance_id,
            &claim.claim_id,
        );
        let outcome_response = self.ports.commands().try_submit(CommandRequest {
            command_id: outcome_command_id,
            network_id: claimed_state.network_id.clone(),
            base_revision: claimed_state.revision,
            base_state_hash: claimed_state.state_hash.clone(),
            read_preconditions: vec![ReadPrecondition::RevisionIs(claimed_state.revision)],
            command: Command::RecordAttributedTaskOutcome(attributed),
        })?;
        let output_revision = require_command_accepted("outcome", &outcome_response)?;

        Ok(TaskDispatchReport {
            actor_id: DISPATCH_ACTOR_ID.to_string(),
            network_id: ready.network_id,
            input_revision: ready.revision,
            output_revision,
            task_instance_id: Some(task_instance_id),
            capability_type_id: Some(capability_type_id),
            claimed: true,
            outcome_recorded: true,
            disposition: if route_error.is_some() {
                TaskDispatchDisposition::Failed
            } else {
                TaskDispatchDisposition::Succeeded
            },
            route_error,
        })
    }
}

fn validate_supported_route(
    task_instance_id: &str,
    capability_type_id: &str,
) -> Result<(), TaskDispatchError> {
    if matches!(
        capability_type_id,
        WORKSPACE_SCAN_CAPABILITY_TYPE_ID | DOCS_WRITER_CAPABILITY_TYPE_ID
    ) {
        Ok(())
    } else {
        Err(TaskDispatchError::UnsupportedRoute {
            task_instance_id: task_instance_id.to_string(),
            capability_type_id: capability_type_id.to_string(),
        })
    }
}

fn validate_route_result(
    capability_type_id: &str,
    result: &CapabilityInvocationResult,
) -> Result<(), ApiError> {
    let required_artifact_type_id = match capability_type_id {
        WORKSPACE_SCAN_CAPABILITY_TYPE_ID => "workspace_snapshot_ref",
        DOCS_WRITER_CAPABILITY_TYPE_ID => "docs_patch",
        _ => {
            return Err(ApiError::ConfigError(format!(
                "task dispatch cannot validate unsupported capability '{capability_type_id}'"
            )))
        }
    };
    if result.emitted_artifacts.iter().any(|artifact| {
        artifact.artifact_type_id == required_artifact_type_id && artifact.schema_version == 1
    }) {
        Ok(())
    } else {
        Err(ApiError::GenerationFailed(format!(
            "task dispatch route '{capability_type_id}' did not emit required artifact '{required_artifact_type_id}' schema '1'"
        )))
    }
}

fn require_command_accepted(
    operation: &str,
    response: &Response,
) -> Result<u64, TaskDispatchError> {
    match response {
        Response::Accepted { revision, .. } | Response::Duplicate { revision, .. } => Ok(*revision),
        Response::Rejected(rejection) if operation == "claim" => {
            Err(TaskDispatchError::ClaimRejected(format!("{rejection:?}")))
        }
        Response::Rejected(rejection) => {
            Err(TaskDispatchError::OutcomeRejected(format!("{rejection:?}")))
        }
    }
}

fn artifact_repo_id(network_id: &str, task_instance_id: &str) -> String {
    #[derive(Serialize)]
    struct Identity<'a> {
        network_id: &'a str,
        task_instance_id: &'a str,
    }

    stable_id(
        "task-dispatch-artifacts",
        &Identity {
            network_id,
            task_instance_id,
        },
    )
}

fn dispatch_identity(
    prefix: &str,
    network_id: &str,
    task_instance_id: &str,
    attempt_identity: &str,
) -> String {
    #[derive(Serialize)]
    struct Identity<'a> {
        network_id: &'a str,
        task_instance_id: &'a str,
        attempt_identity: &'a str,
    }

    stable_id(
        prefix,
        &Identity {
            network_id,
            task_instance_id,
            attempt_identity,
        },
    )
}

fn failure_artifact(
    node: &TaskNode,
    invocation: &CapabilityInvocationPayload,
    error: &str,
) -> ArtifactRecord {
    ArtifactRecord {
        artifact_id: format!("{}::failure_summary", invocation.invocation_id),
        artifact_type_id: "failure_summary".to_string(),
        schema_version: 1,
        content: serde_json::json!({
            "error": error,
            "capability_type_id": node.lineage.capability_type_id,
        }),
        producer: ArtifactProducerRef {
            task_id: node.compiled_task.task_id.clone(),
            capability_instance_id: invocation.capability_instance_id.clone(),
            invocation_id: Some(invocation.invocation_id.clone()),
            output_slot_id: Some("failure_summary".to_string()),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use meld_events::DomainObjectRef;
    use meld_execution::capability::{
        CapabilityCatalog, CapabilityTypeContract, ExecutionClass, ExecutionContract,
        OutputSlotSpec, ScopeContract,
    };
    use meld_execution::task::{
        TaskCompiler, TaskDefinition, TaskDefinitionCompiler, TaskRunContext,
    };
    use meld_execution::task_network::authority::TaskNetworkAuthority;
    use meld_execution::task_network::command::Command;
    use meld_execution::task_network::mutation::{Inject, Mutation, Set};
    use meld_execution::task_network::state::{TaskLineage, TaskNode, TaskStatus};
    use meld_execution::task_network::store::TaskNetworkStoreFactory;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    struct RecordingRoutes {
        scan_calls: Arc<AtomicUsize>,
        docs_calls: Arc<AtomicUsize>,
    }

    impl RecordingRoutes {
        fn artifact(
            request: &TaskDispatchRouteRequest<'_>,
            artifact_type_id: &str,
        ) -> ArtifactRecord {
            ArtifactRecord {
                artifact_id: format!("{}::{artifact_type_id}", request.invocation.invocation_id),
                artifact_type_id: artifact_type_id.to_string(),
                schema_version: 1,
                content: serde_json::json!({ "route": artifact_type_id }),
                producer: ArtifactProducerRef {
                    task_id: request.node.compiled_task.task_id.clone(),
                    capability_instance_id: request.invocation.capability_instance_id.clone(),
                    invocation_id: Some(request.invocation.invocation_id.clone()),
                    output_slot_id: Some(artifact_type_id.to_string()),
                },
            }
        }
    }

    impl TaskDispatchRoutes for RecordingRoutes {
        fn dispatch_workspace_scan(
            &self,
            request: TaskDispatchRouteRequest<'_>,
        ) -> Result<CapabilityInvocationResult, ApiError> {
            self.scan_calls.fetch_add(1, Ordering::SeqCst);
            Ok(CapabilityInvocationResult {
                emitted_artifacts: vec![Self::artifact(&request, "workspace_snapshot_ref")],
            })
        }

        fn dispatch_docs_writer(
            &self,
            request: TaskDispatchRouteRequest<'_>,
        ) -> Result<CapabilityInvocationResult, ApiError> {
            self.docs_calls.fetch_add(1, Ordering::SeqCst);
            Ok(CapabilityInvocationResult {
                emitted_artifacts: vec![Self::artifact(&request, "docs_patch")],
            })
        }
    }

    fn capability_contract(
        capability_type_id: &str,
        artifact_type_id: &str,
    ) -> CapabilityTypeContract {
        CapabilityTypeContract {
            capability_type_id: capability_type_id.to_string(),
            capability_version: 1,
            owning_domain: "execution".to_string(),
            scope_contract: ScopeContract {
                scope_kind: "workspace".to_string(),
                scope_ref_kind: "workspace_root".to_string(),
                allow_fan_out: false,
            },
            binding_contract: Vec::new(),
            input_contract: Vec::new(),
            output_contract: vec![OutputSlotSpec {
                slot_id: artifact_type_id.to_string(),
                artifact_type_id: artifact_type_id.to_string(),
                schema_version: 1,
                guaranteed: true,
            }],
            effect_contract: Vec::new(),
            execution_contract: ExecutionContract {
                execution_class: ExecutionClass::Inline,
                completion_semantics: "artifacts".to_string(),
                retry_class: "none".to_string(),
                cancellation_supported: false,
            },
        }
    }

    fn task_node(task_instance_id: &str, step_id: &str, capability_type_id: &str) -> TaskNode {
        let artifact_type_id = if capability_type_id == WORKSPACE_SCAN_CAPABILITY_TYPE_ID {
            "workspace_snapshot_ref"
        } else {
            "docs_patch"
        };
        let mut catalog = CapabilityCatalog::new();
        catalog
            .register(capability_contract(capability_type_id, artifact_type_id))
            .unwrap();
        let compiled_task = TaskCompiler::new()
            .compile_task_definition(
                &TaskDefinition {
                    task_id: task_instance_id.to_string(),
                    task_version: 1,
                    init_slots: Vec::new(),
                    capability_instances: vec![
                        meld_execution::capability::BoundCapabilityInstance {
                            capability_instance_id: step_id.to_string(),
                            capability_type_id: capability_type_id.to_string(),
                            capability_version: 1,
                            scope_ref: "workspace".to_string(),
                            scope_kind: "workspace".to_string(),
                            binding_values: Vec::new(),
                            input_wiring: Vec::new(),
                        },
                    ],
                },
                &catalog,
            )
            .unwrap();
        TaskNode {
            task_instance_id: task_instance_id.to_string(),
            lifecycle_epoch: 0,
            compiled_task,
            init_sources: Vec::new(),
            task_run_context: TaskRunContext {
                task_run_id: format!("run-{task_instance_id}"),
                session_id: Some("session-docs".to_string()),
                trigger: "test".to_string(),
            },
            lineage: TaskLineage {
                composition_id: "composition-docs".to_string(),
                goal_id: "goal-docs".to_string(),
                method_id: "refresh_docs_v1".to_string(),
                step_id: step_id.to_string(),
                operator_id: step_id.to_string(),
                world_state_frame_id: "frame-docs".to_string(),
                subject: Some(
                    DomainObjectRef::new("workspace_fs", "node", "workspace-root").unwrap(),
                ),
                capability_type_id: capability_type_id.to_string(),
                capability_version: 1,
            },
        }
    }

    fn command_for_head(
        ports: &TaskNetworkAuthorityPorts,
        command_id: &str,
        command: Command,
    ) -> CommandRequest {
        let head = ports.query().head().unwrap();
        CommandRequest {
            command_id: command_id.to_string(),
            network_id: head.network_id,
            base_revision: head.revision,
            base_state_hash: head.state_hash,
            read_preconditions: vec![ReadPrecondition::RevisionIs(head.revision)],
            command,
        }
    }

    fn inject_tasks(ports: &TaskNetworkAuthorityPorts, nodes: Vec<TaskNode>) {
        let mutations = nodes
            .into_iter()
            .map(|node| Mutation::Inject(Inject::new(node, Vec::new())))
            .collect();
        let request = command_for_head(
            ports,
            "inject-tasks",
            Command::ApplyMutationSet(Set::new(
                "network-docs",
                "composition-docs",
                "inject-once",
                mutations,
                Vec::new(),
            )),
        );
        assert!(matches!(
            ports.commands().try_submit(request).unwrap(),
            Response::Accepted { .. }
        ));
    }

    struct Harness {
        authority: TaskNetworkAuthority,
        artifacts: TaskArtifactRepoFactory,
        _temp: tempfile::TempDir,
    }

    impl Harness {
        fn new() -> Self {
            let temp = tempfile::tempdir().unwrap();
            let network_factory = TaskNetworkStoreFactory::new(temp.path().join("networks"));
            let authority =
                TaskNetworkAuthority::open(&network_factory, "network-docs", 8).unwrap();
            let artifact_db = sled::open(temp.path().join("artifacts")).unwrap();
            Self {
                authority,
                artifacts: TaskArtifactRepoFactory::new(artifact_db),
                _temp: temp,
            }
        }
    }

    #[test]
    fn run_once_claims_only_one_ready_task_and_selects_scan_route() {
        let harness = Harness::new();
        let ports = harness.authority.ports();
        inject_tasks(
            &ports,
            vec![
                task_node(
                    "task-a-scan",
                    "scan_workspace",
                    WORKSPACE_SCAN_CAPABILITY_TYPE_ID,
                ),
                task_node(
                    "task-z-docs",
                    "run_docs_writer",
                    DOCS_WRITER_CAPABILITY_TYPE_ID,
                ),
            ],
        );
        let scan_calls = Arc::new(AtomicUsize::new(0));
        let docs_calls = Arc::new(AtomicUsize::new(0));
        let actor = TaskDispatchActor::new(
            ports.clone(),
            harness.artifacts.clone(),
            RecordingRoutes {
                scan_calls: Arc::clone(&scan_calls),
                docs_calls: Arc::clone(&docs_calls),
            },
        );

        let report = actor
            .run_once(TaskDispatchRequest {
                worker_id: "worker-docs".to_string(),
            })
            .unwrap();

        assert_eq!(report.disposition, TaskDispatchDisposition::Succeeded);
        assert_eq!(report.task_instance_id.as_deref(), Some("task-a-scan"));
        assert_eq!(scan_calls.load(Ordering::SeqCst), 1);
        assert_eq!(docs_calls.load(Ordering::SeqCst), 0);
        let state = ports.query().state().unwrap();
        assert!(matches!(
            state.statuses.get("task-a-scan"),
            Some(TaskStatus::Succeeded { .. })
        ));
        assert_eq!(
            state.statuses.get("task-z-docs"),
            Some(&TaskStatus::Pending)
        );
        assert_eq!(state.outcomes.len(), 1);
        assert_eq!(state.publications.len(), 1);
        assert!(state
            .publications
            .values()
            .all(|publication| publication.semantic_lineage.is_some()));
    }

    #[test]
    fn docs_route_persists_artifact_and_submits_attributed_outcome() {
        let harness = Harness::new();
        let ports = harness.authority.ports();
        inject_tasks(
            &ports,
            vec![task_node(
                "task-docs",
                "run_docs_writer",
                DOCS_WRITER_CAPABILITY_TYPE_ID,
            )],
        );
        let actor = TaskDispatchActor::new(
            ports.clone(),
            harness.artifacts.clone(),
            RecordingRoutes {
                scan_calls: Arc::new(AtomicUsize::new(0)),
                docs_calls: Arc::new(AtomicUsize::new(0)),
            },
        );

        let report = actor
            .run_once(TaskDispatchRequest {
                worker_id: "worker-docs".to_string(),
            })
            .unwrap();

        assert_eq!(
            report.capability_type_id.as_deref(),
            Some(DOCS_WRITER_CAPABILITY_TYPE_ID)
        );
        let repo = harness
            .artifacts
            .open_repo(artifact_repo_id("network-docs", "task-docs"))
            .unwrap();
        assert!(repo
            .record()
            .artifacts
            .iter()
            .any(|artifact| artifact.artifact_type_id == "docs_patch"));
        let state = ports.query().state().unwrap();
        let publication = state.publications.values().next().unwrap();
        assert!(publication.semantic_lineage.is_some());
        assert!(publication
            .outcome
            .artifact_records
            .iter()
            .any(|artifact| artifact.artifact_type_id == "docs_patch"));
    }

    #[test]
    fn unsupported_route_is_rejected_before_claim() {
        let harness = Harness::new();
        let ports = harness.authority.ports();
        inject_tasks(
            &ports,
            vec![task_node("task-other", "other", "other.capability")],
        );
        let actor = TaskDispatchActor::new(
            ports.clone(),
            harness.artifacts.clone(),
            RecordingRoutes {
                scan_calls: Arc::new(AtomicUsize::new(0)),
                docs_calls: Arc::new(AtomicUsize::new(0)),
            },
        );

        let error = actor
            .run_once(TaskDispatchRequest {
                worker_id: "worker-docs".to_string(),
            })
            .unwrap_err();

        assert!(matches!(error, TaskDispatchError::UnsupportedRoute { .. }));
        let state = ports.query().state().unwrap();
        assert!(state.claims.is_empty());
        assert_eq!(state.statuses.get("task-other"), Some(&TaskStatus::Pending));
    }
}
