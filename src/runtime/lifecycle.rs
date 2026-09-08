//! Assignment-local activation generation and lifecycle authority.
//!
//! The lifecycle aggregate is the sole writer for assignment currentness and
//! admission. It consumes the exact prepared participant plan and structural
//! owner receipts while leaving semantic readiness, work, and safe-point
//! meaning with the participant owners.

mod contracts;
mod owner;
mod store;

pub use contracts::*;
pub use owner::*;
pub use store::ActivationLifecycleStore;
