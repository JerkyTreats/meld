//! Exact nonce emitter over the ordinary Capability invocation boundary.

use std::collections::BTreeSet;
use std::sync::Arc;

use async_trait::async_trait;
use meld_execution::capability::{CapabilityContractRevision, CapabilityInvoker};

use crate::capability::*;
use crate::error::{ApiError, StorageError};
use crate::execution::{ExecutionEventContext, ExecutionRuntimeContext};
use crate::task::{ArtifactProducerRef, ArtifactRecord};

use super::{emit, NonceEmissionReceipt, NonceRequest};

pub const EMIT: &str = "nonce.emit";
pub const REQUEST: &str = "nonce_request";
pub const RECEIPT: &str = "nonce_emission_receipt";
const VERSION: u32 = 1;

pub fn contract() -> CapabilityTypeContract {
    CapabilityTypeContract {
        capability_type_id: EMIT.into(),
        capability_version: VERSION,
        owning_domain: "nonce".into(),
        scope_contract: ScopeContract {
            scope_kind: "domain_object".into(),
            scope_ref_kind: "subject_id".into(),
            allow_fan_out: false,
        },
        binding_contract: Vec::new(),
        input_contract: vec![InputSlotSpec {
            slot_id: REQUEST.into(),
            accepted_artifact_type_ids: vec![REQUEST.into()],
            schema_versions: ArtifactSchemaVersionRange {
                min: VERSION,
                max: VERSION,
            },
            required: true,
            cardinality: InputCardinality::One,
        }],
        output_contract: vec![OutputSlotSpec {
            slot_id: RECEIPT.into(),
            artifact_type_id: RECEIPT.into(),
            schema_version: VERSION,
            guaranteed: true,
        }],
        effect_contract: vec![EffectSpec {
            effect_id: "nonce.emit".into(),
            kind: EffectKind::Emit,
            target: "nonce".into(),
            exclusive: false,
        }],
        execution_contract: ExecutionContract {
            execution_class: ExecutionClass::Inline,
            completion_semantics: "exact_durable_event_append_proof".into(),
            retry_class: "same_nonce_request".into(),
            cancellation_supported: false,
        },
    }
}

pub struct NonceCapabilityContributor;
impl ProductCapabilityContributor for NonceCapabilityContributor {
    fn owner_domain(&self) -> &str {
        "nonce"
    }
    fn published_contracts(&self) -> Vec<CapabilityContractRevision> {
        let contract = contract();
        vec![CapabilityContractRevision {
            content_identity: contract.content_identity(),
            contract,
            installed_at_seq: 0,
        }]
    }
    fn implementation_offers(&self) -> Vec<CapabilityImplementationOffer> {
        self.published_contracts()
            .into_iter()
            .map(|revision| CapabilityImplementationOffer {
                contract_ref: revision.revision_ref(),
                implementation_ref: "nonce.event-ledger.v1".into(),
                required_binding_ids: BTreeSet::new(),
                execution_class: ExecutionClass::Inline,
                factory: Arc::new(NonceFactory),
            })
            .collect()
    }
}

struct NonceFactory;
impl CapabilityInvokerFactory for NonceFactory {
    fn prepare(
        &self,
        _request: &CapabilityFactoryRequest<'_>,
        _bindings: &OwnerBindingView,
    ) -> Result<PreparedCapabilityInvoker, CapabilityContributionDiagnostic> {
        Ok(Arc::new(NonceEmitter))
    }
}

pub struct NonceEmitter;
#[async_trait]
impl CapabilityInvoker for NonceEmitter {
    type Error = ApiError;
    type ExecutionApi = dyn ExecutionRuntimeContext;
    fn contract(&self) -> CapabilityTypeContract {
        contract()
    }

    async fn invoke(
        &self,
        api: &dyn ExecutionRuntimeContext,
        runtime_init: &CapabilityRuntimeInit,
        payload: &CapabilityInvocationPayload,
        event_context: Option<&ExecutionEventContext>,
    ) -> Result<CapabilityInvocationResult, ApiError> {
        let (request, context) = validated_request(runtime_init, payload, event_context)?;
        let append = api.durable_event_append().ok_or_else(|| {
            ApiError::StorageError(StorageError::EventAuthorityUnavailable(
                "nonce emitter has no bound product Event authority".into(),
            ))
        })?;
        let receipt = emit(&append, &context.session_id, &request).map_err(|error| {
            ApiError::StorageError(StorageError::EventAuthorityUnavailable(error.to_string()))
        })?;
        receipt_result(runtime_init, payload, receipt)
    }

