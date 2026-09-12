//! Native observation and Task invocation over one prepared owner connection.
//!
//! The connection is shared by the owner's observation and invokers, preserving
//! native scheduling order. Only the parent supplies callback grants; transport
//! success never creates a lifecycle receipt or a semantic completion.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use meld_events::events::remote::EventAuthorityContract;
use meld_execution::capability::{
    CapabilityInvocationPayload, CapabilityInvocationResult, CapabilityInvoker,
    CapabilityRuntimeInit, CapabilityTypeContract,
};
use serde::de::DeserializeOwned;

use super::events::OwnerEventCallbacks;
use super::provider::OwnerProviderCallbacks;
use super::*;
use crate::error::ApiError;
use crate::execution::{ExecutionEventContext, ExecutionRuntimeContext};
use crate::provider::{ProviderCompletionPort, ProviderExecutionBinding};
use crate::runtime::contracts::{WorkBudget, WorkerTickReport};
use crate::runtime::error::RuntimeAssemblyError;
use crate::runtime::lifecycle::*;

/// Exact callback routes resolved from the selected owner and native assignment.
/// Observation and Task publication grants are deliberately distinct.
pub struct OwnerRuntimeGrants {
    pub events: Arc<dyn EventAuthorityContract>,
    pub observation_routes: BTreeSet<(String, String)>,
    pub invocation_routes: BTreeMap<OwnerInvocationSelectionV1, BTreeSet<(String, String)>>,
    pub observation_session: String,
    pub retained_observation_publications: BTreeSet<(String, String, String)>,
    pub invocation_continuations:
        BTreeMap<OwnerInvocationSelectionV1, BTreeSet<(String, String, String)>>,
    pub provider_binding: Option<ProviderExecutionBinding>,
    pub observation_provider_frame_types: BTreeSet<String>,
    pub invocation_provider_frame_types: BTreeMap<OwnerInvocationSelectionV1, BTreeSet<String>>,
    pub agent_id: String,
}

/// One inertly prepared owner instance. All native callers use this connection,
/// so observation cannot race an invocation behind the supervisor's ordering.
pub struct PreparedOwnerRuntime {
    description: OwnerDescriptionV1,
    preparation: OwnerRuntimePreparationV1,
    connection: Mutex<OwnerConnection>,
    grants: OwnerRuntimeGrants,
    provider: Mutex<Option<Arc<dyn ProviderCompletionPort>>>,
    graph: Mutex<Option<Arc<meld_world_model::world_state::graph::runtime::GraphRuntime>>>,
}

impl PreparedOwnerRuntime {
    pub(crate) fn bind_graph(
        &self,
        graph: Arc<meld_world_model::world_state::graph::runtime::GraphRuntime>,
    ) -> Result<(), OwnerDiagnosticV1> {
        *self
            .graph
            .lock()
            .map_err(|_| unavailable("Graph binding lock poisoned"))? = Some(graph);
        Ok(())
    }

    pub fn prepare(
        mut connection: OwnerConnection,
        description: OwnerDescriptionV1,
        preparation: OwnerRuntimePreparationV1,
        grants: OwnerRuntimeGrants,
    ) -> Result<Arc<Self>, OwnerDiagnosticV1> {
        super::registration::validate_description(&description.owner_id, &description)?;
        let actual: OwnerDescriptionV1 =
            connection.call(OwnerCommandV1::Describe, &NoOwnerCallbacks)?;
        if serde_json::to_value(&actual).map_err(|error| unavailable(&error.to_string()))?
            != serde_json::to_value(&description)
                .map_err(|error| unavailable(&error.to_string()))?
        {
            return Err(OwnerDiagnosticV1::new(
                "owner_description_changed",
                "runtime executable differs from the selected owner description",
            ));
        }
        if preparation.assignment_id.trim().is_empty()
            || preparation.activation_id.trim().is_empty()
            || preparation.participant_id.trim().is_empty()
            || !preparation.state_root.is_absolute()
        {
            return Err(OwnerDiagnosticV1::new(
                "owner_preparation_invalid",
                "native owner requires exact assignment, activation, participant and state identities",
            ));
        }
        connection.call::<()>(
            OwnerCommandV1::PrepareRuntime {
                preparation: preparation.clone(),
            },
            &NoOwnerCallbacks,
        )?;
        Ok(Arc::new(Self {
            description,
            preparation,
            connection: Mutex::new(connection),
            grants,
            provider: Mutex::new(None),
            graph: Mutex::new(None),
        }))
    }

