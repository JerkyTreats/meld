//! Durable belief-view attestations for agent process readiness.

use serde::{Deserialize, Serialize};

use crate::belief::{BeliefKey, BeliefRevision, BeliefView};
use crate::error::StorageError;

const READINESS_ATTESTATION_HASH_DOMAIN: &[u8] = b"meld.belief-readiness-attestation.v1";
const READINESS_ATTESTATION_V2_HASH_DOMAIN: &[u8] = b"meld.belief-readiness-attestation.v2";
const READINESS_VIEW_HASH_DOMAIN: &[u8] = b"meld.belief-readiness-view.v1";
const READINESS_REVISION_HASH_DOMAIN: &[u8] = b"meld.belief-readiness-revision.v1";
const READINESS_ATTESTATION_SCHEMA_V1: u16 = 1;
const READINESS_ATTESTATION_SCHEMA_V2: u16 = 2;
const READINESS_ATTESTATION_IDENTITY_V1: u16 = 1;
const READINESS_ATTESTATION_IDENTITY_V2: u16 = 2;

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
    /// Durable wire schema version.
    // TODO compat-shim: remove the v1 serde default after every supported store
    // is schema v2 and accepted W3A attestation fixture plus hash parity tests stay green.
    #[serde(default = "legacy_attestation_version")]
    pub schema_version: u16,
    /// Hash preimage version that owns the stable attestation id.
    #[serde(default = "legacy_attestation_version")]
    pub identity_version: u16,
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
    /// Canonical digest of the complete attested revision.
    #[serde(default)]
    pub belief_revision_hash: String,
    /// Identity of the persisted current view.
    pub belief_view_id: String,
    /// Canonical digest of the complete persisted current view.
    pub belief_view_hash: String,
    /// Inclusive lower source sequence consumed by the revision.
    #[serde(default)]
    pub source_cursor_start: u64,
    /// Inclusive upper source sequence consumed by the revision.
    #[serde(default)]
    pub source_cursor_end: u64,
    /// Sequence assigned to the completed readiness read.
    pub attested_at_seq: u64,
}

/// Exact immutable belief products frozen by one readiness attestation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BeliefReadinessSnapshot {
    /// Durable attestation identifying the snapshot.
    pub attestation: BeliefReadinessAttestation,
    /// Complete append-only belief revision.
    pub revision: BeliefRevision,
    /// Complete planner-safe view read by hydration.
    pub view: BeliefView,
}

impl BeliefReadinessSnapshot {
    /// Validate revision identity, content hashes, view identity, and boundary.
    pub fn validate(&self) -> Result<(), StorageError> {
        self.attestation.validate()?;
        if self.revision.revision_id != self.attestation.belief_revision_id
            || self.revision.belief_key != self.attestation.belief_key
            || hash_readiness_revision(&self.revision)? != self.attestation.belief_revision_hash
            || self.revision.source_cursor_start != self.attestation.source_cursor_start
            || self.revision.source_cursor_end != self.attestation.source_cursor_end
            || self.view.view_id != self.attestation.belief_view_id
            || self.view.key != self.attestation.belief_key
            || self.view.current_revision_id.as_deref()
                != Some(self.attestation.belief_revision_id.as_str())
            || hash_readiness_view(&self.view)? != self.attestation.belief_view_hash
        {
            return Err(StorageError::InvalidPath(
                "belief readiness snapshot conflicts with its attestation".to_string(),
            ));
        }
        Ok(())
    }
}

