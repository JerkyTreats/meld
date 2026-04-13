use crate::error::StorageError;
use crate::telemetry::sinks::store::ProgressStore;

pub struct WorldStateReducer;

impl WorldStateReducer {
    pub fn replay_from_spine(_store: &ProgressStore, _after_seq: u64) -> Result<Self, StorageError> {
        Ok(Self)
    }
}
