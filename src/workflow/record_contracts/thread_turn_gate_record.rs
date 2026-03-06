//! Canonical thread turn gate record contract.

use crate::error::ApiError;
use crate::workflow::record_contracts::id_validation::{
    validate_prefixed_id, validate_timestamp_ms,
};
use crate::workflow::record_contracts::schema_version::{
    validate_schema_version, WORKFLOW_RECORD_SCHEMA_VERSION_V1,
};
use serde::{Deserialize, Serialize};

const RECORD_TYPE: &str = "thread_turn_gate";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GateOutcome {
    Pass,
    Fail,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ThreadTurnGateRecordV1 {
    pub schema_version: u32,
    pub thread_id: String,
    pub turn_id: String,
    pub gate_name: String,
    pub outcome: GateOutcome,
    pub reasons: Vec<String>,
    pub evaluated_at_ms: u64,
}

impl ThreadTurnGateRecordV1 {
    pub fn new(
        thread_id: String,
        turn_id: String,
        gate_name: String,
        outcome: GateOutcome,
        reasons: Vec<String>,
        evaluated_at_ms: u64,
    ) -> Self {
        Self {
            schema_version: WORKFLOW_RECORD_SCHEMA_VERSION_V1,
            thread_id,
            turn_id,
            gate_name,
            outcome,
            reasons,
            evaluated_at_ms,
        }
    }
}

pub fn validate_thread_turn_gate_record_v1(
    record: &ThreadTurnGateRecordV1,
) -> Result<(), ApiError> {
    validate_schema_version(RECORD_TYPE, record.schema_version)?;
    validate_prefixed_id(RECORD_TYPE, "thread_id", &record.thread_id, "thread-")?;
    validate_prefixed_id(RECORD_TYPE, "turn_id", &record.turn_id, "turn-")?;
    validate_timestamp_ms(RECORD_TYPE, "evaluated_at_ms", record.evaluated_at_ms)?;
    validate_gate_name(record)?;
    validate_reasons(record)?;
    Ok(())
}

pub fn validate_thread_turn_gate_record_references(
    record: &ThreadTurnGateRecordV1,
) -> Result<(), ApiError> {
    validate_thread_turn_gate_record_v1(record)
}

fn validate_gate_name(record: &ThreadTurnGateRecordV1) -> Result<(), ApiError> {
    if record.gate_name.trim().is_empty() {
        return Err(ApiError::WorkflowRecordContractInvalid {
            record_type: RECORD_TYPE.to_string(),
            reason: "gate_name must not be empty".to_string(),
        });
    }
    Ok(())
}

fn validate_reasons(record: &ThreadTurnGateRecordV1) -> Result<(), ApiError> {
    if matches!(record.outcome, GateOutcome::Fail) && record.reasons.is_empty() {
        return Err(ApiError::WorkflowRecordContractInvalid {
            record_type: RECORD_TYPE.to_string(),
            reason: "reasons must be non empty when outcome is fail".to_string(),
        });
    }

    if record.reasons.iter().any(|reason| reason.trim().is_empty()) {
        return Err(ApiError::WorkflowRecordContractInvalid {
            record_type: RECORD_TYPE.to_string(),
            reason: "reasons must not contain empty values".to_string(),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_record() -> ThreadTurnGateRecordV1 {
        ThreadTurnGateRecordV1::new(
            "thread-a".to_string(),
            "turn-1".to_string(),
            "schema_required_fields".to_string(),
            GateOutcome::Pass,
            vec![],
            1,
        )
    }

    #[test]
    fn validate_accepts_pass_record() {
        validate_thread_turn_gate_record_v1(&valid_record()).unwrap();
    }

    #[test]
    fn validate_rejects_fail_without_reasons() {
        let mut record = valid_record();
        record.outcome = GateOutcome::Fail;
        record.reasons.clear();
        let err = validate_thread_turn_gate_record_v1(&record).unwrap_err();
        assert!(matches!(
            err,
            ApiError::WorkflowRecordContractInvalid { .. }
        ));
    }

    #[test]
    fn validate_rejects_invalid_turn_id() {
        let mut record = valid_record();
        record.turn_id = "bad".to_string();
        let err = validate_thread_turn_gate_record_v1(&record).unwrap_err();
        assert!(matches!(
            err,
            ApiError::WorkflowRecordContractInvalid { .. }
        ));
    }
}
