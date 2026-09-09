use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use meld_execution::capability::{
    CapabilityContractRevision, CapabilityContractRevisionRef, CapabilityInvocationPayload,
    CapabilityRuntimeInit,
};
use meld_world_model::curation::{CurationRuleBinding, CurationRuleTemplate};
use serde::{Deserialize, Serialize};

use crate::execution::ExecutionEventContext;
use crate::runtime::contracts::{WorkBudget, WorkerTickReport};
use crate::runtime::lifecycle::{ParticipantLifecycleContextV1, StructuralWakeRef};
use crate::theory::{
    InstalledPackageLinkView, InstalledTheoryComponentRef, TheoryRevisionRef, TheoryRouteContract,
    TheoryRouteId,
};

pub const OWNER_PROTOCOL_VERSION: u32 = 1;

/// Physical executable selection. The digest is checked before a child starts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnerExecutableV1 {
    pub path: PathBuf,
    pub content_hash: String,
}

/// Explicit transport bounds, independent of any owner's semantic capture limits.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnerConnectionLimitsV1 {
    pub request_timeout_ms: u64,
    pub max_message_bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnerImplementationOfferV1 {
    pub contract_ref: CapabilityContractRevisionRef,
    pub implementation_ref: String,
    pub required_binding_ids: BTreeSet<String>,
}

/// Exact implementation selected by native capability preparation, retained on
/// invocation and recovery alongside the complete Task payload.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnerInvocationSelectionV1 {
    pub contract_ref: CapabilityContractRevisionRef,
    pub implementation_ref: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnerDescriptionV1 {
    pub protocol_version: u32,
    pub owner_id: String,
    pub routes: Vec<TheoryRouteContract>,
    pub capabilities: Vec<CapabilityContractRevision>,
    pub implementations: Vec<OwnerImplementationOfferV1>,
    pub observation_participant: Option<crate::theory::ActivationParticipantSpec>,
    /// Retired in-core stores whose nonempty history this package cannot import.
    #[serde(default)]
    pub incompatible_legacy_trees: BTreeSet<String>,
}

/// Package-authored callback requirements for one exact capability realization.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnerCapabilityGrantV1 {
    pub selection: OwnerInvocationSelectionV1,
    pub publications: BTreeSet<(String, String)>,
    pub provider_frame_types: BTreeSet<String>,
    /// Exact retained-publication routes needed to finish prior owner work.
    #[serde(default)]
    pub continuation_publications: BTreeSet<(String, String, String)>,
}

/// Exact owner resources issued from installed revisions and granted physical
/// bindings. Core fingerprints the values without interpreting policy bodies.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnerPreparedBindingsV1 {
    pub bindings: BTreeMap<String, String>,
    pub observation_publications: BTreeSet<(String, String)>,
    pub observation_provider_frame_types: BTreeSet<String>,
    #[serde(default)]
    pub retained_observation_publications: BTreeSet<(String, String, String)>,
    pub capability_grants: Vec<OwnerCapabilityGrantV1>,
}

/// Existing native products supplied to an observation owner at preparation.
/// Their meaning remains owned by Assignment, Curation, Authority and Graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnerObservationPreparationV1 {
    pub subject: meld_events::DomainObjectRef,
    pub scope: meld_world_model::world_state::graph::contracts::OwnerPublicationScope,
    pub session_id: String,
    pub authority: meld_lang::AuthorityPolicyBinding,
    pub event_routes: Vec<meld_world_model::world_state::graph::admission::GraphOwnerEventRoute>,
}