impl BeliefReadinessAttestation {
    /// Identify an attestation from a validated request and persisted view.
    pub(crate) fn identified(
        request: &BeliefReadinessAttestationRequest,
        revision: &BeliefRevision,
        view: &BeliefView,
    ) -> Result<Self, StorageError> {
        request.validate()?;
        let mut attestation = Self {
            schema_version: READINESS_ATTESTATION_SCHEMA_V2,
            identity_version: READINESS_ATTESTATION_IDENTITY_V2,
            attestation_id: String::new(),
            agent_id: request.agent_id.clone(),
            subscription_id: request.subscription_id.clone(),
            belief_key: request.belief_key.clone(),
            belief_revision_id: request.expected_revision_id.clone(),
            belief_revision_hash: hash_readiness_revision(revision)?,
            belief_view_id: view.view_id.clone(),
            belief_view_hash: hash_readiness_view(view)?,
            source_cursor_start: revision.source_cursor_start,
            source_cursor_end: revision.source_cursor_end,
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
        // TODO compat-shim: remove v1 schema and identity acceptance after the
        // accepted W3A fixture is retired and migration plus hash parity tests
        // remain green with only current identities.
        match (self.schema_version, self.identity_version) {
            (READINESS_ATTESTATION_SCHEMA_V1, READINESS_ATTESTATION_IDENTITY_V1) => {
                if !self.belief_revision_hash.is_empty()
                    || self.source_cursor_start != 0
                    || self.source_cursor_end != 0
                    || self.attested_at_seq == 0
                {
                    return Err(StorageError::InvalidPath(
                        "legacy belief readiness attestation has mixed-version fields".to_string(),
                    ));
                }
            }
            (
                READINESS_ATTESTATION_SCHEMA_V2,
                READINESS_ATTESTATION_IDENTITY_V1 | READINESS_ATTESTATION_IDENTITY_V2,
            ) => {
                readiness_required("attestation revision hash", &self.belief_revision_hash)?;
                if self.source_cursor_start == 0
                    || self.source_cursor_end < self.source_cursor_start
                    || self.attested_at_seq <= self.source_cursor_end
                {
                    return Err(StorageError::InvalidPath(
                        "belief readiness attestation source boundary and sequence are invalid"
                            .to_string(),
                    ));
                }
            }
            _ => {
                return Err(StorageError::InvalidPath(
                    "unsupported belief readiness attestation schema or identity version"
                        .to_string(),
                ));
            }
        }
        if self.attestation_id != self.derive_id()? {
            return Err(StorageError::InvalidPath(
                "belief readiness attestation identity mismatch".to_string(),
            ));
        }
        Ok(())
    }

    fn derive_id(&self) -> Result<String, StorageError> {
        // TODO compat-shim: remove the v1 preimage after accepted W3A identity
        // fixtures and stable-id migration tests no longer require it.
        if self.identity_version == READINESS_ATTESTATION_IDENTITY_V1 {
            return readiness_hash(
                READINESS_ATTESTATION_HASH_DOMAIN,
                &LegacyBeliefReadinessAttestationIdentityV1 {
                    agent_id: &self.agent_id,
                    subscription_id: &self.subscription_id,
                    belief_key: &self.belief_key,
                    belief_revision_id: &self.belief_revision_id,
                    belief_view_id: &self.belief_view_id,
                    belief_view_hash: &self.belief_view_hash,
                    attested_at_seq: self.attested_at_seq,
                },
            );
        }
        #[derive(Serialize)]
        struct Identity<'a> {
            schema_version: u16,
            identity_version: u16,
            agent_id: &'a str,
            subscription_id: &'a str,
            belief_key: &'a BeliefKey,
            belief_revision_id: &'a str,
            belief_revision_hash: &'a str,
            belief_view_id: &'a str,
            belief_view_hash: &'a str,
            source_cursor_start: u64,
            source_cursor_end: u64,
            attested_at_seq: u64,
        }

        readiness_hash(
            READINESS_ATTESTATION_V2_HASH_DOMAIN,
            &Identity {
                schema_version: self.schema_version,
                identity_version: self.identity_version,
                agent_id: &self.agent_id,
                subscription_id: &self.subscription_id,
                belief_key: &self.belief_key,
                belief_revision_id: &self.belief_revision_id,
                belief_revision_hash: &self.belief_revision_hash,
                belief_view_id: &self.belief_view_id,
                belief_view_hash: &self.belief_view_hash,
                source_cursor_start: self.source_cursor_start,
                source_cursor_end: self.source_cursor_end,
                attested_at_seq: self.attested_at_seq,
            },
        )
    }

