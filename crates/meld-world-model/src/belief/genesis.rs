//! Stage 4 epistemic genesis fact contract for an unobserved scope.
//!
//! Owner: world model, jointly with events, which owns the record identity
//! format. The first durable meaning in an empty world is a declaration
//! that the selected scope is unobserved, appended through the canonical
//! append capability like every later observation. This is the
//! compatibility form of a curated observation goal; the Strategy admission
//! gate later replaces the producer, not the record shape.

use serde::{Deserialize, Serialize};

use crate::events::DomainObjectRef;

/// Event type of the stage 4 unobserved-scope genesis fact.
pub const UNOBSERVED_SCOPE_EVENT_TYPE: &str = "world_model.unobserved_scope";

/// Stream the epistemic genesis fact is appended to.
pub const EPISTEMIC_GENESIS_STREAM_ID: &str = "observation";

/// Payload declaring that a selected scope has never been observed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnobservedScopeDeclaration {
    /// Selected subtree subject the declaration covers.
    pub subject: DomainObjectRef,
    /// Provenance of the seeding actor, such as the initialization command.
    pub declared_by: String,
}

impl UnobservedScopeDeclaration {
    /// Scope key used in the frozen genesis record identity.
    ///
    /// One declaration per subject: re-seeding the same subject is
    /// idempotent by record identity.
    pub fn scope_key(&self) -> String {
        self.subject.index_key()
    }

    /// Frozen record identity for this declaration.
    pub fn record_id(&self) -> String {
        meld_events::epistemic_genesis_record_id(
            "world_model",
            EPISTEMIC_GENESIS_STREAM_ID,
            &self.scope_key(),
        )
    }
}