    fn call<T: DeserializeOwned>(
        &self,
        command: OwnerCommandV1,
        scope: OperationScope<'_>,
    ) -> Result<T, OwnerDiagnosticV1> {
        let ledger = self.preparation.ledger_id;
        let events = match scope {
            OperationScope::ReadOnly => {
                OwnerEventCallbacks::read_only(ledger, self.grants.events.clone())
            }
            OperationScope::Observation => self.event_grant(
                &self.grants.observation_session,
                &self.grants.observation_routes,
                &self.grants.retained_observation_publications,
            )?,
            OperationScope::Invocation(context, selection) => {
                let routes = self
                    .grants
                    .invocation_routes
                    .get(selection)
                    .cloned()
                    .unwrap_or_default();
                self.event_grant(
                    &context.session_id,
                    &routes,
                    &self
                        .grants
                        .invocation_continuations
                        .get(selection)
                        .cloned()
                        .unwrap_or_default(),
                )?
            }
        };
        let mut events: Arc<dyn OwnerCallbackPort> = Arc::new(events);
        if let Some(graph) = self
            .graph
            .lock()
            .map_err(|_| unavailable("Graph binding lock poisoned"))?
            .clone()
        {
            events = Arc::new(super::graph::OwnerGraphCallbacks {
                next: events,
                graph,
                events: self.grants.events.clone(),
                ledger_id: ledger,
                inputs: self
                    .preparation
                    .observation
                    .as_ref()
                    .map(|o| o.inputs.clone())
                    .unwrap_or_default(),
            });
        }
        let provider = self
            .provider
            .lock()
            .map_err(|_| unavailable("provider binding lock poisoned"))?
            .clone();
        let context = match scope {
            OperationScope::Invocation(context, _) => Some(context.clone()),
            OperationScope::Observation => Some(ExecutionEventContext {
                session_id: self.grants.observation_session.clone(),
                effect_authority: None,
            }),
            OperationScope::ReadOnly => None,
        };
        let frames = match scope {
            OperationScope::Observation => self.grants.observation_provider_frame_types.clone(),
            OperationScope::Invocation(_, selection) => self
                .grants
                .invocation_provider_frame_types
                .get(selection)
                .cloned()
                .unwrap_or_default(),
            OperationScope::ReadOnly => BTreeSet::new(),
        };
        let callbacks: Arc<dyn OwnerCallbackPort> =
            match (&self.grants.provider_binding, provider, frames.is_empty()) {
                (Some(binding), Some(provider), false) => Arc::new(OwnerProviderCallbacks::new(
                    events,
                    provider,
                    binding.clone(),
                    self.grants.agent_id.clone(),
                    frames,
                    context,
                )?),
                _ => events,
            };
        self.connection
            .lock()
            .map_err(|_| unavailable("runtime connection lock poisoned"))?
            .call(command, callbacks.as_ref())
    }

    fn recover_connection_after_lease(&self) -> Result<(), OwnerDiagnosticV1> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| unavailable("runtime connection lock poisoned"))?;
        if connection.unavailable() {
            // Native readiness is the sole retry boundary. Invocations never
            // replace a failed process or acquire mutable observation tenure.
            connection.reconnect()?;
            connection.call::<()>(
                OwnerCommandV1::PrepareRuntime {
                    preparation: self.preparation.clone(),
                },
                &NoOwnerCallbacks,
            )?;
        }
        Ok(())
    }

    fn event_grant(
        &self,
        session: &str,
        routes: &BTreeSet<(String, String)>,
        retained: &BTreeSet<(String, String, String)>,
    ) -> Result<OwnerEventCallbacks, OwnerDiagnosticV1> {
        if routes.is_empty() && retained.is_empty() {
            return Ok(OwnerEventCallbacks::read_only(
                self.preparation.ledger_id,
                self.grants.events.clone(),
            ));
        }
        OwnerEventCallbacks::publishing_routes(
            self.preparation.ledger_id,
            self.grants.events.clone(),
            &self.description.owner_id,
            routes
                .iter()
                .map(|(kind, stream)| (session.to_string(), kind.clone(), stream.clone()))
                .chain(retained.iter().cloned())
                .collect(),
        )
        .map_err(|error| OwnerDiagnosticV1::new("owner_callback_not_granted", error))
    }

    pub fn invoker(
        self: &Arc<Self>,
        contract: &meld_execution::capability::CapabilityContractRevisionRef,
        implementation_ref: &str,
    ) -> Result<crate::capability::PreparedCapabilityInvoker, OwnerDiagnosticV1> {
        if !self.description.implementations.iter().any(|offer| {
            &offer.contract_ref == contract && offer.implementation_ref == implementation_ref
        }) {
            return Err(OwnerDiagnosticV1::new(
                "selected_implementation_missing",
                "prepared owner did not publish the selected exact implementation",
            ));
        }
        let contract = self
            .description
            .capabilities
            .iter()
            .find(|revision| &revision.revision_ref() == contract)
            .ok_or_else(|| unavailable("selected owner contract is absent"))?
            .contract
            .clone();
        Ok(Arc::new(OwnerInvoker {
            runtime: self.clone(),
            selection: OwnerInvocationSelectionV1 {
                contract_ref: meld_execution::capability::CapabilityContractRevisionRef {
                    selector: meld_lang::CapabilityRef {
                        capability_type_id: contract.capability_type_id.clone(),
                        capability_version: contract.capability_version,
                    },
                    content_identity: contract.content_identity(),
                },
                implementation_ref: implementation_ref.into(),
            },
            contract,
        }))
    }
}

