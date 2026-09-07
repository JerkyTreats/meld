//! Durable native receipt for an exact completed publication invocation.

use super::capability::{DocsCapabilityConfig, DocsPublicationReceipt, PUBLISH_PATCH_SET};
use super::claim_validation::{verify_validated_patch_set, DocsClaimPolicy, ValidatedDocsPatchSet};
use crate::capability::{CapabilityInvocationPayload, CapabilityRuntimeInit};
use crate::error::ApiError;
use crate::execution::ExecutionEventContext;
use meld_events::{
    AppendMode, EventAppendCapability, EventEnvelope, EventReplayCapability, LedgerIdentity,
};

pub(crate) const EVENT_TYPE: &str = "docs.patch_set_published.v1";

pub(super) struct PublicationInvocation<'a> {
    config: &'a DocsCapabilityConfig,
    policy: &'a DocsClaimPolicy,
    patches: &'a ValidatedDocsPatchSet,
    context: &'a ExecutionEventContext,
    binding: serde_json::Value,
}

impl<'a> PublicationInvocation<'a> {
    pub(super) fn new(
        config: &'a DocsCapabilityConfig,
        policy: &'a DocsClaimPolicy,
        patches: &'a ValidatedDocsPatchSet,
        runtime: &CapabilityRuntimeInit,
        payload: &CapabilityInvocationPayload,
        context: Option<&'a ExecutionEventContext>,
    ) -> Result<Self, ApiError> {
        payload.validate_against(runtime)?;
        verify_validated_patch_set(policy, patches)?;
        let context = context
            .ok_or_else(|| invalid("Docs publication requires its admitted effect context"))?;
        let authority = context
            .effect_authority
            .as_ref()
            .ok_or_else(|| invalid("Docs publication requires admitted effect authority"))?;
        authority.subject.validate().map_err(invalid)?;
        if runtime.capability_type_id != PUBLISH_PATCH_SET
            || runtime.capability_version != 1
            || runtime.scope_ref != config.subject_id
            || authority.subject.object_id != config.subject_id
            || authority.issuer_ref != config.agent_id
            || authority.principal_id.trim().is_empty()
            || authority.fence_ref.trim().is_empty()
            || context.session_id.trim().is_empty()
        {
            return Err(invalid(
                "Docs publication disagrees with its bound scope, issuer, or authority",
            ));
        }
        let binding = serde_json::json!({
            "runtime":runtime,"invocation_id":payload.invocation_id,"lineage":payload.upstream_lineage,
            "target_root":config.target_root,"agent_id":config.agent_id,"subject_id":config.subject_id,
            "policy_identity":policy.content_identity(),"validation_fingerprint":patches.validation_fingerprint,
            "issuer":authority.issuer_ref,"principal":authority.principal_id,"subject":authority.subject,
            "fence":authority.fence_ref,"session":context.session_id
        });
        Ok(Self {
            config,
            policy,
            patches,
            context,
            binding,
        })
    }

    fn identity(&self, ledger: LedgerIdentity) -> Result<String, ApiError> {
        let bytes = serde_json::to_vec(&(ledger, &self.binding)).map_err(invalid)?;
        Ok(format!(
            "docs-publication-return-v1::{}",
            blake3::hash(&bytes).to_hex()
        ))
    }

    fn envelope(
        &self,
        ledger: LedgerIdentity,
        receipt: &DocsPublicationReceipt,
    ) -> Result<EventEnvelope, ApiError> {
        Ok(EventEnvelope::with_now_domain(
            &self.context.session_id,
            "docs",
            &self.config.subject_id,
            EVENT_TYPE,
            None,
            serde_json::json!({"binding":self.binding,"receipt":receipt}),
        )
        .with_record_id(self.identity(ledger)?))
    }

    pub(super) fn recover(
        &self,
        events: &EventReplayCapability,
    ) -> Result<Option<DocsPublicationReceipt>, ApiError> {
        let Some(record) = events
            .committed_record(&self.identity(events.ledger_identity())?)
            .map_err(event_error)?
        else {
            return Ok(None);
        };
        let receipt: DocsPublicationReceipt =
            serde_json::from_value(record.data["receipt"].clone()).map_err(invalid)?;
        if receipt.source_fingerprint != self.patches.source_fingerprint
            || receipt.policy_identity != self.patches.policy_identity
            || receipt.validation_fingerprint != self.patches.validation_fingerprint
            || receipt.weighted_groundedness != self.patches.weighted_groundedness
            || receipt.unsupported_claim_mass != self.patches.unsupported_claim_mass
            || receipt.contradiction_claim_mass != self.patches.contradiction_claim_mass
            || receipt.published.len() != self.patches.patches.len()
            || receipt
                .published
                .iter()
                .zip(&self.patches.patches)
                .any(|(actual, expected)| {
                    actual.path != expected.path || actual.content_hash != expected.content_hash
                })
        {
            return Err(invalid(
                "Docs publication receipt disagrees with its validated input",
            ));
        }
        events
            .prove_existing(&self.envelope(events.ledger_identity(), &receipt)?)
            .map_err(event_error)?
            .ok_or_else(|| invalid("Docs publication receipt is not durably proven"))?;
        Ok(Some(receipt))
    }

    pub(super) fn publish(
        &self,
        events: &EventAppendCapability,
    ) -> Result<DocsPublicationReceipt, ApiError> {
        if let Some(receipt) = self.recover(&events.replay_capability())? {
            return Ok(receipt);
        }
        let receipt = super::capability::publish_patch_set(
            &self.config.target_root,
            self.policy,
            self.patches,
        )?;
        events
            .append_durable_proven(
                self.envelope(events.ledger_identity(), &receipt)?,
                AppendMode::Idempotent,
            )
            .map_err(event_error)?;
        Ok(receipt)
    }
}

fn invalid(error: impl ToString) -> ApiError {
    ApiError::ConfigError(error.to_string())
}

fn event_error(error: impl ToString) -> ApiError {
    ApiError::StorageError(crate::error::StorageError::EventAuthorityUnavailable(
        error.to_string(),
    ))
}
