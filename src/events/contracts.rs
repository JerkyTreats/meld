use serde::{Deserialize, Serialize};

use crate::error::StorageError;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DomainObjectRef {
    pub domain_id: String,
    pub object_kind: String,
    pub object_id: String,
}

impl DomainObjectRef {
    pub fn new(
        domain_id: impl Into<String>,
        object_kind: impl Into<String>,
        object_id: impl Into<String>,
    ) -> Result<Self, StorageError> {
        let object_ref = Self {
            domain_id: domain_id.into(),
            object_kind: object_kind.into(),
            object_id: object_id.into(),
        };
        object_ref.validate()?;
        Ok(object_ref)
    }

    pub fn validate(&self) -> Result<(), StorageError> {
        if self.domain_id.trim().is_empty() {
            return Err(StorageError::InvalidPath(
                "domain object ref domain_id must be non-empty".to_string(),
            ));
        }
        if self.object_kind.trim().is_empty() {
            return Err(StorageError::InvalidPath(
                "domain object ref object_kind must be non-empty".to_string(),
            ));
        }
        if self.object_id.trim().is_empty() {
            return Err(StorageError::InvalidPath(
                "domain object ref object_id must be non-empty".to_string(),
            ));
        }
        Ok(())
    }

    pub fn index_key(&self) -> String {
        format!(
            "{}::{}::{}",
            self.domain_id, self.object_kind, self.object_id
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventRelation {
    pub relation_type: String,
    pub src: DomainObjectRef,
    pub dst: DomainObjectRef,
}

impl EventRelation {
    pub fn new(
        relation_type: impl Into<String>,
        src: DomainObjectRef,
        dst: DomainObjectRef,
    ) -> Result<Self, StorageError> {
        let relation = Self {
            relation_type: relation_type.into(),
            src,
            dst,
        };
        relation.validate()?;
        Ok(relation)
    }

    pub fn validate(&self) -> Result<(), StorageError> {
        if self.relation_type.trim().is_empty() {
            return Err(StorageError::InvalidPath(
                "event relation relation_type must be non-empty".to_string(),
            ));
        }
        self.src.validate()?;
        self.dst.validate()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domain_object_ref_round_trips() {
        let object_ref = DomainObjectRef::new("execution", "task_run", "run_a").unwrap();
        let serialized = serde_json::to_string(&object_ref).unwrap();
        let parsed: DomainObjectRef = serde_json::from_str(&serialized).unwrap();
        assert_eq!(parsed.domain_id, "execution");
        assert_eq!(parsed.object_kind, "task_run");
        assert_eq!(parsed.object_id, "run_a");
        assert_eq!(parsed.index_key(), "execution::task_run::run_a");
    }
}