    async fn recover(
        &self,
        events: Option<&meld_events::EventReplayCapability>,
        runtime_init: &CapabilityRuntimeInit,
        payload: &CapabilityInvocationPayload,
        event_context: Option<&ExecutionEventContext>,
    ) -> Result<Option<CapabilityInvocationResult>, ApiError> {
        let (request, context) = validated_request(runtime_init, payload, event_context)?;
        let Some(events) = events else {
            return Ok(None);
        };
        let expected = request
            .envelope(&context.session_id)
            .map_err(|error| invalid(error.to_string()))?;
        let Some(proof) = events.prove_existing(&expected).map_err(|error| {
            ApiError::StorageError(StorageError::EventAuthorityUnavailable(error.to_string()))
        })?
        else {
            return Ok(None);
        };
        receipt_result(
            runtime_init,
            payload,
            NonceEmissionReceipt {
                nonce_id: request.nonce_id,
                event_record_id: proof.record_id().into(),
                position: meld_events::LedgerCursor {
                    ledger_id: proof.ledger_id(),
                    after_seq: proof.seq(),
                },
            },
        )
        .map(Some)
    }
}

fn validated_request<'a>(
    runtime_init: &CapabilityRuntimeInit,
    payload: &CapabilityInvocationPayload,
    event_context: Option<&'a ExecutionEventContext>,
) -> Result<(NonceRequest, &'a ExecutionEventContext), ApiError> {
    payload.validate_against(runtime_init)?;
    let input = payload
        .supplied_inputs
        .iter()
        .find(|input| input.slot_id == REQUEST)
        .ok_or_else(|| invalid("nonce request input is absent"))?;
    let value = match &input.value {
        SuppliedValueRef::StructuredValue(value) => value,
        SuppliedValueRef::Artifact(artifact) => &artifact.content,
    };
    let request: NonceRequest =
        serde_json::from_value(value.clone()).map_err(|error| invalid(error.to_string()))?;
    request
        .validate()
        .map_err(|error| invalid(error.to_string()))?;
    if runtime_init.capability_type_id != EMIT
        || runtime_init.capability_version != VERSION
        || runtime_init.scope_ref != request.subject_ref.object_id
    {
        return Err(invalid(
            "nonce request does not match the bound Capability and subject",
        ));
    }
    let context = event_context
        .ok_or_else(|| invalid("nonce emission requires an admitted effect context"))?;
    let authority = context
        .effect_authority
        .as_ref()
        .ok_or_else(|| invalid("nonce emission has no admitted effect authority"))?;
    if authority.issuer_ref != request.issuer_ref
        || authority.subject != request.subject_ref
        || authority.fence_ref != request.fence_ref
    {
        return Err(invalid(
            "nonce request disagrees with the admitted issuer, subject, or fence",
        ));
    }
    Ok((request, context))
}

fn receipt_result(
    runtime_init: &CapabilityRuntimeInit,
    payload: &CapabilityInvocationPayload,
    receipt: NonceEmissionReceipt,
) -> Result<CapabilityInvocationResult, ApiError> {
    Ok(CapabilityInvocationResult {
        emitted_artifacts: vec![ArtifactRecord {
            artifact_id: format!("{}::{RECEIPT}", payload.invocation_id),
            artifact_type_id: RECEIPT.into(),
            schema_version: VERSION,
            content: serde_json::to_value(receipt).map_err(|error| invalid(error.to_string()))?,
            producer: ArtifactProducerRef {
                task_id: payload
                    .upstream_lineage
                    .as_ref()
                    .map(|lineage| lineage.task_id.clone())
                    .unwrap_or_default(),
                capability_instance_id: runtime_init.capability_instance_id.clone(),
                invocation_id: Some(payload.invocation_id.clone()),
                output_slot_id: Some(RECEIPT.into()),
            },
        }],
    })
}

fn invalid(message: impl Into<String>) -> ApiError {
    ApiError::ConfigError(format!(
        "{}: {}",
        meld_execution::error::TERMINAL_CAPABILITY_FAILURE_MARKER,
        message.into()
    ))
}
