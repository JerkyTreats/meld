use super::contracts::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdapterCompleteness {
    ExplicitComplete,
    ExplicitIncomplete,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencySecurityAdapterValueV1 {
    Inventory(DependencyInventorySnapshotV1),
    Advisories(AdvisoryKnowledgeSnapshotV1),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencySecurityAdapterResultV1 {
    pub assignment_id: String,
    pub package_receipt_id: String,
    pub activation_id: String,
    pub activation_generation_id: String,
    pub operation_key: String,
    pub attempt_id: String,
    pub adapter_implementation_ref: String,
    pub response_identity: String,
    pub observed_scope: Vec<meld_events::DomainObjectRef>,
    pub source_revisions: Vec<String>,
    pub completeness: AdapterCompleteness,
    pub value: DependencySecurityAdapterValueV1,
}

pub trait DependencySecurityTruthAdapter: Send + Sync {
    fn transport(
        &self,
        result: &DependencySecurityAdapterResultV1,
    ) -> Result<DependencySecurityAdapterResultV1, String>;
}

pub struct LocalFixtureAdapter;
impl DependencySecurityTruthAdapter for LocalFixtureAdapter {
    fn transport(
        &self,
        result: &DependencySecurityAdapterResultV1,
    ) -> Result<DependencySecurityAdapterResultV1, String> {
        Ok(result.clone())
    }
}
pub struct FakeSerializedAdapter;
impl DependencySecurityTruthAdapter for FakeSerializedAdapter {
    fn transport(
        &self,
        result: &DependencySecurityAdapterResultV1,
    ) -> Result<DependencySecurityAdapterResultV1, String> {
        serde_json::from_slice(&serde_json::to_vec(result).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn local_and_serialized_values_are_equivalent() {
        let value = DependencySecurityAdapterResultV1 {
            assignment_id: "a".into(),
            package_receipt_id: "p".into(),
            activation_id: "x".into(),
            activation_generation_id: "g".into(),
            operation_key: "o".into(),
            attempt_id: "t".into(),
            adapter_implementation_ref: "local".into(),
            response_identity: "r".into(),
            observed_scope: vec![],
            source_revisions: vec!["s".into()],
            completeness: AdapterCompleteness::ExplicitComplete,
            value: DependencySecurityAdapterValueV1::Advisories(AdvisoryKnowledgeSnapshotV1 {
                snapshot_id: "id".into(),
                source_id: "fixture".into(),
                source_revision: "s".into(),
                covered_ecosystem: PackageEcosystem::Cargo,
                covered_components: vec![],
                advisories: vec![],
                conflicts: vec![],
                completeness: AdvisoryCompleteness::CompleteForDeclaredCoverage,
                acquired_at: 1,
            }),
        };
        assert_eq!(
            LocalFixtureAdapter.transport(&value).unwrap().value,
            FakeSerializedAdapter.transport(&value).unwrap().value
        );
    }
}
