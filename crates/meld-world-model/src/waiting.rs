//! Waiting-on declarations for world-model bounded reports (DBG-016).
//!
//! Owner: world model. A declaration records what a selector found absent
//! when it computed eligibility — the other half of provenance: not why a
//! record exists, but what would have to exist for a quiet actor to commit
//! work. Emission is observational and derives from state the tick already
//! computed; it never gates, reorders, or fails semantic work. The
//! `condition` vocabulary is owned by the emitting actor's domain.

/// Frozen condition vocabulary emitted by world-model actors.
///
/// These constants are the contract between the emitting selectors and
/// every downstream consumer (the eligibility walk, the projections, the
/// debugger surface): both sides compile against the same string, so a
/// rename cannot silently diverge the substrate's answer.
pub mod conditions {
    /// Assessment waits on a current graph anchor for its subject.
    pub const GRAPH_ANCHOR_ABSENT: &str = "graph_anchor_absent";
    /// Assessment waits on an installed belief family revision.
    pub const BELIEF_FAMILY_ABSENT: &str = "belief_family_absent";
    /// The assessment selector found no dirty key and no unassessed
    /// subject binding.
    pub const BELIEF_WORK_INELIGIBLE: &str = "belief_work_ineligible";
    /// No belief family id is configured for the actor at all.
    pub const BELIEF_FAMILIES_UNCONFIGURED: &str = "belief_families_unconfigured";
    /// An active lease owns the selected key.
    pub const ASSESSMENT_LEASE_HELD: &str = "assessment_lease_held";
    /// An assessment attempt failed past its declared preconditions.
    pub const ASSESSMENT_BLOCKED: &str = "assessment_blocked";
    /// Ingestion waits on committed events past its durable cursor.
    pub const LEDGER_QUIET_PAST_CURSOR: &str = "ledger_quiet_past_cursor";
    /// The replay window matched no installed source mapping.
    pub const NO_MAPPABLE_EVENTS: &str = "no_mappable_events";
    /// Goal curation waits on a revision newer than its delivery cursor.
    pub const NO_UNDELIVERED_REVISIONS: &str = "no_undelivered_revisions";
    /// Satisfaction waits on an unreviewed revision or receipted decision.
    pub const NO_PENDING_SATISFACTION_REVIEWS: &str = "no_pending_satisfaction_reviews";
}

/// Structural address that can make a world-model owner eligible again.
///
/// The emitting owner chooses the class and exact durable address. Runtime
/// lifecycle may bind and resolve this value but does not infer its meaning.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum StructuralWakeAddress {
    /// Durable Event position or watermark.
    EventPosition(String),
    /// Durable native-owner revision.
    OwnerRevision(String),
    /// Durable operation completion position.
    DurableOperation(String),
    /// Durable deadline.
    DurableDeadline(String),
    /// Recovery of a physical or authority binding.
    BindingRecovery(String),
    /// Explicit operator action channel.
    OperatorAction(String),
}

/// Durable identity of the world-model resource shared by its native stores.
pub(crate) fn resource_identity(db: &sled::Db) -> Result<String, crate::error::StorageError> {
    let key = b"world_model_resource_identity_v1";
    let candidate = uuid::Uuid::new_v4().to_string();
    let _ = db
        .compare_and_swap(key, None as Option<&[u8]>, Some(candidate.as_bytes()))
        .map_err(|error| crate::error::StorageError::Unavailable(error.to_string()))?;
    let value = db
        .get(key)
        .map_err(|error| crate::error::StorageError::Unavailable(error.to_string()))?
        .ok_or_else(|| {
            crate::error::StorageError::Unavailable(
                "world-model resource identity disappeared".into(),
            )
        })?;
    let identity = std::str::from_utf8(&value)
        .map_err(|error| crate::error::StorageError::InvalidPath(error.to_string()))?;
    uuid::Uuid::parse_str(identity)
        .map_err(|error| crate::error::StorageError::InvalidPath(error.to_string()))?;
    db.flush()
        .map_err(|error| crate::error::StorageError::Unavailable(error.to_string()))?;
    Ok(identity.to_string())
}

pub(crate) fn bound_address<'a>(value: &'a str, resource_id: &str) -> Option<&'a str> {
    value.strip_prefix(&format!("world-model::{resource_id}::"))
}

pub(crate) fn after_position(value: &str, exact_resource: &str) -> bool {
    value
        .strip_prefix(&format!("{exact_resource}::after::"))
        .is_some_and(|position| {
            !position.is_empty()
                && position.bytes().all(|byte| byte.is_ascii_digit())
                && position.parse::<u64>().is_ok()
        })
}

pub(crate) fn bind_waits(waits: &mut [WaitingOnDeclaration], resource_id: &str) {
    for wake in waits.iter_mut().flat_map(|wait| &mut wait.wake_addresses) {
        let value = match wake {
            StructuralWakeAddress::EventPosition(_) => continue,
            StructuralWakeAddress::OwnerRevision(value)
            | StructuralWakeAddress::DurableOperation(value)
            | StructuralWakeAddress::DurableDeadline(value)
            | StructuralWakeAddress::BindingRecovery(value)
            | StructuralWakeAddress::OperatorAction(value) => value,
        };
        // A producer's explicit resource address survives a consumer handoff.
        // Resolvers still validate the complete address against their bound store.
        if !value.starts_with("world-model::") {
            *value = format!("world-model::{resource_id}::{value}");
        }
    }
}

/// One domain-owned statement of what would make work eligible.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaitingOnDeclaration {
    /// Stable domain-vocabulary code naming the awaited condition.
    pub condition: String,
    /// Exact subject key the condition is about, when the domain knows it.
    pub subject_key: Option<String>,
    /// Human-readable detail in the emitting domain's vocabulary.
    pub detail: String,
    /// Complete owner-authored structural addresses that can change eligibility.
    pub wake_addresses: Vec<StructuralWakeAddress>,
}

impl WaitingOnDeclaration {
    /// Build a declaration with an exact subject key.
    pub fn about(
        condition: impl Into<String>,
        subject_key: impl Into<String>,
        detail: impl Into<String>,
        wake_addresses: Vec<StructuralWakeAddress>,
    ) -> Self {
        assert!(
            !wake_addresses.is_empty(),
            "world-model wait requires native wake evidence"
        );
        Self {
            condition: condition.into(),
            subject_key: Some(subject_key.into()),
            detail: detail.into(),
            wake_addresses,
        }
    }

    /// Build a declaration whose condition has no single subject.
    pub fn broad(
        condition: impl Into<String>,
        detail: impl Into<String>,
        wake_addresses: Vec<StructuralWakeAddress>,
    ) -> Self {
        assert!(
            !wake_addresses.is_empty(),
            "world-model wait requires native wake evidence"
        );
        Self {
            condition: condition.into(),
            subject_key: None,
            detail: detail.into(),
            wake_addresses,
        }
    }
}
