//! Subscription compatibility surface for event bus callers.
//!
//! Owner: event ingress.
//! Inputs: callers importing the historical subscription module path.
//! Outputs: the canonical [`EventBus`] producer handle.
//! Does not own: this module does not define independent subscription state.

pub use crate::events::ingress::EventBus;
