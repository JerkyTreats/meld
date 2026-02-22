//! Watch runtime: events, editor bridge, and daemon.

mod editor_bridge;
mod events;
mod runtime;

pub use editor_bridge::EditorHooks;
pub use events::{ChangeEvent, WatchConfig};
pub use runtime::WatchDaemon;
