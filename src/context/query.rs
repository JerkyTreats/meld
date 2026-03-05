//! Context query: view policy, composition, and query service.
//! Single owner of context read behavior; api delegates to this module.

pub mod composition;
pub mod get;
pub mod service;
pub mod view;
pub mod view_policy;

pub use composition::{compose_frames, CompositionPolicy, CompositionSource};
pub use get::get_node_for_cli;
pub use service::get_node as get_node_query;
pub use view::{ContextView, ContextViewBuilder, NodeContext};
pub use view_policy::{get_context_view, FrameFilter, OrderingPolicy, ViewPolicy};
