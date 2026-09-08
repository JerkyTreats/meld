//! Serialized connections to package-owned semantic implementations.
//!
//! The host carries native products and granted callbacks. It does not interpret
//! owner policy, schedule semantic work inside the child, or create an Event ledger.

#[cfg(unix)]
pub mod connection;
pub mod contracts;
pub mod events;
pub mod provider;
#[cfg(unix)]
pub mod registration;
pub mod server;

#[cfg(unix)]
pub use connection::OwnerConnection;

#[cfg(all(test, unix))]
mod tests;
pub use contracts::*;
