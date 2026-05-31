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
        return Err(ApiError::ConfigError(format!(
            "Workflow record contract '{}' invalid: gate_name must not be empty",
            RECORD_TYPE
        )));
    }
    Ok(())
}

fn validate_reasons(record: &ThreadTurnGateRecordV1) -> Result<(), ApiError> {
    if matches!(record.outcome, GateOutcome::Fail) && record.reasons.is_empty() {
        return Err(ApiError::ConfigError(format!(
            "Workflow record contract '{}' invalid: reasons must be non empty when outcome is fail",
            RECORD_TYPE
        )));
    }

    if record.reasons.iter().any(|reason| reason.trim().is_empty()) {
        return Err(ApiError::ConfigError(format!(
            "Workflow record contract '{}' invalid: reasons must not contain empty values",
            RECORD_TYPE
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    type GateRecordMutation = Box<dyn FnOnce(&mut ThreadTurnGateRecordV1)>;

    fn record() -> ThreadTurnGateRecordV1 {
        ThreadTurnGateRecordV1::new(
            "thread-1".to_string(),
            "turn-1".to_string(),
            "schema_gate".to_string(),
            GateOutcome::Fail,
            vec!["missing claims".to_string()],
            1,
        )
    }

    #[test]
    fn thread_turn_gate_record_round_trips_and_validates() {
        let record = record();
        let encoded = serde_json::to_string(&record).unwrap();
        let decoded = serde_json::from_str::<ThreadTurnGateRecordV1>(&encoded).unwrap();

        validate_thread_turn_gate_record_v1(&decoded).unwrap();
        assert_eq!(decoded, record);
    }

    #[test]
    fn thread_turn_gate_record_rejects_required_field_violations() {
        let cases: Vec<(&str, GateRecordMutation, &str)> = vec![
            (
                "bad schema",
                Box::new(|record| record.schema_version = 2),
                "schema_version",
            ),
            (
                "bad thread id",
                Box::new(|record| record.thread_id = "bad".to_string()),
                "thread_id",
            ),
            (
                "bad turn id",
                Box::new(|record| record.turn_id = "bad".to_string()),
                "turn_id",
            ),
            (
                "empty gate name",
                Box::new(|record| record.gate_name.clear()),
                "gate_name must not be empty",
            ),
            (
                "failure without reasons",
                Box::new(|record| record.reasons.clear()),
                "reasons must be non empty",
            ),
            (
                "empty reason",
                Box::new(|record| record.reasons[0].clear()),
                "reasons must not contain empty values",
            ),
            (
                "zero timestamp",
                Box::new(|record| record.evaluated_at_ms = 0),
                "evaluated_at_ms",
            ),
        ];

        for (case_name, mutate, expected) in cases {
            let mut record = record();
            mutate(&mut record);

            let error = match validate_thread_turn_gate_record_v1(&record) {
                Ok(()) => panic!("{case_name} should fail validation"),
                Err(error) => error,
            };

            assert!(
                error.to_string().contains(expected),
                "{case_name} expected error containing '{expected}', got '{error}'"
            );
        }
    }
}
