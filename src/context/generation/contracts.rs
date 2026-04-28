use crate::metadata::frame_types::FrameMetadata;
pub use meld_execution::generation::{
    GeneratedFrameMetadataInput, GenerationOrchestrationRequest, PromptAssemblyOutput,
};

pub type GeneratedMetadataBuilder =
    dyn Fn(&GeneratedFrameMetadataInput) -> FrameMetadata + Send + Sync;
