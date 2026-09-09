//! Package-owned semantics hosted over native Meld contracts.

pub use meld::{
    capability, code_change, config, context, error, execution, init, provider, runtime, task,
    theory,
};
pub mod dependency_security;
