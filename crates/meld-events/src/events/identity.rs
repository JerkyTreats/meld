//! Durable identity for one canonical event ledger.
//!
//! Owner: event authority.
//! Inputs: generated or product-expected UUID values.
//! Outputs: a stable identity carried by every authority capability and
//! sequence-bearing contract.
//! Does not own: product branch identity or process-host authentication.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Stable identity persisted with one event ledger.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LedgerIdentity(Uuid);

impl LedgerIdentity {
    /// Generates a new random ledger identity.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Wraps an existing UUID as a ledger identity.
    pub const fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Returns the underlying UUID value.
    pub const fn as_uuid(self) -> Uuid {
        self.0
    }

    pub(crate) fn encode(self) -> [u8; 16] {
        *self.0.as_bytes()
    }

    pub(crate) fn decode(raw: &[u8]) -> Result<Self, uuid::Error> {
        Uuid::from_slice(raw).map(Self)
    }
}

impl Default for LedgerIdentity {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for LedgerIdentity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl FromStr for LedgerIdentity {
    type Err = uuid::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(value).map(Self)
    }
}

impl From<Uuid> for LedgerIdentity {
    fn from(value: Uuid) -> Self {
        Self(value)
    }
}

impl From<LedgerIdentity> for Uuid {
    fn from(value: LedgerIdentity) -> Self {
        value.0
    }
}