    /// Return whether this record still needs the v1-to-v2 wire upgrade.
    pub(crate) fn requires_legacy_upgrade(&self) -> bool {
        self.schema_version == READINESS_ATTESTATION_SCHEMA_V1
    }

    /// Return whether this record preserves the accepted W3A identity.
    pub(crate) fn has_legacy_identity(&self) -> bool {
        self.identity_version == READINESS_ATTESTATION_IDENTITY_V1
    }

    /// Upgrade a validated v1 record while preserving its accepted identity.
    pub(crate) fn upgrade_legacy(
        mut self,
        revision: &BeliefRevision,
        view: &BeliefView,
    ) -> Result<Self, StorageError> {
        // TODO compat-shim: remove this rewrite after all accepted W3A records
        // are migrated and attestation identity plus reopen tests stay green.
        self.validate()?;
        if !self.requires_legacy_upgrade() {
            return Ok(self);
        }
        if revision.revision_id != self.belief_revision_id
            || revision.belief_key != self.belief_key
            || view.view_id != self.belief_view_id
            || view.key != self.belief_key
            || view.current_revision_id.as_deref() != Some(self.belief_revision_id.as_str())
            || hash_readiness_view(view)? != self.belief_view_hash
        {
            return Err(StorageError::MigrationConflict(
                "legacy belief readiness attestation cannot be proven from durable products"
                    .to_string(),
            ));
        }
        self.schema_version = READINESS_ATTESTATION_SCHEMA_V2;
        self.belief_revision_hash = hash_readiness_revision(revision)?;
        self.source_cursor_start = revision.source_cursor_start;
        self.source_cursor_end = revision.source_cursor_end;
        self.validate()?;
        Ok(self)
    }

    pub(crate) fn legacy_identity_wire(&self) -> LegacyBeliefReadinessAttestationV1<'_> {
        LegacyBeliefReadinessAttestationV1 {
            attestation_id: &self.attestation_id,
            agent_id: &self.agent_id,
            subscription_id: &self.subscription_id,
            belief_key: &self.belief_key,
            belief_revision_id: &self.belief_revision_id,
            belief_view_id: &self.belief_view_id,
            belief_view_hash: &self.belief_view_hash,
            attested_at_seq: self.attested_at_seq,
        }
    }
}

/// Accepted W3A readiness wire used only for exact identity parity.
// TODO compat-shim: remove this v1 wire only after every supported store has
// migrated all standalone attestations and embedded readiness proofs to schema
// v2, and fixture-based reopen tests no longer find accepted W3A records.
#[derive(Serialize)]
pub(crate) struct LegacyBeliefReadinessAttestationV1<'a> {
    attestation_id: &'a str,
    agent_id: &'a str,
    subscription_id: &'a str,
    belief_key: &'a BeliefKey,
    belief_revision_id: &'a str,
    belief_view_id: &'a str,
    belief_view_hash: &'a str,
    attested_at_seq: u64,
}

#[derive(Serialize)]
struct LegacyBeliefReadinessAttestationIdentityV1<'a> {
    agent_id: &'a str,
    subscription_id: &'a str,
    belief_key: &'a BeliefKey,
    belief_revision_id: &'a str,
    belief_view_id: &'a str,
    belief_view_hash: &'a str,
    attested_at_seq: u64,
}

/// Hash one complete persisted belief view for readiness verification.
pub fn hash_readiness_view(view: &BeliefView) -> Result<String, StorageError> {
    readiness_hash(READINESS_VIEW_HASH_DOMAIN, view)
}

/// Hash one complete immutable revision for readiness verification.
pub fn hash_readiness_revision(revision: &BeliefRevision) -> Result<String, StorageError> {
    readiness_hash(READINESS_REVISION_HASH_DOMAIN, revision)
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

fn legacy_attestation_version() -> u16 {
    READINESS_ATTESTATION_SCHEMA_V1
}
