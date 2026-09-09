//! Generic persistent-domain package attachment.
//!
//! This domain owns structural package identity, routing, receipts, and
//! exact resolution. Semantic bodies remain opaque and are installed and
//! verified only through handlers published by their owning domains.

pub mod activation;
pub mod contracts;
pub mod error;
pub mod handler;
pub mod package;
pub mod product;
pub mod receipt;
pub mod registry;
pub mod resolution;
pub mod router;
pub mod topology;
pub use topology::*;

pub use activation::*;
pub use contracts::*;
pub use error::*;
pub use handler::*;
pub use package::*;
pub use product::*;
pub use receipt::*;
pub use registry::*;
pub use resolution::*;
pub use router::*;
