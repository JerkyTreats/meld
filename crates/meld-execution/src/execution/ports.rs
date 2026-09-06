//! Execution adapter ports used by workflow and task runtimes.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

use super::contracts::ProviderExecutionBinding;
use crate::generation::{
    GeneratedFrameMetadataInput, PreviousMetadataSnapshotView, PromptLineageRequest,
};

/// Exact effect boundary derived from an admitted Task's authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionEffectAuthority {
    /// Issuer of the admitted Task, distinct from the principal granting its authority.
    pub issuer_ref: String,
    /// Principal whose effect grant Execution validated.
    pub principal_id: String,
    /// Exact object within the admitted grant.
    pub subject: meld_events::DomainObjectRef,
    /// Opaque live fence retained from the accepted admission.
    pub fence_ref: String,
}

/// Event publication context supplied by callers that want durable envelopes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionEventContext {
    /// Validated effect authority, absent for telemetry-only contexts.
    pub effect_authority: Option<ExecutionEffectAuthority>,
    /// Session identifier used as the event stream root.
    pub session_id: String,
}

/// Execution visible node kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionNodeKind {
    /// File-like workspace node.
    File,
    /// Directory-like workspace node.
    Directory,
}

/// Execution visible frame record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionFrame<F> {
    /// Stable frame identifier from the backing context store.
    pub frame_id: F,
    /// Domain frame type such as a workflow turn output.
    pub frame_type: String,
    /// Agent that produced the frame.
    pub agent_id: String,
    /// Raw frame bytes.
    pub content: Vec<u8>,
}

/// Execution visible node record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionNodeRecord<N> {
    /// Stable node identifier from the backing context store.
    pub node_id: N,
    /// Workspace relative path.
    pub path: String,
    /// File or directory classification.
    pub node_kind: ExecutionNodeKind,
    /// Child node identifiers for directory nodes.
    pub children: Vec<N>,
    /// True when the node remains addressable but is excluded by default.
    pub tombstoned: bool,
}

/// Node record plus frame history used by execution planning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionNodeContext<N, F> {
    /// Current node metadata.
    pub node_record: ExecutionNodeRecord<N>,
    /// Frames selected for the requested view.
    pub frames: Vec<ExecutionFrame<F>>,
    /// Total frame count known to the backing context store.
    pub frame_count: usize,
}

/// World model anchor for an artifact already produced by a task run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskRunArtifactAnchor {
    /// Domain that owns the target object.
    pub target_domain_id: String,
    /// Target object kind inside the owning domain.
    pub target_object_kind: String,
    /// Target object identifier inside the owning domain.
    pub target_object_id: String,
}

/// Read-only view of provider preparation output used by workflow execution.
pub trait ProviderPreparationView {
    /// Stable provider type slug used in prompt lineage records.
    fn provider_type_slug(&self) -> &str;
    /// Concrete model selected for the request.
    fn model_name(&self) -> &str;
}

/// Read access to workspace context and execution frame state.
pub trait ContextReadPort: Send + Sync {
    /// Adapter error type.
    type Error;
    /// Backing agent identity type.
    type AgentIdentity;
    /// Backing node identifier type.
    type NodeId;
    /// Backing frame identifier type.
    type FrameId;
    /// Adapter-specific context view selector.
    type ContextView;
    /// Adapter-specific node context type.
    type NodeContext;
    /// Adapter-specific frame type.
    type Frame;
    /// Adapter-specific node record type.
    type NodeRecord;

