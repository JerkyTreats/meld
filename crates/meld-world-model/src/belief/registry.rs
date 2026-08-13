//! Durable belief-family registry contract with content-hash revisions.
//!
//! Owner: world model. This is the stage 2 theory registry for belief
//! families from the staged initialization pipeline: installation is an
//! idempotent domain command, identical content is a no-op, changed content
//! is a new revision rather than a mutation, and actors resolve the current
//! revision per tick so frame lineage can cite exactly which theory produced
//! which belief.
//!
//! This module carries no implementation. The belief-selection workstream
//! supplies the first store-backed implementor.

use serde::{Deserialize, Serialize};

use crate::belief::contracts::BeliefFamilyConfig;
use crate::error::StorageError;

/// Reference to one installed theory revision, cited in frame lineage.
///
/// Generic over theory kinds so method libraries and curation rules can
/// reuse the same lineage shape in their own registries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TheoryRevisionRef {
    /// Registry the revision lives in, such as `belief_family`.
    pub registry: String,
    /// Identity of the theory artifact inside its registry.
    pub id: String,
    /// Content hash that pins the exact installed revision.
    pub content_hash: String,
}

impl TheoryRevisionRef {
    /// Validate nonempty exact-reference coordinates and expected ownership.
    pub fn validate_for_registry(&self, expected_registry: &str) -> Result<(), StorageError> {
        for (label, value) in [
            ("theory registry", self.registry.as_str()),
            ("theory id", self.id.as_str()),
            ("theory content hash", self.content_hash.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(StorageError::InvalidPath(format!(
                    "{label} must not be empty"
                )));
            }
        }
        if self.registry != expected_registry {
            return Err(StorageError::InvalidPath(format!(
                "theory reference registry '{}' does not match expected registry '{expected_registry}'",
                self.registry
            )));
        }
        Ok(())
    }
}

/// One installed belief-family revision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BeliefFamilyRevision {
    /// Family identity carried inside the config.
    pub family_id: String,
    /// Content hash over the serialized config, the revision identity.
    pub content_hash: String,
    /// Installed family definition.
    pub config: BeliefFamilyConfig,
    /// Sequence observed when this revision was first installed.
    pub installed_at_seq: u64,
}

impl BeliefFamilyRevision {
    /// Lineage reference for this revision.
    pub fn revision_ref(&self) -> TheoryRevisionRef {
        TheoryRevisionRef {
            registry: "belief_family".to_string(),
            id: self.family_id.clone(),
            content_hash: self.content_hash.clone(),
        }
    }
}

/// Disposition of an idempotent theory installation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TheoryInstallDisposition {
    /// New content hash; a new revision was durably recorded.
    Installed,
    /// Same content hash; nothing changed.
    Unchanged,
}

/// Durable registry of belief families keyed by identity and content hash.
///
/// Reinstalling the same content hash must be a no-op. Prior revisions stay
/// resolvable so durable records that cite a revision remain answerable.
pub trait BeliefFamilyRegistry {
    /// Idempotently install a family config as the current revision.
    fn install(
        &mut self,
        config: BeliefFamilyConfig,
        installed_at_seq: u64,
    ) -> Result<(TheoryInstallDisposition, BeliefFamilyRevision), StorageError>;

    /// Current revision for a family, if any is installed.
    fn current(&self, family_id: &str) -> Result<Option<BeliefFamilyRevision>, StorageError>;

    /// Exact revision by family and content hash, if it was ever installed.
    fn resolve(
        &self,
        family_id: &str,
        content_hash: &str,
    ) -> Result<Option<BeliefFamilyRevision>, StorageError>;
}