/// Only selected owner bindings cross this boundary. Core store paths and provider
/// credentials are not implicit members of the binding map.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnerRuntimePreparationV1 {
    pub assignment_id: String,
    pub activation_id: String,
    pub participant_id: String,
    pub state_root: PathBuf,
    pub bindings: BTreeMap<String, String>,
    pub installed_revisions: Vec<InstalledTheoryComponentRef>,
    pub ledger_id: meld_events::LedgerIdentity,
    #[serde(default)]
    pub observation: Option<OwnerObservationPreparationV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum OwnerCommandV1 {
    Describe,
    CloseRevisionStore,
    OpenRevisionStore {
        state_root: PathBuf,
    },
    ValidateTheory {
        route: TheoryRouteId,
        owner_component_id: String,
        canonical_bytes: Vec<u8>,
    },
    InstallTheory {
        route: TheoryRouteId,
        owner_component_id: String,
        canonical_bytes: Vec<u8>,
        installed_at_seq: u64,
    },
    VerifyTheory {
        route: TheoryRouteId,
        reference: TheoryRevisionRef,
    },
    ValidateLinks {
        component: InstalledTheoryComponentRef,
        package: InstalledPackageLinkView,
    },
    PrepareBindings {
        assignment_scope_id: String,
        subject: meld_events::DomainObjectRef,
        bindings: BTreeMap<String, String>,
        installed_revisions: Vec<InstalledTheoryComponentRef>,
    },
    PrepareRuntime {
        preparation: OwnerRuntimePreparationV1,
    },
    ResolveCurationSource {
        template: Box<CurationRuleTemplate>,
        binding: CurationRuleBinding,
    },
    RecoverInvocation {
        selection: OwnerInvocationSelectionV1,
        runtime_init: CapabilityRuntimeInit,
        payload: CapabilityInvocationPayload,
        event_context: Option<ExecutionEventContext>,
    },
    Invoke {
        selection: OwnerInvocationSelectionV1,
        runtime_init: CapabilityRuntimeInit,
        payload: CapabilityInvocationPayload,
        event_context: Option<ExecutionEventContext>,
    },
    Observe {
        context: ParticipantLifecycleContextV1,
        budget: WorkBudget,
    },
    Readiness {
        context: ParticipantLifecycleContextV1,
    },
    Wait {
        context: ParticipantLifecycleContextV1,
        report: WorkerTickReport,
    },
    SafePoint {
        context: ParticipantLifecycleContextV1,
    },
    Stop {
        context: ParticipantLifecycleContextV1,
    },
    Release {
        context: ParticipantLifecycleContextV1,
    },
    Snapshot,
    ResolvesWake {
        wake_ref: StructuralWakeRef,
    },
    Flush,
}

/// Callback permission is bound by the parent to the outstanding native command.
/// The child cannot supply or replace that permission with this wire value.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum OwnerCallbackV1 {
    Append {
        request: meld_events::events::remote::DurableAppendRequest,
    },
    AppendBatch {
        request: meld_events::events::remote::DurableAppendBatchRequest,
    },
    BestEffortAppend {
        request: meld_events::events::remote::BestEffortAppendRequest,
    },
    CommittedRecord {
        request: meld_events::events::remote::CommittedRecordRequest,
    },
    NewestPage {
        request: meld_events::events::remote::NewestPageRequest,
    },
    Barrier {
        request: meld_events::events::remote::BarrierRequest,
    },
    Replay {
        request: meld_events::ReplayRequest,
    },
    Provider {
        request: crate::context::generation::contracts::GenerationOrchestrationRequest,
        messages: Vec<crate::provider::ChatMessage>,
        event_context: Option<ExecutionEventContext>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(deny_unknown_fields)]
#[error("{code}: {message}")]
pub struct OwnerDiagnosticV1 {
    pub code: String,
    pub message: String,
}

impl OwnerDiagnosticV1 {
    pub fn new(code: &str, message: impl ToString) -> Self {
        Self {
            code: code.into(),
            message: message.to_string(),
        }
    }
}

pub type OwnerResult = Result<serde_json::Value, OwnerDiagnosticV1>;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "message", rename_all = "snake_case", deny_unknown_fields)]
pub enum HostMessageV1 {
    Command {
        protocol_version: u32,
        request_id: u64,
        command: Box<OwnerCommandV1>,
    },
    CallbackReturn {
        request_id: u64,
        callback_id: u64,
        result: OwnerResult,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "message", rename_all = "snake_case", deny_unknown_fields)]
pub enum OwnerMessageV1 {
    Return {
        request_id: u64,
        result: OwnerResult,
    },
    Callback {
        request_id: u64,
        callback_id: u64,
        callback: Box<OwnerCallbackV1>,
    },
}

pub trait OwnerCallbackPort: Send + Sync {
    fn call(&self, callback: OwnerCallbackV1) -> OwnerResult;
}

/// Installation and description have no operational grants.
pub struct NoOwnerCallbacks;
impl OwnerCallbackPort for NoOwnerCallbacks {
    fn call(&self, _: OwnerCallbackV1) -> OwnerResult {
        Err(OwnerDiagnosticV1::new(
            "owner_callback_not_granted",
            "this operation has no callback authority",
        ))
    }
}

pub fn encode_owner_result<T: Serialize>(value: T) -> OwnerResult {
    serde_json::to_value(value)
        .map_err(|error| OwnerDiagnosticV1::new("owner_product_encoding", error))
}

/// Retain the owner's terminal-versus-retryable disposition across transport.
pub fn encode_capability_result<T: Serialize>(
    value: Result<T, crate::error::ApiError>,
) -> OwnerResult {
    match value {
        Ok(value) => encode_owner_result(value),
        Err(error) => {
            let code = if matches!(error, crate::error::ApiError::TerminalCapabilityFailure(_)) {
                "terminal_capability_failure"
            } else {
                "owner_capability_failed"
            };
            Err(OwnerDiagnosticV1::new(code, error))
        }
    }
}