    /// Loads the configured agent identity.
    fn get_agent(&self, agent_id: &str) -> Result<Self::AgentIdentity, Self::Error>;
    /// Returns the current frame head for one node and frame type.
    fn get_head(
        &self,
        node_id: &Self::NodeId,
        frame_type: &str,
    ) -> Result<Option<Self::FrameId>, Self::Error>;
    /// Returns the current frame head with optional tombstone visibility.
    fn find_frame_head(
        &self,
        node_id: &Self::NodeId,
        frame_type: &str,
        include_tombstoned: bool,
    ) -> Result<Option<Self::FrameId>, Self::Error>;
    /// Loads an adapter-specific node view.
    fn get_node(
        &self,
        node_id: Self::NodeId,
        view: Self::ContextView,
    ) -> Result<Self::NodeContext, Self::Error>;
    /// Loads frames for one node and frame type through the adapter view.
    fn context_by_type(
        &self,
        node_id: Self::NodeId,
        frame_type: &str,
        max_frames: usize,
    ) -> Result<Self::NodeContext, Self::Error>;
    /// Reads one adapter frame by identifier.
    fn read_frame(&self, frame_id: &Self::FrameId) -> Result<Option<Self::Frame>, Self::Error>;
    /// Reads one adapter node record by identifier.
    fn read_node_record(
        &self,
        node_id: &Self::NodeId,
    ) -> Result<Option<Self::NodeRecord>, Self::Error>;
    /// Reads one adapter node record by workspace path.
    fn read_node_record_by_path(
        &self,
        path: &Path,
        include_tombstoned: bool,
    ) -> Result<Option<Self::NodeRecord>, Self::Error>;
    /// Lists adapter node records.
    fn list_node_records(
        &self,
        include_tombstoned: bool,
    ) -> Result<Vec<Self::NodeRecord>, Self::Error>;
    /// Returns the workspace root when the adapter has one.
    fn workspace_root(&self) -> Option<&Path>;
    /// Reads one execution normalized frame.
    fn read_execution_frame(
        &self,
        frame_id: &Self::FrameId,
    ) -> Result<Option<ExecutionFrame<Self::FrameId>>, Self::Error>;
    /// Reads one execution normalized node record.
    fn read_execution_node_record(
        &self,
        node_id: &Self::NodeId,
    ) -> Result<Option<ExecutionNodeRecord<Self::NodeId>>, Self::Error>;
    /// Reads execution normalized frames for one node and frame type.
    fn context_frames_by_type(
        &self,
        node_id: Self::NodeId,
        frame_type: &str,
        max_frames: usize,
    ) -> Result<ExecutionNodeContext<Self::NodeId, Self::FrameId>, Self::Error>;
}

/// Write access to workspace context frame heads.
pub trait ContextWritePort: Send + Sync {
    /// Adapter error type.
    type Error;
    /// Backing node identifier type.
    type NodeId;
    /// Backing frame identifier type.
    type FrameId;
    /// Adapter frame type accepted for writes.
    type Frame;

    /// Writes one frame and returns its durable identifier.
    fn put_frame(
        &self,
        node_id: Self::NodeId,
        frame: Self::Frame,
        agent_id: String,
    ) -> Result<Self::FrameId, Self::Error>;
    /// Tombstones the current head for one node and frame type.
    fn tombstone_head(
        &self,
        node_id: Self::NodeId,
        frame_type: &str,
    ) -> Result<Option<Self::FrameId>, Self::Error>;
}

/// Read and write access to prompt artifacts.
pub trait PromptArtifactReadPort: Send + Sync {
    /// Adapter error type.
    type Error;
    /// Adapter prompt artifact kind type.
    type ArtifactKind;
    /// Adapter prompt artifact reference type.
    type ArtifactRef;

    /// Reads one prompt artifact as bytes.
    fn read_prompt_artifact_bytes(&self, artifact_id: &str) -> Result<Vec<u8>, Self::Error>;
    /// Persists one UTF-8 prompt artifact and returns its reference.
    fn write_prompt_artifact_utf8(
        &self,
        kind: Self::ArtifactKind,
        value: &str,
    ) -> Result<Self::ArtifactRef, Self::Error>;
}

/// Loads the system prompt for an agent.
pub trait SystemPromptPort: Send + Sync {
    /// Adapter error type.
    type Error;

    /// Returns the system prompt text for one agent.
    fn load_system_prompt(&self, agent_id: &str) -> Result<String, Self::Error>;
}

/// Resolves workspace target selectors into node identifiers.
pub trait NodeResolutionPort: Send + Sync {
    /// Adapter error type.
    type Error;
    /// Backing node identifier type.
    type NodeId;

    /// Resolves a path or node selector against a workspace root.
    fn resolve_workspace_node_id(
        &self,
        workspace_root: &Path,
        path: Option<&Path>,
        node: Option<&str>,
        include_tombstoned: bool,
    ) -> Result<Self::NodeId, Self::Error>;
}

/// Prepares and validates provider bindings before execution.
pub trait ProviderValidationPort: Send + Sync {
    /// Adapter error type.
    type Error;
    /// Generation request type.
    type GenerationRequest;
    /// Prepared provider view returned by validation.
    type ProviderPreparation: ProviderPreparationView;

    /// Validates and prepares provider execution for one generation request.
    fn prepare_provider_for_request(
        &self,
        request: &Self::GenerationRequest,
    ) -> Result<Self::ProviderPreparation, Self::Error>;

    /// Validates a provider binding before it is used by execution.
    fn validate_provider_binding(
        &self,
        binding: &ProviderExecutionBinding,
    ) -> Result<(), Self::Error>;
}

