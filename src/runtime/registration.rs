//! Runtime registration identity and lifecycle projection contracts.
//!
//! Owner: root runtime. A registration is a required binding derived from
//! the selected stewardship expression and physical binding — distinct
//! from the descriptor catalog, which is an internal inventory with no
//! product cardinality meaning. Catalog-only descriptors receive no
//! registration and no lifecycle state; unavailable status is reserved for
//! a required binding that cannot be resolved.
//!
//! Registration-set composition is a public surface: the
//! stewardship-derived set is one producer, and harness or proof callers
//! may supply an explicit set to compose any actor subset. Store and port
//! opening is scoped to the composed set.
//!
//! This module carries no implementation. The supervisor-truth and actor
//! binding workstreams bind classification and factories.

use serde::{Deserialize, Serialize};

use crate::runtime::assembly::RuntimeResource;

/// What kind of runtime participant a registration binds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RegistrationKind {
    /// A bounded actor the supervisor ticks. Holds a lease and reports.
    ActiveActor,
    /// A passive capability other actors call. Never leased, never ticked,
    /// never assigned actor health.
    PassiveService,
}

/// One required runtime binding derived from composition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeRegistration {
    /// Root-local registration identity, used in lifecycle projection and
    /// diagnostics only. Durable correlation prefers existing session,
    /// stream, subject, event, and actor identities.
    pub registration_id: String,
    /// Internal runtime role this registration binds.
    pub runtime_id: String,
    /// Active actor or passive service.
    pub kind: RegistrationKind,
    /// Resources the binding requires before it can resolve.
    pub required_resources: Vec<RuntimeResource>,
}

/// An explicit set of registrations composing one runtime assembly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistrationSet {
    /// Registrations in stable order.
    pub registrations: Vec<RuntimeRegistration>,
}

/// Truthful lifecycle projection for one registration.
///
/// Projected by root from domain-owned reports; domains never depend on
/// this shape. A missing report is never projected as healthy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RegistrationLifecycle {
    /// A required resource or factory cannot be resolved. Never reported
    /// as healthy.
    UnresolvedRequiredBinding,
    /// The binding resolved and the participant is starting.
    Starting,
    /// The actor's last bounded tick committed work.
    ActiveWorking,
    /// The actor's last bounded tick truthfully found no eligible work.
    ActiveIdle,
    /// The actor's last bounded tick reported fatal errors.
    Unhealthy,
    /// The participant stopped and holds no lease.
    Stopped,
}
