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

/// Structural validation category for canonical append ingress.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventAppendValidationCode {
    /// Idempotent append omitted a stable record id.
    MissingIdempotencyRecordId,
    /// An envelope identity component violated the frozen grammar.
    MalformedEnvelopeIdentifier,
    /// The envelope repeated one object coordinate.
    DuplicateObjectReference,
    /// A relation endpoint was not carried by the envelope object set.
    RelationEndpointNotDeclared,
    /// A relation name violated the frozen grammar.
    MalformedRelationType,
}

/// Typed structural validation failure returned before append admission.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventAppendValidationIssue {
    /// Stable machine-readable validation category.
    pub code: EventAppendValidationCode,
    /// Field path rejected by ingress validation.
    pub field: String,
    /// Bounded human-readable explanation.
    pub message: String,
}

/// Maximum bytes for namespace-like event identifiers.
pub const MAX_EVENT_NAMESPACE_IDENTIFIER_BYTES: usize = 128;
/// Maximum bytes for opaque event identifiers.
pub const MAX_EVENT_OPAQUE_IDENTIFIER_BYTES: usize = 1024;

/// Structural identifier field validated at append ingress.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventStructuralIdentifierKind {
    /// Session identifier with opaque printable content.
    SessionId,
    /// Domain namespace identifier.
    DomainId,
    /// Stream identifier with opaque printable content.
    StreamId,
    /// Event type namespace identifier.
    EventType,
    /// Idempotency record identifier with opaque printable content.
    RecordId,
    /// Object domain namespace identifier.
    ObjectDomainId,
    /// Object kind namespace identifier.
    ObjectKind,
    /// Object identifier with opaque printable content.
    ObjectId,
    /// Relation type namespace identifier.
    RelationType,
}

impl EventStructuralIdentifierKind {
    /// Return the frozen byte bound for this identifier class.
    pub fn max_bytes(self) -> usize {
        match self {
            Self::DomainId
            | Self::EventType
            | Self::ObjectDomainId
            | Self::ObjectKind
            | Self::RelationType => MAX_EVENT_NAMESPACE_IDENTIFIER_BYTES,
            Self::SessionId | Self::StreamId | Self::RecordId | Self::ObjectId => {
                MAX_EVENT_OPAQUE_IDENTIFIER_BYTES
            }
        }
    }

    fn namespace(self) -> bool {
        matches!(
            self,
            Self::DomainId
                | Self::EventType
                | Self::ObjectDomainId
                | Self::ObjectKind
                | Self::RelationType
        )
    }
}

/// Validate one structural identifier against the frozen Wave 1 grammar.
pub fn validate_event_structural_identifier(
    kind: EventStructuralIdentifierKind,
    value: &str,
) -> Result<(), EventAppendValidationIssue> {
    let valid_length = !value.is_empty() && value.len() <= kind.max_bytes();
    let valid_content = if kind.namespace() {
        value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"._-".contains(&byte)
        }) && value
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
            && value
                .as_bytes()
                .last()
                .is_some_and(u8::is_ascii_alphanumeric)
    } else {
        value.trim() == value && value.chars().all(|character| !character.is_control())
    };
    if valid_length && valid_content {
        Ok(())
    } else {
        Err(EventAppendValidationIssue {
            code: if kind == EventStructuralIdentifierKind::RelationType {
                EventAppendValidationCode::MalformedRelationType
            } else {
                EventAppendValidationCode::MalformedEnvelopeIdentifier
            },
            field: format!("{kind:?}").to_ascii_lowercase(),
            message: format!(
                "identifier must satisfy the {:?} grammar and fit within {} bytes",
                kind,
                kind.max_bytes()
            ),
        })
    }
}

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

    #[test]
    fn append_validation_issue_has_stable_wire_shape() {
        let issue = EventAppendValidationIssue {
            code: EventAppendValidationCode::MissingIdempotencyRecordId,
            field: "record_id".to_string(),
            message: "idempotent append requires a record id".to_string(),
        };

        let value = serde_json::to_value(&issue).unwrap();

        assert_eq!(value["code"], "missing_idempotency_record_id");
        assert_eq!(value["field"], "record_id");
    }

    #[test]
    fn structural_identifier_grammar_has_frozen_bounds() {
        assert!(validate_event_structural_identifier(
            EventStructuralIdentifierKind::EventType,
            "execution.task.completed"
        )
        .is_ok());
        assert!(validate_event_structural_identifier(
            EventStructuralIdentifierKind::EventType,
            "Execution Task"
        )
        .is_err());
        assert!(validate_event_structural_identifier(
            EventStructuralIdentifierKind::RecordId,
            "publication:network-a:outcome-a"
        )
        .is_ok());
        assert!(validate_event_structural_identifier(
            EventStructuralIdentifierKind::RecordId,
            " record-a"
        )
        .is_err());
    }
}
