//! Prompt context domain: filesystem CAS artifacts and lineage contracts.

pub mod contracts;
pub mod orchestration;
pub mod storage;

pub use contracts::{
    PromptContextArtifactKind, PromptContextArtifactRef, PromptContextLineageContract,
    MAX_CONTEXT_ARTIFACT_BYTES, MAX_PROMPT_ARTIFACT_BYTES,
};
pub use orchestration::{
    persist_prompt_context_lineage, prepare_generated_lineage, PreparedPromptContextLineage,
    PromptContextLineageInput,
};
pub use storage::PromptContextArtifactStorage;
