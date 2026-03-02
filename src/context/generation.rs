//! Context generation: plan and executor for running generation plans against the queue.
//! Behavior-named; executor runs the plan; queue and provider stay in their domains.

pub mod contracts;
pub mod executor;
pub mod metadata_construction;
pub mod orchestration;
pub mod plan;
pub mod prompt_collection;
pub mod provider_execution;
pub mod run;

pub use executor::{GenerationExecutor, QueueSubmitter};
pub use plan::{
    FailurePolicy, GenerationErrorDetail, GenerationItem, GenerationNodeType, GenerationPlan,
    GenerationResult, LevelSummary, PlanPriority,
};
pub use run::{run_generate, GenerateRequest};
