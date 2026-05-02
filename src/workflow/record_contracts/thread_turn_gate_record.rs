pub use meld_execution::workflow::record_contracts::thread_turn_gate_record::{
    GateOutcome, ThreadTurnGateRecordV1,
};

pub fn validate_thread_turn_gate_record_v1(
    record: &ThreadTurnGateRecordV1,
) -> Result<(), crate::error::ApiError> {
    meld_execution::workflow::record_contracts::thread_turn_gate_record::validate_thread_turn_gate_record_v1(record)
        .map_err(Into::into)
}

pub fn validate_thread_turn_gate_record_references(
    record: &ThreadTurnGateRecordV1,
) -> Result<(), crate::error::ApiError> {
    meld_execution::workflow::record_contracts::thread_turn_gate_record::validate_thread_turn_gate_record_references(record)
        .map_err(Into::into)
}