#[derive(Clone, Copy)]
enum OperationScope<'a> {
    Observation,
    ReadOnly,
    Invocation(&'a ExecutionEventContext, &'a OwnerInvocationSelectionV1),
}

impl NativeObservationOwnerFactory for Arc<PreparedOwnerRuntime> {
    fn build(&self) -> Box<dyn NativeObservationOwner> {
        Box::new(OwnerObservation {
            runtime: self.clone(),
            context: None,
            owns_session: false,
        })
    }

    fn bind_provider(&self, provider: Arc<dyn ProviderCompletionPort>) -> bool {
        if self.grants.provider_binding.is_none() {
            return false;
        }
        let Ok(mut slot) = self.provider.lock() else {
            return false;
        };
        if slot.is_some() {
            return false;
        }
        *slot = Some(provider);
        true
    }
}

struct OwnerInvoker {
    runtime: Arc<PreparedOwnerRuntime>,
    contract: CapabilityTypeContract,
    selection: OwnerInvocationSelectionV1,
}

#[async_trait]
impl CapabilityInvoker for OwnerInvoker {
    type Error = ApiError;
    type ExecutionApi = dyn ExecutionRuntimeContext;

    fn contract(&self) -> CapabilityTypeContract {
        self.contract.clone()
    }

    async fn recover(
        &self,
        _: Option<&meld_events::EventReplayCapability>,
        runtime_init: &CapabilityRuntimeInit,
        payload: &CapabilityInvocationPayload,
        event_context: Option<&ExecutionEventContext>,
    ) -> Result<Option<CapabilityInvocationResult>, ApiError> {
        self.runtime
            .call(
                OwnerCommandV1::RecoverInvocation {
                    selection: self.selection.clone(),
                    runtime_init: runtime_init.clone(),
                    payload: payload.clone(),
                    event_context: event_context.cloned(),
                },
                OperationScope::ReadOnly,
            )
            .map_err(api_error)
    }

    async fn invoke(
        &self,
        _: &dyn ExecutionRuntimeContext,
        runtime_init: &CapabilityRuntimeInit,
        payload: &CapabilityInvocationPayload,
        event_context: Option<&ExecutionEventContext>,
    ) -> Result<CapabilityInvocationResult, ApiError> {
        let context = event_context.ok_or_else(|| {
            api_error(OwnerDiagnosticV1::new(
                "owner_invocation_not_authorized",
                "external invocation requires its native Execution context",
            ))
        })?;
        self.runtime
            .call(
                OwnerCommandV1::Invoke {
                    selection: self.selection.clone(),
                    runtime_init: runtime_init.clone(),
                    payload: payload.clone(),
                    event_context: Some(context.clone()),
                },
                OperationScope::Invocation(context, &self.selection),
            )
            .map_err(api_error)
    }
}

struct OwnerObservation {
    runtime: Arc<PreparedOwnerRuntime>,
    context: Option<ParticipantLifecycleContextV1>,
    owns_session: bool,
}

