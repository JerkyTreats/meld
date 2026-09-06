use meld_events::{DomainObjectRef, LedgerCursor};
use serde::{Deserialize, Serialize};

pub const OWNER_ID: &str = "nonce";
pub const EVENT_TYPE: &str = "nonce";
pub const CONTRACT_REVISION: &str = "nonce.publication.v1";
pub const MAX_CORRELATIONS: usize = 32;
const MAX_REFERENCE_BYTES: usize = 1024;

#[derive(Debug, thiserror::Error)]
pub enum NonceError {
    #[error("invalid nonce request: {0}")]
    Invalid(String),
    #[error(transparent)]
    Event(#[from] meld_events::error::EventAuthorityError),
    #[error(transparent)]
    Publication(#[from] meld_world_model::error::StorageError),
    #[error(transparent)]
    Encoding(#[from] serde_json::Error),
}

/// Closed owner request. References remain opaque to the nonce domain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NonceRequest {
    pub nonce_id: String,
    pub issuer_ref: String,
    pub subject_ref: DomainObjectRef,
    pub correlation_refs: Vec<String>,
    pub fence_ref: String,
    pub contract_revision: String,
}

impl NonceRequest {
    pub fn new(
        issuer_ref: String,
        subject_ref: DomainObjectRef,
        correlation_refs: Vec<String>,
        fence_ref: String,
    ) -> Result<Self, NonceError> {
        let mut request = Self {
            nonce_id: String::new(),
            issuer_ref,
            subject_ref,
            correlation_refs,
            fence_ref,
            contract_revision: CONTRACT_REVISION.into(),
        };
        request.nonce_id = request.derived_identity()?;
        request.validate()?;
        Ok(request)
    }

    fn derived_identity(&self) -> Result<String, NonceError> {
        let bytes = serde_json::to_vec(&(
            &self.issuer_ref,
            &self.subject_ref,
            &self.correlation_refs,
            &self.fence_ref,
            &self.contract_revision,
        ))?;
        Ok(format!("nonce-v1::{}", blake3::hash(&bytes).to_hex()))
    }

    pub fn validate(&self) -> Result<(), NonceError> {
        self.subject_ref
            .validate()
            .map_err(|error| NonceError::Invalid(error.to_string()))?;
        if self.contract_revision != CONTRACT_REVISION {
            return Err(NonceError::Invalid(
                "unsupported publication contract revision".into(),
            ));
        }
        if self.correlation_refs.len() > MAX_CORRELATIONS {
            return Err(NonceError::Invalid("correlation bound exceeded".into()));
        }
        for reference in std::iter::once(&self.issuer_ref)
            .chain(std::iter::once(&self.fence_ref))
            .chain(self.correlation_refs.iter())
        {
            if reference.trim().is_empty() || reference.len() > MAX_REFERENCE_BYTES {
                return Err(NonceError::Invalid(
                    "reference is empty or exceeds its bound".into(),
                ));
            }
        }
        if self
            .correlation_refs
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != self.correlation_refs.len()
        {
            return Err(NonceError::Invalid(
                "duplicate correlation reference".into(),
            ));
        }
        if self.nonce_id != self.derived_identity()? {
            return Err(NonceError::Invalid(
                "nonce identity disagrees with its exact request".into(),
            ));
        }
        Ok(())
    }
}

/// Exact ledger position proven by Event authority, independent of attempt outcome.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NonceEmissionReceipt {
    pub nonce_id: String,
    pub event_record_id: String,
    pub position: LedgerCursor,
}
