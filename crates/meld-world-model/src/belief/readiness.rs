//! Durable belief-view attestations for agent process readiness.

use serde::{Deserialize, Serialize};

use crate::belief::{BeliefKey, BeliefView};
use crate::error::StorageError;

const READINESS_ATTESTATION_HASH_DOMAIN: &[u8] = b"meld.belief-readiness-attestation.v1";
const READINESS_VIEW_HASH_DOMAIN: &[u8] = b"meld.belief-readiness-view.v1";

/// Request to attest the exact current belief view used by agent hydration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BeliefReadinessAttestationRequest {
    /// Agent whose hydration consumes the attested view.
    pub agent_id: String,
    /// Agent subscription that selected the belief stream.
    pub subscription_id: String,
    /// Exact current belief stream to attest.
    pub belief_key: BeliefKey,
    /// Revision identity expected at the durable belief head.
    pub expected_revision_id: String,
    /// Sequence assigned to the completed readiness read.
    pub attested_at_seq: u64,
}

impl BeliefReadinessAttestationRequest {
    /// Validate stable consumer identity and the expected belief revision.
    pub fn validate(&self) -> Result<(), StorageError> {
        readiness_required("attestation agent id", &self.agent_id)?;
        readiness_required("attestation subscription id", &self.subscription_id)?;
        readiness_required(
            "attestation expected revision id",
            &self.expected_revision_id,
        )?;
        self.belief_key.validate()?;
        if self.attested_at_seq == 0 {
            return Err(StorageError::InvalidPath(
                "belief readiness attestation sequence must be greater than zero".to_string(),
            ));
        }
        Ok(())
    }
}

/// Belief-owned proof that one exact persisted current view was readable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BeliefReadinessAttestation {
    /// Deterministic identity derived from every following field.
    pub attestation_id: String,
    /// Agent whose hydration consumed the view.
    pub agent_id: String,
    /// Agent subscription that selected the view.
    pub subscription_id: String,
    /// Exact belief stream read by hydration.
    pub belief_key: BeliefKey,
    /// Revision at the durable belief head when attested.
    pub belief_revision_id: String,
    /// Identity of the persisted current view.
    pub belief_view_id: String,
    /// Canonical digest of the complete persisted current view.
    pub belief_view_hash: String,
    /// Sequence assigned to the completed readiness read.
    pub attested_at_seq: u64,
}

impl BeliefReadinessAttestation {
    /// Identify an attestation from a validated request and persisted view.
    pub(crate) fn identified(
        request: &BeliefReadinessAttestationRequest,
        view: &BeliefView,
    ) -> Result<Self, StorageError> {
        request.validate()?;
        let mut attestation = Self {
            attestation_id: String::new(),
            agent_id: request.agent_id.clone(),
            subscription_id: request.subscription_id.clone(),
            belief_key: request.belief_key.clone(),
            belief_revision_id: request.expected_revision_id.clone(),
            belief_view_id: view.view_id.clone(),
            belief_view_hash: hash_readiness_view(view)?,
            attested_at_seq: request.attested_at_seq,
        };
        attestation.attestation_id = attestation.derive_id()?;
        attestation.validate()?;
        Ok(attestation)
    }

    /// Validate required products and deterministic attestation identity.
    pub fn validate(&self) -> Result<(), StorageError> {
        readiness_required("attestation id", &self.attestation_id)?;
        readiness_required("attestation agent id", &self.agent_id)?;
        readiness_required("attestation subscription id", &self.subscription_id)?;
        readiness_required("attestation revision id", &self.belief_revision_id)?;
        readiness_required("attestation view id", &self.belief_view_id)?;
        readiness_required("attestation view hash", &self.belief_view_hash)?;
        self.belief_key.validate()?;
        if self.attested_at_seq == 0 {
            return Err(StorageError::InvalidPath(
                "belief readiness attestation sequence must be greater than zero".to_string(),
            ));
        }
        if self.attestation_id != self.derive_id()? {
            return Err(StorageError::InvalidPath(
                "belief readiness attestation identity mismatch".to_string(),
            ));
        }
        Ok(())
    }

    fn derive_id(&self) -> Result<String, StorageError> {
        #[derive(Serialize)]
        struct Identity<'a> {
            agent_id: &'a str,
            subscription_id: &'a str,
            belief_key: &'a BeliefKey,
            belief_revision_id: &'a str,
            belief_view_id: &'a str,
            belief_view_hash: &'a str,
            attested_at_seq: u64,
        }

        readiness_hash(
            READINESS_ATTESTATION_HASH_DOMAIN,
            &Identity {
                agent_id: &self.agent_id,
                subscription_id: &self.subscription_id,
                belief_key: &self.belief_key,
                belief_revision_id: &self.belief_revision_id,
                belief_view_id: &self.belief_view_id,
                belief_view_hash: &self.belief_view_hash,
                attested_at_seq: self.attested_at_seq,
            },
        )
    }
}

/// Hash one complete persisted belief view for readiness verification.
pub fn hash_readiness_view(view: &BeliefView) -> Result<String, StorageError> {
    readiness_hash(READINESS_VIEW_HASH_DOMAIN, view)
}

fn readiness_hash(domain: &[u8], value: &impl Serialize) -> Result<String, StorageError> {
    let encoded = serde_json::to_vec(value).map_err(|error| {
        StorageError::IoError(std::io::Error::new(std::io::ErrorKind::InvalidData, error))
    })?;
    let mut hasher = blake3::Hasher::new();
    hasher.update(domain);
    hasher.update(&encoded);
    Ok(hasher.finalize().to_hex().to_string())
}

fn readiness_required(field: &str, value: &str) -> Result<(), StorageError> {
    if value.trim().is_empty() {
        return Err(StorageError::InvalidPath(format!(
            "{field} must be non-empty"
        )));
    }
    Ok(())
}
