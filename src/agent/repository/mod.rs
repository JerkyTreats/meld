//! Agent repository port and adapters.

pub mod contract;
pub mod xdg;

pub use contract::{AgentRepository, StoredAgentConfig};
pub use xdg::XdgAgentRepository;