impl Drop for OwnerObservation {
    fn drop(&mut self) {
        // Losing the native lifecycle handle ends this process's mutable tenure.
        // Retained invokers cannot keep it alive, and termination supplies no
        // semantic release receipt. Recovery must still drain the predecessor.
        if self.owns_session {
            if let Ok(mut connection) = self.runtime.connection.lock() {
                connection.invalidate();
            }
        }
    }
}

impl NativeObservationOwner for OwnerObservation {
    fn tick(&mut self, budget: WorkBudget) -> WorkerTickReport {
        let result = self
            .context
            .clone()
            .ok_or_else(|| unavailable("observation has no native readiness context"))
            .and_then(|context| {
                self.runtime.call(
                    OwnerCommandV1::Observe { context, budget },
                    OperationScope::Observation,
                )
            });
        result.unwrap_or_else(|error| {
            WorkerTickReport::fatal(
                &self.runtime.preparation.participant_id,
                &self.runtime.description.owner_id,
                None,
                "owner_transport_unavailable",
                &error.code,
                error.message,
            )
        })
    }
}

impl NativeOwnerLifecycle for OwnerObservation {
    fn native_snapshot(&self) -> Result<NativeOwnerLifecycleSnapshot, RuntimeAssemblyError> {
        self.runtime
            .call(OwnerCommandV1::Snapshot, OperationScope::ReadOnly)
            .map_err(runtime_error)
    }

    fn native_readiness(
        &mut self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<OwnerReadinessReceiptV1, RuntimeAssemblyError> {
        self.owns_session = true;
        self.runtime
            .recover_connection_after_lease()
            .map_err(runtime_error)?;
        let receipt = self
            .runtime
            .call(
                OwnerCommandV1::Readiness {
                    context: context.clone(),
                },
                OperationScope::Observation,
            )
            .map_err(runtime_error)?;
        self.context = Some(context.clone());
        Ok(receipt)
    }

    fn native_wait(
        &self,
        context: &ParticipantLifecycleContextV1,
        report: &WorkerTickReport,
    ) -> Result<OwnerWaitReceiptV1, RuntimeAssemblyError> {
        self.runtime
            .call(
                OwnerCommandV1::Wait {
                    context: context.clone(),
                    report: report.clone(),
                },
                OperationScope::Observation,
            )
            .map_err(runtime_error)
    }

    fn native_safe_point(
        &mut self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<OwnerSafePointReceiptV1, RuntimeAssemblyError> {
        self.runtime
            .call(
                OwnerCommandV1::SafePoint {
                    context: context.clone(),
                },
                OperationScope::Observation,
            )
            .map_err(runtime_error)
    }
    fn native_stop(
        &mut self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<OwnerStopReceiptV1, RuntimeAssemblyError> {
        let receipt = self
            .runtime
            .call(
                OwnerCommandV1::Stop {
                    context: context.clone(),
                },
                OperationScope::Observation,
            )
            .map_err(runtime_error)?;
        self.context = None;
        Ok(receipt)
    }
    fn native_release(
        &mut self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<OwnerReleaseReceiptV1, RuntimeAssemblyError> {
        let receipt = self
            .runtime
            .call(
                OwnerCommandV1::Release {
                    context: context.clone(),
                },
                OperationScope::Observation,
            )
            .map_err(runtime_error)?;
        self.context = None;
        self.owns_session = false;
        Ok(receipt)
    }
    fn native_resolves_wake(
        &self,
        wake_ref: &StructuralWakeRef,
    ) -> Result<bool, RuntimeAssemblyError> {
        self.runtime
            .call(
                OwnerCommandV1::ResolvesWake {
                    wake_ref: wake_ref.clone(),
                },
                OperationScope::ReadOnly,
            )
            .map_err(runtime_error)
    }
}

fn unavailable(message: &str) -> OwnerDiagnosticV1 {
    OwnerDiagnosticV1::new("owner_runtime_unavailable", message)
}
fn api_error(error: OwnerDiagnosticV1) -> ApiError {
    let failure = ApiError::GenerationFailed(error.to_string());
    if error.code == "terminal_capability_failure" {
        ApiError::TerminalCapabilityFailure(Box::new(failure))
    } else {
        failure
    }
}
fn runtime_error(error: OwnerDiagnosticV1) -> RuntimeAssemblyError {
    RuntimeAssemblyError::RuntimeHandleConstruction(error.to_string())
}
