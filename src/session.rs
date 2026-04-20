//! Minimal session lifecycle domain.

pub mod contracts;
pub mod events;
pub mod policy;
pub mod runtime;
pub mod storage;

pub use contracts::{SessionKind, SessionMeta, SessionRecord};
pub use policy::{PrunePolicy, SessionStatus};
pub use runtime::SessionRuntime;
pub use storage::SessionStore;
