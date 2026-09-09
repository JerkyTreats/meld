//! Package-owned semantics hosted over native Meld contracts.

pub use meld::{
    capability, config, context, error, execution, init, provider, runtime, task, theory,
};
pub mod docs;

#[cfg(test)]
#[path = "../../test_support.rs"]
mod test_support;
