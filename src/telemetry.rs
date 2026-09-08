//! Telemetry domain: events, sessions, sinks, and emission.

mod types;

pub mod contracts;
pub mod emission;
pub mod events;
pub mod facade;
pub mod sessions;
pub mod sinks;
pub mod summary;

pub use crate::session::{PrunePolicy, SessionStatus};
pub use contracts::{DomainObjectRef, EventRelation};
pub use events::{
    FrameMetadataValidationEventData, ProgressEvent, PromptContextLineageEventData,
    ProviderLifecycleEventData, SessionEndedData, SessionStartedData, SummaryEventData,
    WorkflowForceResetEventData, WorkflowTargetEventData, WorkflowTurnEventData,
};
pub use sessions::ProgressRuntime;
pub use types::{new_session_id, now_millis};
