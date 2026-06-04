//! Domain object and relation records carried by events.
//!
//! Owner: event contracts.
//! Inputs: producer-owned object coordinates and relation names.
//! Outputs: validated object references and directed relation records for
//! downstream materializers.
//! Does not own: this module does not store graph facts or interpret payload
//! semantics.
//!
//! # Example
//!
//! ```rust
//! use meld_events::{DomainObjectRef, EventRelation};
//!
//! let task = DomainObjectRef::new("execution", "task_run", "run-a").unwrap();
//! let artifact =
//!     DomainObjectRef::new("execution", "artifact", "artifact-a").unwrap();
//! let relation = EventRelation::new("produced", task.clone(), artifact).unwrap();
//!
//! assert_eq!(task.index_key(), "execution::task_run::run-a");
//! assert_eq!(relation.relation_type, "produced");
//! ```

use serde::{Deserialize, Serialize};

use crate::error::StorageError;

/// Stable object coordinate carried on event envelopes for downstream graph materializers.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DomainObjectRef {
    /// Domain that owns the object namespace.
    pub domain_id: String,
    /// Type of object within the owning domain.
    pub object_kind: String,
    /// Domain-local object identifier.
    pub object_id: String,
}

impl DomainObjectRef {
    /// Builds an object reference and rejects empty coordinate components.
    pub fn new(
        domain_id: impl Into<String>,
        object_kind: impl Into<String>,
        object_id: impl Into<String>,
    ) -> Result<Self, StorageError> {
        let object_ref = Self {
            domain_id: domain_id.into(),
            object_kind: object_kind.into(),
            object_id: object_id.into(),
        };
        object_ref.validate()?;
        Ok(object_ref)
    }

    /// Validates that each coordinate component is present.
    pub fn validate(&self) -> Result<(), StorageError> {
        if self.domain_id.trim().is_empty() {
            return Err(StorageError::InvalidPath(
                "domain object ref domain_id must be non-empty".to_string(),
            ));
        }
        if self.object_kind.trim().is_empty() {
            return Err(StorageError::InvalidPath(
                "domain object ref object_kind must be non-empty".to_string(),
            ));
        }
        if self.object_id.trim().is_empty() {
            return Err(StorageError::InvalidPath(
                "domain object ref object_id must be non-empty".to_string(),
            ));
        }
        Ok(())
    }

    /// Returns the canonical index key used by downstream graph stores.
    pub fn index_key(&self) -> String {
        format!(
            "{}::{}::{}",
            self.domain_id, self.object_kind, self.object_id
        )
    }
}

/// Directed relationship between two event-carried domain objects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventRelation {
    /// Domain-specific relationship name.
    pub relation_type: String,
    /// Source object for the directed edge.
    pub src: DomainObjectRef,
    /// Destination object for the directed edge.
    pub dst: DomainObjectRef,
}

impl EventRelation {
    /// Builds a relation and validates the relationship type and endpoints.
    pub fn new(
        relation_type: impl Into<String>,
        src: DomainObjectRef,
        dst: DomainObjectRef,
    ) -> Result<Self, StorageError> {
        let relation = Self {
            relation_type: relation_type.into(),
            src,
            dst,
        };
        relation.validate()?;
        Ok(relation)
    }

    /// Validates that the relationship and both endpoints are well formed.
    pub fn validate(&self) -> Result<(), StorageError> {
        if self.relation_type.trim().is_empty() {
            return Err(StorageError::InvalidPath(
                "event relation relation_type must be non-empty".to_string(),
            ));
        }
        self.src.validate()?;
        self.dst.validate()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domain_object_ref_round_trips() {
        let object_ref = DomainObjectRef::new("execution", "task_run", "run_a").unwrap();
        let serialized = serde_json::to_string(&object_ref).unwrap();
        let parsed: DomainObjectRef = serde_json::from_str(&serialized).unwrap();
        assert_eq!(parsed.domain_id, "execution");
        assert_eq!(parsed.object_kind, "task_run");
        assert_eq!(parsed.object_id, "run_a");
        assert_eq!(parsed.index_key(), "execution::task_run::run_a");
    }
}