/// Executes provider completion requests.
#[async_trait]
pub trait ProviderExecutionPort: Send + Sync {
    /// Adapter error type.
    type Error;
    /// Generation request type.
    type GenerationRequest: Sync;
    /// Prepared provider type.
    type ProviderPreparation: Sync;
    /// Chat message type sent to the provider.
    type ChatMessage: Send;
    /// Completion response type returned by the provider.
    type CompletionResponse;

    /// Runs one provider completion request.
    async fn execute_completion(
        &self,
        request: &Self::GenerationRequest,
        preparation: &Self::ProviderPreparation,
        messages: Vec<Self::ChatMessage>,
        event_context: Option<&ExecutionEventContext>,
    ) -> Result<Self::CompletionResponse, Self::Error>;
}

/// Persists prompt lineage artifacts for a generation request.
pub trait PromptLineagePort: Send + Sync {
    /// Adapter error type.
    type Error;
    /// Prompt lineage input type.
    type PromptLineageRequest;
    /// Prepared lineage output type.
    type PreparedPromptLineage;

    /// Builds prompt lineage records for one rendered prompt.
    fn prepare_prompt_lineage(
        &self,
        input: &PromptLineageRequest,
        agent_id: &str,
        provider: &str,
        model: &str,
        provider_type: &str,
    ) -> Result<Self::PreparedPromptLineage, Self::Error>;
}

/// Builds generated frame metadata and loads prior metadata snapshots.
pub trait GeneratedMetadataPort: Send + Sync {
    /// Adapter error type.
    type Error;
    /// Generation request type.
    type GenerationRequest;
    /// Metadata input type.
    type GeneratedMetadataInput;
    /// Previous metadata snapshot type.
    type PreviousMetadataSnapshotView;
    /// Generated frame metadata type.
    type FrameMetadata;
    /// Metadata builder implementation type.
    type GeneratedMetadataBuilder: ?Sized;

    /// Loads prior metadata for the target generation request.
    fn load_previous_metadata_snapshot(
        &self,
        request: &Self::GenerationRequest,
    ) -> Result<PreviousMetadataSnapshotView, Self::Error>;

    /// Builds and validates metadata for one generated frame.
    fn build_and_validate_generated_metadata(
        &self,
        request: &Self::GenerationRequest,
        input: &GeneratedFrameMetadataInput,
        metadata_builder: &Self::GeneratedMetadataBuilder,
    ) -> Result<Self::FrameMetadata, Self::Error>;
}

/// Publishes execution event envelopes.
pub trait EventPublicationPort: Send + Sync {
    /// Adapter error type.
    type Error;
    /// Event envelope type accepted by the adapter.
    type EventEnvelope;

    /// Neutral durable append authority when this execution context is explicitly bound.
    /// Callers requiring an append receipt must refuse an unavailable binding.
    fn durable_event_append(&self) -> Option<meld_events::EventAppendCapability> {
        None
    }

    /// Publishes one execution envelope in the supplied event context.
    fn publish_execution_envelope(
        &self,
        event_context: &ExecutionEventContext,
        envelope: Self::EventEnvelope,
    ) -> Result<(), Self::Error>;
}

/// Emits runtime progress events for interactive callers.
pub trait ExecutionProgressPort: Send + Sync {
    /// Adapter error type.
    type Error;

    /// Emits one progress event payload in the supplied event context.
    fn emit_progress_event(
        &self,
        event_context: &ExecutionEventContext,
        event_type: &str,
        payload: Value,
    ) -> Result<(), Self::Error>;
}

/// Executes workspace scans behind the workspace domain boundary.
///
/// The adapter owns filesystem walking and workspace tree updates. Canonical
/// event append stays behind the runtime publication boundary: the outcome
/// carries publication candidates instead of appended events.
pub trait WorkspaceScanPort: Send + Sync {
    /// Adapter error type.
    type Error;
    /// Typed scan request accepted by the adapter.
    type ScanRequest;
    /// Typed scan outcome produced by the adapter.
    type ScanOutcome;

    /// Executes one workspace scan and returns its typed outcome.
    fn scan_workspace(&self, request: &Self::ScanRequest)
        -> Result<Self::ScanOutcome, Self::Error>;
}

/// Reads task-run artifact anchors from the world model.
pub trait WorldModelQueryPort: Send + Sync {
    /// Adapter error type.
    type Error;

    /// Returns the current artifact anchor for one task run and artifact type.
    fn current_artifact_for_task_run(
        &self,
        task_run_id: &str,
        artifact_type_id: &str,
    ) -> Result<Option<TaskRunArtifactAnchor>, Self::Error>;
}

