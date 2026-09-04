//! Durable validation and direct lowering of complete Agent-authorized Tasks.
//!
//! Execution consumes one already-selected Task body. This domain validates
//! exact producer lineage, Capability contracts, bindings, inputs, and authority
//! fences before writing one durable admission decision. Admitted Tasks lower
//! directly into one attributed Task Network region without Method search or
//! semantic repair.

mod admission;
mod lowering;

pub(crate) use admission::ValidatedTaskAdmissionWrite;
pub use admission::{
    ExecutionTask, TaskAdmissionApi, TaskAdmissionDecision, TaskAdmissionLineage,
    TaskAdmissionRecord, TaskAdmissionRequest,
};
pub(crate) use lowering::ValidatedTaskRegionWrite;
pub use lowering::{
    TaskAdmissionLowerer, TaskAdmissionLoweringPlan, TaskAdmissionRuntimeActor,
    TaskAdmissionRuntimeItem, TaskAdmissionRuntimeReport, TaskAdmissionRuntimeRequest,
};
