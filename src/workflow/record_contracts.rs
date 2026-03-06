//! Canonical workflow record schema contracts.

pub mod id_validation;
pub mod prompt_link_record;
pub mod schema_version;
pub mod thread_turn_gate_record;

pub use prompt_link_record::{
    prompt_link_record_from_contract_v1, validate_prompt_link_record_references,
    validate_prompt_link_record_v1, PromptLinkRecordInputV1, PromptLinkRecordV1,
};
pub use schema_version::WORKFLOW_RECORD_SCHEMA_VERSION_V1;
pub use thread_turn_gate_record::{
    validate_thread_turn_gate_record_references, validate_thread_turn_gate_record_v1, GateOutcome,
    ThreadTurnGateRecordV1,
};