/// Typed belief status label carried by [`BeliefSubjectSignal`].
///
/// Mirrors the world model's belief status set: this crate cannot depend on
/// the world model, so the owning adapter maps its status into this label at
/// the port boundary and must be extended in lockstep. Serialized as the
/// bare variant name (`"Settled"`), byte-identical to the string labels it
/// replaced, so existing bundle canonical JSON and digests are unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BeliefStatusLabel {
    /// View is settled on a current revision.
    Settled,
    /// View is flagged stale by freshness tracking.
    Stale,
    /// View needs a new observation before it can settle.
    NeedsObservation,
    /// View needs assessment of gathered observations.
    NeedsAssessment,
    /// Assessment is queued but not complete.
    AssessmentPending,
    /// View is invalid and must not be trusted.
    Invalid,
}

impl BeliefStatusLabel {
    /// Stable string form, identical to the serde-serialized value.
    pub fn as_str(&self) -> &'static str {
        match self {
            BeliefStatusLabel::Settled => "Settled",
            BeliefStatusLabel::Stale => "Stale",
            BeliefStatusLabel::NeedsObservation => "NeedsObservation",
            BeliefStatusLabel::NeedsAssessment => "NeedsAssessment",
            BeliefStatusLabel::AssessmentPending => "AssessmentPending",
            BeliefStatusLabel::Invalid => "Invalid",
        }
    }
}

impl std::fmt::Display for BeliefStatusLabel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Planner-safe belief signal for one subject, projected for context-side
/// selection and prompt conditioning. This is a thin view over the world
/// model's belief query surface; it never carries prompt text.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BeliefSubjectSignal {
    /// Current belief revision id, when the view has settled at least once.
    pub revision_id: Option<String>,
    /// Belief status label for the current view.
    pub status: BeliefStatusLabel,
    /// Planner-facing confidence for the current view.
    pub confidence: f64,
    /// True when the view is flagged stale by freshness tracking.
    pub stale: bool,
    /// True when unresolved counterevidence contradicts the view.
    pub contradicted: bool,
    /// Evidence ids counted as unresolved contradicted claims.
    pub contradicted_evidence_ids: Vec<String>,
    /// Evidence ids hydratable for provenance.
    pub evidence_ids: Vec<String>,
    /// Source fact ids hydratable for provenance.
    pub source_fact_ids: Vec<String>,
    /// Sequence the view is current as of; selection is deterministic at
    /// this sequence.
    pub as_of_seq: u64,
}

/// Read port for current belief signals consumed by context assembly.
///
/// Owned by the world model side of the boundary; the context domain only
/// reads projected signals and never mutates belief state through it.
pub trait BeliefContextReadPort: Send + Sync {
    /// Adapter error type.
    type Error;

    /// Reads the current belief signal for one workspace node subject under
    /// the given belief family, or `None` when no belief covers the subject.
    fn current_belief_signal(
        &self,
        node_id_hex: &str,
        family_id: &str,
    ) -> Result<Option<BeliefSubjectSignal>, Self::Error>;
}

/// Loads workflow profiles for execution.
pub trait WorkflowProfileLoadPort: Send + Sync {
    /// Adapter error type.
    type Error;
    /// Workflow profile type returned by the adapter.
    type WorkflowProfile;

    /// Loads one workflow profile by identifier.
    fn load_workflow_profile(
        &self,
        workflow_id: &str,
    ) -> Result<Self::WorkflowProfile, Self::Error>;
}

/// Composite context required for deterministic execution planning.
pub trait ExecutionContext:
    ContextReadPort
    + ContextWritePort
    + PromptArtifactReadPort
    + SystemPromptPort
    + NodeResolutionPort
    + ProviderValidationPort
    + ProviderExecutionPort
    + PromptLineagePort
    + GeneratedMetadataPort
{
}

impl<T> ExecutionContext for T where
    T: ContextReadPort
        + ContextWritePort
        + PromptArtifactReadPort
        + SystemPromptPort
        + NodeResolutionPort
        + ProviderValidationPort
        + ProviderExecutionPort
        + PromptLineagePort
        + GeneratedMetadataPort
{
}

/// Composite context required for execution runtimes with events and progress.
pub trait ExecutionRuntimeContext:
    ExecutionContext + EventPublicationPort + ExecutionProgressPort
{
}

impl<T> ExecutionRuntimeContext for T where
    T: ExecutionContext + EventPublicationPort + ExecutionProgressPort
{
}
